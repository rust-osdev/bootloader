# How to use the APIC

## src/main.rs

In the example provided we will be using a dynamic memory mapping. This is for exemplification only and can be changed
with your own implementation of the memory mapper.

## src/apic.rs

Here we create an `AcpiHandlerImpl` that implements the `AcpiHandler` trait from the `apic` crate.
We have also added an enum with all the APIC registers from
the [OS Dev Wiki](https://wiki.osdev.org/APIC)
The main functions of the file are:

- Init the Local APIC
- Init the IO APIC
- Map the APIC

## src/frame_allocator.rs

This implements a basic frame allocator based on the [blog post](https://os.phil-opp.com/heap-allocation/)

## src/gdt.rs

Implements a basic Global Descriptor Table based on
the [blog post](https://os.phil-opp.com/double-fault-exceptions/#the-global-descriptor-table/) as well as ensures that
all segment registers are written, including `ss` and `ds`.

## src/idt.rs
Implements a basic Interrupt Descriptor Table based on the [blog post](https://os.phil-opp.com/hardware-interrupts/)