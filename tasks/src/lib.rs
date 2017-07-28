#![no_std]
#![feature(asm)]

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}

extern crate x86;

pub mod stack {
    use core::ops::Range;
    pub type Stack = Range<usize>;

    trait IsStack {
        fn is_initialized(&self) -> bool;
    }

    impl IsStack for Stack {
        fn is_initialized(&self) -> bool {
            false
        }
    }
}


struct TaskContext {
}

struct Task {
    task_context: TaskContext,
}


pub fn initilaze_tss() {
    // TODO: create a valid TSS structure

    // TODO: find the TSS descriptor memory address:
    let sdgt: usize;
    unsafe {
        asm!("

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

    // TODO: write the TSS structure at the memory address load the TSS

    /* TODO
    ; load TSS
    mov ax, gdt64.tss ; relative index of tss in gdt64
    ltr ax
    */
}