default: run

.PHONY: build objcopy run clean cleanall

build:
	RUST_TARGET_PATH=$(shell pwd) xargo build --target x86_64-bootloader --release

UNAME := $(shell uname)
ifeq ($(UNAME), Linux)
objcopy: build
	objcopy -O binary -S target/x86_64-bootloader/release/bootloader bootimage.bin
endif
ifeq ($(UNAME), Darwin)
objcopy: build
	/usr/local/opt/binutils/bin/objcopy -O binary -S target/x86_64-bootloader/release/bootloader bootimage.bin
endif

run: objcopy
	qemu-system-x86_64 -hda bootimage.bin -d int -s

clean:
	rm -f bootimage.bin

cleanall: clean
	rm -rf target/

