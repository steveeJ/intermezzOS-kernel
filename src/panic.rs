extern crate x86;

use core::fmt;
use ::CONTEXT;

#[lang = "panic_fmt"]
#[no_mangle]
pub extern fn rust_begin_panic(msg: fmt::Arguments,
                               file: &'static str,
                               line: u32) -> ! {
    // Disable interrupts
    unsafe {
        x86::shared::irq::disable();
    }

    kprint_force!(CONTEXT, "KERNEL PANIC in {}:{}! Message: {}\n", file, line, msg);
    loop { unsafe { asm!("hlt") }}
}