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
    security,
    syscall,
    task::{process, scheduler},
};
use x86_64::VirtAddr;

entry_point!(main);

fn main(boot_info: &'static BootInfo) -> ! {
    kernel::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    kernel::user_paging::init(phys_mem_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_map) };
    let heap_frames = frame_allocator.allocated_frame_count();

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialisation failed");
    let _ = kernel::frame_ownership::init_from_memory_map(
        &boot_info.memory_map,
        frame_allocator.allocated_frame_count(),
    );
    unsafe {
        kernel::user_paging::set_boot_frame_allocator(
            &boot_info.memory_map,
            heap_frames + kernel::frame_ownership::MAX_TRACKED_FRAMES,
        );
    }

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

#[test_case]
fn phase10_permission_predicates_cover_user_and_admin() {
    let user = security::Credentials::shell_user();
    let admin = security::Credentials::admin();
    assert!(security::can_access(
        user,
        user.user,
        security::FileMode::user_file(),
        security::AccessKind::Write
    )
    .is_ok());
    assert!(security::can_access(
        admin,
        user.user,
        security::FileMode::read_only(),
        security::AccessKind::Manage
    )
    .is_ok());
    assert!(security::can_access(
        user,
        admin.user,
        security::FileMode::system_executable(),
        security::AccessKind::Write
    )
    .is_err());
}

#[test_case]
fn phase10_checked_storage_enforces_file_policy() {
    kernel::storage::format().expect("format should seed protected files");
    let user = security::Credentials::shell_user();
    kernel::storage::write_file_checked(user, "/phase10.txt", "owned")
        .expect("user should write own file");
    assert_eq!(
        kernel::storage::read_file_checked(user, "/phase10.txt")
            .expect("user should read own file"),
        Some("owned".into())
    );
    let metadata = kernel::storage::stat_file("/phase10.txt")
        .expect("stat should succeed")
        .expect("file should exist");
    assert_eq!(metadata.owner, user.user);
    assert!(kernel::storage::write_file_checked(user, "/bin/echo", "blocked").is_err());
    kernel::storage::delete_file_checked(user, "/phase10.txt")
        .expect("user should delete own file");
}

#[test_case]
fn phase10_execute_permission_is_required_for_loader_launch() {
    kernel::storage::format().expect("format should seed executable manifests");
    let admin = security::Credentials::admin();
    let user = security::Credentials::shell_user();
    security::set_current_credentials(admin);
    kernel::storage::write_file(
        "/bin/blocked",
        "ares-exec-v1\nname=blocked\nkind=builtin-alias\nentry=echo\nrequires=execute\ntrust=system\nowner=admin\ndescription=Blocked test",
    )
    .expect("admin should seed test manifest");
    kernel::storage::chmod_execute_checked(admin, "/bin/blocked", false)
        .expect("admin should remove execute");

    security::set_current_credentials(user);
    let before = kernel::task::program_loader::status().denied_launch_count;
    assert_eq!(
        kernel::task::userspace::run_program("blocked", &["nope"]),
        Err("permission denied")
    );
    assert!(kernel::task::program_loader::status().denied_launch_count > before);

    security::set_current_credentials(admin);
    kernel::storage::delete_file("/bin/blocked").expect("cleanup should succeed");
    security::set_current_credentials(user);
}

#[test_case]
fn phase10_process_ownership_controls_termination() {
    let tick = TICK_COUNTER.load(Ordering::Relaxed);
    let admin = security::Credentials::admin();
    let user = security::Credentials::shell_user();
    let pid = process::create_kernel_process_as("phase10-owned", tick, admin)
        .expect("process should be created");
    assert!(!process::terminate_process_checked(user, pid, 0));
    assert!(process::terminate_process_checked(admin, pid, 0));
}

#[test_case]
fn phase10_security_syscalls_report_identity_and_denials() {
    security::set_current_credentials(security::Credentials::shell_user());
    kernel::storage::format().expect("format should seed protected files");
    let before = syscall::invoke_raw(syscall::SyscallId::DeniedAccessCount as u64, 0)
        .expect("denied counter syscall should succeed");
    assert!(kernel::storage::write_file_checked(
        security::Credentials::shell_user(),
        "/bin/echo",
        "blocked"
    )
    .is_err());
    assert_eq!(
        syscall::invoke_raw(syscall::SyscallId::CurrentUser as u64, 0),
        Ok(security::Credentials::shell_user().user.as_u64())
    );
    assert!(
        syscall::invoke_raw(syscall::SyscallId::DeniedAccessCount as u64, 0)
            .expect("denied counter syscall should succeed")
            > before
    );
    assert!(kernel::security::phase10_smoke_check());
    assert!(kernel::storage::phase10_smoke_check());
}

#[test_case]
fn phase11_elf_image_parser_validates_seed_fixture() {
    let image = kernel::exec_image::parse_elf64_image(
        "hello",
        "/bin/hello.elf",
        kernel::storage::phase11_sample_elf_image().as_bytes(),
        kernel::task::program_loader::ProgramTrust::User,
        security::Credentials::shell_user().user,
    )
    .expect("sample ELF image should parse");
    assert_eq!(image.format, kernel::exec_image::ExecutableFormat::Elf64);
    assert_eq!(image.entry_point, 0x400000);
    assert_eq!(image.segments.len(), 1);
}

#[test_case]
fn phase11_loader_discovers_and_validates_image_programs() {
    kernel::storage::format().expect("format should seed image manifests");
    let program = kernel::task::program_loader::program_info("hello")
        .expect("hello image manifest should be discoverable");
    assert_eq!(program.kind, kernel::task::program_loader::ProgramKind::Elf64Image);
    assert_eq!(program.image_path.as_deref(), Some("/bin/hello.elf"));
    assert!(program.image.is_some());
    let image = kernel::task::program_loader::validate_program_image(
        security::Credentials::shell_user(),
        "hello",
    )
    .expect("image should validate");
    let descriptor = kernel::address_space::descriptor_for_image(
        kernel::address_space::AddressSpaceId::from_raw(11),
        &image,
    )
    .expect("address-space descriptor should validate");
    assert_eq!(descriptor.regions.len(), 1);
}

#[test_case]
fn phase11_image_execution_reaches_guarded_mvp() {
    kernel::storage::format().expect("format should seed image manifests");
    security::set_current_credentials(security::Credentials::shell_user());
    let before = kernel::task::program_loader::status().unsupported_execution_count;
    let output = kernel::task::userspace::run_program("hello", &[]).expect("hello should execute");
    assert!(output.contains("hello"));
    assert!(kernel::task::program_loader::status().unsupported_execution_count > before);
}

#[test_case]
fn phase11_referenced_image_requires_execute_permission() {
    kernel::storage::format().expect("format should seed image manifests");
    let admin = security::Credentials::admin();
    kernel::storage::chmod_execute_checked(admin, "/bin/hello.elf", false)
        .expect("admin should remove execute from image");
    assert_eq!(
        kernel::task::program_loader::validate_program_image(
            security::Credentials::shell_user(),
            "hello"
        ),
        Err(kernel::task::program_loader::ProgramLoadError::PermissionDenied)
    );
    kernel::storage::chmod_execute_checked(admin, "/bin/hello.elf", true)
        .expect("admin should restore execute");
}

#[test_case]
fn phase11_status_syscalls_report_image_counts() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(syscall::invoke_raw(syscall::SyscallId::ImageCount as u64, 0).unwrap() >= 1);
    assert!(syscall::invoke_raw(syscall::SyscallId::ValidImageCount as u64, 0).unwrap() >= 1);
    assert_eq!(
        syscall::invoke_raw(syscall::SyscallId::ImageCount as u64, 1),
        Err(syscall::SyscallError::InvalidArgument)
    );
    assert!(kernel::task::program_loader::phase11_smoke_check());
}

#[test_case]
fn phase12_load_plan_generates_copy_and_zero_fill_actions() {
    let image = kernel::exec_image::parse_elf64_image(
        "hello",
        "/bin/hello.elf",
        kernel::storage::phase11_sample_elf_image().as_bytes(),
        kernel::task::program_loader::ProgramTrust::User,
        security::Credentials::shell_user().user,
    )
    .expect("sample ELF image should parse");
    let plan = kernel::load_plan::build_load_plan(&image).expect("load plan should build");
    assert_eq!(plan.total_pages, 1);
    assert_eq!(plan.stack_pages, kernel::load_plan::STACK_RESERVATION_PAGES);
    assert_eq!(plan.regions.len(), 1);
    assert!(matches!(
        plan.regions[0].actions[0],
        kernel::load_plan::LoadAction::Copy { len: 4, .. }
    ));
    assert!(matches!(
        plan.regions[0].actions[1],
        kernel::load_plan::LoadAction::ZeroFill { len: 4092, .. }
    ));
}

#[test_case]
fn phase12_load_plan_rejects_unsafe_or_invalid_regions() {
    let unsafe_region = kernel::load_plan::LoadRegion {
        start: 0x400000,
        size: kernel::load_plan::PAGE_SIZE,
        page_count: 1,
        permissions: kernel::load_plan::LoadPermissions::from_bits(
            kernel::load_plan::LoadPermissions::WRITE
                | kernel::load_plan::LoadPermissions::EXECUTE,
        ),
        actions: alloc::vec::Vec::new(),
    };
    assert_eq!(
        kernel::load_plan::validate_regions(&[unsafe_region]),
        Err(kernel::load_plan::LoadPlanError::WritableExecutable)
    );

    let image = kernel::exec_image::ExecutableImage {
        name: "bad-entry".into(),
        source_path: "/bin/bad.elf".into(),
        format: kernel::exec_image::ExecutableFormat::Elf64,
        entry_point: 0x500000,
        image_size: 128,
        trust: kernel::task::program_loader::ProgramTrust::User,
        owner: security::Credentials::shell_user().user,
        segments: alloc::vec![kernel::exec_image::ImageSegment {
            virtual_address: 0x400000,
            file_offset: 120,
            file_size: 4,
            memory_size: 0x1000,
            flags: kernel::exec_image::SegmentFlags::from_bits(
                kernel::exec_image::SegmentFlags::READ
                    | kernel::exec_image::SegmentFlags::EXECUTE,
            ),
        }],
    };
    assert_eq!(
        kernel::load_plan::build_load_plan(&image),
        Err(kernel::load_plan::LoadPlanError::EntryOutsideExecutableSegment)
    );
}

#[test_case]
fn phase12_loader_prepare_path_reports_status() {
    kernel::storage::format().expect("format should seed image manifests");
    let before = kernel::task::program_loader::status();
    let prepared = kernel::task::program_loader::prepare_program_image(
        security::Credentials::shell_user(),
        "hello",
    )
    .expect("prepare should succeed");
    assert_eq!(prepared.load_plan.total_pages, 1);
    assert_eq!(prepared.address_space.reservation.user_pages, 3);
    let after = kernel::task::program_loader::status();
    assert!(after.prepared_image_count > before.prepared_image_count);
    assert!(after.total_planned_pages > before.total_planned_pages);
}

#[test_case]
fn phase12_syscalls_and_smoke_report_load_plan_status() {
    kernel::storage::format().expect("format should seed image manifests");
    let before = syscall::invoke_raw(syscall::SyscallId::PreparedImageCount as u64, 0)
        .expect("prepared count syscall should succeed");
    kernel::task::program_loader::prepare_program_image(
        security::Credentials::shell_user(),
        "hello",
    )
    .expect("prepare should succeed");
    assert!(
        syscall::invoke_raw(syscall::SyscallId::PreparedImageCount as u64, 0)
            .expect("prepared count syscall should succeed")
            > before
    );
    assert!(syscall::invoke_raw(syscall::SyscallId::TotalPlannedPages as u64, 0).unwrap() > 0);
    assert_eq!(
        syscall::invoke_raw(syscall::SyscallId::PreparedImageCount as u64, 1),
        Err(syscall::SyscallError::InvalidArgument)
    );
    assert!(kernel::task::program_loader::phase12_smoke_check());
    assert!(kernel::task::userspace::run_program("hello", &[])
        .expect("hello should execute")
        .contains("hello"));
}

#[test_case]
fn phase13_mapping_stub_generates_frame_tokens_and_accounting() {
    let image = kernel::exec_image::parse_elf64_image(
        "hello",
        "/bin/hello.elf",
        kernel::storage::phase11_sample_elf_image().as_bytes(),
        kernel::task::program_loader::ProgramTrust::User,
        security::Credentials::shell_user().user,
    )
    .expect("sample ELF image should parse");
    let plan = kernel::load_plan::build_load_plan(&image).expect("load plan should build");
    let mapped = kernel::mapping_stub::map_load_plan(
        security::Credentials::shell_user(),
        kernel::mapping_stub::MappingId::from_raw(13),
        kernel::address_space::AddressSpaceId::from_raw(13),
        &plan,
    )
    .expect("mapping stub should build");
    assert_eq!(mapped.total_pages, plan.total_pages);
    assert_eq!(mapped.regions[0].pages.len(), plan.total_pages);
    assert_eq!(mapped.regions[0].pages[0].frame.as_u64(), 130_000);
    assert_eq!(mapped.copied_bytes, 4);
    assert_eq!(mapped.zero_filled_bytes, 4092);
    assert_eq!(mapped.state, kernel::address_space::MappingState::MappedStub);
}

#[test_case]
fn phase13_registry_add_list_lookup_and_status() {
    let image = kernel::exec_image::parse_elf64_image(
        "hello",
        "/bin/hello.elf",
        kernel::storage::phase11_sample_elf_image().as_bytes(),
        kernel::task::program_loader::ProgramTrust::User,
        security::Credentials::shell_user().user,
    )
    .expect("sample ELF image should parse");
    let plan = kernel::load_plan::build_load_plan(&image).expect("load plan should build");
    let before = kernel::mapping_stub::status();
    let mapped = kernel::mapping_stub::register_mapping(
        security::Credentials::shell_user(),
        kernel::address_space::AddressSpaceId::from_raw(14),
        &plan,
    )
    .expect("registry mapping should succeed");
    let listed = kernel::mapping_stub::list_mappings();
    assert!(listed.iter().any(|entry| entry.id == mapped.id));
    assert_eq!(
        kernel::mapping_stub::get_mapping(mapped.id)
            .expect("lookup should find mapping")
            .image_name,
        "hello"
    );
    let after = kernel::mapping_stub::status();
    assert!(after.mapped_count > before.mapped_count);
    assert!(after.total_pages >= before.total_pages + mapped.total_pages);
}

#[test_case]
fn phase13_mapping_rejects_unsafe_permissions() {
    let unsafe_plan = kernel::load_plan::LoadPlan {
        image_name: "unsafe".into(),
        source_path: "/bin/unsafe.elf".into(),
        entry_point: 0x400000,
        regions: alloc::vec![kernel::load_plan::LoadRegion {
            start: 0x400000,
            size: kernel::load_plan::PAGE_SIZE,
            page_count: 1,
            permissions: kernel::load_plan::LoadPermissions::from_bits(
                kernel::load_plan::LoadPermissions::WRITE
                    | kernel::load_plan::LoadPermissions::EXECUTE,
            ),
            actions: alloc::vec![],
        }],
        total_pages: 1,
        stack_pages: 0,
    };
    assert_eq!(
        kernel::mapping_stub::map_load_plan(
            security::Credentials::shell_user(),
            kernel::mapping_stub::MappingId::from_raw(99),
            kernel::address_space::AddressSpaceId::from_raw(99),
            &unsafe_plan,
        ),
        Err(kernel::mapping_stub::MappingStubError::UnsafePermissions)
    );
}

#[test_case]
fn phase13_loader_map_path_process_metadata_and_syscalls() {
    kernel::storage::format().expect("format should seed image manifests");
    let before = kernel::task::program_loader::status();
    let mapped = kernel::task::program_loader::map_prepared_program(
        security::Credentials::shell_user(),
        "hello",
    )
    .expect("map path should succeed");
    assert_eq!(mapped.mapped.total_pages, 1);
    assert_eq!(
        mapped.address_space.reservation.mapping_state,
        kernel::address_space::MappingState::MappedStub
    );

    let after = kernel::task::program_loader::status();
    assert!(after.mapped_image_count > before.mapped_image_count);
    assert!(after.total_mapped_pages > before.total_mapped_pages);
    assert!(after.copied_bytes > before.copied_bytes);
    assert!(after.zero_filled_bytes > before.zero_filled_bytes);

    let has_mapped_record = process::get_all_processes_with_details()
        .iter()
        .any(|(_, name, state, _, owner, _, load)| {
            *name == "image-mapped-stub"
                && *state == process::ProcessState::Blocked
                && *owner == security::Credentials::shell_user()
                && load
                    .as_ref()
                    .map(|load| {
                        load.state == process::ProcessLoadState::MappedStub
                            && load.mapping_id == Some(mapped.mapped.id)
                            && load.copied_bytes == mapped.mapped.copied_bytes
                            && load.zero_filled_bytes == mapped.mapped.zero_filled_bytes
                    })
                    .unwrap_or(false)
        });
    assert!(has_mapped_record);

    assert!(syscall::invoke_raw(syscall::SyscallId::MappedImageCount as u64, 0).unwrap() > 0);
    assert!(syscall::invoke_raw(syscall::SyscallId::TotalMappedPages as u64, 0).unwrap() > 0);
    assert!(syscall::invoke_raw(syscall::SyscallId::MappedCopiedBytes as u64, 0).unwrap() > 0);
    assert!(syscall::invoke_raw(syscall::SyscallId::MappedZeroFilledBytes as u64, 0).unwrap() > 0);
    assert_eq!(
        syscall::invoke_raw(syscall::SyscallId::MappedImageCount as u64, 1),
        Err(syscall::SyscallError::InvalidArgument)
    );
}

#[test_case]
fn phase13_smoke_preserves_guarded_execution() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase13_smoke_check());
    assert!(kernel::task::userspace::run_program("hello", &[])
        .expect("hello should execute")
        .contains("hello"));
}

#[test_case]
fn phase14_frame_ownership_allocates_releases_and_reports_status() {
    let before = kernel::frame_ownership::status();
    assert!(before.initialized);
    assert!(before.tracked_frames > 0);
    assert!(before.available_frames > 0);

    let frame = kernel::frame_ownership::allocate_frame(kernel::frame_ownership::FrameOwner::Test)
        .expect("owned frame should allocate");
    assert_eq!(frame.start_address % 4096, 0);

    let allocated = kernel::frame_ownership::status();
    assert_eq!(
        allocated.allocated_frames,
        before.allocated_frames.saturating_add(1)
    );

    kernel::frame_ownership::release_frame(frame.token).expect("owned frame should release");
    let released = kernel::frame_ownership::status();
    assert_eq!(released.allocated_frames, before.allocated_frames);
    assert!(released.release_count > before.release_count);
}

#[test_case]
fn phase14_frame_status_syscalls_and_smoke_work() {
    assert!(kernel::frame_ownership::phase14_smoke_check());
    assert!(syscall::invoke_raw(syscall::SyscallId::FrameTrackedCount as u64, 0).unwrap() > 0);
    assert!(syscall::invoke_raw(syscall::SyscallId::FrameAvailableCount as u64, 0).unwrap() > 0);
    assert_eq!(
        syscall::invoke_raw(syscall::SyscallId::FrameTrackedCount as u64, 1),
        Err(syscall::SyscallError::InvalidArgument)
    );
}

#[test_case]
fn phase15_frame_backing_consumes_owned_frames_and_accounts_actions() {
    kernel::storage::format().expect("format should seed image manifests");
    let frame_before = kernel::frame_ownership::status();
    let backed = kernel::task::program_loader::back_mapped_program(
        security::Credentials::shell_user(),
        "hello",
    )
    .expect("frame backing should succeed");
    assert_eq!(backed.backed.total_pages, 1);
    assert_eq!(backed.backed.copied_bytes, 4);
    assert_eq!(backed.backed.zero_filled_bytes, 4092);
    assert_eq!(
        backed.backed.state,
        kernel::address_space::MappingState::FrameBacked
    );
    assert_eq!(backed.backed.regions[0].pages[0].copied_bytes, 4);
    assert_eq!(backed.backed.regions[0].pages[0].zero_filled_bytes, 4092);
    assert!(
        kernel::frame_ownership::status().allocated_frames > frame_before.allocated_frames
    );
}

#[test_case]
fn phase15_loader_status_process_metadata_and_syscalls_report_backing() {
    kernel::storage::format().expect("format should seed image manifests");
    let before = kernel::task::program_loader::status();
    let backed = kernel::task::program_loader::back_mapped_program(
        security::Credentials::shell_user(),
        "hello",
    )
    .expect("frame backing should succeed");
    let after = kernel::task::program_loader::status();
    assert!(after.frame_backed_image_count > before.frame_backed_image_count);
    assert!(after.total_frame_backed_pages > before.total_frame_backed_pages);

    let has_backed_record = process::get_all_processes_with_details()
        .iter()
        .any(|(_, name, state, _, owner, _, load)| {
            *name == "image-frame-backed"
                && *state == process::ProcessState::Blocked
                && *owner == security::Credentials::shell_user()
                && load
                    .as_ref()
                    .map(|load| {
                        load.state == process::ProcessLoadState::FrameBacked
                            && load.mapping_id == Some(backed.backed.mapping_id)
                            && load.copied_bytes == backed.backed.copied_bytes
                    })
                    .unwrap_or(false)
        });
    assert!(has_backed_record);

    assert!(syscall::invoke_raw(syscall::SyscallId::FrameBackedImageCount as u64, 0).unwrap() > 0);
    assert!(syscall::invoke_raw(syscall::SyscallId::TotalFrameBackedPages as u64, 0).unwrap() > 0);
    assert_eq!(
        syscall::invoke_raw(syscall::SyscallId::FrameBackedImageCount as u64, 1),
        Err(syscall::SyscallError::InvalidArgument)
    );
    assert!(kernel::task::userspace::run_program("hello", &[])
        .expect("hello should execute")
        .contains("hello"));
}

#[test_case]
fn phase15_smoke_reports_frame_backed_image_status() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase15_smoke_check());
}

#[test_case]
fn phase16_inactive_page_table_translates_backed_pages() {
    kernel::storage::format().expect("format should seed image manifests");
    let built = kernel::task::program_loader::build_user_page_table(
        security::Credentials::shell_user(),
        "hello",
    )
    .expect("inactive page table should build");
    let page = built.backed.backed.regions[0].pages[0].virtual_address;
    assert_eq!(built.page_table.mapped_pages, built.backed.backed.total_pages);
    assert!(built.page_table.kernel_shared);
    assert!(!built.page_table.cr3_switch_ready);
    assert_eq!(
        kernel::user_memory::translate(&built.page_table, page),
        Some(built.backed.backed.regions[0].pages[0].frame.start_address)
    );
    assert_eq!(
        kernel::user_memory::translate(&built.page_table, page + 3),
        Some(built.backed.backed.regions[0].pages[0].frame.start_address + 3)
    );
}

#[test_case]
fn phase16_loader_process_metadata_syscalls_and_smoke_work() {
    kernel::storage::format().expect("format should seed image manifests");
    let before = kernel::task::program_loader::status();
    let built = kernel::task::program_loader::build_user_page_table(
        security::Credentials::shell_user(),
        "hello",
    )
    .expect("inactive page table should build");
    let after = kernel::task::program_loader::status();
    assert!(after.user_page_table_count > before.user_page_table_count);
    assert!(after.total_user_page_table_pages > before.total_user_page_table_pages);

    let has_page_table_record = process::get_all_processes_with_details()
        .iter()
        .any(|(_, name, state, _, owner, _, load)| {
            *name == "image-page-table"
                && *state == process::ProcessState::Blocked
                && *owner == security::Credentials::shell_user()
                && load
                    .as_ref()
                    .map(|load| {
                        load.state == process::ProcessLoadState::PageTableReady
                            && load.mapping_id == Some(built.backed.backed.mapping_id)
                    })
                    .unwrap_or(false)
        });
    assert!(has_page_table_record);

    assert!(syscall::invoke_raw(syscall::SyscallId::UserPageTableCount as u64, 0).unwrap() > 0);
    assert!(syscall::invoke_raw(syscall::SyscallId::TotalUserPageTablePages as u64, 0).unwrap() > 0);
    assert_eq!(
        syscall::invoke_raw(syscall::SyscallId::UserPageTableCount as u64, 1),
        Err(syscall::SyscallError::InvalidArgument)
    );
    assert!(kernel::task::program_loader::phase16_smoke_check());
}

#[test_case]
fn phase17_user_selectors_and_entry_context_are_valid() {
    kernel::storage::format().expect("format should seed image manifests");
    let prepared = kernel::task::program_loader::prepare_user_context(
        security::Credentials::shell_user(),
        "hello",
    )
    .expect("user context should prepare");
    assert!(prepared.context.selectors_ready);
    assert!(prepared.context.entry_ready);
    assert!(!prepared.context.ring3_entered);
    assert_ne!(prepared.context.entry.code_selector, 0);
    assert_ne!(prepared.context.entry.stack_selector, 0);
    assert_eq!(prepared.context.entry.rflags & 0x200, 0x200);
    assert_eq!(
        prepared.context.entry.rip,
        prepared.page_table.backed.mapped.prepared.load_plan.entry_point
    );
}

#[test_case]
fn phase17_loader_process_metadata_syscalls_and_smoke_work() {
    kernel::storage::format().expect("format should seed image manifests");
    let before = kernel::task::program_loader::status();
    let prepared = kernel::task::program_loader::prepare_user_context(
        security::Credentials::shell_user(),
        "hello",
    )
    .expect("user context should prepare");
    let after = kernel::task::program_loader::status();
    assert!(after.user_context_count > before.user_context_count);

    let has_context_record = process::get_all_processes_with_details()
        .iter()
        .any(|(_, name, state, _, owner, _, load)| {
            *name == "image-user-context"
                && *state == process::ProcessState::Blocked
                && *owner == security::Credentials::shell_user()
                && load
                    .as_ref()
                    .map(|load| {
                        load.state == process::ProcessLoadState::UserContextReady
                            && load.entry_point == prepared.context.entry.rip
                    })
                    .unwrap_or(false)
        });
    assert!(has_context_record);

    assert!(syscall::invoke_raw(syscall::SyscallId::UserContextCount as u64, 0).unwrap() > 0);
    assert_eq!(
        syscall::invoke_raw(syscall::SyscallId::UserContextCount as u64, 1),
        Err(syscall::SyscallError::InvalidArgument)
    );
    assert!(kernel::task::program_loader::phase17_smoke_check());
}

#[test_case]
fn phase18_controlled_ring3_trampoline_enters_and_traps_back() {
    kernel::storage::format().expect("format should seed image manifests");
    let entered = kernel::task::program_loader::enter_controlled_ring3_trampoline(
        security::Credentials::shell_user(),
        "hello",
    )
    .expect("controlled ring3 trampoline should run");
    assert!(entered.result.ring3_entered);
    assert!(entered.result.trapped_back);
    assert_eq!(entered.result.trap_vector, kernel::interrupts::USER_TRAP_VECTOR);
}

#[test_case]
fn phase18_process_metadata_syscalls_and_smoke_work() {
    kernel::storage::format().expect("format should seed image manifests");
    let before = kernel::task::program_loader::status();
    let entered = kernel::task::program_loader::enter_controlled_ring3_trampoline(
        security::Credentials::shell_user(),
        "hello",
    )
    .expect("controlled ring3 trampoline should run");
    let after = kernel::task::program_loader::status();
    assert!(after.ring3_entry_count > before.ring3_entry_count);
    assert!(after.ring3_trap_count > before.ring3_trap_count);

    let has_trap_record = process::get_all_processes_with_details()
        .iter()
        .any(|(_, name, state, _, owner, _, load)| {
            *name == "image-ring3-trap"
                && *state == process::ProcessState::Blocked
                && *owner == security::Credentials::shell_user()
                && load
                    .as_ref()
                    .map(|load| {
                        load.state == process::ProcessLoadState::UserTrapped
                            && load.entry_point == entered.result.entry_rip
                    })
                    .unwrap_or(false)
        });
    assert!(has_trap_record);

    assert!(syscall::invoke_raw(syscall::SyscallId::Ring3EntryCount as u64, 0).unwrap() > 0);
    assert!(syscall::invoke_raw(syscall::SyscallId::Ring3TrapCount as u64, 0).unwrap() > 0);
    assert_eq!(
        syscall::invoke_raw(syscall::SyscallId::Ring3EntryCount as u64, 1),
        Err(syscall::SyscallError::InvalidArgument)
    );
    assert!(kernel::task::program_loader::phase18_smoke_check());
}

#[test_case]
fn phase19_user_syscall_abi_dispatches_and_returns() {
    let returned = kernel::user_syscall::dispatch_from_user(kernel::user_syscall::tick_probe_frame())
        .expect("user syscall frame should dispatch");
    assert_eq!(returned.syscall_id, syscall::SyscallId::GetTickCount as u64);
    assert_eq!(returned.error, None);
    assert!(returned.returned_to_user);
}

#[test_case]
fn phase19_loader_process_metadata_syscalls_and_smoke_work() {
    kernel::storage::format().expect("format should seed image manifests");
    let before = kernel::task::program_loader::status();
    let probe = kernel::task::program_loader::run_user_syscall_probe(
        security::Credentials::shell_user(),
        "hello",
    )
    .expect("user syscall probe should run");
    let after = kernel::task::program_loader::status();
    assert!(probe.syscall_return.returned_to_user);
    assert!(after.user_syscall_count > before.user_syscall_count);
    assert!(after.user_syscall_return_count > before.user_syscall_return_count);

    let has_syscall_record = process::get_all_processes_with_details()
        .iter()
        .any(|(_, name, state, _, owner, _, load)| {
            *name == "image-user-syscall"
                && *state == process::ProcessState::Blocked
                && *owner == security::Credentials::shell_user()
                && load
                    .as_ref()
                    .map(|load| load.state == process::ProcessLoadState::UserSyscallReturned)
                    .unwrap_or(false)
        });
    assert!(has_syscall_record);

    assert!(syscall::invoke_raw(syscall::SyscallId::UserSyscallCount as u64, 0).unwrap() > 0);
    assert!(syscall::invoke_raw(syscall::SyscallId::UserSyscallReturnCount as u64, 0).unwrap() > 0);
    assert_eq!(
        syscall::invoke_raw(syscall::SyscallId::UserSyscallCount as u64, 1),
        Err(syscall::SyscallError::InvalidArgument)
    );
    assert!(kernel::task::program_loader::phase19_smoke_check());
}

#[test_case]
fn phase20_run_hello_returns_guarded_elf_output() {
    kernel::storage::format().expect("format should seed image manifests");
    security::set_current_credentials(security::Credentials::shell_user());
    let output = kernel::task::userspace::run_program("hello", &[]).expect("hello should execute");
    assert!(output.contains("hello"));
    assert!(output.contains("exit=0"));
}

#[test_case]
fn phase20_loader_process_metadata_syscalls_and_smoke_work() {
    kernel::storage::format().expect("format should seed image manifests");
    let before = kernel::task::program_loader::status();
    let execution = kernel::task::program_loader::execute_minimal_user_elf(
        security::Credentials::shell_user(),
        "hello",
    )
    .expect("hello should execute");
    let after = kernel::task::program_loader::status();
    assert_eq!(execution.exit_code, 0);
    assert!(after.user_elf_execution_count > before.user_elf_execution_count);
    assert!(after.user_elf_exit_count > before.user_elf_exit_count);

    let has_elf_record = process::get_all_processes_with_details()
        .iter()
        .any(|(_, name, state, _, owner, _, load)| {
            *name == "image-user-elf"
                && *state == process::ProcessState::Blocked
                && *owner == security::Credentials::shell_user()
                && load
                    .as_ref()
                    .map(|load| load.state == process::ProcessLoadState::UserElfExited)
                    .unwrap_or(false)
        });
    assert!(has_elf_record);

    assert!(syscall::invoke_raw(syscall::SyscallId::UserElfExecutionCount as u64, 0).unwrap() > 0);
    assert!(syscall::invoke_raw(syscall::SyscallId::UserElfExitCount as u64, 0).unwrap() > 0);
    assert_eq!(
        syscall::invoke_raw(syscall::SyscallId::UserElfExecutionCount as u64, 1),
        Err(syscall::SyscallError::InvalidArgument)
    );
    assert!(kernel::task::program_loader::phase20_smoke_check());
}

#[test_case]
fn phase21_hw_page_tables_smoke_works() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase21_smoke_check());
}

#[test_case]
fn phase22_cr3_activation_smoke_works() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase22_smoke_check());
}

#[test_case]
fn phase23_iretq_smoke_works() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase23_smoke_check());
}

#[test_case]
fn phase24_user_trap_smoke_works() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase24_smoke_check());
}

#[test_case]
fn phase25_hw_syscall_smoke_works() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase25_smoke_check());
}

#[test_case]
fn phase26_user_copy_smoke_works() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase26_smoke_check());
}

#[test_case]
fn phase27_reloc_smoke_works() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase27_smoke_check());
}

#[test_case]
fn phase28_hw_hello_smoke_works() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase28_smoke_check());
}

#[test_case]
fn phase29_allowlist_smoke_works() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase29_smoke_check());
}

#[test_case]
fn phase30_cr3_switch_smoke_works() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase30_cr3_switch_smoke());
}

#[test_case]
fn phase31_sched_cr3_smoke_works() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase31_sched_cr3_smoke());
}

#[test_case]
fn phase32_user_frame_smoke_works() {
    assert!(kernel::task::program_loader::phase32_user_frame_smoke());
}

#[test_case]
fn phase33_multi_elf_smoke_works() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase33_multi_elf_smoke());
}

#[test_case]
fn phase34_exit_wait_smoke_works() {
    assert!(kernel::task::program_loader::phase34_exit_wait_smoke());
}

#[test_case]
fn phase35_syscall_table_smoke_works() {
    assert!(kernel::task::program_loader::phase35_syscall_table_smoke());
}

#[test_case]
fn phase36_storage_copyin_smoke_works() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase36_storage_copyin_smoke());
}

#[test_case]
fn phase37_manifest_elf_smoke_works() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase37_manifest_elf_smoke());
}

#[test_case]
fn phase38_demand_zero_smoke_works() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase38_demand_zero_smoke());
}

#[test_case]
fn phase39_dynamic_smoke_works() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase39_dynamic_smoke());
}

#[test_case]
fn phase40_integration_smoke_works() {
    kernel::storage::format().expect("format should seed image manifests");
    assert!(kernel::task::program_loader::phase40_integration_smoke());
}
