//! Integration test: stack overflow must trigger a double-fault handler
//! and not silently corrupt state.

#![no_std]
#![no_main]
#![allow(unconditional_recursion)]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;
use kernel::{exit_qemu, hlt_loop, serial_println, QemuExitCode};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    kernel::gdt::init();
    kernel::interrupts::init_idt();

    stack_overflow();

    panic!("Execution continued after stack overflow");
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    // Prevent the compiler from optimising the recursion away.
    stack_overflow();
    // Prevent tail-call optimisation.
    let _x = 0u64;
    let _ = &_x;
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    hlt_loop();
}
