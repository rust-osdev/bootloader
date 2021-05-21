.section .boot, "awx"
.code16

# From http://wiki.osdev.org/Detecting_Memory_(x86)#Getting_an_E820_Memory_Map

# use the INT 0x15, eax= 0xE820 BIOS function to get a memory map
# inputs: es:di -> destination buffer for 24 byte entries
# outputs: bp = entry count, trashes all registers except esi
do_e820:
	xor ebx, ebx		# ebx must be 0 to start
	xor bp, bp		# keep an entry count in bp
	mov edx, 0x0534D4150	# Place "SMAP" into edx
	mov eax, 0xe820
	mov dword ptr es:[di + 20], 1	# force a valid ACPI 3.X entry
	mov ecx, 24		# ask for 24 bytes
	int 0x15
	jc .failed	# carry set on first call means "unsupported function"
	mov edx, 0x0534D4150	# Some BIOSes apparently trash this register?
	cmp eax, edx		# on success, eax must have been reset to "SMAP"
	jne .failed
	test ebx, ebx		# ebx = 0 implies list is only 1 entry long (worthless)
	je .failed
	jmp .jmpin
.e820lp:
	mov eax, 0xe820		# eax, ecx get trashed on every int 0x15 call
	mov dword ptr es:[di + 20], 1	# force a valid ACPI 3.X entry
	mov ecx, 24		# ask for 24 bytes again
	int 0x15
	jc .e820f		# carry set means "end of list already reached"
	mov edx, 0x0534D4150	# repair potentially trashed register
.jmpin:
	jcxz .skipent		# skip any 0 length entries
	cmp cl, 20		# got a 24 byte ACPI 3.X response?
	jbe .notext
	test byte ptr es:[di + 20], 1	# if so: is the "ignore this data" bit clear?
	je .skipent
.notext:
	mov ecx, es:[di + 8]	# get lower uint32_t of memory region length
	or ecx, es:[di + 12]	# "or" it with upper uint32_t to test for zero
	jz .skipent		# if length uint64_t is 0, skip entry
	inc bp			# got a good entry: ++count, move to next storage spot
	add di, 24
.skipent:
	test ebx, ebx		# if ebx resets to 0, list is complete
	jne .e820lp
.e820f:
	mov [mmap_ent], bp	# store the entry count
	clc			# there is "jc" on end of list to this point, so the carry must be cleared
	ret
.failed:
	stc			# "function unsupported" error exit
	ret

mmap_ent: .word 0
