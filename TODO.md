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

## OS Debugging

### Assembly Debugging with GDB
-[x] "layout split" activates source and asm view
-[ ] Scroll to address?
-[ ] Scroll to function?

-[ ] Function Prologue
    https://stackoverflow.com/questions/25545994/how-does-gdb-determine-the-address-to-break-at-when-you-do-break-function-name