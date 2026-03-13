//! CPU performance counters and system metrics.
//!
//! * `TICK_COUNTER` – counts PIT timer interrupts (incremented in the IRQ0
//!   handler at ~100 Hz by default).
//! * `PerformanceCounters` – a snapshot of performance metrics readable from
//!   any kernel code.

use core::sync::atomic::{AtomicU64, Ordering};

/// Global timer-tick counter, incremented by the PIT IRQ handler.
pub static TICK_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Read the CPU's time-stamp counter (RDTSC).
///
/// Returns the number of CPU cycles since the last reset.
#[inline(always)]
pub fn rdtsc() -> u64 {
    // SAFETY: RDTSC is always available on x86_64 and is unprivileged.
    unsafe { core::arch::x86_64::_rdtsc() }
}

/// A snapshot of the system's performance metrics.
#[derive(Debug, Clone, Copy)]
pub struct PerformanceCounters {
    /// Value of the TSC at the time the snapshot was taken.
    tsc: u64,
    /// Number of PIT timer ticks at the time the snapshot was taken.
    timer_ticks: u64,
}

impl PerformanceCounters {
    /// Capture a snapshot of all performance counters.
    pub fn read() -> Self {
        // Serialise the TSC read to prevent out-of-order execution.
        let tsc = rdtsc_serialised();
        let timer_ticks = TICK_COUNTER.load(Ordering::Relaxed);
        Self { tsc, timer_ticks }
    }

    /// Return the raw TSC value from this snapshot.
    pub fn tsc(&self) -> u64 {
        self.tsc
    }

    /// Return the timer-tick count from this snapshot.
    pub fn ticks(&self) -> u64 {
        self.timer_ticks
    }

    /// Estimate the CPU clock frequency in MHz.
    ///
    /// This heuristic assumes the PIT fires at exactly 100 Hz.  It takes two
    /// snapshots separated by a busy wait of ~10 ms and measures the TSC delta.
    /// The result is a rough estimate useful for display purposes.
    ///
    /// Returns 0 if timer interrupts are not firing or the measurement times out.
    pub fn cpu_frequency_mhz() -> u64 {
        const MAX_SPIN_ITERS: u64 = 1_000_000_000;

        let start = Self::read();
        let target_tick = start.ticks() + 10;
        let mut iters: u64 = 0;

        // Busy wait for ~10 timer ticks (≈100 ms at 100 Hz PIT rate).
        while TICK_COUNTER.load(Ordering::Relaxed) < target_tick {
            core::hint::spin_loop();
            iters += 1;
            if iters >= MAX_SPIN_ITERS {
                return 0; // Timed out: timer interrupts are not firing.
            }
        }
        let end = Self::read();

        let tsc_delta = end.tsc().saturating_sub(start.tsc());
        let tick_delta = end.ticks().saturating_sub(start.ticks());

        if tick_delta == 0 {
            return 0;
        }

        // Each tick ≈ 10 ms  ⟹  tick_delta ticks ≈ tick_delta × 10 ms.
        // freq [MHz] = cycles / (time_in_us) = tsc_delta / (tick_delta * 10_000)
        tsc_delta / (tick_delta * 10_000)
    }
}

/// RDTSC with CPUID serialisation to prevent instruction reordering.
fn rdtsc_serialised() -> u64 {
    // SAFETY: CPUID and RDTSC are always available on x86_64.
    unsafe {
        core::arch::x86_64::__cpuid(0);
        core::arch::x86_64::_rdtsc()
    }
}

// ─────────────────────────────────── tests ───────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_tsc_increases() {
        let a = rdtsc();
        let b = rdtsc();
        assert!(b >= a, "TSC should be monotonically non-decreasing");
    }

    #[test_case]
    fn test_tick_counter_readable() {
        let _ = TICK_COUNTER.load(Ordering::Relaxed);
    }

    #[test_case]
    fn test_performance_counters_read() {
        let counters = PerformanceCounters::read();
        // TSC should be non-zero after boot.
        assert!(counters.tsc() > 0);
    }
}
