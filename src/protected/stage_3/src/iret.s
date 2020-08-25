.att_syntax prefix
iret_asm_test:
    mov    $0x23, %eax
    mov    %eax, %ds
    mov    %eax, %es
    mov    %eax, %fs
    mov    %eax, %gs
    mov    %esp, %eax
    
    push   $0x23
    push   %eax

    pushf

    push   $0x1b
    pushl  $iret_test
    
    iret