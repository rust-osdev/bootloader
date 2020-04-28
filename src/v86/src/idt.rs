use core::marker::PhantomData;
use bit_field::BitField;

use bit_field::BitField;

/// An Interrupt Descriptor Table with 32 entries.
#[derive(Clone)]
#[repr(C, align(16))]
pub struct InterruptDescriptorTable {
    pub divide_error: Entry<HandlerFunc>,
    pub debug: Entry<HandlerFunc>,
    pub non_maskable_interrupt: Entry<HandlerFunc>,
    pub breakpoint: Entry<HandlerFunc>,
    pub overflow: Entry<HandlerFunc>,
    pub bound_range_exceeded: Entry<HandlerFunc>,
    pub invalid_opcode: Entry<HandlerFunc>,
    pub device_not_available: Entry<HandlerFunc>,
    pub double_fault: Entry<DivergingHandlerFuncWithErrCode>,
    coprocessor_segment_overrun: Entry<HandlerFunc>,
    pub invalid_tss: Entry<HandlerFuncWithErrCode>,
    pub segment_not_present: Entry<HandlerFuncWithErrCode>,
    pub stack_segment_fault: Entry<HandlerFuncWithErrCode>,
    pub general_protection_fault: Entry<HandlerFuncWithErrCode>,
    pub page_fault: Entry<HandlerFuncWithErrCode>,
    reserved_1: Entry<HandlerFunc>,
    pub x87_floating_point: Entry<HandlerFunc>,
    pub alignment_check: Entry<HandlerFuncWithErrCode>,
    pub machine_check: Entry<DivergingHandlerFunc>,
    pub simd_floating_point: Entry<HandlerFunc>,
    pub virtualization: Entry<HandlerFunc>,
    reserved_2: [Entry<HandlerFunc>; 9],
    pub security_exception: Entry<HandlerFuncWithErrCode>,
    reserved_3: Entry<HandlerFunc>,
}

impl InterruptDescriptorTable {
    /// Creates a new IDT filled with non-present entries.
    #[inline]
    pub const fn new() -> InterruptDescriptorTable {
        InterruptDescriptorTable {
            divide_error: Entry::missing(),
            debug: Entry::missing(),
            non_maskable_interrupt: Entry::missing(),
            breakpoint: Entry::missing(),
            overflow: Entry::missing(),
            bound_range_exceeded: Entry::missing(),
            invalid_opcode: Entry::missing(),
            device_not_available: Entry::missing(),
            double_fault: Entry::missing(),
            coprocessor_segment_overrun: Entry::missing(),
            invalid_tss: Entry::missing(),
            segment_not_present: Entry::missing(),
            stack_segment_fault: Entry::missing(),
            general_protection_fault: Entry::missing(),
            page_fault: Entry::missing(),
            reserved_1: Entry::missing(),
            x87_floating_point: Entry::missing(),
            alignment_check: Entry::missing(),
            machine_check: Entry::missing(),
            simd_floating_point: Entry::missing(),
            virtualization: Entry::missing(),
            reserved_2: [Entry::missing(); 9],
            security_exception: Entry::missing(),
            reserved_3: Entry::missing(),
        }
    }

    /// Loads the IDT in the CPU using the `lidt` command.
    pub fn load(&'static self) {
        unsafe { self.load_unsafe() }
    }

    /// Loads the IDT in the CPU using the `lidt` command.
    ///
    /// # Safety
    ///
    /// As long as it is the active IDT, you must ensure that:
    ///
    /// - `self` is never destroyed.
    /// - `self` always stays at the same memory location. It is recommended to wrap it in
    /// a `Box`.
    ///
    pub unsafe fn load_unsafe(&self) {
        use core::mem::size_of;

        let ptr = DescriptorTablePointer {
            base: self as *const _ as u32,
            limit: (size_of::<Self>() - 1) as u16,
        };

        llvm_asm!("lidt ($0)" :: "r" (&ptr) : "memory");
    }
}

/// A struct describing a pointer to a descriptor table (GDT / IDT).
/// This is in a format suitable for giving to 'lgdt' or 'lidt'.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DescriptorTablePointer {
    /// Size of the DT.
    pub limit: u16,
    /// Pointer to the memory region containing the DT.
    pub base: u32,
}

/// An Interrupt Descriptor Table entry.
///
/// The generic parameter can either be `HandlerFunc` or `HandlerFuncWithErrCode`, depending
/// on the interrupt vector.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Entry<F> {
    offset_low: u16,
    gdt_selector: u16,
    zero: u8,
    options: EntryOptions,
    offset_high: u16,
    phantom: PhantomData<F>,
}

impl<F> Entry<F> {
    /// Creates a non-present IDT entry (but sets the must-be-one bits).
    #[inline]
    pub const fn missing() -> Self {
        Entry {
            gdt_selector: 0,
            offset_low: 0,
            offset_high: 0,
            zero: 0,
            options: EntryOptions::minimal(),
            phantom: PhantomData,
        }
    }
    /// Set the handler address for the IDT entry and sets the present bit.
    ///
    /// For the code selector field, this function uses the code segment selector currently
    /// active in the CPU.
    ///
    /// The function returns a mutable reference to the entry's options that allows
    /// further customization.
    #[inline]
    fn set_handler_addr(&mut self, addr: u32) -> &mut EntryOptions {
        self.offset_low = addr as u16;
        self.offset_high = (addr >> 16) as u16;

        let segment: u16;
        unsafe { llvm_asm!("mov %cs, $0" : "=r" (segment) ) };

        self.gdt_selector = segment;

        self.options.set_present(true);
        &mut self.options
    }
}

macro_rules! impl_set_handler_fn {
    ($h:ty) => {
        impl Entry<$h> {
            /// Set the handler function for the IDT entry and sets the present bit.
            ///
            /// For the code selector field, this function uses the code segment selector currently
            /// active in the CPU.
            ///
            /// The function returns a mutable reference to the entry's options that allows
            /// further customization.
            #[inline]
            pub fn set_handler_fn(&mut self, handler: $h) -> &mut EntryOptions {
                self.set_handler_addr(handler as u32)
            }
        }
    };
}

impl_set_handler_fn!(HandlerFunc);
impl_set_handler_fn!(HandlerFuncWithErrCode);
impl_set_handler_fn!(DivergingHandlerFunc);
impl_set_handler_fn!(DivergingHandlerFuncWithErrCode);

/// Represents the options field of an IDT entry.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntryOptions(u8);

impl EntryOptions {
    /// Creates a minimal options field with all the must-be-one bits set.
    #[inline]
    const fn minimal() -> Self {
        EntryOptions(0b1110)
    }

    /// Set or reset the preset bit.
    #[inline]
    pub fn set_present(&mut self, present: bool) -> &mut Self {
        self.0.set_bit(15, present);
        self
    }
}

/// A handler function for an interrupt or an exception without error code.
pub type HandlerFunc = extern "x86-interrupt" fn(&mut InterruptStackFrame);
/// A handler function for an exception that pushes an error code.
pub type HandlerFuncWithErrCode =
    extern "x86-interrupt" fn(&mut InterruptStackFrame, error_code: u64);
/// A handler function that must not return, e.g. for a machine check exception.
pub type DivergingHandlerFunc = extern "x86-interrupt" fn(&mut InterruptStackFrame) -> !;
/// A handler function with an error code that must not return, e.g. for a double fault exception.
pub type DivergingHandlerFuncWithErrCode =
    extern "x86-interrupt" fn(&mut InterruptStackFrame, error_code: u64) -> !;

/// Represents the interrupt stack frame pushed by the CPU on interrupt or exception entry.
#[derive(Clone)]
#[repr(C)]
pub struct InterruptStackFrame {
    pub eip: u32,
    pub cs: u32,
    pub eflags: u32,
}
