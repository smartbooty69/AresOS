//! Integration tests for Phase 5 preemption and process foundations.

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::{panic::PanicInfo, sync::atomic::Ordering};
use kernel::{
    allocator, block, device, hlt_loop, memory,
    performance::{metrics::TICK_COUNTER, process_metrics},
    syscall,
    task::{process, scheduler},
};
use x86_64::VirtAddr;

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    kernel::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialisation failed");

    test_main();
    hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}

#[test_case]
fn preemption_tick_requests_accumulate() {
    let before = scheduler::stats();
    for _ in 0..(scheduler::SCHED_QUANTUM_TICKS * 2) {
        scheduler::on_timer_tick();
    }
    let after = scheduler::stats();

    assert!(after.timer_ticks >= before.timer_ticks + scheduler::SCHED_QUANTUM_TICKS * 2);
    assert!(after.reschedule_requests >= before.reschedule_requests + 2);
}

#[test_case]
fn process_registry_lifecycle() {
    let created_tick = TICK_COUNTER.load(Ordering::Relaxed);
    let before_count = process::process_count();

    let pid = process::create_kernel_process("phase5-proc", created_tick)
        .expect("process should be created");

    assert!(process::process_count() >= before_count + 1);

    assert!(process::set_process_state(pid, process::ProcessState::Ready));
    let ready = process::get_ready_processes();
    assert!(ready.iter().any(|p| *p == pid));

    assert!(process::add_process_cpu_ticks(pid, 42));
    assert!(process::record_context_switch(pid));

    assert!(process::terminate_process(pid, 0));
    let reaped = process::reap_terminated_processes();
    assert!(reaped >= 1);
}

#[test_case]
fn fairness_metrics_detect_imbalance() {
    let balanced = [
        (1u64, "p1", 1000u64),
        (2u64, "p2", 1005u64),
        (3u64, "p3", 1002u64),
        (4u64, "p4", 1001u64),
    ];
    let balanced_metrics = process_metrics::compute_fairness_metrics(&balanced);
    assert!(balanced_metrics.is_fair());

    let imbalanced = [
        (1u64, "p1", 5000u64),
        (2u64, "p2", 1000u64),
        (3u64, "p3", 1000u64),
        (4u64, "p4", 1000u64),
    ];
    let imbalanced_metrics = process_metrics::compute_fairness_metrics(&imbalanced);
    assert!(!imbalanced_metrics.is_fair());
    assert!(imbalanced_metrics.has_severe_starvation());
}

#[test_case]
fn syscall_invalid_paths_are_rejected() {
    assert_eq!(
        syscall::invoke_raw(999, 0),
        Err(syscall::SyscallError::InvalidSyscall)
    );
    assert_eq!(
        syscall::invoke_raw(syscall::SyscallId::GetTickCount as u64, 123),
        Err(syscall::SyscallError::InvalidArgument)
    );
    assert_eq!(
        syscall::invoke_raw(syscall::SyscallId::StorageFileCount as u64, 123),
        Err(syscall::SyscallError::InvalidArgument)
    );
}

#[test_case]
fn process_lifecycle_unknown_pid_operations_fail() {
    let missing = process::ProcessId::from_raw(u64::MAX);
    assert!(!process::set_process_state(missing, process::ProcessState::Ready));
    assert!(!process::add_process_cpu_ticks(missing, 1));
    assert!(!process::record_context_switch(missing));
    assert!(!process::terminate_process(missing, -1));
}

#[test_case]
fn storage_and_userspace_smoke_flow() {
    kernel::storage::init();
    let files = kernel::storage::list_files().expect("storage should be mounted");
    assert!(!files.is_empty());
    let readme = kernel::storage::read_file("/README.txt")
        .expect("storage read should be available")
        .expect("README should exist");
    assert!(readme.contains("AresOS"));

    let output = kernel::task::userspace::run_program("echo", &["ok", "flow"])
        .expect("echo should run");
    assert_eq!(output, "ok flow");

    let fsinfo = kernel::task::userspace::run_program("fsinfo", &[])
        .expect("fsinfo should run through storage syscalls");
    assert!(fsinfo.contains("mounted=true"));
}

#[test_case]
fn phase7_storage_persists_across_remount() {
    kernel::storage::format().expect("format should succeed");
    kernel::storage::write_file("/phase7.txt", "persistent")
        .expect("write should succeed");
    kernel::storage::remount().expect("remount should succeed");

    let contents = kernel::storage::read_file("/phase7.txt")
        .expect("read should succeed")
        .expect("file should exist after remount");
    assert_eq!(contents, "persistent");

    kernel::storage::delete_file("/phase7.txt").expect("delete should succeed");
    assert_eq!(
        kernel::storage::read_file("/phase7.txt").expect("read should succeed"),
        None
    );
}

#[test_case]
fn phase7_storage_syscall_wrappers_cover_file_lifecycle() {
    kernel::storage::format().expect("format should succeed");
    syscall::storage_write_file("/syscall.txt", "through-syscall")
        .expect("storage write syscall wrapper should succeed");
    assert_eq!(
        syscall::storage_read_file("/syscall.txt")
            .expect("storage read syscall wrapper should succeed"),
        Some("through-syscall".into())
    );
    assert!(
        syscall::storage_list_files()
            .expect("storage list syscall wrapper should succeed")
            .iter()
            .any(|path| path == "/syscall.txt")
    );
    syscall::storage_delete_file("/syscall.txt")
        .expect("storage delete syscall wrapper should succeed");
    assert_eq!(
        syscall::storage_read_file("/syscall.txt")
            .expect("storage read syscall wrapper should succeed"),
        None
    );
}

#[test_case]
fn phase8_device_and_block_registries_initialize() {
    device::init();
    block::init();

    let device_summary = device::summary();
    assert!(device_summary.total > 0);
    assert!(device_summary.block >= 1);

    let blocks = block::list_block_devices();
    assert!(!blocks.is_empty());
    assert!(blocks.iter().any(|entry| entry.driver_backed));
}

#[test_case]
fn phase8_storage_uses_driver_backed_block_manager() {
    kernel::storage::init();
    let info = kernel::storage::info().expect("storage info should be available");
    assert!(info.mounted);
    assert!(info.driver_backed);
    assert_eq!(info.backend_name, "qemu-sim-block0");
    assert!(kernel::storage::phase8_smoke_check());
}

#[test_case]
fn phase8_device_syscalls_report_counts() {
    kernel::storage::init();
    assert!(syscall::invoke_raw(syscall::SyscallId::DeviceCount as u64, 0).unwrap() > 0);
    assert!(syscall::invoke_raw(syscall::SyscallId::BlockDeviceCount as u64, 0).unwrap() > 0);
    assert_eq!(
        syscall::invoke_raw(syscall::SyscallId::DeviceCount as u64, 1),
        Err(syscall::SyscallError::InvalidArgument)
    );
}

#[test_case]
fn phase9_program_manifest_parser_rejects_invalid_records() {
    use kernel::task::program_loader::{parse_manifest, ProgramKind, ProgramLoadError};

    let valid = parse_manifest(
        "ares-exec-v1\nname=echo\nkind=builtin-alias\nentry=echo\ndescription=Echo text",
    )
    .expect("valid manifest should parse");
    assert_eq!(valid.name, "echo");
    assert_eq!(valid.kind, ProgramKind::BuiltinAlias);
    assert_eq!(
        parse_manifest("bad\nname=echo\nkind=builtin-alias\nentry=echo"),
        Err(ProgramLoadError::InvalidVersion)
    );
    assert_eq!(
        parse_manifest("ares-exec-v1\nkind=builtin-alias\nentry=echo"),
        Err(ProgramLoadError::MissingName)
    );
}

#[test_case]
fn phase9_loader_discovers_bin_programs() {
    kernel::storage::format().expect("format should seed executable manifests");
    let programs = kernel::task::program_loader::discover_programs();
    assert!(programs.iter().any(|program| program.name == "echo"));
    assert!(programs.iter().any(|program| program.source_path == "/bin/fsinfo"));
}

#[test_case]
fn phase9_run_program_uses_loader_path() {
    kernel::storage::format().expect("format should seed executable manifests");
    let before = kernel::task::program_loader::status().launch_count;
    let output = kernel::task::userspace::run_program("echo", &["from", "loader"])
        .expect("echo should run through loader");
    assert_eq!(output, "from loader");
    assert!(kernel::task::program_loader::status().launch_count > before);
}

#[test_case]
fn phase9_malformed_program_file_does_not_panic() {
    kernel::storage::format().expect("format should succeed");
    kernel::storage::write_file("/bin/bad", "not-a-manifest").expect("write should succeed");
    let programs = kernel::task::program_loader::discover_programs();
    assert!(!programs.iter().any(|program| program.name == "bad"));
    assert_eq!(
        kernel::task::program_loader::program_info("bad"),
        Err(kernel::task::program_loader::ProgramLoadError::NotFound)
    );
}

#[test_case]
fn phase9_loader_syscalls_report_status() {
    kernel::storage::format().expect("format should seed executable manifests");
    assert!(syscall::invoke_raw(syscall::SyscallId::ProgramCount as u64, 0).unwrap() >= 4);
    assert_eq!(
        syscall::invoke_raw(syscall::SyscallId::ProgramLaunchCount as u64, 1),
        Err(syscall::SyscallError::InvalidArgument)
    );
    assert!(kernel::task::program_loader::phase9_smoke_check());
}
