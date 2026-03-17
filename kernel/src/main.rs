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

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("AresOS v{} booting...", env!("CARGO_PKG_VERSION"));

    kernel::init();

    // Initialise memory subsystem.
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::init(&boot_info.memory_map) };

    // Set up the kernel heap.
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialisation failed");

    println!("Memory subsystem initialised.");

    // Display performance counters at startup.
    let counters = PerformanceCounters::read();
    println!(
        "CPU frequency estimate: {} MHz",
        PerformanceCounters::cpu_frequency_mhz()
    );
    println!("System ticks since boot: {}", counters.ticks());

    let live_context_switch = cfg!(feature = "live-context-switch");
    kernel::task::scheduler::set_context_switching_enabled(live_context_switch);
    kernel::task::scheduler::spawn_demo_context_tasks();
    println!(
        "Context demo tasks registered (live switch mode: {}).",
        live_context_switch
    );

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
