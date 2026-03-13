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
    task::{executor::Executor, keyboard, Task},
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

    // Run the async executor with the keyboard task.
    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::print_keypresses()));
    println!("Kernel ready. Entering event loop.");
    executor.run();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop();
}
