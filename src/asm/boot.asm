extern kmain
extern kinit
global start

section .boot
bits 32
start:
    call kinit

    ; Point the first and last entries of the level 4 page table to the first entry in the
    ; level 3 page table
    mov eax, pagetable_3
    or eax, 0b11 ; present + writable
    mov dword [pagetable_4 +   0 * 8], eax ; first
    mov dword [pagetable_4 + 511 * 8], eax ; last

    ; Point the first entry of the level 3 page table to the first entry in the
    ; level 2 page table
    mov eax, pagetable_2
    or eax, 0b11 ; present + writable
    mov dword [pagetable_3 + 0], eax

    ; point each page table level two entry to a page
    mov ecx, 0         ; counter variable
.map_pagetable_2:
    mov eax, 0x200000  ; 2MiB
    mul ecx
    or eax, 0b10000011 ; PDE.PS (Page Size) bit indicating 2MiB pages + present + writable
    mov [pagetable_2 + ecx * 8], eax

    inc ecx
    cmp ecx, 512
    jne .map_pagetable_2

    ; move page table address to cr3
    mov eax, pagetable_4
    mov cr3, eax

    ; enable PAE
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; set the long mode bit
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    ; enable paging
    mov eax, cr0
    or eax, (1 << 31 | 1 << 16)
    mov cr0, eax

    ; load global descriptor table
    lgdt [gdt64.pointer]

    ; update selectors
    mov ax, gdt64.data
    mov ss, ax
    mov ds, ax
    mov es, ax

    ; long jump to kmain setting `cs` register to `gdt64.code`
    jmp gdt64.code:kmain

    ; shouldn't ever happen
    hlt

section .bss
align 4096
pagetable_4:
    resb 4096
pagetable_3:
    resb 4096
pagetable_2:
    resb 4096

section .rodata
; 64-ia-32-architectures-software-developer-system-programming-manual-325384.pdf page 156
gdt64: ; global (segment) descriptor table
    dq 0 ; zero/invalid entry
.code: equ $ - gdt64
    dq (1<<41) | (1<<43) | (1<<44) | (1<<47) | (1<<53) ; (code segment (41 readable, 43+44 segment type code, 47 present bit, 53 64-bit flag)
.data: equ $ - gdt64
    dq (1<<41) | (1<<44) | (1<<47); data segment (41 writable, 44 segment type data, 47 present bit)
; .tss: equ $ - gdt64 ; TSS Descriptor
;     dq 0 ; reserve memory
;     dq 0 ; for the tss
.pointer:
    dw $ - gdt64 - 1
    dq gdt64
