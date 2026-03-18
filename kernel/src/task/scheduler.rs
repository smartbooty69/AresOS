//! Scheduler signaling primitives for preemption groundwork.

use super::context::{switch_context, CpuContext, RunnableContext};
use alloc::{collections::VecDeque, vec::Vec};
use crate::performance::process_metrics::{self, EventType, ProcessMetricsGlobal};
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::interrupts;

/// Number of timer ticks per scheduling quantum.
pub const SCHED_QUANTUM_TICKS: u64 = 5;
pub const FAIRNESS_CHECK_INTERVAL_TICKS: u64 = 10;

static SCHED_QUANTUM_TICKS_RUNTIME: AtomicU64 = AtomicU64::new(SCHED_QUANTUM_TICKS);
static FAIRNESS_CHECK_INTERVAL_TICKS_RUNTIME: AtomicU64 =
    AtomicU64::new(FAIRNESS_CHECK_INTERVAL_TICKS);

static NEED_RESCHEDULE: AtomicBool = AtomicBool::new(false);
static TIMER_TICKS: AtomicU64 = AtomicU64::new(0);
static RESCHEDULE_REQUESTS: AtomicU64 = AtomicU64::new(0);
static RESCHEDULE_POINTS: AtomicU64 = AtomicU64::new(0);
static DEMO_CONTEXT_TASKS_SPAWNED: AtomicBool = AtomicBool::new(false);
static DEMO_A_COUNT: AtomicU64 = AtomicU64::new(0);
static DEMO_B_COUNT: AtomicU64 = AtomicU64::new(0);
// Phase 5: Independent multi-task counters for fairness testing
static KERNEL_TASK_1_COUNT: AtomicU64 = AtomicU64::new(0);
static KERNEL_TASK_2_COUNT: AtomicU64 = AtomicU64::new(0);
static KERNEL_TASK_3_COUNT: AtomicU64 = AtomicU64::new(0);
static KERNEL_TASK_4_COUNT: AtomicU64 = AtomicU64::new(0);
static PREEMPT_MISSES: AtomicU64 = AtomicU64::new(0);
static LAST_SWITCH_TICK: AtomicU64 = AtomicU64::new(0);
static WATCHDOG_TRIPS: AtomicU64 = AtomicU64::new(0);
static IRQ_PREEMPT_PENDING: AtomicBool = AtomicBool::new(false);
static IRQ_PREEMPT_REQUESTS: AtomicU64 = AtomicU64::new(0);
static IRQ_PREEMPT_CHECKPOINTS: AtomicU64 = AtomicU64::new(0);
static LAST_IRQ_RIP: AtomicU64 = AtomicU64::new(0);
static LAST_IRQ_RSP: AtomicU64 = AtomicU64::new(0);
static LAST_IRQ_CS: AtomicU64 = AtomicU64::new(0);
static LAST_IRQ_RFLAGS: AtomicU64 = AtomicU64::new(0);
static LAST_IRQ_HAS_RSP: AtomicBool = AtomicBool::new(false);
static IRQ_FRAME_INVALID: AtomicU64 = AtomicU64::new(0);
static IRQ_FORCED_ATTEMPTS: AtomicU64 = AtomicU64::new(0);
static IRQ_FORCED_BLOCKED: AtomicU64 = AtomicU64::new(0);
static IRQ_FORCED_SUCCEEDED: AtomicU64 = AtomicU64::new(0);
static LAST_OBSERVED_TIMER_TICK: AtomicU64 = AtomicU64::new(0);
static STAGNANT_SPIN_COUNT: AtomicU64 = AtomicU64::new(0);
static TIMER_STALL_FALLBACKS: AtomicU64 = AtomicU64::new(0);
static IRQ_HANDOFF_QUEUED: AtomicU64 = AtomicU64::new(0);
static IRQ_HANDOFF_CONSUMED: AtomicU64 = AtomicU64::new(0);
static HANDOFF_PENDING: AtomicBool = AtomicBool::new(false);
static DEMO_CTX_A_PTR: AtomicU64 = AtomicU64::new(0);
static DEMO_CTX_B_PTR: AtomicU64 = AtomicU64::new(0);
static DEMO_CURRENT_SLOT: AtomicU64 = AtomicU64::new(0);
static CONTEXT_SWITCH_ENABLED_ATOMIC: AtomicBool = AtomicBool::new(false);
static SCHEDULER_LOCK_CONTENTION: AtomicU64 = AtomicU64::new(0);

const CONTEXT_LAB_MAX_STALL_TICKS: u64 = 10_000;
const CONTEXT_LAB_TIMER_STALL_SPIN_THRESHOLD: u64 = 20_000;
const CONTEXT_LAB_LOG_INTERVAL: u64 = 50_000;

lazy_static! {
    static ref CONTEXT_SCHEDULER: Mutex<ContextScheduler> = Mutex::new(ContextScheduler::new());
}

/// Per-task performance metrics for preemptive scheduling.
#[derive(Debug, Clone, Copy, Default)]
pub struct TaskMetrics {
    /// Context switches for this task (times preempted or voluntarily yielded).
    pub switches: u64,
    /// CPU time accounted in scheduler ticks.
    pub cpu_ticks: u64,
    /// Tick when this task was created.
    pub created_tick: u64,
    /// Preemption attempts on this task from IRQ context.
    pub preemption_attempts: u64,
    /// Successful preemptions of this task.
    pub preemption_successes: u64,
}

struct ContextTask {
    name: &'static str,
    runnable: RunnableContext,
    metrics: TaskMetrics,
}

struct ContextScheduler {
    tasks: Vec<ContextTask>,
    ready_queue: VecDeque<usize>,
    current: Option<usize>,
    switches: u64,
    switch_enabled: bool,
    last_switch_tick: u64,
}

impl ContextScheduler {
    const fn new() -> Self {
        Self {
            tasks: Vec::new(),
            ready_queue: VecDeque::new(),
            current: None,
            switches: 0,
            switch_enabled: false,
            last_switch_tick: 0,
        }
    }

    fn spawn(&mut self, name: &'static str, entry: extern "C" fn() -> !) -> usize {
        let id = self.tasks.len();
        self.tasks.push(ContextTask {
            name,
            runnable: RunnableContext::new(entry),
            metrics: TaskMetrics {
                switches: 0,
                cpu_ticks: 0,
                created_tick: TIMER_TICKS.load(Ordering::Relaxed),
                preemption_attempts: 0,
                preemption_successes: 0,
            },
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

        let now_tick = TIMER_TICKS.load(Ordering::Relaxed);
        let elapsed = now_tick.saturating_sub(self.last_switch_tick);
        if elapsed > 0 {
            self.update_cpu_ticks(elapsed);
        }

        // Record context switch on current task
        self.tasks[current].metrics.switches = self.tasks[current].metrics.switches.saturating_add(1);
        self.tasks[current].metrics.preemption_successes = 
            self.tasks[current].metrics.preemption_successes.saturating_add(1);

        // Record context switch on next task
        self.tasks[next].metrics.switches = self.tasks[next].metrics.switches.saturating_add(1);

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
        self.last_switch_tick = now_tick;
        Some((current_ctx as *mut CpuContext, next_ctx as *const CpuContext))
    }

    fn update_cpu_ticks(&mut self, ticks: u64) {
        if let Some(current) = self.current {
            if current < self.tasks.len() {
                self.tasks[current].metrics.cpu_ticks = 
                    self.tasks[current].metrics.cpu_ticks.saturating_add(ticks);
            }
        }
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

    fn get_task_metrics(&self, id: usize) -> Option<TaskMetrics> {
        self.tasks.get(id).map(|task| task.metrics)
    }

    fn get_all_task_metrics(&self) -> Vec<(usize, &'static str, TaskMetrics)> {
        self.tasks
            .iter()
            .enumerate()
            .map(|(id, task)| (id, task.name, task.metrics))
            .collect()
    }
}

fn lock_context_scheduler() -> spin::MutexGuard<'static, ContextScheduler> {
    if let Some(guard) = CONTEXT_SCHEDULER.try_lock() {
        return guard;
    }
    SCHEDULER_LOCK_CONTENTION.fetch_add(1, Ordering::Relaxed);
    CONTEXT_SCHEDULER.lock()
}

#[derive(Debug, Clone, Copy)]
pub struct ContextSchedulerStats {
    pub tasks: usize,
    pub switches: u64,
    pub switching_enabled: bool,
    pub demo_a_count: u64,
    pub demo_b_count: u64,
    pub preempt_misses: u64,
    pub watchdog_trips: u64,
    pub irq_preempt_requests: u64,
    pub irq_preempt_checkpoints: u64,
    pub last_irq_rip: u64,
    pub last_irq_rsp: u64,
    pub last_irq_cs: u64,
    pub last_irq_rflags: u64,
    pub last_irq_has_rsp: bool,
    pub irq_frame_invalid: u64,
    pub irq_forced_attempts: u64,
    pub irq_forced_blocked: u64,
    pub irq_forced_succeeded: u64,
    pub timer_stall_fallbacks: u64,
    pub irq_handoff_queued: u64,
    pub irq_handoff_consumed: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct SchedulerStats {
    pub timer_ticks: u64,
    pub quantum_ticks: u64,
    pub fairness_check_interval_ticks: u64,
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
    let quantum_ticks = SCHED_QUANTUM_TICKS_RUNTIME.load(Ordering::Relaxed).max(1);
    if ticks % quantum_ticks == 0 {
        NEED_RESCHEDULE.store(true, Ordering::Relaxed);
        RESCHEDULE_REQUESTS.fetch_add(1, Ordering::Relaxed);
        IRQ_PREEMPT_PENDING.store(true, Ordering::Relaxed);
        IRQ_PREEMPT_REQUESTS.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn scheduler_quantum_ticks() -> u64 {
    SCHED_QUANTUM_TICKS_RUNTIME.load(Ordering::Relaxed)
}

pub fn set_scheduler_quantum_ticks(ticks: u64) {
    SCHED_QUANTUM_TICKS_RUNTIME.store(ticks.max(1), Ordering::Relaxed);
}

pub fn fairness_check_interval_ticks() -> u64 {
    FAIRNESS_CHECK_INTERVAL_TICKS_RUNTIME.load(Ordering::Relaxed)
}

pub fn set_fairness_check_interval_ticks(ticks: u64) {
    FAIRNESS_CHECK_INTERVAL_TICKS_RUNTIME.store(ticks.max(1), Ordering::Relaxed);
}

/// Called by the timer IRQ handler with interrupted execution context.
pub fn on_timer_interrupt_context(interrupted_rip: u64, interrupted_rsp: u64) {
    on_timer_interrupt_context_detailed(interrupted_rip, interrupted_rsp, 0, 0, true);
}

pub fn on_timer_interrupt_context_detailed(
    interrupted_rip: u64,
    interrupted_rsp: u64,
    interrupted_cs: u64,
    interrupted_rflags: u64,
    has_rsp: bool,
) {
    LAST_IRQ_RIP.store(interrupted_rip, Ordering::Relaxed);
    LAST_IRQ_RSP.store(interrupted_rsp, Ordering::Relaxed);
    LAST_IRQ_CS.store(interrupted_cs, Ordering::Relaxed);
    LAST_IRQ_RFLAGS.store(interrupted_rflags, Ordering::Relaxed);
    LAST_IRQ_HAS_RSP.store(has_rsp, Ordering::Relaxed);

    if !is_canonical_address(interrupted_rip)
        || (has_rsp && !is_canonical_address(interrupted_rsp))
    {
        IRQ_FRAME_INVALID.fetch_add(1, Ordering::Relaxed);
    }
}

/// IRQ-tail preemption hook.
///
/// Currently this only records telemetry because forced context switches
/// directly from `extern "x86-interrupt"` handlers require a custom low-level
/// interrupt return path.
pub fn try_forced_preempt_from_irq_tail() -> bool {
    if !IRQ_PREEMPT_PENDING.load(Ordering::Relaxed) {
        return false;
    }

    IRQ_FORCED_ATTEMPTS.fetch_add(1, Ordering::Relaxed);

    if !CONTEXT_SWITCH_ENABLED_ATOMIC.load(Ordering::Relaxed) {
        IRQ_FORCED_BLOCKED.fetch_add(1, Ordering::Relaxed);
        return false;
    }

    let ctx_a = DEMO_CTX_A_PTR.load(Ordering::Relaxed);
    let ctx_b = DEMO_CTX_B_PTR.load(Ordering::Relaxed);
    if ctx_a == 0 || ctx_b == 0 {
        IRQ_FORCED_BLOCKED.fetch_add(1, Ordering::Relaxed);
        return false;
    }

    if HANDOFF_PENDING.load(Ordering::Relaxed) {
        IRQ_FORCED_BLOCKED.fetch_add(1, Ordering::Relaxed);
        return false;
    }

    HANDOFF_PENDING.store(true, Ordering::Relaxed);
    IRQ_HANDOFF_QUEUED.fetch_add(1, Ordering::Relaxed);

    false
}

fn consume_irq_handoff_token(_current_slot: u64) {
    if !HANDOFF_PENDING.swap(false, Ordering::Relaxed) {
        return;
    }

    IRQ_HANDOFF_CONSUMED.fetch_add(1, Ordering::Relaxed);
    IRQ_FORCED_SUCCEEDED.fetch_add(1, Ordering::Relaxed);
    IRQ_PREEMPT_CHECKPOINTS.fetch_add(1, Ordering::Relaxed);
    RESCHEDULE_POINTS.fetch_add(1, Ordering::Relaxed);

    // Route through the scheduler's prepare_live_switch() so that
    // CONTEXT_SCHEDULER.current stays in sync with the actual running task.
    //
    // The old raw-pointer path (DEMO_CTX_A_PTR / DEMO_CTX_B_PTR) bypassed
    // next_pair(), leaving CONTEXT_SCHEDULER.current pointing at task-A even
    // while task-B was running.  The next try_context_reschedule() call from
    // B's loop would therefore see current=0(A), pick next=1(B), and call
    // switch_context(&mut tasks[0], &tasks[1]) — saving B's live registers
    // into A's context slot and "restoring" B from its stale entry-point.
    // That corrupted A's saved context, reset B to its creation state, and
    // made HANDOFF_PENDING permanently unproductive (handoff_queued stalled
    // at 1 forever).
    let maybe_switch = {
        let mut sched = CONTEXT_SCHEDULER.lock();
        if sched.switch_enabled() {
            sched.prepare_live_switch()
        } else {
            None
        }
    };

    if let Some((current, next)) = maybe_switch {
        unsafe {
            switch_context(&mut *current, &*next);
        }
        LAST_SWITCH_TICK.store(TIMER_TICKS.load(Ordering::Relaxed), Ordering::Relaxed);
    } else {
        IRQ_FORCED_BLOCKED.fetch_add(1, Ordering::Relaxed);
    }

    IRQ_PREEMPT_PENDING.store(false, Ordering::Relaxed);
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
    lock_context_scheduler().spawn(name, entry)
}

pub fn set_context_switching_enabled(enabled: bool) {
    lock_context_scheduler().set_switch_enabled(enabled);
    CONTEXT_SWITCH_ENABLED_ATOMIC.store(enabled, Ordering::Relaxed);
}

/// Get metrics for a specific task by ID.
pub fn get_task_metrics(id: usize) -> Option<TaskMetrics> {
    lock_context_scheduler().get_task_metrics(id)
}

/// Get metrics for all context tasks.
pub fn get_all_task_metrics() -> Vec<(usize, &'static str, TaskMetrics)> {
    lock_context_scheduler().get_all_task_metrics()
}

pub fn try_context_reschedule() -> bool {
    let maybe_live_switch = {
        let mut scheduler = lock_context_scheduler();
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
    ProcessMetricsGlobal::record_preemption();
    process_metrics::log_event(EventType::Preempted, 0);
    true
}

pub fn yield_now() {
    let _ = try_context_reschedule();
}

pub fn preempt_if_requested() {
    if take_reschedule_request() {
        record_reschedule_point();
        if try_context_reschedule() {
            LAST_SWITCH_TICK.store(TIMER_TICKS.load(Ordering::Relaxed), Ordering::Relaxed);
        } else {
            PREEMPT_MISSES.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Context-task checkpoint for deferred preemption requested by timer IRQ.
pub fn preempt_if_irq_pending() {
    if IRQ_PREEMPT_PENDING.swap(false, Ordering::Relaxed) {
        IRQ_PREEMPT_CHECKPOINTS.fetch_add(1, Ordering::Relaxed);
        preempt_if_requested();
    }
}

fn context_lab_watchdog_check() {
    let now = TIMER_TICKS.load(Ordering::Relaxed);
    let last = LAST_SWITCH_TICK.load(Ordering::Relaxed);
    if now.saturating_sub(last) > CONTEXT_LAB_MAX_STALL_TICKS {
        WATCHDOG_TRIPS.fetch_add(1, Ordering::Relaxed);
        panic!(
            "Context-lab watchdog: no context switch for {} ticks",
            now.saturating_sub(last)
        );
    }
}

fn context_lab_timer_progress_check() {
    let now = TIMER_TICKS.load(Ordering::Relaxed);
    let last = LAST_OBSERVED_TIMER_TICK.swap(now, Ordering::Relaxed);

    if now == last {
        let stagnant = STAGNANT_SPIN_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if stagnant >= CONTEXT_LAB_TIMER_STALL_SPIN_THRESHOLD {
            STAGNANT_SPIN_COUNT.store(0, Ordering::Relaxed);
            TIMER_STALL_FALLBACKS.fetch_add(1, Ordering::Relaxed);
            yield_now();
        }
    } else {
        STAGNANT_SPIN_COUNT.store(0, Ordering::Relaxed);
    }
}

fn is_canonical_address(addr: u64) -> bool {
    let sign = (addr >> 47) & 1;
    let upper = addr >> 48;
    upper == 0 || (sign == 1 && upper == 0xFFFF)
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
    let scheduler = lock_context_scheduler();
    let (demo_a_count, demo_b_count) = scheduler.demo_counts();
    ContextSchedulerStats {
        tasks: scheduler.context_task_count(),
        switches: scheduler.context_switch_count(),
        switching_enabled: scheduler.switch_enabled(),
        demo_a_count,
        demo_b_count,
        preempt_misses: PREEMPT_MISSES.load(Ordering::Relaxed),
        watchdog_trips: WATCHDOG_TRIPS.load(Ordering::Relaxed),
        irq_preempt_requests: IRQ_PREEMPT_REQUESTS.load(Ordering::Relaxed),
        irq_preempt_checkpoints: IRQ_PREEMPT_CHECKPOINTS.load(Ordering::Relaxed),
        last_irq_rip: LAST_IRQ_RIP.load(Ordering::Relaxed),
        last_irq_rsp: LAST_IRQ_RSP.load(Ordering::Relaxed),
        last_irq_cs: LAST_IRQ_CS.load(Ordering::Relaxed),
        last_irq_rflags: LAST_IRQ_RFLAGS.load(Ordering::Relaxed),
        last_irq_has_rsp: LAST_IRQ_HAS_RSP.load(Ordering::Relaxed),
        irq_frame_invalid: IRQ_FRAME_INVALID.load(Ordering::Relaxed),
        irq_forced_attempts: IRQ_FORCED_ATTEMPTS.load(Ordering::Relaxed),
        irq_forced_blocked: IRQ_FORCED_BLOCKED.load(Ordering::Relaxed),
        irq_forced_succeeded: IRQ_FORCED_SUCCEEDED.load(Ordering::Relaxed),
        timer_stall_fallbacks: TIMER_STALL_FALLBACKS.load(Ordering::Relaxed),
        irq_handoff_queued: IRQ_HANDOFF_QUEUED.load(Ordering::Relaxed),
        irq_handoff_consumed: IRQ_HANDOFF_CONSUMED.load(Ordering::Relaxed),
    }
}

pub fn context_task_names() -> Vec<&'static str> {
    lock_context_scheduler()
        .tasks
        .iter()
        .map(|task| task.name)
        .collect()
}

extern "C" fn demo_context_task_a() -> ! {
    loop {
        interrupts::enable();
        DEMO_CURRENT_SLOT.store(0, Ordering::Relaxed);
        let count = DEMO_A_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if count % CONTEXT_LAB_LOG_INTERVAL == 0 {
            let b = DEMO_B_COUNT.load(Ordering::Relaxed);
            let context = context_stats();
            crate::println!(
                "ContextLab A={}, B={}, ticks={}, switches={}, irq_forced_ok={}, irq_forced_blocked={}, handoff_q={}, handoff_c={}, misses={}, timer_stall_fallbacks={}",
                count,
                b,
                stats().timer_ticks,
                context.switches,
                context.irq_forced_succeeded,
                context.irq_forced_blocked,
                context.irq_handoff_queued,
                context.irq_handoff_consumed,
                context.preempt_misses,
                context.timer_stall_fallbacks
            );
            crate::serial_println!(
                "ContextLab A={}, B={}, ticks={}, switches={}, irq_forced_ok={}, irq_forced_blocked={}, handoff_q={}, handoff_c={}, misses={}, timer_stall_fallbacks={}",
                count,
                b,
                stats().timer_ticks,
                context.switches,
                context.irq_forced_succeeded,
                context.irq_forced_blocked,
                context.irq_handoff_queued,
                context.irq_handoff_consumed,
                context.preempt_misses,
                context.timer_stall_fallbacks
            );
        }
        consume_irq_handoff_token(0);
        preempt_if_irq_pending();
        context_lab_timer_progress_check();
        context_lab_watchdog_check();
    }
}

extern "C" fn demo_context_task_b() -> ! {
    loop {
        interrupts::enable();
        DEMO_CURRENT_SLOT.store(1, Ordering::Relaxed);
        let count = DEMO_B_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
        if count % CONTEXT_LAB_LOG_INTERVAL == 0 {
            let a = DEMO_A_COUNT.load(Ordering::Relaxed);
            let context = context_stats();
            crate::println!(
                "ContextLab A={}, B={}, ticks={}, switches={}, irq_forced_ok={}, irq_forced_blocked={}, handoff_q={}, handoff_c={}, misses={}, timer_stall_fallbacks={}",
                a,
                count,
                stats().timer_ticks,
                context.switches,
                context.irq_forced_succeeded,
                context.irq_forced_blocked,
                context.irq_handoff_queued,
                context.irq_handoff_consumed,
                context.preempt_misses,
                context.timer_stall_fallbacks
            );
            crate::serial_println!(
                "ContextLab A={}, B={}, ticks={}, switches={}, irq_forced_ok={}, irq_forced_blocked={}, handoff_q={}, handoff_c={}, misses={}, timer_stall_fallbacks={}",
                a,
                count,
                stats().timer_ticks,
                context.switches,
                context.irq_forced_succeeded,
                context.irq_forced_blocked,
                context.irq_handoff_queued,
                context.irq_handoff_consumed,
                context.preempt_misses,
                context.timer_stall_fallbacks
            );
        }
        consume_irq_handoff_token(1);
        preempt_if_irq_pending();
        context_lab_timer_progress_check();
        context_lab_watchdog_check();
    }
}

// Phase 5: Independent kernel task entry points for fairness testing
extern "C" fn kernel_task_1() -> ! {
    const LOG_INTERVAL: u64 = 100_000;
    let mut local_count = 0u64;
    loop {
        interrupts::enable();
        increment_kernel_task_counter(1);
        local_count += 1;
        if local_count % LOG_INTERVAL == 0 {
            let counters = get_kernel_task_counters();
            crate::println!("Phase5-Fairness: T1={}, T2={}, T3={}, T4={}", 
                counters[0], counters[1], counters[2], counters[3]);
        }
        preempt_if_irq_pending();
    }
}

extern "C" fn kernel_task_2() -> ! {
    loop {
        interrupts::enable();
        increment_kernel_task_counter(2);
        preempt_if_irq_pending();
    }
}

extern "C" fn kernel_task_3() -> ! {
    loop {
        interrupts::enable();
        increment_kernel_task_counter(3);
        preempt_if_irq_pending();
    }
}

extern "C" fn kernel_task_4() -> ! {
    loop {
        interrupts::enable();
        increment_kernel_task_counter(4);
        preempt_if_irq_pending();
    }
}

pub fn spawn_demo_context_tasks() {
    if DEMO_CONTEXT_TASKS_SPAWNED
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        LAST_SWITCH_TICK.store(TIMER_TICKS.load(Ordering::Relaxed), Ordering::Relaxed);
        spawn_context_task("ctx-demo-a", demo_context_task_a);
        spawn_context_task("ctx-demo-b", demo_context_task_b);

        let (ctx_a, ctx_b) = {
            let scheduler = lock_context_scheduler();
            let a = &scheduler.tasks[0].runnable.context as *const CpuContext as u64;
            let b = &scheduler.tasks[1].runnable.context as *const CpuContext as u64;
            (a, b)
        };
        DEMO_CTX_A_PTR.store(ctx_a, Ordering::Relaxed);
        DEMO_CTX_B_PTR.store(ctx_b, Ordering::Relaxed);
        DEMO_CURRENT_SLOT.store(0, Ordering::Relaxed);
    }
}

pub fn stats() -> SchedulerStats {
    let context = context_stats();
    SchedulerStats {
        timer_ticks: TIMER_TICKS.load(Ordering::Relaxed),
        quantum_ticks: scheduler_quantum_ticks(),
        fairness_check_interval_ticks: fairness_check_interval_ticks(),
        reschedule_requests: RESCHEDULE_REQUESTS.load(Ordering::Relaxed),
        reschedule_points: RESCHEDULE_POINTS.load(Ordering::Relaxed),
        pending: NEED_RESCHEDULE.load(Ordering::Relaxed),
        context_tasks: context.tasks,
        context_switches: context.switches,
        context_switching_enabled: context.switching_enabled,
    }
}

// Phase 5: Public accessors for multi-task fairness testing
pub fn get_kernel_task_counters() -> [u64; 4] {
    [
        KERNEL_TASK_1_COUNT.load(Ordering::Relaxed),
        KERNEL_TASK_2_COUNT.load(Ordering::Relaxed),
        KERNEL_TASK_3_COUNT.load(Ordering::Relaxed),
        KERNEL_TASK_4_COUNT.load(Ordering::Relaxed),
    ]
}

pub fn increment_kernel_task_counter(task_id: usize) -> u64 {
    match task_id {
        1 => KERNEL_TASK_1_COUNT.fetch_add(1, Ordering::Relaxed),
        2 => KERNEL_TASK_2_COUNT.fetch_add(1, Ordering::Relaxed),
        3 => KERNEL_TASK_3_COUNT.fetch_add(1, Ordering::Relaxed),
        4 => KERNEL_TASK_4_COUNT.fetch_add(1, Ordering::Relaxed),
        _ => 0,
    }
}

/// Phase 5: Spawn 4 independent kernel tasks for fairness testing.
pub fn spawn_kernel_tasks_phase5() {
    spawn_context_task("kernel-task-1", kernel_task_1);
    spawn_context_task("kernel-task-2", kernel_task_2);
    spawn_context_task("kernel-task-3", kernel_task_3);
    spawn_context_task("kernel-task-4", kernel_task_4);
}

pub fn scheduler_lock_contention() -> u64 {
    SCHEDULER_LOCK_CONTENTION.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn scheduler_tick_requests_reschedule() {
        let before = stats();
        let quantum = scheduler_quantum_ticks();
        for _ in 0..quantum {
            on_timer_tick();
        }
        let after = stats();
        assert!(after.timer_ticks >= before.timer_ticks + quantum);
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
