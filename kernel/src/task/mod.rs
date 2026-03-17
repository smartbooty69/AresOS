//! Cooperative async task system.
//!
//! Provides a simple executor that drives `core::future::Future` tasks to
//! completion using a spinlock-based task queue.

pub mod context;
pub mod executor;
pub mod keyboard;
pub mod process;
pub mod scheduler;
pub mod timer;

use alloc::boxed::Box;
use core::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
    task::{Context, Poll},
};
use crate::performance::metrics::TICK_COUNTER;

/// Unique identifier for a kernel task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(u64);

impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// Lifecycle state for a kernel task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Sleeping,
    Finished,
}

/// A pinned, heap-allocated future representing a single kernel task.
pub struct Task {
    id: TaskId,
    name: &'static str,
    created_tick: u64,
    state: TaskState,
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    /// Create a new task from any `Future` that outputs `()`.
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        Self::named("unnamed", future)
    }

    /// Create a new named task.
    pub fn named(name: &'static str, future: impl Future<Output = ()> + 'static) -> Task {
        Task {
            id: TaskId::new(),
            name,
            created_tick: TICK_COUNTER.load(Ordering::Relaxed),
            state: TaskState::Ready,
            future: Box::pin(future),
        }
    }

    pub fn id(&self) -> TaskId {
        self.id
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn created_tick(&self) -> u64 {
        self.created_tick
    }

    pub fn state(&self) -> TaskState {
        self.state
    }

    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }

    pub(super) fn set_state(&mut self, state: TaskState) {
        self.state = state;
    }
}
