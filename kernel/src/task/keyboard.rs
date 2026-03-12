//! Asynchronous keyboard input handler.
//!
//! The keyboard IRQ handler writes raw scancodes into a lock-free
//! `ArrayQueue`.  The async `print_keypresses` future drains that queue,
//! translates scancodes to key events with the `pc-keyboard` crate, and
//! prints printable characters to the VGA console.

use crate::{print, println};
use conquer_once::spin::OnceCell;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::{
    stream::{Stream, StreamExt},
    task::AtomicWaker,
};
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

/// Maximum number of unprocessed scancodes to buffer.
const SCANCODE_QUEUE_SIZE: usize = 100;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

/// Called by the keyboard IRQ handler to push a raw scancode into the queue.
///
/// This function is designed to be safe to call from an interrupt context:
/// it never blocks and never allocates.
pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if queue.push(scancode).is_err() {
            // Drop the scancode; the queue is full.
        } else {
            WAKER.wake();
        }
    } else {
        // Queue not yet initialised (very early boot); drop the scancode.
    }
}

/// An async `Stream` that yields raw scancodes from the IRQ handler.
pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(SCANCODE_QUEUE_SIZE))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Default for ScancodeStream {
    fn default() -> Self {
        Self::new()
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue not initialised");

        // Fast path: a scancode is already in the queue.
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        // No scancode yet – register the waker so we are polled again when
        // the IRQ handler pushes a new scancode.
        WAKER.register(cx.waker());

        // Re-check after registering to avoid a TOCTOU race.
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}

/// A future that reads keypresses from the keyboard and prints them.
///
/// This is the main keyboard task; spawn it with the executor at boot.
pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(
        layouts::Us104Key,
        ScancodeSet1,
        HandleControl::Ignore,
    );

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => print!("{}", character),
                    DecodedKey::RawKey(key) => print!("{:?}", key),
                }
            }
        }
    }
    println!("Keyboard stream ended.");
}
