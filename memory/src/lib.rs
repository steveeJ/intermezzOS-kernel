//! This modules contains memory management functionality

#![feature(asm)]
#![no_std]

/// This function reads the CR3 register
/// TODO: should interrupts be disabled for this function?
pub unsafe fn get_cr3() -> usize {
    let cr3: usize;

    // Read cr3 via r8
    unsafe {
        asm!("
                mov r8, cr3
                "
                // output operands
                : "={r8}"(cr3)
                // input operands
                :
                // clobbers
                : "r8"
                // options
                : "intel"
            );
    };
    cr3
}

pub mod stack {}

pub mod paging {
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}