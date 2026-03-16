//! Asynchronous timer primitives driven by the PIT tick interrupt.
//!
//! This module lets tasks sleep for a duration without busy waiting.

use crate::{performance::metrics::TICK_COUNTER, println};
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::Ordering,
    task::{Context, Poll},
    time::Duration,
};
use futures_util::task::AtomicWaker;

/// PIT frequency used by the kernel timer IRQ handler.
pub const PIT_HZ: u64 = 100;

const TICK_MILLIS: u64 = 1_000 / PIT_HZ;

static TIMER_WAKER: AtomicWaker = AtomicWaker::new();

/// Called by the timer IRQ handler to wake timer futures.
pub(crate) fn notify_tick() {
    TIMER_WAKER.wake();
}

/// Future that resolves once a target tick count is reached.
pub struct Sleep {
    wake_tick: u64,
}

impl Sleep {
    fn new(duration: Duration) -> Self {
        let now = TICK_COUNTER.load(Ordering::Relaxed);
        let ticks = duration_to_ticks(duration);
        Self {
            wake_tick: now.saturating_add(ticks),
        }
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if TICK_COUNTER.load(Ordering::Relaxed) >= self.wake_tick {
            return Poll::Ready(());
        }

        TIMER_WAKER.register(cx.waker());

        if TICK_COUNTER.load(Ordering::Relaxed) >= self.wake_tick {
            TIMER_WAKER.take();
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

/// Return a future that completes after `duration` has elapsed.
pub fn sleep(duration: Duration) -> Sleep {
    Sleep::new(duration)
}

/// Return a future that completes after `ticks` timer ticks.
pub fn sleep_ticks(ticks: u64) -> Sleep {
    let now = TICK_COUNTER.load(Ordering::Relaxed);
    Sleep {
        wake_tick: now.saturating_add(ticks),
    }
}

/// Periodically print uptime to the console.
pub async fn log_uptime() {
    loop {
        sleep(Duration::from_secs(5)).await;
        let ticks = TICK_COUNTER.load(Ordering::Relaxed);
        println!("Uptime: {}s ({} ticks)", ticks / PIT_HZ, ticks);
    }
}

fn duration_to_ticks(duration: Duration) -> u64 {
    let millis = duration.as_millis();
    if millis == 0 {
        return 0;
    }

    let ticks = (millis + u128::from(TICK_MILLIS) - 1) / u128::from(TICK_MILLIS);
    ticks.min(u128::from(u64::MAX)) as u64
}
