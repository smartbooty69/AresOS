//! Process abstraction for Phase 5 preemptive scheduling.
//!
//! Provides process identification, lifecycle management, and registry for
//! multi-process kernel support. Processes wrap kernel tasks with isolated
//! kernel stacks and state tracking.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use crate::performance::process_metrics::{self, EventType, ProcessMetricsGlobal};
use lazy_static::lazy_static;
use spin::Mutex;

/// Process identifier: unique per-process handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcessId(u64);

impl ProcessId {
    /// Create a new PID from a raw value.
    pub const fn from_raw(id: u64) -> Self {
        ProcessId(id)
    }

    /// Get the raw numeric PID.
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

/// Process state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// Process is newly created, not yet scheduled.
    New,
    /// Process is ready to run.
    Ready,
    /// Process is currently executing on CPU.
    Running,
    /// Process is blocked waiting for I/O or other event.
    Blocked,
    /// Process has terminated.
    Terminated,
}

impl ProcessState {
    /// Check if the process can be scheduled.
    pub fn is_runnable(self) -> bool {
        matches!(self, ProcessState::Ready | ProcessState::Running)
    }
}

/// Process metadata and lifecycle tracking.
#[derive(Debug, Clone)]
pub struct Process {
    /// Unique process identifier.
    id: ProcessId,
    /// Human-readable name for debugging.
    name: &'static str,
    /// Current state of the process.
    state: ProcessState,
    /// Exit code when terminated (None if still running).
    exit_code: Option<i32>,
    /// Tick when process was created.
    created_tick: u64,
    /// Cumulative CPU ticks used by this process.
    cpu_ticks: u64,
    /// Number of context switches for this process.
    switches: u64,
    /// Parent process ID (None for init process).
    parent_pid: Option<ProcessId>,
}

impl Process {
    /// Create a new process with the given name.
    pub fn new(id: ProcessId, name: &'static str, created_tick: u64) -> Self {
        Process {
            id,
            name,
            state: ProcessState::New,
            exit_code: None,
            created_tick,
            cpu_ticks: 0,
            switches: 0,
            parent_pid: None,
        }
    }

    pub fn id(&self) -> ProcessId {
        self.id
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn state(&self) -> ProcessState {
        self.state
    }

    pub fn set_state(&mut self, state: ProcessState) {
        self.state = state;
    }

    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    pub fn exit_with_code(&mut self, code: i32) {
        self.exit_code = Some(code);
        self.state = ProcessState::Terminated;
    }

    pub fn created_tick(&self) -> u64 {
        self.created_tick
    }

    pub fn cpu_ticks(&self) -> u64 {
        self.cpu_ticks
    }

    pub fn add_cpu_ticks(&mut self, ticks: u64) {
        self.cpu_ticks = self.cpu_ticks.saturating_add(ticks);
    }

    pub fn switches(&self) -> u64 {
        self.switches
    }

    pub fn record_switch(&mut self) {
        self.switches = self.switches.saturating_add(1);
    }

    pub fn parent_pid(&self) -> Option<ProcessId> {
        self.parent_pid
    }

    pub fn set_parent(&mut self, parent_pid: ProcessId) {
        self.parent_pid = Some(parent_pid);
    }
}

/// Global PID allocator for process creation.
struct PidAllocator {
    next_pid: u64,
}

impl PidAllocator {
    const fn new() -> Self {
        PidAllocator {
            next_pid: 1, // PID 0 is reserved for idle/kernel
        }
    }

    fn allocate(&mut self) -> ProcessId {
        let pid = self.next_pid;
        self.next_pid = self.next_pid.saturating_add(1);
        ProcessId::from_raw(pid)
    }
}

/// Global process registry.
pub struct ProcessRegistry {
    allocator: PidAllocator,
    processes: BTreeMap<ProcessId, Process>,
    max_processes: usize,
}

impl ProcessRegistry {
    const fn new() -> Self {
        ProcessRegistry {
            allocator: PidAllocator::new(),
            processes: BTreeMap::new(),
            max_processes: 256,
        }
    }

    /// Create and register a new process.
    pub fn create_process(&mut self, name: &'static str, created_tick: u64) -> Option<ProcessId> {
        if self.processes.len() >= self.max_processes {
            return None; // Process table full
        }

        let pid = self.allocator.allocate();
        let process = Process::new(pid, name, created_tick);
        self.processes.insert(pid, process);
        Some(pid)
    }

    /// Get a reference to a process.
    pub fn get_process(&self, pid: ProcessId) -> Option<&Process> {
        self.processes.get(&pid)
    }

    /// Get a mutable reference to a process.
    pub fn get_process_mut(&mut self, pid: ProcessId) -> Option<&mut Process> {
        self.processes.get_mut(&pid)
    }

    /// Update process state.
    pub fn set_process_state(&mut self, pid: ProcessId, state: ProcessState) -> bool {
        if let Some(process) = self.processes.get_mut(&pid) {
            process.set_state(state);
            true
        } else {
            false
        }
    }

    /// Record a context switch for a process.
    pub fn record_context_switch(&mut self, pid: ProcessId) -> bool {
        if let Some(process) = self.processes.get_mut(&pid) {
            process.record_switch();
            true
        } else {
            false
        }
    }

    /// Update CPU ticks for a process.
    pub fn add_cpu_ticks(&mut self, pid: ProcessId, ticks: u64) -> bool {
        if let Some(process) = self.processes.get_mut(&pid) {
            process.add_cpu_ticks(ticks);
            true
        } else {
            false
        }
    }

    /// Get all runnable processes.
    pub fn ready_processes(&self) -> Vec<ProcessId> {
        self.processes
            .iter()
            .filter(|(_, p)| p.state().is_runnable())
            .map(|(pid, _)| *pid)
            .collect()
    }

    /// Terminate a process with exit code.
    pub fn terminate_process(&mut self, pid: ProcessId, exit_code: i32) -> bool {
        if let Some(process) = self.processes.get_mut(&pid) {
            process.exit_with_code(exit_code);
            true
        } else {
            false
        }
    }

    /// Get total process count.
    pub fn process_count(&self) -> usize {
        self.processes.len()
    }

    /// Get snapshot of all processes.
    pub fn all_processes(&self) -> Vec<(ProcessId, &'static str, ProcessState, u64)> {
        self.processes
            .iter()
            .map(|(pid, p)| (*pid, p.name(), p.state(), p.cpu_ticks()))
            .collect()
    }

    /// Reap terminated processes and reclaim resources.
    pub fn reap_terminated(&mut self) -> u64 {
        let before = self.processes.len();
        self.processes.retain(|_, p| !matches!(p.state(), ProcessState::Terminated));
        (before - self.processes.len()) as u64
    }
}

lazy_static! {
    static ref PROCESS_REGISTRY: Mutex<ProcessRegistry> = Mutex::new(ProcessRegistry::new());
}

/// Public API: Create a new kernel process.
pub fn create_kernel_process(name: &'static str, created_tick: u64) -> Option<ProcessId> {
    let created = PROCESS_REGISTRY
        .lock()
        .create_process(name, created_tick);

    if let Some(pid) = created {
        ProcessMetricsGlobal::record_process_creation();
        process_metrics::log_event(EventType::Ready, pid.as_u64());
    }

    created
}

/// Public API: Get process by ID.
pub fn get_process(pid: ProcessId) -> Option<ProcessId> {
    let registry = PROCESS_REGISTRY.lock();
    registry.get_process(pid).map(|_| pid)
}

/// Public API: Update process state.
pub fn set_process_state(pid: ProcessId, state: ProcessState) -> bool {
    PROCESS_REGISTRY.lock().set_process_state(pid, state)
}

/// Public API: Record context switch for process.
pub fn record_context_switch(pid: ProcessId) -> bool {
    PROCESS_REGISTRY.lock().record_context_switch(pid)
}

/// Public API: Add CPU ticks to process.
pub fn add_process_cpu_ticks(pid: ProcessId, ticks: u64) -> bool {
    PROCESS_REGISTRY.lock().add_cpu_ticks(pid, ticks)
}

/// Public API: Get all runnable process IDs.
pub fn get_ready_processes() -> Vec<ProcessId> {
    PROCESS_REGISTRY.lock().ready_processes()
}

/// Public API: Terminate process with exit code.
pub fn terminate_process(pid: ProcessId, exit_code: i32) -> bool {
    let terminated = PROCESS_REGISTRY.lock().terminate_process(pid, exit_code);
    if terminated {
        ProcessMetricsGlobal::record_process_termination();
        process_metrics::log_event(EventType::Terminated, pid.as_u64());
    }
    terminated
}

/// Public API: Get total process count.
pub fn process_count() -> usize {
    PROCESS_REGISTRY.lock().process_count()
}

/// Public API: Get snapshot of all processes for telemetry.
pub fn get_all_processes() -> Vec<(ProcessId, &'static str, ProcessState, u64)> {
    PROCESS_REGISTRY.lock().all_processes()
}

/// Public API: Reap terminated processes.
pub fn reap_terminated_processes() -> u64 {
    PROCESS_REGISTRY.lock().reap_terminated()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn pid_creation() {
        let pid1 = ProcessId::from_raw(1);
        let pid2 = ProcessId::from_raw(2);
        assert_ne!(pid1, pid2);
        assert_eq!(pid1.as_u64(), 1);
    }

    #[test_case]
    fn process_state_transitions() {
        let mut process = Process::new(ProcessId::from_raw(1), "test", 0);
        assert_eq!(process.state(), ProcessState::New);

        process.set_state(ProcessState::Ready);
        assert_eq!(process.state(), ProcessState::Ready);
        assert!(process.state().is_runnable());

        process.exit_with_code(0);
        assert_eq!(process.state(), ProcessState::Terminated);
        assert!(!process.state().is_runnable());
    }

    #[test_case]
    fn process_metrics_accumulation() {
        let mut process = Process::new(ProcessId::from_raw(1), "test", 100);
        assert_eq!(process.cpu_ticks(), 0);
        assert_eq!(process.switches(), 0);

        process.add_cpu_ticks(50);
        process.record_switch();

        assert_eq!(process.cpu_ticks(), 50);
        assert_eq!(process.switches(), 1);
    }
}
