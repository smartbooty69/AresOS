//! Simple cooperative task executor.
//!
//! Maintains a FIFO queue of ready tasks and polls them until they complete.
//! The executor uses a wake-queue to avoid busy-polling tasks that are not
//! yet ready.

use super::{Task, TaskId, TaskState};
use alloc::{collections::BTreeMap, sync::Arc, task::Wake, vec::Vec};
use crate::performance::metrics::TICK_COUNTER;
use core::{
    sync::atomic::{AtomicUsize, Ordering},
    task::{Context, Poll, Waker},
};
use crossbeam_queue::ArrayQueue;
use lazy_static::lazy_static;
use spin::Mutex;

/// Maximum number of wake notifications that can be queued simultaneously.
const WAKE_QUEUE_SIZE: usize = 100;

/// Number of timer ticks between per-task fairness preemption checks.
/// Even if a task remains ready, another task gets a chance to run.
const FAIRNESS_CHECK_INTERVAL_TICKS: u64 = 10;

static ACTIVE_TASKS: AtomicUsize = AtomicUsize::new(0);
static LAST_FAIRNESS_CHECK_TICK: AtomicUsize = AtomicUsize::new(0);
static SLEEPING_TASKS: AtomicUsize = AtomicUsize::new(0);
static READY_QUEUE_DEPTH: AtomicUsize = AtomicUsize::new(0);
static COMPLETED_TASKS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy)]
struct RegistryEntry {
    name: &'static str,
    state: TaskState,
    created_tick: u64,
    state_since_tick: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct TaskSnapshot {
    pub id: u64,
    pub name: &'static str,
    pub state: TaskState,
    pub age_ticks: u64,
    pub state_age_ticks: u64,
}

lazy_static! {
    static ref TASK_REGISTRY: Mutex<BTreeMap<TaskId, RegistryEntry>> = Mutex::new(BTreeMap::new());
}

#[derive(Debug, Clone, Copy)]
pub struct ExecutorStats {
    pub active_tasks: usize,
    pub sleeping_tasks: usize,
    pub ready_queue_depth: usize,
    pub completed_tasks: usize,
}

/// A simple round-robin executor for kernel tasks.
pub struct Executor {
    tasks: BTreeMap<TaskId, Task>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    /// Create a new, empty executor.
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(WAKE_QUEUE_SIZE)),
            waker_cache: BTreeMap::new(),
        }
    }

    /// Enqueue a task for execution.
    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id();
        let task_name = task.name();
        let created_tick = task.created_tick();
        let task_state = task.state();

        TASK_REGISTRY.lock().insert(
            task_id,
            RegistryEntry {
                name: task_name,
                state: task_state,
                created_tick,
                state_since_tick: created_tick,
            },
        );

        if self.tasks.insert(task_id, task).is_some() {
            panic!("task with same ID already in executor");
        }
        ACTIVE_TASKS.fetch_add(1, Ordering::Relaxed);
        self.task_queue
            .push(task_id)
            .unwrap_or_else(|_| panic!("task queue full when spawning task {:?}", task_id));
        READY_QUEUE_DEPTH.store(self.task_queue.len(), Ordering::Relaxed);
    }

    pub fn stats(&self) -> ExecutorStats {
        let sleeping_tasks = self
            .tasks
            .values()
            .filter(|task| task.state() == TaskState::Sleeping)
            .count();
        ExecutorStats {
            active_tasks: self.tasks.len(),
            sleeping_tasks,
            ready_queue_depth: self.task_queue.len(),
            completed_tasks: COMPLETED_TASKS.load(Ordering::Relaxed),
        }
    }

    fn run_ready_tasks(&mut self) {
        let Self {
            tasks,
            task_queue,
            waker_cache,
        } = self;

        while let Some(task_id) = task_queue.pop() {
            READY_QUEUE_DEPTH.store(task_queue.len(), Ordering::Relaxed);

            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue,
            };

            if task.state() == TaskState::Sleeping {
                SLEEPING_TASKS.fetch_sub(1, Ordering::Relaxed);
            }
            task.set_state(TaskState::Ready);
            if let Some(entry) = TASK_REGISTRY.lock().get_mut(&task_id) {
                entry.state = TaskState::Ready;
                entry.state_since_tick = TICK_COUNTER.load(Ordering::Relaxed);
            }

            let waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, task_queue.clone()));
            let mut context = Context::from_waker(waker);
            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    task.set_state(TaskState::Finished);
                    if let Some(entry) = TASK_REGISTRY.lock().get_mut(&task_id) {
                        entry.state = TaskState::Finished;
                        entry.state_since_tick = TICK_COUNTER.load(Ordering::Relaxed);
                    }
                    tasks.remove(&task_id);
                    waker_cache.remove(&task_id);
                    TASK_REGISTRY.lock().remove(&task_id);
                    ACTIVE_TASKS.fetch_sub(1, Ordering::Relaxed);
                    COMPLETED_TASKS.fetch_add(1, Ordering::Relaxed);
                }
                Poll::Pending => {
                    task.set_state(TaskState::Sleeping);
                    if let Some(entry) = TASK_REGISTRY.lock().get_mut(&task_id) {
                        entry.state = TaskState::Sleeping;
                        entry.state_since_tick = TICK_COUNTER.load(Ordering::Relaxed);
                    }
                    SLEEPING_TASKS.fetch_add(1, Ordering::Relaxed);
                }
            }

            // Fairness checkpoint: periodically break to allow other ready tasks a chance to run
            let now = TICK_COUNTER.load(Ordering::Relaxed) as usize;
            let last_check = LAST_FAIRNESS_CHECK_TICK.load(Ordering::Relaxed);
            if now.saturating_sub(last_check) >= FAIRNESS_CHECK_INTERVAL_TICKS as usize {
                LAST_FAIRNESS_CHECK_TICK.store(now, Ordering::Relaxed);
                if !task_queue.is_empty() {
                    // Other tasks are ready, give them a chance
                    break;
                }
            }
        }
    }

    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts::{self, enable_and_hlt};

        interrupts::disable();
        if self.task_queue.is_empty() {
            enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }

    /// Run until all tasks have completed.
    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            if crate::task::scheduler::take_reschedule_request() {
                let _ = crate::task::scheduler::try_context_reschedule();
                crate::task::scheduler::record_reschedule_point();
            }
            self.sleep_if_idle();
        }
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

pub fn system_stats() -> ExecutorStats {
    ExecutorStats {
        active_tasks: ACTIVE_TASKS.load(Ordering::Relaxed),
        sleeping_tasks: SLEEPING_TASKS.load(Ordering::Relaxed),
        ready_queue_depth: READY_QUEUE_DEPTH.load(Ordering::Relaxed),
        completed_tasks: COMPLETED_TASKS.load(Ordering::Relaxed),
    }
}

pub fn system_task_snapshots() -> Vec<TaskSnapshot> {
    let now = TICK_COUNTER.load(Ordering::Relaxed);
    TASK_REGISTRY
        .lock()
        .iter()
        .map(|(id, entry)| TaskSnapshot {
            id: id.as_u64(),
            name: entry.name,
            state: entry.state,
            age_ticks: now.saturating_sub(entry.created_tick),
            state_age_ticks: now.saturating_sub(entry.state_since_tick),
        })
        .collect()
}

// ──────────────────────────────── task waker ─────────────────────────────────

struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }

    fn wake_task(&self) {
        self.task_queue
            .push(self.task_id)
            .unwrap_or_else(|_| panic!("task queue full when waking task {:?}", self.task_id));
        READY_QUEUE_DEPTH.store(self.task_queue.len(), Ordering::Relaxed);
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
