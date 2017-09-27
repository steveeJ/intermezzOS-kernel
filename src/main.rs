#![feature(lang_items)]
#![feature(asm)]
#![feature(naked_functions)]
#![feature(abi_x86_interrupt)]
#![feature(iterator_step_by)]
#![feature(use_extern_macros)]
#![feature(range_contains)]
#![feature(compiler_builtins_lib)]

#![no_std]
#![no_main]

extern crate compiler_builtins;

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
use core::mem;

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
        // (0x71ae) as u16);
        core::u16::MAX);


    static ref TSI: Mutex<tasks::TaskStateInformation> = {
        let tasklist = [
            tasks::TaskEntry {
                    name: "Task 0",
                    esf: interrupts::ExceptionStackFrame{
                        code_segment: 0x8,
                        stack_segment: 0x10,
                        instruction_pointer: task0 as usize,
                        cpu_flags: 0x200202,
                        stack_pointer: TASK0_STACK.top,
                    },
                    stack: TASK0_STACK,
                    registers: tasks::TaskRegisters::empty(),
                    blocked: false,
                    },
            tasks::TaskEntry {
                    name: "Task 1",
                    esf: interrupts::ExceptionStackFrame{
                        code_segment: 0x8,
                        stack_segment: 0x10,
                        instruction_pointer: task1 as usize,
                        cpu_flags: 0x200202,
                        stack_pointer: TASK1_STACK.top,
                    },
                    stack: TASK1_STACK,
                    registers: tasks::TaskRegisters::empty(),
                    blocked: false,
                    },
            tasks::TaskEntry {
                    name: "Task 2",
                    esf: interrupts::ExceptionStackFrame{
                        code_segment: 0x8,
                        stack_segment: 0x10,
                        instruction_pointer: task2 as usize,
                        cpu_flags: 0x200202,
                        stack_pointer: TASK2_STACK.top,
                    },
                    stack: TASK2_STACK,
                    registers: tasks::TaskRegisters::empty(),
                    blocked: false,
                    },
        ];
        Mutex::new(tasks::TaskStateInformation::new(tasklist))
    };
}

// static TASK_COUNTER: &'static mut u64 = &mut 0;

#[no_mangle]
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
                  "pagetable_4_lower[  0]: {:x} / {:b}",
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

    // paging_demo();
    // hlt();

    // initilaze_tss();

    let isr_de = make_idt_entry!(isr0, esf: &ExceptionStackFrame, false, {
        panic!("Divide-by-Zero-Error Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(0, isr_de);

    let isr_db = make_idt_entry!(isr1, esf: &ExceptionStackFrame, false, {
        panic!("Debug Exception occurred. Exception Information: \n{}", esf);
    });
    CONTEXT.idt.set_handler(1, isr_db);

    let isr_nmi = make_idt_entry!(isr2, esf: &ExceptionStackFrame, false, {
        panic!("Non-Maskable-Interrupt Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(2, isr_nmi);

    let isr_bp = make_idt_entry!(isr3, esf: &ExceptionStackFrame, false, {
        panic!("Breakpoint Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(3, isr_bp);

    let isr_of = make_idt_entry!(isr4, esf: &ExceptionStackFrame, false, {
        panic!("Overflow Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(4, isr_of);

    let isr_br = make_idt_entry!(isr5, esf: &ExceptionStackFrame, false, {
        panic!("Bound-Range Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(5, isr_br);

    let isr_ud = make_idt_entry!(isr6, esf: &ExceptionStackFrame, false, {
        panic!("Invalid-Opcode Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(6, isr_ud);

    let isr_nm = make_idt_entry!(isr7, esf: &ExceptionStackFrame, false, {
        panic!("Device-Not-Available Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(7, isr_nm);

    let isr_df = make_idt_entry!(isr8, esf: &ErrorExceptionStackFrame, false, {
        panic!("Double-Fault Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(8, isr_df);

    // 9 Coprocessor-Segment-Overrun

    let isr_ts = make_idt_entry!(isr10, esf: &ErrorExceptionStackFrame, false, {
        panic!("Invalid-TSS Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(10, isr_ts);

    let isr_np = make_idt_entry!(isr11, esf: &ErrorExceptionStackFrame, false, {
        panic!("Segment-Not-Present Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(11, isr_np);

    let isr_ss = make_idt_entry!(isr12, esf: &ErrorExceptionStackFrame, false, {
        panic!("Stack Exception occurred. Exception Information: \n{}", esf);
    });
    CONTEXT.idt.set_handler(12, isr_ss);

    let isr_gp = make_idt_entry!(isr13, esf: &ErrorExceptionStackFrame, false, {
        panic!("General-Protection Exception occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(13, isr_gp);

    let isr_pf = make_idt_entry!(isr14, esf: &ErrorExceptionStackFrame, false, {
        panic!("Page-Fault Exception occurred while accessing : {:X}
        Exception Information:\n{}",
               unsafe { get_register!("cr2") },
               esf);
    });
    CONTEXT.idt.set_handler(14, isr_pf);

    // 15 Reserved
    // 16 x87 Floating-Point Exception-Pending

    let isr_ac = make_idt_entry!(isr17, esf: &ErrorExceptionStackFrame, false, {
        panic!("Alignment-Check fault occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(17, isr_ac);

    let isr_mc = make_idt_entry!(isr17, esf: &ExceptionStackFrame, false, {
        panic!("Machine-Check fault occurred. Exception Information: \n{}",
               esf);
    });
    CONTEXT.idt.set_handler(18, isr_mc);

    // IRQ0 (0) on PIC1 (32), so IDT index is 32
    // Timer ISR to increase the clock and call the scheduler
    // If the scheduler choses a different task, the dispatcher
    // must be called by the interrupt return (`iretq`)
    let timer = make_idt_entry!(isr32, esf: &mut ExceptionStackFrame, true, {
        let rbp_on_stack: *mut usize = unsafe { (get_register!("rbp") as *mut usize) };

        // The prologue decreased the address by the register pushes
        // TODO: explain why we decrease he offset by 1
        let rax_offset = 1 - (mem::size_of::<tasks::TaskRegisters>() as isize / 8);
        let rax_on_stack: *mut usize = unsafe { rbp_on_stack.offset(rax_offset) };
        let registers_on_stack: &mut tasks::TaskRegisters =
            unsafe { mem::transmute::<*mut usize, &mut tasks::TaskRegisters>(rax_on_stack) };

        // kprintln!(CONTEXT, "{}", registers_on_stack);

        let begin_tsc = unsafe { x86::bits64::time::rdtsc() };


        CLOCK.tick();

        #[naked]
        #[inline(always)]
        fn dummy_time_fn(interval: &str,
                         ticks: &u64,
                         uptime: &clock::Duration,
                         interval_ticks: &u64,
                         esf: &mut ExceptionStackFrame) {
            let _ = interval;
            let _ = ticks;
            let _ = uptime;
            let _ = interval_ticks;
            let _ = esf;
        }

        #[naked]
        #[inline(always)]
        fn dummy_esf_fn(esf: &mut ExceptionStackFrame) {
            let _ = esf;
        }

        fn test_time(interval: &str,
                     ticks: &u64,
                     uptime: &clock::Duration,
                     interval_ticks: &u64,
                     esf: &mut ExceptionStackFrame) {
            kprintln!(CONTEXT,
                      "Interval {}/{} ({} ticks): {} / ESF: {}",
                      interval,
                      interval_ticks,
                      ticks,
                      uptime,
                      0,
                    //   esf,
                      );
        };

        fn manage_tasks(esf: &mut ExceptionStackFrame, registers: &mut tasks::TaskRegisters) {
            if let Some(mut tsi) = TSI.try_lock() {
                // store esf of preempted task for reference
                let preempted_esf = esf.clone();
                let last_scheduled_task = tsi.current_task;

                // assume that the current task had time to run if the stackframe pointer has been set accordingly
                let continue_to_next = {
                    tsi.get_current_task().stack.contains(registers.rbp) ||
                    {
                        if !tsi.get_current_task().stack.contains(esf.stack_pointer) {
                            kprintln_try!(CONTEXT,
                                          "Stack overflow in task {}!\nStack: {:x}\nESF: {:x}\nREGS: {:x}",
                                          tsi.current_task,
                                          tsi.get_current_task().stack,
                                          esf,
                                          registers);
                            tsi.get_current_task_mut().blocked = true;
                        }
                        true
                    }
                };

                if !continue_to_next {
                    *esf = tsi.get_current_task().esf;
                    *registers = tsi.get_current_task().registers;
                    kprintln_try!(CONTEXT,
                                  "{:x} not in {:x}",
                                  registers.rbp,
                                  tsi.get_current_task().stack);
                } else if tsi.schedule_next() {
                    tsi.get_current_task_mut().registers = *registers;
                    *esf = *tsi.mangle_esf_for_next(esf);
                    *registers = tsi.get_current_task().registers;
                };

                // kprintln_try!(CONTEXT, "New StackFrame: {}", next_esf);
                kprintln_try!(CONTEXT,
                                "TS: {} ({:?}) -> {} @ ({:?})",
                                last_scheduled_task,
                                preempted_esf,
                                tsi.current_task,
                                esf,
                                );

                if esf.cpu_flags & 0x200 == 0 {
                    panic!("About to return to a process with interrupts disabled!");
                }
            } else {
                panic!("We didn't get the TSI lock! ESF: {}", esf);
            }
        };

        /*
        type time_fn_t = fn(&str, &u64, &clock::Duration, &u64, &mut ExceptionStackFrame);
        type esf_fn_t = fn(&mut ExceptionStackFrame);

        type interval_t = (&'static str, u64, time_fn_t, esf_fn_t);

        let intervals: [interval_t; 2] =
            [("1s", (1_000_000_000) / CLOCK.resolution, test_time, dummy_esf_fn),
             ("11ms", (1 * 11_363_636) / CLOCK.resolution, dummy_time_fn, manage_tasks) /*("500ms", 500_000_000 / CLOCK.resolution, test_time)*/];

        let (ticks, uptime) = CLOCK.ticks().unwrap();

        for &(interval, interval_ticks, time_fn, esf_fn) in intervals.iter() {
            let remainder = ticks % (interval_ticks as u64);
            if remainder == 0 {
                time_fn(interval, &ticks, &uptime, &interval_ticks, esf);
                esf_fn(esf);
            }
        }

        */

        manage_tasks(esf, registers_on_stack);

        pic::eoi_for(32);

        let end_tsc = unsafe { x86::bits64::time::rdtsc() };
        // kprintln_try!(CONTEXT, "ISR32 cycles: {}", end_tsc - begin_tsc);
    });
    CONTEXT.idt.set_handler(32, timer);

    // Keyboard uses IRQ1 and PIC1 has been remapped to 0x20 (32); therefore
    // the index in the IDT for IRQ1 will be 32 + 1 = 33
    let keyboard = make_idt_entry!(isr33, esf: &ExceptionStackFrame, true, {
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

    schedule_and_dispatch();

    panic!("the boot task was rescheduled");
}

fn task0() {
    CLOCK.start();
    kprintln!(CONTEXT,
              "System clock set up. Frequency: {} / Resolution: {}ns",
              CLOCK.frequency,
              CLOCK.resolution);

    kprintln!(CONTEXT,
              "Kernel initialized, final step: enabling interrupts");
    CONTEXT.idt.enable_interrupts();

    loop {
        hlt();
    }
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

const STACKS_TOP: usize = 0x1_000_000; // 15.7MiB
const STACK_SIZE: usize = 0x_002_000; // 64KiB
// const STACK_ALIGNMENT: usize = 0x10;
use tasks::stack::Stack;
const TASK0_STACK: Stack = Stack {
    top: STACKS_TOP - 10 * STACK_SIZE,
    bottom: STACKS_TOP - (10 + 1) * STACK_SIZE,
};
const TASK1_STACK: Stack = Stack {
    top: STACKS_TOP - 20 * STACK_SIZE,
    bottom: STACKS_TOP - (20 + 1) * STACK_SIZE,
};
const TASK2_STACK: Stack = Stack {
    top: STACKS_TOP - 30 * STACK_SIZE,
    bottom: STACKS_TOP - (30 + 1) * STACK_SIZE,
};

/// Scheduler and Dispatch function - called only via the timer interrupt
///
#[naked]
fn schedule_and_dispatch() {
    let rbp;
    let rsp;
    let rip;

    {
        let tsi = TSI.lock();
        let te = tsi.get_current_task();

        rbp = te.stack.top;
        rsp = te.esf.stack_pointer;
        rip = te.esf.instruction_pointer;
    };

    unsafe {
        asm!("
            mov rbp, $0
            jmp $1
            "
            : // output operands
            : // input operands
            "r"(rbp)
            "r"(rip)
            "{rsp}="(rsp)
            : // clobbers
            : // options
            "intel" "volatile"
            );
    };
}

fn fill_stack(i: u64, d: u64) -> u64 {
    const slice_length: usize = 1;
    let slice: [u64; slice_length] = [0xdeadbeef; slice_length];
    let slice_start_addr = &slice[0] as *const u64;
    let slice_end_addr = &slice[slice_length - 1] as *const u64;

    let mut j = 1000;
    while {
              j -= 1;
              j > 0
          } {
        nop();
    }
    if i < d { fill_stack(i + 1, d) } else { i }
}

fn finish() {
    loop {
        hlt();
    }
}

fn task1() {
    fill_stack(0, 1000000);
    let mut i: u64 = 2;
    let mut prev_i: u64 = 0;
    loop {
        if i != prev_i + 2 || i % 2 != 0 {
            panic!("Wrong calculations: {}/{:x} {}/{:x}",
                   i,
                   &i,
                   prev_i,
                   &prev_i,
                   );
        }
        prev_i = i;
        i += 2;

        ge(i as isize, prev_i as isize);
    }
}

fn task2() {
    let mut i: u64 = 3;
    let mut prev_i: u64 = 1;
    loop {
        if i != prev_i + 2 || i % 2 != 1 {
            panic!("Wrong calculations: {}/{:x} {}/{:x}",
                   i,
                   &i,
                   prev_i,
                   &prev_i,
                   );
        }
        prev_i = i;
        i += 2;

        ge(i as isize, prev_i as isize);
    }
}

#[inline(never)]
fn ge(a: isize, b: isize) -> (bool, isize) {
    let diff = a - b;
    let ge = a >= b;
    (ge, diff)
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
//         let ptr = TASK0_STACK.start;
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