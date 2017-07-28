#![feature(lang_items)]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(abi_x86_interrupt)]

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
use x86::shared::io::inb;

extern crate keyboard;
extern crate pic;
extern crate tasks;
extern crate clock;
use clock::Clock;

extern crate memory;

#[cfg(not(test))]
pub mod panic;

#[macro_use]
extern crate intermezzos;

lazy_static! {
    static ref CONTEXT: intermezzos::kernel::Context = intermezzos::kernel::Context::new();

    /// This can be used to calculate the divisors that divide to even numbers:
    /// ```
    /// #![feature(inclusive_range_syntax)]
    ///
    /// fn main() {
    ///     let base_freq: f64 = 1193182 as f64;
    ///     for i in 1 ... 0xFFFF {
    ///         if base_freq as u64 % i == 0 {
    ///             println!("{:x}", i);
    ///         }
    ///     }
    /// ```
    ///
    /// Output:
    /// ```
    /// 1
    /// 2
    /// 29
    /// 52
    /// 38d7
    /// 71ae
    /// ```
    static ref CLOCK: clock::pit::Pit = clock::pit::new(0,
        (clock::pit::consts::BASE_FREQUENCY/2000) as u16);

}


#[no_mangle]
pub extern "C" fn kinit() {
    /// According to gdb the page tables are located as follows
    /// (gdb) print &p2_table
    /// $3 = (<data variable, no debug info> *) 0x110000
    /// (gdb) print &p3_table
    /// $4 = (<data variable, no debug info> *) 0x10f000
    /// (gdb) print &pagetable_4
    /// $5 = (<data variable, no debug info> *) 0x10e000

    unsafe {
        asm!("
        nop
        " :::: "intel")
    }
}


pub fn hlt() {
    unsafe {
        asm!("
                hlt
                "
                // output operands
                :
                // input operands
                :
                // clobbers
                :
                // options
                : "intel");
    }
}

pub fn nop() {
    unsafe {
        asm!("
                nop
                "
                // output operands
                :
                // input operands
                :
                // clobbers
                :
                // options
                : "intel");
    }
}

fn paging_demo() {
    let cr3 = unsafe { memory::get_cr3() };
    kprintln!(CONTEXT, "CR3: {:X}", cr3);

    const PAGETABLE_4_SIZE: usize = 512;
    let pagetable_4_lower;
    let pagetable_4_upper;

    /// Address verification helper
    /// * https://play.rust-lang.org/?gist=e019dd5957382c87e40db4125592b6ff&version=nightly
    unsafe {
        let cr3_lower = cr3 as *const usize;
        let cr3_upper = (cr3 | 0b1111111111111111_111111111 << 39) as *const usize;
        pagetable_4_lower = ::core::slice::from_raw_parts(cr3_lower, PAGETABLE_4_SIZE);
        pagetable_4_upper = ::core::slice::from_raw_parts(cr3_upper, PAGETABLE_4_SIZE);
    }
    kprintln!(CONTEXT,
              "pagetable_4_lower[0]: {:X} / {:b}",
              pagetable_4_lower[0],
              pagetable_4_lower[0]);
    kprintln!(CONTEXT,
              "pagetable_4_lower[{}] {:X} / {:b}",
              PAGETABLE_4_SIZE,
              pagetable_4_lower[PAGETABLE_4_SIZE - 1],
              pagetable_4_lower[PAGETABLE_4_SIZE - 1]);

    kprintln!(CONTEXT,
              "Accessing the address that uses the 512th index to trigger the access bit!");

    kprintln!(CONTEXT,
              "pagetable_4_upper[{}] {:X} / {:b}",
              PAGETABLE_4_SIZE,
              pagetable_4_upper[PAGETABLE_4_SIZE - 1],
              pagetable_4_upper[PAGETABLE_4_SIZE - 1]);

    kprintln!(CONTEXT,
              "pagetable_4_lower[{}] {:X} / {:b}",
              PAGETABLE_4_SIZE,
              pagetable_4_lower[PAGETABLE_4_SIZE - 1],
              pagetable_4_lower[PAGETABLE_4_SIZE - 1]);
}

#[repr(C,packed)]
struct sgdt {
    pointer: u64,
    length: u16,
}


#[no_mangle]
pub extern "C" fn kmain() -> ! {
    // Disable interrupts
    unsafe {
        x86::shared::irq::disable();
    }

    paging_demo();

    // initilaze_tss();

    pic::remap();

    let gpf = make_idt_entry!(isr13, esf, {
        panic!("GPF occurred. Exception Information: \n{}", esf);
    });
    CONTEXT.idt.set_handler(13, gpf);

    // IRQ0 (0) on PIC1 (32), so IDT index is 32
    let timer = make_idt_entry!(isr32, esf, {
        CLOCK.tick();

        // TODO: verify that register content is actually on the stack
        // TODO: verify that we understand the stack pointer / layout
        // TODO: save the rflags register on current task's stack
        // TODO: call scheduler
        // TODO: write pointer for next task's stack into (RSP)

        /// The IP can point to naked function
        // esf.instruction_pointer = testfn as usize;

        pic::eoi_for(32);
    });
    CONTEXT.idt.set_handler(32, timer);
    CLOCK.start();
    kprintln!(CONTEXT,
              "System clock started. Frequency: {} / Resolution: {}ns",
              CLOCK.frequency,
              CLOCK.resolution);

    // Keyboard uses IRQ1 and PIC1 has been remapped to 0x20 (32); therefore
    // the index in the IDT for IRQ1 will be 32 + 1 = 33
    let keyboard = make_idt_entry!(isr33, esf, {
        // Ignore the esf
        let _ = esf;

        let scancode = unsafe { inb(0x60) };

        if let Some(c) = keyboard::from_scancode(scancode as usize) {
            kprint!(CONTEXT, "{}", c);
        } else {
            // kprint!(CONTEXT, "\nUnmapped: {:#X}", scancode);
        }

        pic::eoi_for(33);
    });
    CONTEXT.idt.set_handler(33, keyboard);

    kprintln!(CONTEXT,
              "Kernel initialized, final step: enabling interrupts");

    CONTEXT.idt.enable_interrupts();

    loop {
        scheduler();
    }
    panic!("the main loop was escaped")
}

/// TODO: remove me and explain what happened
pub fn stack_debug() {
    let mut sp: usize;
    unsafe {
        asm!("
                "
                : "={sp}" (sp)
                :
                :
                : "intel" );
        kprintln!(CONTEXT, "{}", sp);
        kprintln!(CONTEXT, "Last pushed item: {}", *((sp) as *const usize));

        asm!("
                mov r8, 1337
                push r8
                "
                : "={sp}"(sp)
                :
                : "r8"
                : "intel" );
        let last_pushed = *((sp) as *const usize);
        kprintln!(CONTEXT, "Last pushed item: {}", last_pushed);

        asm!("
                "
                : "={sp}" (sp)
                :
                :
                : "intel" );
        kprintln!(CONTEXT, "{}", sp);

        asm!("
                pop r8
                "
                :
                :
                :
                : "intel" );
    }
}

const STACKS_START: usize = 0x200000; // 2MiB
const STACK_SIZE: usize =    0x10000; // 65KiB

use tasks::stack::Stack;

static BOOT_STACK: Stack =
    STACKS_START+0*STACK_SIZE .. STACKS_START+1*STACK_SIZE-0x8;
static TASK1_STACK: Stack =
    STACKS_START+1*STACK_SIZE .. STACKS_START+2*STACK_SIZE-0x8;
static TASK2_STACK: Stack =
    STACKS_START+2*STACK_SIZE .. STACKS_START+3*STACK_SIZE-0x8;

struct TaskEntry {
    rip: fn(),
    stack: &'static Stack,
}

static mut TASKS: [TaskEntry; 2] = [
    TaskEntry{rip: task1, stack: &TASK1_STACK},
    TaskEntry{rip: task2, stack: &TASK2_STACK},
];

static mut CURRENT_TASK: usize = 0;

fn scheduler() {
    fn interval_fn(interval: &str, ticks: &u64, uptime: &clock::Duration, interval_ticks: &u64) {
        kprintln!(CONTEXT,
                  "Interval {}/{} ({} ticks): {}",
                  interval,
                  interval_ticks,
                  ticks,
                  uptime);
    }

    let intervals: [(&str, u64, fn(&str, &u64, &clock::Duration, &u64)); 1] = [
        ("10ms", 10_000_000/CLOCK.resolution, interval_fn),
    ];

    let (ticks, uptime) = CLOCK.ticks().unwrap();
    for &(interval, interval_ticks, f) in intervals.iter() {
        let remainder = ticks % (interval_ticks as u64);
        if remainder == 0 {
            f(interval, &ticks, &uptime, &interval_ticks);
        }
    }
    hlt();
}

fn dispatcher() {
}

fn task1() {
    loop {
        for _ in 0..10000000 { nop() };
        kprintln!(CONTEXT, "this is task 1");
        stack_debug();
    }
}

fn task2() {
    loop {
        for _ in 0..10000000 { nop() };
        kprintln!(CONTEXT, "this is task 1");
        stack_debug();
    }
}