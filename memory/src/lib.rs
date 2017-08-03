//! This modules contains memory management functionality

#![feature(asm)]
#![no_std]

/// This macro reads a specific register
#[macro_export]
macro_rules! get_register {
    ( $reg_name:expr ) => {{
        let reg: usize;

        // Read register via r8
        asm!(concat!("mov r8, ", $reg_name)
            // output operands
            : "={r8}"(reg)
            // input operands
            :
            // clobbers
            : "r8"
            // options
            : "intel"
            );
        reg
    }}
}

pub mod stack {}

pub mod paging {}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}