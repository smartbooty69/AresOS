//! Kernel entry point.

#![no_std]
#![no_main]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;
use kernel::{
    allocator, hlt_loop, memory,
    performance::metrics::PerformanceCounters,
    println,
    task::{executor::Executor, keyboard, timer, Task},
};
use x86_64::VirtAddr;

entry_point!(kernel_main);

fn run_phase21_to_30_smokes() {
    let phase21_ok = kernel::task::program_loader::phase21_smoke_check();
    let (hw_built, hw_verified, hw_rejected, _, _, _, _) = kernel::user_paging::status();
    println!(
        "Phase21-HwPageTables: built={}, verified={}, rejected={}, tables_ok={}",
        hw_built, hw_verified, hw_rejected, phase21_ok
    );
    kernel::serial_println!(
        "Phase21-HwPageTables: built={}, verified={}, rejected={}, tables_ok={}",
        hw_built, hw_verified, hw_rejected, phase21_ok
    );
    let phase22_ok = kernel::task::program_loader::phase22_smoke_check();
    let (cr3_act, cr3_restore, _, _, _, _, _) = kernel::user_paging::status();
    println!(
        "Phase22-Cr3: activations={}, restores={}, verify_ok={}",
        cr3_act, cr3_restore, phase22_ok
    );
    kernel::serial_println!(
        "Phase22-Cr3: activations={}, restores={}, verify_ok={}",
        cr3_act, cr3_restore, phase22_ok
    );
    let phase23_ok = kernel::task::program_loader::phase23_smoke_check();
    let (iretq_entries, iretq_trapped, _, _) = kernel::user_entry::status();
    println!(
        "Phase23-Iretq: entries={}, trapped={}, entry_ok={}",
        iretq_entries, iretq_trapped, phase23_ok
    );
    kernel::serial_println!(
        "Phase23-Iretq: entries={}, trapped={}, entry_ok={}",
        iretq_entries, iretq_trapped, phase23_ok
    );
    let phase24_ok = kernel::task::program_loader::phase24_smoke_check();
    let (trap_count, trap_returns, _, _) = kernel::user_entry::status();
    println!(
        "Phase24-UserTrap: traps={}, returns={}, vector_ok={}",
        trap_count, trap_returns, phase24_ok
    );
    kernel::serial_println!(
        "Phase24-UserTrap: traps={}, returns={}, vector_ok={}",
        trap_count, trap_returns, phase24_ok
    );
    let phase25_ok = kernel::task::program_loader::phase25_smoke_check();
    let (hw_syscalls, hw_sysrets) = kernel::user_syscall_hw::status();
    println!(
        "Phase25-SyscallHw: syscalls={}, sysrets={}, hw_ok={}",
        hw_syscalls, hw_sysrets, phase25_ok
    );
    kernel::serial_println!(
        "Phase25-SyscallHw: syscalls={}, sysrets={}, hw_ok={}",
        hw_syscalls, hw_sysrets, phase25_ok
    );
    let phase26_ok = kernel::task::program_loader::phase26_smoke_check();
    let (copy_ok_count, copy_rejected) = kernel::user_copy::status();
    println!(
        "Phase26-Copyin: copies={}, rejected={}, copy_ok={}",
        copy_ok_count, copy_rejected, phase26_ok
    );
    kernel::serial_println!(
        "Phase26-Copyin: copies={}, rejected={}, copy_ok={}",
        copy_ok_count, copy_rejected, phase26_ok
    );
    let phase27_ok = kernel::task::program_loader::phase27_smoke_check();
    let (reloc_applied, reloc_rejected) = kernel::elf_reloc::status();
    println!(
        "Phase27-Reloc: applied={}, rejected={}, reloc_ok={}",
        reloc_applied, reloc_rejected, phase27_ok
    );
    kernel::serial_println!(
        "Phase27-Reloc: applied={}, rejected={}, reloc_ok={}",
        reloc_applied, reloc_rejected, phase27_ok
    );
    let phase28_ok = kernel::task::program_loader::phase28_smoke_check();
    let hw_elf_status = kernel::task::program_loader::status();
    println!(
        "Phase28-HwHello: executions={}, exits={}, hello_hw_ok={}",
        hw_elf_status.hw_elf_execution_count,
        hw_elf_status.user_elf_exit_count,
        phase28_ok
    );
    kernel::serial_println!(
        "Phase28-HwHello: executions={}, exits={}, hello_hw_ok={}",
        hw_elf_status.hw_elf_execution_count,
        hw_elf_status.user_elf_exit_count,
        phase28_ok
    );
    let phase29_ok = kernel::task::program_loader::phase29_smoke_check();
    println!(
        "Phase29-Allowlist: programs=2, exit42_ok={}, hello_ok={}",
        phase29_ok, phase28_ok
    );
    kernel::serial_println!(
        "Phase29-Allowlist: programs=2, exit42_ok={}, hello_ok={}",
        phase29_ok, phase28_ok
    );
    let phase30_ok = kernel::task::program_loader::phase30_cr3_switch_smoke();
    let (_, _, _, _, _, cr3_switches, isolated) = kernel::user_paging::status();
    println!(
        "Phase30-Cr3Switch: switches={}, isolated={}, switch_ok={}",
        cr3_switches, isolated, phase30_ok
    );
    kernel::serial_println!(
        "Phase30-Cr3Switch: switches={}, isolated={}, switch_ok={}",
        cr3_switches, isolated, phase30_ok
    );
    kernel::task::program_loader::set_hw_user_elf_ready();
}

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("AresOS v{} booting...", env!("CARGO_PKG_VERSION"));

    kernel::init();

    // Initialise memory subsystem.
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    kernel::user_paging::init(phys_mem_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_map) };
    let heap_frames = frame_allocator.allocated_frame_count();

    // Set up the kernel heap.
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialisation failed");
    kernel::frame_ownership::init_from_memory_map(
        &boot_info.memory_map,
        frame_allocator.allocated_frame_count(),
    )
    .expect("frame ownership initialisation failed");
    let skip_frames = heap_frames + kernel::frame_ownership::MAX_TRACKED_FRAMES;
    unsafe {
        kernel::user_paging::set_boot_frame_allocator(&boot_info.memory_map, skip_frames);
    }
    kernel::task::keyboard::init_scancode_queue();
    kernel::storage::init();
    let boot_tick = kernel::performance::metrics::TICK_COUNTER.load(core::sync::atomic::Ordering::Relaxed);
    let _ = kernel::task::process::create_kernel_process("shell", boot_tick);

    println!("Memory subsystem initialised.");
    let storage_smoke_ok = match kernel::storage::list_files() {
        Ok(files) => !files.is_empty(),
        Err(_) => false,
    };
    let readme_smoke_ok = matches!(kernel::storage::read_file("/README.txt"), Ok(Some(_)));
    let run_smoke_ok = kernel::task::userspace::run_program("echo", &["phase6-smoke"]).is_ok();
    println!(
        "Phase6-Smoke: mounted={}, list_ok={}, cat_ok={}, run_ok={}",
        kernel::storage::is_mounted(),
        storage_smoke_ok,
        readme_smoke_ok,
        run_smoke_ok
    );
    kernel::serial_println!(
        "Phase6-Smoke: mounted={}, list_ok={}, cat_ok={}, run_ok={}",
        kernel::storage::is_mounted(),
        storage_smoke_ok,
        readme_smoke_ok,
        run_smoke_ok
    );
    let phase7_storage_ok = kernel::storage::phase7_smoke_check();
    println!(
        "Phase7-Storage: mounted={}, persistent_rw_ok={}",
        kernel::storage::is_mounted(),
        phase7_storage_ok
    );
    kernel::serial_println!(
        "Phase7-Storage: mounted={}, persistent_rw_ok={}",
        kernel::storage::is_mounted(),
        phase7_storage_ok
    );
    let phase8_storage_ok = kernel::storage::phase8_smoke_check();
    let device_summary = kernel::device::summary();
    let (block_devices, driver_backed_blocks, backend) = kernel::block::summary();
    println!(
        "Phase8-Devices: total={}, pci={}, block={}, block_devices={}, driver_backed={}, storage_backend={}, storage_ok={}",
        device_summary.total,
        device_summary.pci,
        device_summary.block,
        block_devices,
        driver_backed_blocks,
        backend,
        phase8_storage_ok
    );
    kernel::serial_println!(
        "Phase8-Devices: total={}, pci={}, block={}, block_devices={}, driver_backed={}, storage_backend={}, storage_ok={}",
        device_summary.total,
        device_summary.pci,
        device_summary.block,
        block_devices,
        driver_backed_blocks,
        backend,
        phase8_storage_ok
    );
    let phase9_launch_ok = kernel::task::program_loader::phase9_smoke_check();
    let loader_status = kernel::task::program_loader::status();
    println!(
        "Phase9-Loader: programs={}, launch_ok={}, storage_backed={}, launches={}, failed_launches={}",
        loader_status.program_count,
        phase9_launch_ok,
        kernel::storage::is_mounted(),
        loader_status.launch_count,
        loader_status.failed_launch_count
    );
    kernel::serial_println!(
        "Phase9-Loader: programs={}, launch_ok={}, storage_backed={}, launches={}, failed_launches={}",
        loader_status.program_count,
        phase9_launch_ok,
        kernel::storage::is_mounted(),
        loader_status.launch_count,
        loader_status.failed_launch_count
    );
    let credentials = kernel::security::current_credentials();
    let policy_ok = kernel::security::phase10_smoke_check();
    let denied_ok = kernel::storage::phase10_smoke_check();
    println!(
        "Phase10-Security: user={}, role={}, policy_ok={}, denied_ok={}, denied_access={}, denied_execute={}",
        credentials.user.as_u64(),
        credentials.role.name(),
        policy_ok,
        denied_ok,
        kernel::security::denied_access_count(),
        kernel::security::denied_execute_count()
    );
    kernel::serial_println!(
        "Phase10-Security: user={}, role={}, policy_ok={}, denied_ok={}, denied_access={}, denied_execute={}",
        credentials.user.as_u64(),
        credentials.role.name(),
        policy_ok,
        denied_ok,
        kernel::security::denied_access_count(),
        kernel::security::denied_execute_count()
    );
    let phase11_images_ok = kernel::task::program_loader::phase11_smoke_check();
    let image_status = kernel::task::program_loader::status();
    let exec_blocked_ok = image_status.unsupported_execution_count > 0;
    println!(
        "Phase11-Images: images={}, valid={}, rejected={}, exec_blocked_ok={}",
        image_status.image_count,
        image_status.valid_image_count,
        image_status.invalid_image_count,
        phase11_images_ok && exec_blocked_ok
    );
    kernel::serial_println!(
        "Phase11-Images: images={}, valid={}, rejected={}, exec_blocked_ok={}",
        image_status.image_count,
        image_status.valid_image_count,
        image_status.invalid_image_count,
        phase11_images_ok && exec_blocked_ok
    );
    let phase12_load_plan_ok = kernel::task::program_loader::phase12_smoke_check();
    let load_plan_status = kernel::task::program_loader::status();
    println!(
        "Phase12-LoadPlan: prepared={}, rejected={}, pages={}, exec_blocked_ok={}",
        load_plan_status.prepared_image_count,
        load_plan_status.rejected_load_plan_count,
        load_plan_status.total_planned_pages,
        phase12_load_plan_ok
    );
    kernel::serial_println!(
        "Phase12-LoadPlan: prepared={}, rejected={}, pages={}, exec_blocked_ok={}",
        load_plan_status.prepared_image_count,
        load_plan_status.rejected_load_plan_count,
        load_plan_status.total_planned_pages,
        phase12_load_plan_ok
    );
    let phase13_mapping_ok = kernel::task::program_loader::phase13_smoke_check();
    let mapping_status = kernel::task::program_loader::status();
    println!(
        "Phase13-MappingStub: mapped={}, rejected={}, pages={}, copied={}, zeroed={}, exec_blocked_ok={}",
        mapping_status.mapped_image_count,
        mapping_status.rejected_mapping_count,
        mapping_status.total_mapped_pages,
        mapping_status.copied_bytes,
        mapping_status.zero_filled_bytes,
        phase13_mapping_ok
    );
    kernel::serial_println!(
        "Phase13-MappingStub: mapped={}, rejected={}, pages={}, copied={}, zeroed={}, exec_blocked_ok={}",
        mapping_status.mapped_image_count,
        mapping_status.rejected_mapping_count,
        mapping_status.total_mapped_pages,
        mapping_status.copied_bytes,
        mapping_status.zero_filled_bytes,
        phase13_mapping_ok
    );
    let phase14_frames_ok = kernel::frame_ownership::phase14_smoke_check();
    let frame_status = kernel::frame_ownership::status();
    println!(
        "Phase14-Frames: initialized={}, tracked={}, available={}, allocated={}, allocations={}, releases={}, failures={}, smoke_ok={}",
        frame_status.initialized,
        frame_status.tracked_frames,
        frame_status.available_frames,
        frame_status.allocated_frames,
        frame_status.allocation_count,
        frame_status.release_count,
        frame_status.failed_allocation_count,
        phase14_frames_ok
    );
    kernel::serial_println!(
        "Phase14-Frames: initialized={}, tracked={}, available={}, allocated={}, allocations={}, releases={}, failures={}, smoke_ok={}",
        frame_status.initialized,
        frame_status.tracked_frames,
        frame_status.available_frames,
        frame_status.allocated_frames,
        frame_status.allocation_count,
        frame_status.release_count,
        frame_status.failed_allocation_count,
        phase14_frames_ok
    );
    run_phase21_to_30_smokes();
    let phase15_backing_ok = kernel::task::program_loader::phase15_smoke_check();
    let backing_status = kernel::task::program_loader::status();
    let backing_frames = kernel::frame_ownership::status();
    println!(
        "Phase15-FrameBackedImage: backed={}, rejected={}, pages={}, frame_allocated={}, copied={}, zeroed={}, smoke_ok={}",
        backing_status.frame_backed_image_count,
        backing_status.rejected_frame_backing_count,
        backing_status.total_frame_backed_pages,
        backing_frames.allocated_frames,
        backing_status.copied_bytes,
        backing_status.zero_filled_bytes,
        phase15_backing_ok
    );
    kernel::serial_println!(
        "Phase15-FrameBackedImage: backed={}, rejected={}, pages={}, frame_allocated={}, copied={}, zeroed={}, smoke_ok={}",
        backing_status.frame_backed_image_count,
        backing_status.rejected_frame_backing_count,
        backing_status.total_frame_backed_pages,
        backing_frames.allocated_frames,
        backing_status.copied_bytes,
        backing_status.zero_filled_bytes,
        phase15_backing_ok
    );
    let phase16_tables_ok = kernel::task::program_loader::phase16_smoke_check();
    let table_status = kernel::task::program_loader::status();
    println!(
        "Phase16-PageTables: tables={}, rejected={}, pages={}, translate_ok={}, cr3_switched=false",
        table_status.user_page_table_count,
        table_status.rejected_user_page_table_count,
        table_status.total_user_page_table_pages,
        phase16_tables_ok
    );
    kernel::serial_println!(
        "Phase16-PageTables: tables={}, rejected={}, pages={}, translate_ok={}, cr3_switched=false",
        table_status.user_page_table_count,
        table_status.rejected_user_page_table_count,
        table_status.total_user_page_table_pages,
        phase16_tables_ok
    );
    let phase17_context_ok = kernel::task::program_loader::phase17_smoke_check();
    let context_status = kernel::task::program_loader::status();
    let user_selectors = kernel::gdt::user_selectors();
    println!(
        "Phase17-UserContext: contexts={}, rejected={}, user_code={}, user_data={}, entry_ok={}, ring3_entered=false",
        context_status.user_context_count,
        context_status.rejected_user_context_count,
        user_selectors.code.0,
        user_selectors.data.0,
        phase17_context_ok
    );
    kernel::serial_println!(
        "Phase17-UserContext: contexts={}, rejected={}, user_code={}, user_data={}, entry_ok={}, ring3_entered=false",
        context_status.user_context_count,
        context_status.rejected_user_context_count,
        user_selectors.code.0,
        user_selectors.data.0,
        phase17_context_ok
    );
    let phase18_ring3_ok = kernel::task::program_loader::phase18_smoke_check();
    let ring3_status = kernel::task::program_loader::status();
    println!(
        "Phase18-Ring3: entries={}, traps={}, rejected={}, trap_vector={}, survived={}",
        ring3_status.ring3_entry_count,
        ring3_status.ring3_trap_count,
        ring3_status.rejected_ring3_count,
        kernel::interrupts::USER_TRAP_VECTOR,
        phase18_ring3_ok
    );
    kernel::serial_println!(
        "Phase18-Ring3: entries={}, traps={}, rejected={}, trap_vector={}, survived={}",
        ring3_status.ring3_entry_count,
        ring3_status.ring3_trap_count,
        ring3_status.rejected_ring3_count,
        kernel::interrupts::USER_TRAP_VECTOR,
        phase18_ring3_ok
    );
    let phase19_syscall_ok = kernel::task::program_loader::phase19_smoke_check();
    let user_syscall_status = kernel::task::program_loader::status();
    println!(
        "Phase19-SyscallReturn: syscalls={}, returns={}, rejected={}, abi_ok={}, returned_ok={}",
        user_syscall_status.user_syscall_count,
        user_syscall_status.user_syscall_return_count,
        user_syscall_status.rejected_user_syscall_count,
        phase19_syscall_ok,
        phase19_syscall_ok
    );
    kernel::serial_println!(
        "Phase19-SyscallReturn: syscalls={}, returns={}, rejected={}, abi_ok={}, returned_ok={}",
        user_syscall_status.user_syscall_count,
        user_syscall_status.user_syscall_return_count,
        user_syscall_status.rejected_user_syscall_count,
        phase19_syscall_ok,
        phase19_syscall_ok
    );
    let phase20_user_elf_ok = kernel::task::program_loader::phase20_smoke_check();
    let user_elf_status = kernel::task::program_loader::status();
    println!(
        "Phase20-UserElf: executions={}, exits={}, rejected={}, hello_ok={}",
        user_elf_status.user_elf_execution_count,
        user_elf_status.user_elf_exit_count,
        user_elf_status.rejected_user_elf_count,
        phase20_user_elf_ok
    );
    kernel::serial_println!(
        "Phase20-UserElf: executions={}, exits={}, rejected={}, hello_ok={}",
        user_elf_status.user_elf_execution_count,
        user_elf_status.user_elf_exit_count,
        user_elf_status.rejected_user_elf_count,
        phase20_user_elf_ok
    );

    // Display performance counters at startup.
    let counters = PerformanceCounters::read();
    println!(
        "CPU frequency estimate: {} MHz",
        PerformanceCounters::cpu_frequency_mhz()
    );
    println!("System ticks since boot: {}", counters.ticks());
    println!(
        "Preemption metrics: total_preemptions={}, lock_contention={}, fairness_violations={}",
        counters.total_preemptions(),
        counters.scheduler_lock_contention(),
        counters.fairness_violations()
    );

    let preemption_mode = cfg!(feature = "preemption");
    println!("Kernel features: preemption={}", preemption_mode);

    if preemption_mode {
        println!("Phase 5: Preemption mode active. Spawning 4 kernel tasks for fairness testing.");
        println!("Console: type 'help' to list runtime scheduler commands.");
        kernel::task::scheduler::set_context_switching_enabled(true);
        kernel::task::scheduler::spawn_kernel_tasks_phase5();
        println!(
            "Kernel tasks spawned. Starting preemptive scheduler. quantum_ticks={}, fairness_interval_ticks={}",
            kernel::task::scheduler::scheduler_quantum_ticks(),
            kernel::task::scheduler::fairness_check_interval_ticks()
        );
        kernel::task::scheduler::run_context_lab();
    }

    kernel::task::scheduler::set_context_switching_enabled(false);

    // Run the async executor with the keyboard task.
    let mut executor = Executor::new();
    executor.spawn(Task::named("keyboard", keyboard::print_keypresses()));
    executor.spawn(Task::named("uptime", timer::log_uptime()));
    executor.spawn(Task::named("scheduler-stats", timer::log_scheduler_stats()));
    executor.spawn(Task::named(
        "scheduler-groundwork",
        timer::log_scheduler_groundwork(),
    ));
    executor.spawn(Task::named("task-registry", timer::log_task_registry()));
    executor.spawn(Task::named("task-watchdog", timer::task_watchdog()));
    
    if cfg!(feature = "preemption") {
        executor.spawn(Task::named("fairness-monitor", timer::log_preemption_fairness()));
    }

    let stats = executor.stats();
    let context_names = kernel::task::scheduler::context_task_names();
    println!(
        "Tasks: active={}, sleeping={}, ready={}, completed={}",
        stats.active_tasks,
        stats.sleeping_tasks,
        stats.ready_queue_depth,
        stats.completed_tasks
    );
    println!("Context tasks: {:?}", context_names);
    println!("Kernel ready. Entering event loop.");
    executor.run();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop();
}
