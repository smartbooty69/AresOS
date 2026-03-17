//! Process-level performance metrics and observability for Phase 5.
//!
//! Tracks per-process scheduler events, CPU time, and fairness metrics
//! for real-time performance analysis.

use crate::performance::metrics::TICK_COUNTER;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

/// Per-process performance snapshot.
#[derive(Debug, Clone, Copy)]
pub struct ProcessMetricsSnapshot {
    pub pid: u64,
    pub cpu_ticks: u64,
    pub context_switches: u64,
    pub created_tick: u64,
    pub preemption_attempts: u64,
    pub preemption_successes: u64,
}

/// Scheduler-wide fairness metrics.
#[derive(Debug, Clone, Copy)]
pub struct FairnessMetrics {
    pub total_processes: u64,
    pub ready_processes: u64,
    pub max_cpu_ticks: u64,
    pub min_cpu_ticks: u64,
    pub fairness_score: f64, // ratio of max/min
    pub total_context_switches: u64,
}

impl FairnessMetrics {
    /// Compute fairness score: max(cpu_ticks) / min(cpu_ticks).
    /// Result is only valid if ready_processes > 0.
    pub fn compute(
        total_processes: u64,
        ready_processes: u64,
        max_cpu_ticks: u64,
        min_cpu_ticks: u64,
        total_context_switches: u64,
    ) -> Self {
        let fairness_score = if min_cpu_ticks > 0 && ready_processes > 0 {
            max_cpu_ticks as f64 / min_cpu_ticks as f64
        } else {
            1.0
        };

        FairnessMetrics {
            total_processes,
            ready_processes,
            max_cpu_ticks,
            min_cpu_ticks,
            fairness_score,
            total_context_switches,
        }
    }

    /// Check if scheduling is fair (fairness score <= 1.10 = 10% deviation).
    pub fn is_fair(&self) -> bool {
        self.fairness_score <= 1.10
    }

    /// Check for severe fairness violation (fairness score > 1.50 = 50% deviation).
    pub fn has_severe_starvation(&self) -> bool {
        self.fairness_score > 1.50
    }
}

/// In-memory event log ring buffer for scheduler events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventLogEntry {
    pub tick: u64,
    pub event_type: EventType,
    pub pid: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// Process preempted after quantum expiry.
    Preempted,
    /// Process entered blocked state.
    Blocked,
    /// Process became ready.
    Ready,
    /// Process terminated.
    Terminated,
    /// Fairness violation detected.
    StarvationDetected,
}

/// Event ring buffer capacity (~6KB with current entry size).
pub const EVENT_LOG_CAPACITY: usize = 256;

const EMPTY_EVENT: EventLogEntry = EventLogEntry {
    tick: 0,
    event_type: EventType::Ready,
    pid: 0,
};

struct EventLogRing {
    entries: [EventLogEntry; EVENT_LOG_CAPACITY],
    head: usize,
    len: usize,
}

impl EventLogRing {
    const fn new() -> Self {
        Self {
            entries: [EMPTY_EVENT; EVENT_LOG_CAPACITY],
            head: 0,
            len: 0,
        }
    }

    fn push(&mut self, entry: EventLogEntry) {
        self.entries[self.head] = entry;
        self.head = (self.head + 1) % EVENT_LOG_CAPACITY;
        if self.len < EVENT_LOG_CAPACITY {
            self.len += 1;
        }
    }

    fn latest(&self) -> Option<EventLogEntry> {
        if self.len == 0 {
            return None;
        }
        let latest_idx = if self.head == 0 {
            EVENT_LOG_CAPACITY - 1
        } else {
            self.head - 1
        };
        Some(self.entries[latest_idx])
    }
}

static EVENT_LOG: Mutex<EventLogRing> = Mutex::new(EventLogRing::new());

/// Global performance counter increments.
pub static PROCESS_CREATIONS: AtomicU64 = AtomicU64::new(0);
pub static PROCESS_TERMINATIONS: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_PREEMPTIONS: AtomicU64 = AtomicU64::new(0);
pub static FAIRNESS_VIOLATIONS: AtomicU64 = AtomicU64::new(0);

pub struct ProcessMetricsGlobal;

impl ProcessMetricsGlobal {
    /// Increment process creation counter.
    pub fn record_process_creation() {
        PROCESS_CREATIONS.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment process termination counter.
    pub fn record_process_termination() {
        PROCESS_TERMINATIONS.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment preemption counter.
    pub fn record_preemption() {
        TOTAL_PREEMPTIONS.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment fairness violation counter.
    pub fn record_fairness_violation() {
        FAIRNESS_VIOLATIONS.fetch_add(1, Ordering::Relaxed);
    }

    /// Get global metrics snapshot.
    pub fn global_snapshot() -> (u64, u64, u64, u64) {
        (
            PROCESS_CREATIONS.load(Ordering::Relaxed),
            PROCESS_TERMINATIONS.load(Ordering::Relaxed),
            TOTAL_PREEMPTIONS.load(Ordering::Relaxed),
            FAIRNESS_VIOLATIONS.load(Ordering::Relaxed),
        )
    }
}

/// Append an event to the in-memory ring buffer.
pub fn log_event(event_type: EventType, pid: u64) {
    let mut log = EVENT_LOG.lock();
    log.push(EventLogEntry {
        tick: TICK_COUNTER.load(Ordering::Relaxed),
        event_type,
        pid,
    });
}

/// Return up to `max_events` recent events, newest-first.
pub fn recent_events(max_events: usize) -> Vec<EventLogEntry> {
    if max_events == 0 {
        return Vec::new();
    }

    let log = EVENT_LOG.lock();
    let take = core::cmp::min(max_events, log.len);
    let mut out = Vec::with_capacity(take);
    let mut idx = if log.head == 0 {
        EVENT_LOG_CAPACITY - 1
    } else {
        log.head - 1
    };

    for _ in 0..take {
        out.push(log.entries[idx]);
        idx = if idx == 0 {
            EVENT_LOG_CAPACITY - 1
        } else {
            idx - 1
        };
    }
    out
}

/// Return the current number of events in the ring buffer.
pub fn event_count() -> usize {
    EVENT_LOG.lock().len
}

/// Return the latest event without allocating.
pub fn latest_event() -> Option<EventLogEntry> {
    EVENT_LOG.lock().latest()
}

/// Compute fairness metrics from process snapshots.
pub fn compute_fairness_metrics(
    processes: &[(u64, &str, u64)], // (pid, name, cpu_ticks)
) -> FairnessMetrics {
    let total_processes = processes.len() as u64;
    let ready_processes = total_processes; // Simplified: assume all tracked processes are ready

    if processes.is_empty() {
        return FairnessMetrics::compute(0, 0, 0, 0, 0);
    }

    let max_cpu_ticks = processes.iter().map(|(_, _, ticks)| *ticks).max().unwrap_or(0);
    let min_cpu_ticks = processes.iter().map(|(_, _, ticks)| *ticks).min().unwrap_or(0);

    FairnessMetrics::compute(
        total_processes,
        ready_processes,
        max_cpu_ticks,
        min_cpu_ticks,
        0, // total_context_switches populated by caller if needed
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn fairness_metrics_perfect_balance() {
        let metrics = FairnessMetrics::compute(4, 4, 1000, 1000, 10);
        assert!(metrics.is_fair());
        assert_eq!(metrics.fairness_score, 1.0);
    }

    #[test_case]
    fn fairness_metrics_slight_imbalance() {
        let metrics = FairnessMetrics::compute(4, 4, 1050, 1000, 50);
        assert!(metrics.is_fair()); // 1.05 < 1.10
    }

    #[test_case]
    fn fairness_metrics_severe_starvation() {
        let metrics = FairnessMetrics::compute(4, 4, 5000, 1000, 100);
        assert!(!metrics.is_fair()); // 5.0 > 1.10
        assert!(metrics.has_severe_starvation()); // 5.0 > 1.50
    }

    #[test_case]
    fn global_metrics_increment() {
        ProcessMetricsGlobal::record_process_creation();
        ProcessMetricsGlobal::record_preemption();
        let (creates, _terms, preempts, _violations) = ProcessMetricsGlobal::global_snapshot();
        assert!(creates >= 1);
        assert!(preempts >= 1);
    }

    #[test_case]
    fn event_log_records_entries() {
        let before = event_count();
        log_event(EventType::Preempted, 42);
        let after = event_count();
        assert!(after >= before + 1);

        let latest = latest_event().expect("expected one event");
        assert_eq!(latest.event_type, EventType::Preempted);
        assert_eq!(latest.pid, 42);
    }
}
