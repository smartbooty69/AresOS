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
const WATCHDOG_SLEEP_THRESHOLD_TICKS: u64 = PIT_HZ * 30;

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

/// Periodically print scheduler counters.
pub async fn log_scheduler_stats() {
    loop {
        sleep(Duration::from_secs(5)).await;
        let stats = crate::task::executor::system_stats();
        println!(
            "Scheduler: active={}, sleeping={}, ready={}, completed={}",
            stats.active_tasks,
            stats.sleeping_tasks,
            stats.ready_queue_depth,
            stats.completed_tasks
        );
    }
}

/// Periodically print per-task runtime snapshots.
pub async fn log_task_registry() {
    loop {
        sleep(Duration::from_secs(10)).await;
        let snapshots = crate::task::executor::system_task_snapshots();
        if snapshots.is_empty() {
            println!("Tasks: no active tasks in registry");
            continue;
        }

        println!("Task registry ({} active):", snapshots.len());
        for task in snapshots {
            println!(
                "  #{} {} state={:?} age={} ticks state_age={} ticks",
                task.id, task.name, task.state, task.age_ticks, task.state_age_ticks
            );
        }
    }
}

pub async fn task_watchdog() {
    loop {
        sleep(Duration::from_secs(5)).await;
        let snapshots = crate::task::executor::system_task_snapshots();
        for task in snapshots {
            if task.state == crate::task::TaskState::Sleeping
                && task.state_age_ticks >= WATCHDOG_SLEEP_THRESHOLD_TICKS
            {
                println!(
                    "Watchdog: task #{} ({}) sleeping for {} ticks",
                    task.id, task.name, task.state_age_ticks
                );
            }
        }
    }
}

pub async fn log_scheduler_groundwork() {
    loop {
        sleep(Duration::from_secs(10)).await;
        let stats = crate::task::scheduler::stats();
        let context = crate::task::scheduler::context_stats();
        println!(
            "Preemptive-groundwork: ticks={}, requests={}, points={}, pending={}, ctx_tasks={}, ctx_switches={}, ctx_live_switch={}, demo_a={}, demo_b={}, misses={}, watchdog_trips={}, irq_req={}, irq_ckpt={}, irq_rip={:#x}, irq_rsp={:#x}",
            stats.timer_ticks,
            stats.reschedule_requests,
            stats.reschedule_points,
            stats.pending,
            stats.context_tasks,
            stats.context_switches,
            stats.context_switching_enabled,
            context.demo_a_count,
            context.demo_b_count,
            context.preempt_misses,
            context.watchdog_trips,
            context.irq_preempt_requests,
            context.irq_preempt_checkpoints,
            context.last_irq_rip,
            context.last_irq_rsp
        );
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
