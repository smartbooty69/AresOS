//! Scheduler signaling primitives for preemption groundwork.

use super::context::{switch_context, CpuContext, RunnableContext};
use alloc::{collections::VecDeque, vec::Vec};
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

/// Number of timer ticks per scheduling quantum.
pub const SCHED_QUANTUM_TICKS: u64 = 5;

static NEED_RESCHEDULE: AtomicBool = AtomicBool::new(false);
static TIMER_TICKS: AtomicU64 = AtomicU64::new(0);
static RESCHEDULE_REQUESTS: AtomicU64 = AtomicU64::new(0);
static RESCHEDULE_POINTS: AtomicU64 = AtomicU64::new(0);
static DEMO_CONTEXT_TASKS_SPAWNED: AtomicBool = AtomicBool::new(false);
static DEMO_A_COUNT: AtomicU64 = AtomicU64::new(0);
static DEMO_B_COUNT: AtomicU64 = AtomicU64::new(0);

lazy_static! {
    static ref CONTEXT_SCHEDULER: Mutex<ContextScheduler> = Mutex::new(ContextScheduler::new());
}

struct ContextTask {
    name: &'static str,
    runnable: RunnableContext,
}

struct ContextScheduler {
    tasks: Vec<ContextTask>,
    ready_queue: VecDeque<usize>,
    current: Option<usize>,
    switches: u64,
    switch_enabled: bool,
}

impl ContextScheduler {
    const fn new() -> Self {
        Self {
            tasks: Vec::new(),
            ready_queue: VecDeque::new(),
            current: None,
            switches: 0,
            switch_enabled: false,
        }
    }

    fn spawn(&mut self, name: &'static str, entry: extern "C" fn() -> !) -> usize {
        let id = self.tasks.len();
        self.tasks.push(ContextTask {
            name,
            runnable: RunnableContext::new(entry),
        });
        self.ready_queue.push_back(id);
        id
    }

    fn next_pair(&mut self) -> Option<(usize, usize)> {
        if self.tasks.len() < 2 {
            return None;
        }

        if self.current.is_none() {
            self.current = self.ready_queue.pop_front();
        }

        let current = self.current?;
        let next = self.ready_queue.pop_front()?;
        self.ready_queue.push_back(current);
        self.current = Some(next);
        Some((current, next))
    }

    fn try_switch(&mut self) -> bool {
        let (current, next) = match self.next_pair() {
            Some(pair) => pair,
            None => return false,
        };

        if current == next {
            return false;
        }

        self.switches = self.switches.saturating_add(1);
        !self.switch_enabled
    }

    fn prepare_live_switch(&mut self) -> Option<(*mut CpuContext, *const CpuContext)> {
        let (current, next) = self.next_pair()?;
        if current == next {
            return None;
        }

        let (current_ctx, next_ctx) = if current < next {
            let (left, right) = self.tasks.split_at_mut(next);
            (
                &mut left[current].runnable.context,
                &right[0].runnable.context,
            )
        } else {
            let (left, right) = self.tasks.split_at_mut(current);
            (
                &mut right[0].runnable.context,
                &left[next].runnable.context,
            )
        };

        self.switches = self.switches.saturating_add(1);
        Some((current_ctx as *mut CpuContext, next_ctx as *const CpuContext))
    }

    fn first_context(&mut self) -> Option<*const CpuContext> {
        if self.tasks.is_empty() {
            return None;
        }

        if self.current.is_none() {
            self.current = self.ready_queue.pop_front();
        }

        let current = self.current?;
        Some(&self.tasks[current].runnable.context as *const CpuContext)
    }

    fn demo_counts(&self) -> (u64, u64) {
        (
            DEMO_A_COUNT.load(Ordering::Relaxed),
            DEMO_B_COUNT.load(Ordering::Relaxed),
        )
    }

    fn context_task_count(&self) -> usize {
        self.tasks.len()
    }

    fn context_switch_count(&self) -> u64 {
        self.switches
    }

    fn set_switch_enabled(&mut self, enabled: bool) {
        self.switch_enabled = enabled;
    }

    fn switch_enabled(&self) -> bool {
        self.switch_enabled
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ContextSchedulerStats {
    pub tasks: usize,
    pub switches: u64,
    pub switching_enabled: bool,
    pub demo_a_count: u64,
    pub demo_b_count: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct SchedulerStats {
    pub timer_ticks: u64,
    pub reschedule_requests: u64,
    pub reschedule_points: u64,
    pub pending: bool,
    pub context_tasks: usize,
    pub context_switches: u64,
    pub context_switching_enabled: bool,
}

/// Called from the timer interrupt handler.
pub fn on_timer_tick() {
    let ticks = TIMER_TICKS.fetch_add(1, Ordering::Relaxed) + 1;
    if ticks % SCHED_QUANTUM_TICKS == 0 {
        NEED_RESCHEDULE.store(true, Ordering::Relaxed);
        RESCHEDULE_REQUESTS.fetch_add(1, Ordering::Relaxed);
    }
}

/// Consume a pending reschedule request.
pub fn take_reschedule_request() -> bool {
    NEED_RESCHEDULE.swap(false, Ordering::Relaxed)
}

/// Mark that execution reached a scheduler reschedule point.
pub fn record_reschedule_point() {
    RESCHEDULE_POINTS.fetch_add(1, Ordering::Relaxed);
}

pub fn spawn_context_task(name: &'static str, entry: extern "C" fn() -> !) -> usize {
    CONTEXT_SCHEDULER.lock().spawn(name, entry)
}

pub fn set_context_switching_enabled(enabled: bool) {
    CONTEXT_SCHEDULER.lock().set_switch_enabled(enabled);
}

pub fn try_context_reschedule() -> bool {
    let maybe_live_switch = {
        let mut scheduler = CONTEXT_SCHEDULER.lock();
        if scheduler.switch_enabled() {
            scheduler.prepare_live_switch()
        } else {
            return scheduler.try_switch();
        }
    };

    let (current, next) = match maybe_live_switch {
        Some(pair) => pair,
        None => return false,
    };

    unsafe {
        switch_context(&mut *current, &*next);
    }
    true
}

pub fn yield_now() {
    let _ = try_context_reschedule();
}

pub fn run_context_lab() -> ! {
    let mut boot_context = CpuContext::capture();
    let first = {
        let mut scheduler = CONTEXT_SCHEDULER.lock();
        scheduler
            .first_context()
            .expect("context lab requires at least one context task")
    };

    unsafe {
        switch_context(&mut boot_context, &*first);
    }

    panic!("context lab returned to boot context unexpectedly");
}

pub fn context_stats() -> ContextSchedulerStats {
    let scheduler = CONTEXT_SCHEDULER.lock();
    let (demo_a_count, demo_b_count) = scheduler.demo_counts();
    ContextSchedulerStats {
        tasks: scheduler.context_task_count(),
        switches: scheduler.context_switch_count(),
        switching_enabled: scheduler.switch_enabled(),
        demo_a_count,
        demo_b_count,
    }
}

pub fn context_task_names() -> Vec<&'static str> {
    CONTEXT_SCHEDULER
        .lock()
        .tasks
        .iter()
        .map(|task| task.name)
        .collect()
}

extern "C" fn demo_context_task_a() -> ! {
    loop {
        let count = DEMO_A_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if count % 250_000 == 0 {
            let b = DEMO_B_COUNT.load(Ordering::Relaxed);
            crate::println!("ContextLab A={}, B={}", count, b);
        }
        yield_now();
    }
}

extern "C" fn demo_context_task_b() -> ! {
    loop {
        let count = DEMO_B_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if count % 250_000 == 0 {
            let a = DEMO_A_COUNT.load(Ordering::Relaxed);
            crate::println!("ContextLab A={}, B={}", a, count);
        }
        yield_now();
    }
}

pub fn spawn_demo_context_tasks() {
    if DEMO_CONTEXT_TASKS_SPAWNED
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        spawn_context_task("ctx-demo-a", demo_context_task_a);
        spawn_context_task("ctx-demo-b", demo_context_task_b);
    }
}

pub fn stats() -> SchedulerStats {
    let context = context_stats();
    SchedulerStats {
        timer_ticks: TIMER_TICKS.load(Ordering::Relaxed),
        reschedule_requests: RESCHEDULE_REQUESTS.load(Ordering::Relaxed),
        reschedule_points: RESCHEDULE_POINTS.load(Ordering::Relaxed),
        pending: NEED_RESCHEDULE.load(Ordering::Relaxed),
        context_tasks: context.tasks,
        context_switches: context.switches,
        context_switching_enabled: context.switching_enabled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn scheduler_tick_requests_reschedule() {
        let before = stats();
        for _ in 0..SCHED_QUANTUM_TICKS {
            on_timer_tick();
        }
        let after = stats();
        assert!(after.timer_ticks >= before.timer_ticks + SCHED_QUANTUM_TICKS);
        assert!(after.reschedule_requests >= before.reschedule_requests + 1);
    }

    #[test_case]
    fn take_request_clears_pending_flag() {
        NEED_RESCHEDULE.store(true, Ordering::Relaxed);
        assert!(take_reschedule_request());
        assert!(!stats().pending);
    }

    #[test_case]
    fn context_task_names_initially_empty() {
        assert!(context_task_names().is_empty());
    }
}
