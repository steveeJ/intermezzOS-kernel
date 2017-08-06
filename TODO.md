# Software Prerequisites

## Function Call Conventions

### retq

`retq` is equivalent to `addq $8, %rsp; jmpq -8(%rsp)`


### Rust Function Assembly

#### Regular Function

Function with no arguments
```
   │0x106dc0 <intermezzos::scheduler>       push   %rbp                                                 │
   │0x106dc1 <intermezzos::scheduler+1>     mov    %rsp,%rbp                                            │
   │0x106dc4 <intermezzos::scheduler+4>     sub    $0x150,%rsp                                          │
   │0x106dcb <intermezzos::scheduler+11>    lea    0x6b74(%rip),%rax        # 0x10d946                  │
B+>│0x106dd2 <intermezzos::scheduler+18>    mov    %rax,%rdi
```

Empty function with no arguments
```
   │0x107580 <intermezzos::dispatcher>                      push   %rbp                                 │
   │0x107581 <intermezzos::dispatcher+1>                    mov    %rsp,%rbp                            │
  >│0x107584 <intermezzos::dispatcher+4>                    pop    %rbp                                 │
   │0x107585 <intermezzos::dispatcher+5>                    retq
```

#### Naked Functions
```
B+>│0x106dc7 <intermezzos::scheduler+7>     mov    %rax,%rdi                                            │
```


## Paging

### P4 recursive mapping verification with gdb


```asm
mov eax, p3_table
or eax, 0b11 ; ?
mov dword [p4_table +   0 * 8], eax ; first
mov dword [p4_table + 511 * 8], eax ; last
```

The `p4_table` address is 0x10e000, this is also the content of the _cr3_ register.


```
(gdb) x /1gt (0x10e000 + 0*8)
0x10e000:       0000000000000000000000000000000000000000000100001111000000100011
(gdb) x /1gt (0x10e000 + 511*8)
0x10eff8:       0000000000000000000000000000000000000000000100001111000000000011
```

The first entry of P4 has been accessed, indicated by the bit 6.

# Hardware Prerequisites

## Heap/Stack


## MMU

### Segmentation

* SS: Stack Segment Register

#### Privilege Level
32-bit:
* requested
* current
* descriptor: bit-ness of the code,

64-bit: only

### Paging (CS3 Register)

## Interrupt

### IDT makro:

* Floating Point Register FP[1-8]


# Status Quo and Goals

## Boot Task

1. Disable Interrupts
1. Set up Interrupt Handlers
    2. (Excpetions, etc.)
    2. Keyboard
    2. PIT
1. Initialize System Variables
    2. (Idt, VGA Buffer, etc.)
    2. Clock
    2. Task Stacks
1. Enable Interrupts
1. Endless Sleep

## Preemptive Context-Switching

### Use Timer Interrupt For Context Switches
The x86-interrupt ABI supported by Rust/LLVM can do a context switch from and back to any instruction in the code.

DONE
* Jump to instruction address
* Switch to new stack

Problems/TODO
- [ ] Push return address to new task's stack
- [ ] 16-byte Stack Alignment !?
- [x] One thread with a simple counting for loop doesn't work properly
    * Store/Restore the registers?
    **This is done by the compiler, but not all registers were passed as clobbers**
- [ ] Two threads with a simple counting for loop don't work properly

    **Error Case - General-Protection Exception when switching from Task 1 to Task 2:**
        * CPU-Flags: 0x200246: '1000000000001001000110'
    * GPE when switching from Task 1 to Task 2
        * Error Code: 0xE678
        * RIP is on `iretq` from ISR32
        * Last context switch was to ISR17 (alignment-check)!?

    ~~The issue seems to be that the ISR pro-/epilogue is interruptible.
    When debugging, several interrupts can get queued so the problem appears quickly.~~
    When compiled with `--release`, the function `task1()` is significantly reduced, and no real checks are undertaken.
    See [this gist](https://gist.github.com/steveeJ/13dfb34cc7ca4e026ed38eedf303cfb9).
    The debug binary contains the code which is broken, probably due to a wrong context switch.

    **Error Case - the counter var i in task1 is uneven**
    This seems to stem from a wrong _rbp_ register, which points to a variable in a different stack.

    ```
    (gdb) print &i
    $5 = (u64 *) 0x4ffec8
    (gdb) print &prev_i
    $3 = (u64 *) 0x4ffed0
    (gdb) i r rsp rbp
    rsp            0x3ffe58 0x3ffe58
    rbp            0x4ffff8 0x4ffff8
    ```
    The _rbp_ register is used by the instructions to access local variables, which will not work if it's not pointing to the currently running task's stack.

    Taking a look at the ISR32 pro-/epilogues:

    ```
    0x109c50 <intermezzos::kmain::isr32>            push   rbp
    0x109c51 <intermezzos::kmain::isr32+1>          mov    rbp,rsp
    0x109c54 <intermezzos::kmain::isr32+4>          push   r15
    (...)
    0x10a02e <intermezzos::kmain::isr32+990>        pop    r15
    0x10a030 <intermezzos::kmain::isr32+992>        pop    rbp
    0x10a031 <intermezzos::kmain::isr32+993>        iretq
    ```

    I was able to verify that this incorrect address is being pushed from the stack:
    ```
    (gdb) i r rbp rsp
    rbp            0x7f830  0x7f830
    rsp            0x7f830  0x7f830
    (gdb) x /xxg (0x7f830)
    0x7f830:        0x00000000004ffff8
    (gdb) ni
    0x000000000010a031 in intermezzos::kmain::isr32 (esf=0x10827e <intermezzos::kmain+3886>)
        at <make_idt_entry macros>:14
    (gdb) i r rbp rsp
    rbp            0x4ffff8 0x4ffff8
    rsp            0x7f838  0x7f838
    ```
    It's unclear how this address gets onto the stack.

- [ ] Check out The exact difference of the IdtEntry's block true/false behavior
    * false: 10001111 (64-bit Trap Gate)
    * true:  10001110 (64-bit Interrupt Gate)
    From AMD Manual "Table 4-6. System-Segment Descriptor Types—Long Mode (continued)"
    Probably it's not ideal that all exceptions/interrupts use the same setting.
- [ ] Use a separate software interrupt for the scheduler/dispatcher?

## OS Debugging

### Assembly Debugging with GDB
-[x] "layout split" activates source and asm view
-[ ] Scroll to address?
-[ ] Scroll to function?

-[ ] Function Prologue
    https://stackoverflow.com/questions/25545994/how-does-gdb-determine-the-address-to-break-at-when-you-do-break-function-name


# Complications

## Mutable Borrow from Two Different Slice Elements

```
error[E0499]: cannot borrow `self.tasks[..].esf` as mutable more than once at a time
   --> src/main.rs:515:29
    |
508 |         let old_esf = &mut self.tasks[self.current_task].esf;
    |                            --------------------------------- first mutable borrow occurs here
...
515 |         let next_esf = &mut self.tasks[self.next_task].esf;
    |                             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ second mutable borrow occurs here
...
547 |     }
    |     - first borrow ends here
```