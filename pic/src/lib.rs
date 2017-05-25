#![feature(asm)]
#![no_std]

extern crate x86;
use x86::shared::io::{inb, outb};

// PIC input/output ports
const PIC1_CMD_IO_PORT: u16 = 0x0020;
const PIC2_CMD_IO_PORT: u16 = 0x00A0;
const PIC1_DATA_IO_PORT: u16 = 0x0021;
const PIC2_DATA_IO_PORT: u16 = 0x00A1;

// PIC commands
const ICW1: u8 = 0x11;
const ICW4: u8 = 0x1;

// new interrupt vector offsets for remapped PICs
const PIC1_VECTOR_OFFSET: u8 = 0x20;
const PIC2_VECTOR_OFFSET: u8 = 0x28;

/// Initializes and remaps PIC interrupts to other vectors numbers.
/// Hardware interrupts are mapped to 8-15 (primary PIC) and
/// 70-78 (secondary PIC) vector numbers. This leads to problem
/// in protected mode as 7-8 vector numbers are reserved for
/// exceptions.
///
/// This function remaps interrupts to new vector number offsets,
/// from 0x20 for the primary PIC and from 0x28 for secondary and
/// bind primary PIC with secondary PIC through IRQ 2 line.
pub fn remap() {
    unsafe {
        let pic1_mask = inb(PIC1_DATA_IO_PORT);
        let pic2_mask = inb(PIC2_DATA_IO_PORT);

        // initialize both PICs
        outb(PIC1_CMD_IO_PORT, ICW1);
        outb(PIC2_CMD_IO_PORT, ICW1);

        // set vector offset of pic1 to 0x20
        outb(PIC1_DATA_IO_PORT, PIC1_VECTOR_OFFSET);
        // set vector offset of pic2 to 0x28
        outb(PIC2_DATA_IO_PORT, PIC2_VECTOR_OFFSET);

        // tell PIC1 about PIC2 at IRQ2 (0000 0100)
        outb(PIC1_DATA_IO_PORT, 4);

        // tell PIC2 its cascade identity (0000 0010)
        outb(PIC2_DATA_IO_PORT, 2);

        // set both PICs to 8086 mode
        outb(PIC1_DATA_IO_PORT, ICW4);
        outb(PIC2_DATA_IO_PORT, ICW4);

        // restore masks
        outb(PIC1_DATA_IO_PORT, pic1_mask);
        outb(PIC2_DATA_IO_PORT, pic2_mask);
    }
}

pub fn eoi_for(interrupt_number: isize) {
    unsafe {
        match interrupt_number {
            i if i >= 40 => {
                outb(0xA0, 0x20);
                outb(0x20, 0x20);
            },
            32...40 => outb(0x20, 0x20),
            _ => {},
        }
    }
}

// PIT constants
/// References
/// * http://wiki.osdev.org/Programmable_Interval_Timer
/// * https://en.wikibooks.org/wiki/X86_Assembly/Programmable_Interval_Timer
pub const PIT_BASE_FREQUENCY: u32 = 1193182;
//const PIT_CHANNEL0_IO_PORT: u16 = 0x40;
//const PIT_CHANNEL1_IO_PORT: u16 = 0x41;
//const PIT_CHANNEL2_IO_PORT: u16 = 0x42;
//const PIT_COMMAND_PORT: u16 = 0x43;
//const PIT_CHANNEL_COUNT: usize = 3;
//const PIT_CHANNEL_IO_PORTS: [u16; PIT_CHANNEL_COUNT] = [
//    PIT_CHANNEL0_IO_PORT,
//    PIT_CHANNEL1_IO_PORT,
//    PIT_CHANNEL2_IO_PORT
//];

#[allow(unused_variables)]
pub fn set_pit_divisor(channel: usize, divisor: u16) {
    let byte = PIT_BASE_FREQUENCY/(divisor as u32);
    let lobyte = (byte & 0xFF) as u8;
    let hibyte = ((byte >> 8) & 0xFF) as u8;
    unsafe {
//        outb(PIT_COMMAND_PORT, 0b00110100); // channel 0, lobyte/hibyte, rate generator
//        outb(PIT_COMMAND_PORT, lobyte);
//        outb(PIT_COMMAND_PORT, hibyte);
        asm!("
            mov al, 0x36
            out 0x43, al

            mov ax, $0
            out 0x40,al
            mov al,ah
            out 0x40,al
        " :: "r"(divisor) :: "intel" );
    }
}

#[test]
fn test_channel_io_ports_arr() {
}