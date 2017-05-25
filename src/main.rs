#![feature(lang_items)]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(abi_x86_interrupt)]
#![feature(const_fn)]

#![no_std]
#![no_main]

#[macro_use]
extern crate lazy_static;

extern crate rlibc;
extern crate spin;

extern crate console;

#[macro_use]
extern crate interrupts;
extern crate x86;
use x86::bits64::irq::IdtEntry;
use x86::shared::io::{inb};

extern crate keyboard;
extern crate pic;

#[cfg(not(test))]
pub mod panic;

use spin::Mutex;

#[macro_use]
extern crate intermezzos;

lazy_static! {
    static ref CONTEXT: intermezzos::kernel::Context = intermezzos::kernel::Context::new();
}


static mut TICK_COUNTER: Mutex<u64> = Mutex::new(0);
static mut UPTIME_SECONDS: u64 = 7290;
const TICK_DIVISOR: u64 = 1 << 9;
const TICK_FREQUENCY: u64 = (pic::PIT_BASE_FREQUENCY as u64)/TICK_DIVISOR;

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    pic::remap();

    let gpf = make_idt_entry!(isr13, {
        panic!("omg GPF");
    });

    let nanoseconds = 1000000/TICK_FREQUENCY;
    kprintln!(CONTEXT, "Timer has {}ns accuracy.", nanoseconds);

    pic::set_pit_divisor(0, TICK_DIVISOR as u16);
    // IRQ0 (0) on PIC1 (32), so IDT index is 32
    let timer = make_idt_entry!(isr32, {
        pic::eoi_for(32);

        let mut tick_counter = unsafe {
            let option_guard = TICK_COUNTER.try_lock();
            if option_guard.is_none() {
                kprintln!(CONTEXT, "error: isr32 called simultaniously");
                return
            }
            let mut option_guard = option_guard.unwrap();
            *option_guard += 1;
            *option_guard
        };

        if (tick_counter % TICK_FREQUENCY) == 0 {
            let mut seconds = unsafe {
                UPTIME_SECONDS += 1;
                UPTIME_SECONDS
            };
            // assert_eq!(tick_counter, seconds * TICK_FREQUENCY);
            let hours = seconds / 3600;
            let minutes =  (seconds % 3600) / 60;
            seconds = (seconds % 3600) % 60;
            kprintln!(CONTEXT, "[{:02}:{:02}:{:02}] {}", hours, minutes, seconds, tick_counter);
        }
    });

    // Keyboard uses IRQ1 and PIC1 has been remapped to 0x20 (32); therefore
    // the index in the IDT for IRQ1 will be 32 + 1 = 33
    let keyboard = make_idt_entry!(isr33, {
        let scancode = unsafe { inb(0x60) };

        if let Some(c) = keyboard::from_scancode(scancode as usize) {
            kprint!(CONTEXT, "{}", c);
        }

        pic::eoi_for(33);
    });

    CONTEXT.idt.set_handler(13, gpf);
    CONTEXT.idt.set_handler(32, timer);
    CONTEXT.idt.set_handler(33, keyboard);

    kprintln!(CONTEXT, "Kernel initialized.");

    CONTEXT.idt.enable_interrupts();

    loop {
        unsafe { asm!("hlt") }
    }

    panic!("the loop was escaped")
}
