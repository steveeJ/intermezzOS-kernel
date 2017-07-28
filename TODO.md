# Software Prerequisites

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

