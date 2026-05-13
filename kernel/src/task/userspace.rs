//! User-space program facade and Phase 9 loader dispatch.

use alloc::{format, string::String};
use core::sync::atomic::Ordering;

pub fn run_program(name: &str, args: &[&str]) -> Result<String, &'static str> {
    let credentials = crate::security::current_credentials();
    let program = match crate::task::program_loader::resolve_program_for(credentials, name) {
        Ok(program) => program,
        Err(crate::task::program_loader::ProgramLoadError::PermissionDenied) => {
            return Err("permission denied");
        }
        Err(crate::task::program_loader::ProgramLoadError::UnsupportedExecution) => {
            match crate::task::program_loader::execute_minimal_user_elf(credentials, name) {
                Ok(execution) => {
                    crate::task::program_loader::record_launch_success();
                    return Ok(execution.output);
                }
                Err(_) => return Err("unsupported executable image"),
            }
        }
        Err(crate::task::program_loader::ProgramLoadError::ImageInvalid) => {
            return Err("invalid executable image");
        }
        Err(_) if is_builtin_entry(name) => crate::task::program_loader::LoadedProgram {
            name: String::from(name),
            source_path: String::from("<builtin>"),
            kind: crate::task::program_loader::ProgramKind::BuiltinAlias,
            entry: String::from(name),
            image_path: None,
            description: String::new(),
            requires_execute: false,
            trust: crate::task::program_loader::ProgramTrust::System,
            owner: String::from("kernel"),
            image: None,
            image_error: None,
        },
        Err(_) => {
            crate::task::program_loader::record_launch_failure();
            return Err("program not found");
        }
    };

    let result = dispatch_builtin(&program.entry, args);
    match result {
        Ok(output) => {
            record_program_process(credentials, &program);
            crate::task::program_loader::record_launch_success();
            Ok(output)
        }
        Err(err) => {
            crate::task::program_loader::record_launch_failure();
            Err(err)
        }
    }
}

fn dispatch_builtin(entry: &str, args: &[&str]) -> Result<String, &'static str> {
    match entry {
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
            let blocks =
                crate::syscall::invoke_raw(crate::syscall::SyscallId::BlockDeviceCount as u64, 0)
                    .map_err(|_| "syscall failed")?;
            let programs =
                crate::syscall::invoke_raw(crate::syscall::SyscallId::ProgramCount as u64, 0)
                    .map_err(|_| "syscall failed")?;
            Ok(format!(
                "mounted={} files={} block_devices={} programs={}",
                mounted == 1,
                files,
                blocks,
                programs
            ))
        }
        _ => Err("unknown user program"),
    }
}

fn is_builtin_entry(name: &str) -> bool {
    matches!(name, "echo" | "time" | "sysinfo" | "fsinfo")
}

fn record_program_process(
    credentials: crate::security::Credentials,
    program: &crate::task::program_loader::LoadedProgram,
) {
    let tick =
        crate::performance::metrics::TICK_COUNTER.load(core::sync::atomic::Ordering::Relaxed);
    let image = crate::task::process::ProcessImageMetadata {
        source_path: static_source_path(&program.source_path),
        format: crate::exec_image::ExecutableFormat::BuiltinAlias,
        entry_point: 0,
        segment_count: 0,
        address_space_id: None,
        trust: program.trust,
        owner: credentials,
    };
    if let Some(pid) =
        crate::task::process::create_kernel_process_as_with_image("program", tick, credentials, image)
    {
        let _ = crate::task::process::set_process_state(pid, crate::task::process::ProcessState::Ready);
        let _ = crate::task::process::terminate_process(pid, 0);
    }
}

fn static_source_path(path: &str) -> &'static str {
    match path {
        "/bin/echo" => "/bin/echo",
        "/bin/time" => "/bin/time",
        "/bin/sysinfo" => "/bin/sysinfo",
        "/bin/fsinfo" => "/bin/fsinfo",
        _ => "<builtin>",
    }
}
