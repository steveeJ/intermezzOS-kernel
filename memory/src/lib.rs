//! This modules contains memory management functionality

#![feature(asm)]
#![no_std]

/// This macro reads a specific register
#[macro_export]
macro_rules! get_register {
    ( $reg_name:expr ) => {{
        let reg: usize;
        let r8_before: usize;
        let r8_after: usize;

        asm!(""
            // output operands
            : "={r8}"(r8_before)
            // input operands
            :
            // clobbers
            :
            // options
            : "intel"
            );

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

        asm!(""
            // output operands
            :
            // input operands
            : "w{r8}"(r8_before)
            // clobbers
            :
            // options
            : "intel"
            );

        asm!(""
            // output operands
            : "={r8}"(r8_after)
            // input operands
            :
            // clobbers
            :
            // options
            : "intel"
            );

        assert_eq!(r8_before, r8_after);
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