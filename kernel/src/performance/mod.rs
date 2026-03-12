//! Performance monitoring module.
//!
//! Provides CPU metrics, high-resolution timing via the TSC, and a simple
//! profiler for measuring kernel subsystem latencies.

pub mod metrics;
pub mod profiler;

pub use metrics::PerformanceCounters;
pub use profiler::Profiler;
