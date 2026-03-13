//! Lightweight kernel profiler.
//!
//! Measures wall-clock time (in TSC cycles) spent in labelled sections of
//! kernel code.  Up to `MAX_ENTRIES` named measurements are kept; once the
//! buffer is full, the oldest entry is silently overwritten (ring-buffer
//! semantics).

use super::metrics::rdtsc;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

/// Maximum number of profiling entries that can be retained simultaneously.
pub const MAX_ENTRIES: usize = 64;

/// A single profiling measurement.
#[derive(Debug, Clone, Copy)]
pub struct ProfileEntry {
    /// A short, static label identifying the measured section.
    pub label: &'static str,
    /// TSC value at the start of the section.
    pub start_tsc: u64,
    /// TSC value at the end of the section.
    pub end_tsc: u64,
}

impl ProfileEntry {
    /// Number of TSC cycles elapsed in this entry.
    pub fn elapsed_cycles(&self) -> u64 {
        self.end_tsc.saturating_sub(self.start_tsc)
    }
}

/// Ring-buffer of profile entries, protected by a spinlock.
static ENTRIES: Mutex<[Option<ProfileEntry>; MAX_ENTRIES]> = Mutex::new([None; MAX_ENTRIES]);

/// Write index into the ring buffer.
static WRITE_INDEX: AtomicUsize = AtomicUsize::new(0);

/// A scoped profiler.  Records the TSC at construction and commits an entry
/// when dropped.
///
/// # Example
/// ```no_run
/// let _p = Profiler::start("memory_init");
/// // … code to profile …
/// // Entry is recorded automatically when `_p` is dropped.
/// ```
pub struct Profiler {
    label: &'static str,
    start_tsc: u64,
}

impl Profiler {
    /// Begin timing a labelled section.
    pub fn start(label: &'static str) -> Self {
        Self {
            label,
            start_tsc: rdtsc(),
        }
    }

    /// Commit the current measurement without waiting for `Drop`.
    pub fn finish(self) {
        // Consuming `self` prevents the `Drop` impl from double-recording.
        let end_tsc = rdtsc();
        record(self.label, self.start_tsc, end_tsc);
        core::mem::forget(self);
    }
}

impl Drop for Profiler {
    fn drop(&mut self) {
        let end_tsc = rdtsc();
        record(self.label, self.start_tsc, end_tsc);
    }
}

fn record(label: &'static str, start_tsc: u64, end_tsc: u64) {
    let idx = WRITE_INDEX.fetch_add(1, Ordering::Relaxed) % MAX_ENTRIES;
    let entry = ProfileEntry {
        label,
        start_tsc,
        end_tsc,
    };
    ENTRIES.lock()[idx] = Some(entry);
}

/// Call `f` with each recorded [`ProfileEntry`] in insertion order.
pub fn with_entries<F: FnMut(&ProfileEntry)>(mut f: F) {
    let entries = ENTRIES.lock();
    for entry in entries.iter().flatten() {
        f(entry);
    }
}

/// Print all profiling entries to the VGA console.
pub fn print_report() {
    crate::println!("── Profiler report ──");
    with_entries(|e| {
        crate::println!("  {:32} {} cycles", e.label, e.elapsed_cycles());
    });
    crate::println!("─────────────────────");
}

// ─────────────────────────────────── tests ───────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_profiler_records_entry() {
        let p = Profiler::start("test_section");
        // Do some trivial work so the TSC advances.
        for _ in 0..100 {
            core::hint::spin_loop();
        }
        p.finish();

        let mut found = false;
        with_entries(|e| {
            if e.label == "test_section" {
                found = true;
                // The section must have taken at least 1 cycle.
                assert!(e.elapsed_cycles() > 0);
            }
        });
        assert!(found, "profiler entry was not recorded");
    }

    #[test_case]
    fn test_profiler_drop_records_entry() {
        {
            let _p = Profiler::start("drop_test_section");
            // dropped here
        }
        let mut found = false;
        with_entries(|e| {
            if e.label == "drop_test_section" {
                found = true;
            }
        });
        assert!(found, "drop did not record profiler entry");
    }

    #[test_case]
    fn test_elapsed_cycles_non_zero() {
        let entry = ProfileEntry {
            label: "synthetic",
            start_tsc: 100,
            end_tsc: 200,
        };
        assert_eq!(entry.elapsed_cycles(), 100);
    }

    #[test_case]
    fn test_elapsed_cycles_saturates() {
        let entry = ProfileEntry {
            label: "saturated",
            start_tsc: 200,
            end_tsc: 100,
        };
        // end < start should saturate to 0 rather than wrapping.
        assert_eq!(entry.elapsed_cycles(), 0);
    }
}
