# Code originally taken from https://gitlab.redox-os.org/redox-os/bootloader
#
# Copyright (c) 2017 Redox OS, licensed under MIT License

.section .boot, "awx"
.code16

vesa:
vesa_getcardinfo:
    mov ax, 0x4F00
    mov di, offset VBECardInfo
    int 0x10
    cmp ax, 0x4F
    je vesa_findmode
    mov eax, 1
    ret
vesa_resetlist:
    # if needed, reset mins/maxes/stuff
    xor cx, cx
    mov [vesa_minx], cx
    mov [vesa_miny], cx
    mov [config_xres], cx
    mov [config_yres], cx
vesa_findmode:
    mov si, [VBECardInfo_videomodeptr]
    mov ax, [VBECardInfo_videomodeptr+2]
    mov fs, ax
    sub si, 2
vesa_searchmodes:
    add si, 2
    mov cx, fs:[si]
    cmp cx, 0xFFFF
    jne vesa_getmodeinfo
    cmp word ptr [vesa_goodmode], 0
    je vesa_resetlist
    jmp vesa_findmode
vesa_getmodeinfo:
    push esi
    mov [vesa_currentmode], cx
    mov ax, 0x4F01
    mov di, offset VBEModeInfo
    int 0x10
    pop esi
    cmp ax, 0x4F
    je vesa_foundmode
    mov eax, 1
    ret
vesa_foundmode:
    # check minimum values, really not minimums from an OS perspective but ugly for users
    cmp byte ptr [VBEModeInfo_bitsperpixel], 32
    jb vesa_searchmodes
vesa_testx:
    mov cx, [VBEModeInfo_xresolution]
    cmp word ptr [config_xres], 0
    je vesa_notrequiredx
    cmp cx, [config_xres]
    je vesa_testy
    jmp vesa_searchmodes
vesa_notrequiredx:
    cmp cx, [vesa_minx]
    jb vesa_searchmodes
vesa_testy:
    mov cx, [VBEModeInfo_yresolution]
    cmp word ptr [config_yres], 0
    je vesa_notrequiredy
    cmp cx, [config_yres]
    jne vesa_searchmodes    # as if there weren't enough warnings, USE WITH CAUTION
    cmp word ptr [config_xres], 0
    jnz vesa_setmode
    jmp vesa_testgood
vesa_notrequiredy:
    cmp cx, [vesa_miny]
    jb vesa_searchmodes
vesa_testgood:
    mov al, 13
    call print_char
    mov cx, [vesa_currentmode]
    mov [vesa_goodmode], cx
    push esi
    #  call print_dec
    #  mov al, ':'
    #  call print_char
    mov cx, [VBEModeInfo_xresolution]
    call print_dec
    mov al, 'x'
    call print_char
    mov cx, [VBEModeInfo_yresolution]
    call print_dec
    mov al, '@'
    call print_char
    xor ch, ch
    mov cl, [VBEModeInfo_bitsperpixel]
    call print_dec
vesa_confirm_mode:
    mov si, offset vesa_modeok
    call print
    # xor ax, ax
    # int 0x16 # read key press
    pop esi
    cmp al, al # originally `cmp al, 'y'` to compare key press
    je vesa_setmode
    cmp al, 's'
    je vesa_savemode
    jmp vesa_searchmodes
vesa_savemode:
    mov cx, [VBEModeInfo_xresolution]
    mov [config_xres], cx
    mov cx, [VBEModeInfo_yresolution]
    mov [config_yres], cx
   # call save_config
vesa_setmode:
    mov bx, [vesa_currentmode]
    cmp bx, 0
    je vesa_nomode
    or bx, 0x4000
    mov ax, 0x4F02
    int 0x10
vesa_nomode:
    cmp ax, 0x4F
    je vesa_returngood
    mov eax, 1
    ret
vesa_returngood:
    xor eax, eax
    ret

vesa_modeok:
    .ascii ": Is this OK? (s)ave/(y)es/(n)o    "
    .byte 8,8,8,8,0

vesa_goodmode: .2byte 0
vesa_currentmode: .2byte 0
# useful functions

#  print a number in decimal
#  IN
#    cx: the number
#  CLOBBER
#    al, cx, si
print_dec:
    mov si, offset print_dec_number
print_dec_clear:
    mov al, '0'
    mov [si], al
    inc si
    cmp si, offset print_dec_numberend
    jb print_dec_clear
    dec si
    call convert_dec
    mov si, offset print_dec_number
print_dec_lp:
    lodsb
    cmp si, offset print_dec_numberend
    jae print_dec_end
    cmp al, '0'
    jbe print_dec_lp
print_dec_end:
    dec si
    call print
    ret

print_dec_number: .skip 7, 0
print_dec_numberend: .skip 1, 0

convert_dec:
    dec si
    mov bx, si        # place to convert into must be in si, number to convert must be in cx
convert_dec_cnvrt:
    mov si, bx
    sub si, 4
convert_dec_ten4:    inc si
    cmp cx, 10000
    jb convert_dec_ten3
    sub cx, 10000
    inc byte ptr [si]
    jmp convert_dec_cnvrt
convert_dec_ten3:    inc si
    cmp cx, 1000
    jb convert_dec_ten2
    sub cx, 1000
    inc byte ptr [si]
    jmp convert_dec_cnvrt
convert_dec_ten2:    inc si
    cmp cx, 100
    jb convert_dec_ten1
    sub cx, 100
    inc byte ptr [si]
    jmp convert_dec_cnvrt
convert_dec_ten1:    inc si
    cmp cx, 10
    jb convert_dec_ten0
    sub cx, 10
    inc byte ptr [si]
    jmp convert_dec_cnvrt
convert_dec_ten0:    inc si
    cmp cx, 1
    jb convert_dec_return
    sub cx, 1
    inc byte ptr [si]
    jmp convert_dec_cnvrt
convert_dec_return:
    ret


VBECardInfo:
	VBECardInfo_signature: .skip 4, 0
	VBECardInfo_version: .skip 2, 0
	VBECardInfo_oemstring: .skip 4, 0
	VBECardInfo_capabilities: .skip 4, 0
	VBECardInfo_videomodeptr: .skip 4, 0
	VBECardInfo_totalmemory: .skip 2, 0
	VBECardInfo_oemsoftwarerev: .skip 2, 0
	VBECardInfo_oemvendornameptr: .skip 4, 0
	VBECardInfo_oemproductnameptr: .skip 4, 0
	VBECardInfo_oemproductrevptr: .skip 4, 0
	VBECardInfo_reserved: .skip 222, 0
	VBECardInfo_oemdata: .skip 256, 0

VBEModeInfo:
	VBEModeInfo_attributes: .skip 2, 0
	VBEModeInfo_winA: .skip 1, 0
	VBEModeInfo_winB: .skip 1, 0
	VBEModeInfo_granularity: .skip 2, 0
	VBEModeInfo_winsize: .skip 2, 0
	VBEModeInfo_segmentA: .skip 2, 0
	VBEModeInfo_segmentB: .skip 2, 0
	VBEModeInfo_winfuncptr: .skip 4, 0
	VBEModeInfo_bytesperscanline: .skip 2, 0
	VBEModeInfo_xresolution: .skip 2, 0
	VBEModeInfo_yresolution: .skip 2, 0
	VBEModeInfo_xcharsize: .skip 1, 0
	VBEModeInfo_ycharsize: .skip 1, 0
	VBEModeInfo_numberofplanes: .skip 1, 0
	VBEModeInfo_bitsperpixel: .skip 1, 0
	VBEModeInfo_numberofbanks: .skip 1, 0
	VBEModeInfo_memorymodel: .skip 1, 0
	VBEModeInfo_banksize: .skip 1, 0
	VBEModeInfo_numberofimagepages: .skip 1, 0
	VBEModeInfo_unused: .skip 1, 0
	VBEModeInfo_redmasksize: .skip 1, 0
	VBEModeInfo_redfieldposition: .skip 1, 0
	VBEModeInfo_greenmasksize: .skip 1, 0
	VBEModeInfo_greenfieldposition: .skip 1, 0
	VBEModeInfo_bluemasksize: .skip 1, 0
	VBEModeInfo_bluefieldposition: .skip 1, 0
	VBEModeInfo_rsvdmasksize: .skip 1, 0
	VBEModeInfo_rsvdfieldposition: .skip 1, 0
	VBEModeInfo_directcolormodeinfo: .skip 1, 0
	VBEModeInfo_physbaseptr: .skip 4, 0
	VBEModeInfo_offscreenmemoryoffset: .skip 4, 0
	VBEModeInfo_offscreenmemsize: .skip 2, 0
	VBEModeInfo_reserved: .skip 206, 0

# VBE.ModeAttributes:
# 	ModeAttributes_available equ 1 << 0
# 	ModeAttributes_bios equ 1 << 2
# 	ModeAttributes_color equ 1 << 3
# 	ModeAttributes_graphics equ 1 << 4
# 	ModeAttributes_vgacompatible equ 1 << 5
# 	ModeAttributes_notbankable equ 1 << 6
# 	ModeAttributes_linearframebuffer equ 1 << 7
	
VBEEDID:
	VBEEDID_header: .skip 8, 0
	VBEEDID_manufacturer: .skip 2, 0
	VBEEDID_productid: .skip 2, 0
	VBEEDID_serial: .skip 4, 0
	VBEEDID_manufactureweek: .skip 1, 0
	VBEEDID_manufactureyear: .skip 1, 0
	VBEEDID_version: .skip 1, 0
	VBEEDID_revision: .skip 1, 0
	VBEEDID_input: .skip 1, 0
	VBEEDID_horizontalsize: .skip 1, 0
	VBEEDID_verticalsize: .skip 1, 0
	VBEEDID_gamma: .skip 1, 0
	VBEEDID_displaytype: .skip 1, 0
	VBEEDID_chromaticity: .skip 10, 0
	VBEEDID_timingI: .skip 1, 0
	VBEEDID_timingII: .skip 1, 0
	VBEEDID_timingreserved: .skip 1, 0
	VBEEDID_standardtiming:	.skip 16, 0	# format: db (horizontal-248)/8, aspectratio | verticalfrequency - 60
		# VBEEDID_standardtiming_aspect.16.10	equ 0 	# mul horizontal by 10, shr 4 to get vertical resolution
		# VBEEDID_standardtiming_aspect.4.3	equ 1 << 6	# mul horizontal by 3, shr 2 to get vertical resolution
		# VBEEDID_standardtiming_aspect.5.4	equ 2 << 6	# shl horizontal by 2, div by 5 to get vertical resolution
		# VBEEDID_standardtiming_aspect.16.9	equ 3 << 6	# mul horizontal by 9, shr by 4 to get vertical resolution
	VBEEDID_descriptorblock1: .skip 18, 0
	VBEEDID_descriptorblock2: .skip 18, 0
	VBEEDID_descriptorblock3: .skip 18, 0
	VBEEDID_descriptorblock4: .skip 18, 0
	VBEEDID_extensionflag: .skip 1, 0
	VBEEDID_checksum: .skip 1, 0

config:
  config_xres: .2byte 0
  config_yres: .2byte 0

# print a string
# IN
#   si: points at zero-terminated String
# CLOBBER
#   si, ax
print:
    pushf
    cld
print_loop:
    lodsb
    test al, al
    jz print_done
    call print_char
    jmp print_loop
print_done:
    popf
    ret


# print a character
# IN
#   al: character to print
print_char:
    pusha
    mov bx, 7
    mov ah, 0x0e
    int 0x10
    popa
    ret
