//! This module contains methods and macros to create and register interrupt descriptors and
//! interrupt handlers

#![feature(asm)]
#![feature(naked_functions)]
#![feature(const_fn)]
#![no_std]

extern crate x86;
extern crate pic;
extern crate spin;

use spin::Mutex;
use x86::shared::dtables;
use x86::shared::dtables::DescriptorTablePointer;
use x86::bits64::irq::IdtEntry;

#[derive(Clone,Copy)]
#[repr(C)]
pub struct ExceptionStackFrame {
    /// This value points to the instruction that should be executed when the interrupt
    /// handler returns. For most interrupts, this value points to the instruction immediately
    /// following the last executed instruction. However, for some exceptions (e.g., page faults),
    /// this value points to the faulting instruction, so that the instruction is restarted on
    /// return. See the documentation of the `Idt` fields for more details.
    pub instruction_pointer: usize,
    /// The code segment selector, padded with zeros.
    pub code_segment: u64,
    /// The flags register before the interrupt handler was invoked.
    pub cpu_flags: u64,
    /// The stack pointer at the time of the interrupt.
    pub stack_pointer: usize,
    /// The stack segment descriptor at the time of the interrupt (often zero in 64-bit mode).
    pub stack_segment: u64,
}

impl core::fmt::Display for ExceptionStackFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "
        Instruction Pointer: 0x{:x}
        Code Segment: 0x{:x}
        CPU Flags: 0x{:x}
        Stack Pointer: 0x{:x}
        Stack Segment: 0x{:x}",
            self.instruction_pointer,
            self.code_segment,
            self.cpu_flags,
            self.stack_pointer,
            self.stack_segment,
        );
        Ok(())
    }
}

impl core::fmt::Debug for ExceptionStackFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        // write!(f, "IP:0x{:X} CS:0x{:X} Flags:0x{:X} SP: 0x{:X} SS: 0x{:X}",
        write!(f, "IP:0x{:06x} SP: 0x{:06x}",
            self.instruction_pointer,
            // self.code_segment,
            // self.cpu_flags,
            self.stack_pointer,
            // self.stack_segment,
        );
        Ok(())
    }
}

#[repr(C)]
pub struct ErrorExceptionStackFrame {
    /// Some Exceptions like the GPF pass an error code
    pub error_code: u64,
    /// This value points to the instruction that should be executed when the interrupt
    /// handler returns. For most interrupts, this value points to the instruction immediately
    /// following the last executed instruction. However, for some exceptions (e.g., page faults),
    /// this value points to the faulting instruction, so that the instruction is restarted on
    /// return. See the documentation of the `Idt` fields for more details.
    pub instruction_pointer: usize,
    /// The code segment selector, padded with zeros.
    pub code_segment: u64,
    /// The flags register before the interrupt handler was invoked.
    pub cpu_flags: u64,
    /// The stack pointer at the time of the interrupt.
    pub stack_pointer: usize,
    /// The stack segment descriptor at the time of the interrupt (often zero in 64-bit mode).
    pub stack_segment: u64,
}

impl core::fmt::Display for ErrorExceptionStackFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "
        Error Code: 0x{:X}
        Instruction Pointer: 0x{:X}
        Code Segment: 0x{:X}
        CPU Flags: 0x{:X}
        Stack Pointer: 0x{:X}
        Stack Segment: 0x{:X}",
            self.error_code,
            self.instruction_pointer,
            self.code_segment,
            self.cpu_flags,
            self.stack_pointer,
            self.stack_segment,
        );
        Ok(())
    }
}

/// Creates an IDT entry.
///
/// Creates an IDT entry that executes the expression in `body`.
#[macro_export]
macro_rules! make_idt_entry {
    ($name:ident, $esf:ident, $esf_type:ident, $ir_gate:expr, $body:expr) => {{

        use x86::bits64::irq::IdtEntry;
        use interrupts::$esf_type;
        extern "x86-interrupt" fn $name($esf: &mut $esf_type) {
            unsafe {
                asm!(""
                    // output operands
                    :
                    // input operands
                    :
                    // clobbers
                    : "rax", "rbx", "rcx", "rdx", "rsi", "rdi", "r8",  "r9",  "r10", "r11", "r12", "r13", "r14", "r15", "rbp"
                    // options
                    : "intel" "volatile"
                );
            }
            $body
        };

        use x86::shared::paging::VAddr;
        use x86::shared::PrivilegeLevel;

        let handler = VAddr::from_usize($name as usize);

        // last is "block". It influences the Gate's type field as follows:
        // * false: 1111 (64-bit Trap Gate)
        // * true:  1110 (64-bit Interrupt Gate)
        // Ref.: AMD64 Architecture Programmer’s Manual Volume 2: System Programming
        // Table 4-6. System-Segment Descriptor Types—Long Mode (continued)
        IdtEntry::new(handler, 0x8, PrivilegeLevel::Ring0, $ir_gate)
    }};
}

/// The Interrupt Descriptor Table
///
/// The CPU will look at this table to find the appropriate interrupt handler.
static IDT: Mutex<[IdtEntry; 256]> = Mutex::new([IdtEntry::MISSING; 256]);

/// Pointer to the Interrupt Descriptor Table
pub struct IdtRef {
    ptr: DescriptorTablePointer<IdtEntry>,
    idt: &'static Mutex<[IdtEntry; 256]>,
}

unsafe impl Sync for IdtRef {}

impl IdtRef {
    /// Creates a new pointer struct to the IDT.
    pub fn new() -> IdtRef {
        let r = IdtRef {
            ptr: DescriptorTablePointer::new_idtp(&IDT.lock()[..]),
            idt: &IDT,
        };

        // This block is safe because by referencing IDT above, we know that we've constructed an
        // IDT.
        unsafe { dtables::lidt(&r.ptr) };

        r
    }

    /// Sets an IdtEntry as a handler for interrupt specified by `index`.
    pub fn set_handler(&self, index: usize, entry: IdtEntry) {
        self.idt.lock()[index] = entry;
    }

    /// Enables interrupts.
    pub fn enable_interrupts(&self) {
        // This unsafe fn is okay because, by virtue of having an IdtRef, we know that we have a
        // valid Idt.
        unsafe {
            x86::shared::irq::enable();
        }
    }
}