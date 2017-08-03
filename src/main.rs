#![feature(lang_items)]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(abi_x86_interrupt)]
#![feature(iterator_step_by)]
#![feature(use_extern_macros)]

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

#[macro_use]
extern crate memory;

#[cfg(not(test))]
pub mod panic;

#[macro_use]
extern crate intermezzos;

use spin::Mutex;

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
        (0x52) as u16);
        // (clock::pit::consts::BASE_FREQUENCY/2000) as u16);

    static ref TSI: Mutex<TaskStateInformation> = Mutex::new(
        TaskStateInformation {
            current_task: 0,
            tasks: [
                TaskEntry {
                    ip: TASK_ENTRY_UNITIALIZED_USIZE,
                    sp: TASK_ENTRY_UNITIALIZED_USIZE,
                    cpu_flags: TASK_ENTRY_UNITIALIZED_U64,
                },
                TaskEntry {
                    ip: task1 as usize,
                    sp: TASK1_STACK.end,
                    cpu_flags: TASK_ENTRY_UNITIALIZED_U64,
                },
                TaskEntry {
                    ip: task2 as usize,
                    sp: TASK2_STACK.end,
                    cpu_flags: TASK_ENTRY_UNITIALIZED_U64,
                },
            ],
        }
    );
}


#[no_mangle]
/// According to gdb the page tables are located as follows
/// (gdb) print &p2_table
/// $3 = (<data variable, no debug info> *) 0x110000
/// (gdb) print &p3_table
/// $4 = (<data variable, no debug info> *) 0x10f000
/// (gdb) print &pagetable_4
/// $5 = (<data variable, no debug info> *) 0x10e000
pub extern "C" fn kinit() {

    unsafe {
        asm!("
        nop
        " :::: "intel")
    }
}


#[naked]
#[inline(always)]
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

#[naked]
#[inline(always)]
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
    let cr3 = unsafe { get_register!("cr3") };
    kprintln_try!(CONTEXT,
                  "pagetable_4 address (according to CR3 register): 0x{:x}",
                  cr3);

    const PAGETABLE_4_SIZE: usize = 512;
    let pagetable_4_lower;
    let pagetable_4_upper;

    // Address verification helper
    // * https://play.rust-lang.org/?gist=e019dd5957382c87e40db4125592b6ff&version=nightly
    unsafe {
        let cr3_lower = cr3 as *const usize;
        let cr3_upper = (cr3 | 0b1111111111111111_111111111 << 39) as *const usize;
        pagetable_4_lower = ::core::slice::from_raw_parts(cr3_lower, PAGETABLE_4_SIZE);
        pagetable_4_upper = ::core::slice::from_raw_parts(cr3_upper, PAGETABLE_4_SIZE);
    }
    kprintln_try!(CONTEXT,
                  "pagetable_4_lower[0]: {:x} / {:b}",
                  pagetable_4_lower[0],
                  pagetable_4_lower[0]);
    kprintln_try!(CONTEXT,
                  "pagetable_4_lower[{}] {:x} / {:b}",
                  PAGETABLE_4_SIZE,
                  pagetable_4_lower[PAGETABLE_4_SIZE - 1],
                  pagetable_4_lower[PAGETABLE_4_SIZE - 1]);

    kprintln_try!(CONTEXT,
                  "Accessing the address that uses the 512th index to trigger the access bit!");

    kprintln_try!(CONTEXT,
                  "pagetable_4_upper[{}] {:x} / {:b}",
                  PAGETABLE_4_SIZE,
                  pagetable_4_upper[PAGETABLE_4_SIZE - 1],
                  pagetable_4_upper[PAGETABLE_4_SIZE - 1]);

    kprintln_try!(CONTEXT,
                  "pagetable_4_lower[{}] {:x} / {:b}",
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

    // initilaze_tss();

    let isr_de = make_idt_entry!(isr0, esf, ExceptionStackFrame, {
        panic!("Divide-by-Zero-Error Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(0, isr_de);

    let isr_db = make_idt_entry!(isr1, esf, ExceptionStackFrame, {
        panic!("Debug Exception occurred. Exception Information: \n{}", esf);
    });
    CONTEXT.idt.set_handler(1, isr_db);

    let isr_nmi = make_idt_entry!(isr2, esf, ExceptionStackFrame, {
        panic!("Non-Maskable-Interrupt Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(2, isr_nmi);

    let isr_bp = make_idt_entry!(isr3, esf, ExceptionStackFrame, {
        panic!("Breakpoint Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(3, isr_bp);

    let isr_of = make_idt_entry!(isr4, esf, ExceptionStackFrame, {
        panic!("Overflow Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(4, isr_of);

    let isr_br = make_idt_entry!(isr5, esf, ExceptionStackFrame, {
        panic!("Bound-Range Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(5, isr_br);

    let isr_ud = make_idt_entry!(isr6, esf, ExceptionStackFrame, {
        panic!("Invalid-Opcode Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(6, isr_ud);

    let isr_nm = make_idt_entry!(isr7, esf, ExceptionStackFrame, {
        panic!("Device-Not-Available Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(7, isr_nm);

    let isr_df = make_idt_entry!(isr8, esf, ErrorExceptionStackFrame, {
        panic!("Double-Fault Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(8, isr_df);

    // 9 Coprocessor-Segment-Overrun

    let isr_ts = make_idt_entry!(isr10, esf, ErrorExceptionStackFrame, {
        panic!("Invalid-TSS Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(10, isr_ts);

    let isr_np = make_idt_entry!(isr11, esf, ErrorExceptionStackFrame, {
        panic!("Segment-Not-Present Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(11, isr_np);

    let isr_ss = make_idt_entry!(isr12, esf, ErrorExceptionStackFrame, {
        panic!("Stack Exception occurred. Exception Information: \n{}", esf);
    });
    CONTEXT.idt.set_handler(12, isr_ss);

    let isr_gp = make_idt_entry!(isr13, esf, ErrorExceptionStackFrame, {
        panic!("General-Protection Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(13, isr_gp);

    let isr_pf = make_idt_entry!(isr14, esf, ErrorExceptionStackFrame, {
        panic!("Page-Fault Exception occurred while accessing : {:X}
        Exception Information:\n{}",
               unsafe { get_register!("cr2") },
               esf);
    });
    CONTEXT.idt.set_handler(14, isr_pf);

    // 15 Reserved
    // 16 x87 Floating-Point Exception-Pending

    let isr_ac = make_idt_entry!(isr17, esf, ErrorExceptionStackFrame, {
        panic!("Alignment-Check fault occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(17, isr_ac);

    let isr_mc = make_idt_entry!(isr17, esf, ExceptionStackFrame, {
        panic!("Machine-Check fault occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(18, isr_mc);

    // IRQ0 (0) on PIC1 (32), so IDT index is 32
    let timer = make_idt_entry!(isr32, esf, ExceptionStackFrame, {
        CLOCK.tick();

        fn schedule_and_dispatch(interval: &str,
                                 ticks: &u64,
                                 uptime: &clock::Duration,
                                 interval_ticks: &u64,
                                 esf: &mut ExceptionStackFrame) {
            // kprintln!(CONTEXT,
            //           "Interval {}/{} ({} ticks): {}",
            //           interval,
            //           interval_ticks,
            //           ticks,
            //           uptime);

            let mut tsi = TSI.lock();
            let next_task = scheduler(&tsi);
            if next_task != tsi.current_task {
                // Dispatch the next_task
                kprintln_try!(CONTEXT,
                              "Switching from Task {} to Task {}.\nOld StackFrame: {}",
                              tsi.current_task,
                              next_task,
                              esf);
                let alligned_stack_pointer = (esf.stack_pointer + 0x10 - 1) & !(0x10 - 1);
                // assert_eq!(esf.stack_pointer, alligned_stack_pointer);

                tsi.tasks[tsi.current_task].ip = esf.instruction_pointer;
                tsi.tasks[tsi.current_task].sp = esf.stack_pointer;
                tsi.tasks[tsi.current_task].cpu_flags = esf.cpu_flags;

                // assert_eq!(esf.instruction_pointer % 0x8, 0);

                if tsi.tasks[next_task].ip != TASK_ENTRY_UNITIALIZED_USIZE {
                    esf.instruction_pointer = tsi.tasks[next_task].ip;
                }

                if tsi.tasks[next_task].sp != TASK_ENTRY_UNITIALIZED_USIZE {
                    esf.stack_pointer = tsi.tasks[next_task].sp;
                }

                if tsi.tasks[next_task].cpu_flags != TASK_ENTRY_UNITIALIZED_U64 {
                    esf.cpu_flags = tsi.tasks[next_task].cpu_flags;
                }

                kprintln_try!(CONTEXT, "New StackFrame: {}", esf);

                tsi.current_task = next_task;
            };
        }



        let intervals: [(&str,
                        u64,
                        fn(&str,
                           &u64,
                           &clock::Duration,
                           &u64,
                           &mut ExceptionStackFrame)); 1] =
            [("10ms", 10_000_000 / CLOCK.resolution, schedule_and_dispatch)];

        let (ticks, uptime) = CLOCK.ticks().unwrap();
        for &(interval, interval_ticks, f) in intervals.iter() {
            let remainder = ticks % (interval_ticks as u64);
            if remainder == 0 {
                f(interval, &ticks, &uptime, &interval_ticks, esf);
            }
        }

        pic::eoi_for(32);
    });
    CONTEXT.idt.set_handler(32, timer);

    // Keyboard uses IRQ1 and PIC1 has been remapped to 0x20 (32); therefore
    // the index in the IDT for IRQ1 will be 32 + 1 = 33
    let keyboard = make_idt_entry!(isr33, esf, ExceptionStackFrame, {
        // Ignore the esf
        let _ = esf;

        let scancode = unsafe { inb(0x60) };

        if let Some(c) = keyboard::from_scancode(scancode as usize) {
            kprint_try!(CONTEXT, "{}", c);
        } else {
            // kprint!(CONTEXT, "\nUnmapped: {:#X}", scancode);
        }

        pic::eoi_for(33);
    });
    CONTEXT.idt.set_handler(33, keyboard);

    pic::remap();

    kprintln!(CONTEXT,
              "System clock set up. Frequency: {} / Resolution: {}ns",
              CLOCK.frequency,
              CLOCK.resolution);

    paging_demo();

    kprintln!(CONTEXT,
              "Kernel initialized, final step: enabling interrupts");

    CONTEXT.idt.enable_interrupts();
    CLOCK.start();

    loop {
        hlt();
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

use tasks::stack::Stack;

const TASK_ENTRY_UNITIALIZED_USIZE: usize = 0xdeadbeef;
const TASK_ENTRY_UNITIALIZED_U64: u64 = TASK_ENTRY_UNITIALIZED_USIZE as u64;


struct TaskEntry {
    ip: usize,
    sp: usize,
    cpu_flags: u64,
    // stack: &'static Stack,
}

struct TaskStateInformation {
    current_task: usize,
    tasks: [TaskEntry; 3],
}

const STACKS_START: usize = 0x200_000; // 2MiB
const STACK_SIZE: usize = 0x100_000; // 1MiB
const STACK_ALIGNMENT: usize = 0x10;
static TASK1_STACK: Stack = STACKS_START + 1 * STACK_SIZE..STACKS_START + 2 * STACK_SIZE;
static TASK2_STACK: Stack = STACKS_START + 2 * STACK_SIZE..STACKS_START + 3 * STACK_SIZE;

/// Get next Task ID
fn scheduler(tsi: &TaskStateInformation) -> usize {
    (tsi.current_task + 1) % tsi.tasks.len()
}

fn task1() {
    let mut i: u64 = 2;
    let mut prev_i: u64 = 0;
    loop {
        assert_eq!(i, prev_i + 2);
        assert_eq!(i % 2, 0);

        prev_i = i;
        i += 2;
    }
}

fn task2() {
    let mut i: u64 = 3;
    let mut prev_i: u64 = 1;
    loop {
        assert_eq!(i, prev_i + 2);
        assert_eq!(i % 2, 1);

        prev_i = i;
        i += 2;
    }
}

// fn test_libfringe() {
//     unsafe extern "C" fn adder(arg: usize, stack_ptr: StackPointer) -> ! {
//         kprintln!(CONTEXT, "it's alive! arg: {}", arg);
//         let (arg, stack_ptr) = arch::swap(arg + 1, stack_ptr, None);
//         kprintln!(CONTEXT, "still alive! arg: {}", arg);
//         arch::swap(arg + 1, stack_ptr, None);
//         panic!("i should be dead");
//     }

//     unsafe {
//         // let ptr = heap::allocate(16384, STACK_ALIGNMENT);
//         let ptr = TASK1_STACK.start;
//         let mut slice = Box::from_raw(slice::from_raw_parts_mut(ptr, STACK_SIZE));
//         // let stack = SliceStack::new(&mut slice[4096..8192]);
//         let stack = SliceStack::new(&mut slice[..]);
//         assert_eq!(stack.base() as usize & (STACK_ALIGNMENT - 1), 0);
//         assert_eq!(stack.limit() as usize & (STACK_ALIGNMENT - 1), 0);

//         // let stack = OsStack::new(4 << 20).unwrap();
//         let stack_ptr = arch::init(&stack, adder);

//         let (ret, stack_ptr) = arch::swap(10, stack_ptr, Some(&stack));
//         assert_eq!(ret, 11);
//         let (ret, _) = arch::swap(50, stack_ptr, Some(&stack));
//         assert_eq!(ret, 51);
//     }
// }