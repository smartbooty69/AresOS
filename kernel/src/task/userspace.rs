//! Minimal user-space program facade for Phase 6 shell bring-up.

use alloc::{format, string::String};
use core::sync::atomic::Ordering;

pub fn run_program(name: &str, args: &[&str]) -> Result<String, &'static str> {
    match name {
        "echo" => {
            let mut out = String::new();
            for (idx, part) in args.iter().enumerate() {
                if idx > 0 {
                    out.push(' ');
                }
                out.push_str(part);
            }
            Ok(out)
        }
        "time" => {
            let ticks = crate::syscall::invoke_raw(crate::syscall::SyscallId::GetTickCount as u64, 0)
                .map_err(|_| "syscall failed")?;
            Ok(format!(
                "uptime_ticks={} uptime_secs={}",
                ticks,
                ticks / crate::task::timer::PIT_HZ
            ))
        }
        "sysinfo" => {
            let ticks =
                crate::syscall::invoke_raw(crate::syscall::SyscallId::GetTickCount as u64, 0)
                    .map_err(|_| "syscall failed")?;
            let procs =
                crate::syscall::invoke_raw(crate::syscall::SyscallId::GetProcessCount as u64, 0)
                    .map_err(|_| "syscall failed")?;
            let preemptions = crate::syscall::invoke_raw(
                crate::syscall::SyscallId::GetTotalPreemptions as u64,
                0,
            )
            .map_err(|_| "syscall failed")?;
            let tick_counter = crate::performance::metrics::TICK_COUNTER.load(Ordering::Relaxed);
            Ok(format!(
                "ticks={} process_count={} preemptions={} tick_counter={}",
                ticks, procs, preemptions, tick_counter
            ))
        }
        "fsinfo" => {
            let mounted =
                crate::syscall::invoke_raw(crate::syscall::SyscallId::StorageMounted as u64, 0)
                    .map_err(|_| "syscall failed")?;
            let files =
                crate::syscall::invoke_raw(crate::syscall::SyscallId::StorageFileCount as u64, 0)
                    .map_err(|_| "syscall failed")?;
            Ok(format!("mounted={} files={}", mounted == 1, files))
        }
        _ => Err("unknown user program"),
    }
}
