//! Asynchronous keyboard input handler.
//!
//! The keyboard IRQ handler writes raw scancodes into a lock-free
//! `ArrayQueue`.  The async `print_keypresses` future drains that queue,
//! translates scancodes to key events with the `pc-keyboard` crate, and
//! prints printable characters to the VGA console.

use alloc::{string::String, vec::Vec};
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
use lazy_static::lazy_static;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;

/// Maximum number of unprocessed scancodes to buffer.
const SCANCODE_QUEUE_SIZE: usize = 100;

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

lazy_static! {
    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
        Mutex::new(Keyboard::new(
            layouts::Us104Key,
            ScancodeSet1,
            HandleControl::Ignore,
        ));
    static ref CONSOLE_LINE: Mutex<String> = Mutex::new(String::new());
}

/// Initialise the keyboard scancode queue.
///
/// Safe to call multiple times; only the first call initialises storage.
pub fn init_scancode_queue() {
    let _ = SCANCODE_QUEUE.try_init_once(|| ArrayQueue::new(SCANCODE_QUEUE_SIZE));
}

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
        init_scancode_queue();
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

    while let Some(scancode) = scancodes.next().await {
        process_scancode(scancode, true);
    }
    println!("Keyboard stream ended.");
}

/// Poll queued scancodes and process keyboard-console commands.
///
/// This is used by preemption-mode context tasks where the async keyboard
/// task is not running.
pub fn poll_console_commands() {
    while let Some(scancode) = try_pop_scancode() {
        process_scancode(scancode, true);
    }
}

fn try_pop_scancode() -> Option<u8> {
    SCANCODE_QUEUE.try_get().ok().and_then(|queue| queue.pop())
}

fn process_scancode(scancode: u8, echo: bool) {
    let mut keyboard = KEYBOARD.lock();
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => handle_console_char(character, echo),
                DecodedKey::RawKey(key) => {
                    if echo {
                        print!("{:?}", key);
                    }
                }
            }
        }
    }
}

fn handle_console_char(character: char, echo: bool) {
    match character {
        '\n' | '\r' => {
            if echo {
                println!("");
            }
            let command = {
                let mut line = CONSOLE_LINE.lock();
                let command = line.trim().to_string();
                line.clear();
                command
            };

            if command.is_empty() {
                return;
            }

            execute_console_command(&command);
        }
        '\u{8}' | '\u{7f}' => {
            let mut line = CONSOLE_LINE.lock();
            if !line.is_empty() {
                line.pop();
                if echo {
                    print!("\u{8} \u{8}");
                }
            }
        }
        c => {
            let mut line = CONSOLE_LINE.lock();
            line.push(c);
            if echo {
                print!("{}", c);
            }
        }
    }
}

fn execute_console_command(command: &str) {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    match parts.as_slice() {
        ["help"] => {
            println!("Console commands:");
            println!("  help");
            println!("  sched show");
            println!("  sched quantum <ticks>");
            println!("  sched fairness <ticks>");
            println!("  sched maxproc <count>");
        }
        ["sched", "show"] => {
            let quantum = crate::task::scheduler::scheduler_quantum_ticks();
            let fairness = crate::task::scheduler::fairness_check_interval_ticks();
            let max_proc = crate::task::process::max_processes();
            println!(
                "Scheduler config: quantum_ticks={}, fairness_check_ticks={}, max_processes={}",
                quantum, fairness, max_proc
            );
            crate::serial_println!(
                "Scheduler config: quantum_ticks={}, fairness_check_ticks={}, max_processes={}",
                quantum, fairness, max_proc
            );
        }
        ["sched", "quantum", value] => match value.parse::<u64>() {
            Ok(ticks) => {
                crate::task::scheduler::set_scheduler_quantum_ticks(ticks);
                println!(
                    "Updated scheduler quantum to {} ticks",
                    crate::task::scheduler::scheduler_quantum_ticks()
                );
            }
            Err(_) => println!("Invalid quantum value: {}", value),
        },
        ["sched", "fairness", value] => match value.parse::<u64>() {
            Ok(ticks) => {
                crate::task::scheduler::set_fairness_check_interval_ticks(ticks);
                println!(
                    "Updated fairness check interval to {} ticks",
                    crate::task::scheduler::fairness_check_interval_ticks()
                );
            }
            Err(_) => println!("Invalid fairness value: {}", value),
        },
        ["sched", "maxproc", value] => match value.parse::<usize>() {
            Ok(max_proc) => {
                crate::task::process::set_max_processes(max_proc);
                println!(
                    "Updated max processes to {}",
                    crate::task::process::max_processes()
                );
            }
            Err(_) => println!("Invalid maxproc value: {}", value),
        },
        _ => {
            println!("Unknown command: {}", command);
            println!("Type 'help' for available commands");
        }
    }
}

#[cfg(test)]
mod tests {
    #[test_case]
    fn scheduler_console_updates_apply() {
        crate::task::scheduler::set_scheduler_quantum_ticks(5);
        crate::task::scheduler::set_fairness_check_interval_ticks(10);
        crate::task::process::set_max_processes(256);

        super::execute_console_command("sched quantum 7");
        super::execute_console_command("sched fairness 13");
        super::execute_console_command("sched maxproc 321");

        assert_eq!(crate::task::scheduler::scheduler_quantum_ticks(), 7);
        assert_eq!(crate::task::scheduler::fairness_check_interval_ticks(), 13);
        assert_eq!(crate::task::process::max_processes(), 321);
    }
}
