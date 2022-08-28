use core::arch::asm;

pub trait UnwrapOrFail {
    type Out;

    fn unwrap_or_fail(self, code: u8) -> Self::Out;
}

impl<T> UnwrapOrFail for Option<T> {
    type Out = T;

    fn unwrap_or_fail(self, code: u8) -> Self::Out {
        match self {
            Some(v) => v,
            None => fail(code),
        }
    }
}

impl<T, E> UnwrapOrFail for Result<T, E> {
    type Out = T;

    fn unwrap_or_fail(self, code: u8) -> Self::Out {
        match self {
            Ok(v) => v,
            Err(_) => fail(code),
        }
    }
}

#[no_mangle]
pub extern "C" fn print_char(c: u8) {
    let ax = u16::from(c) | 0x0e00;
    unsafe {
        asm!("push bx", "mov bx, 0", "int 0x10", "pop bx", in("ax") ax);
    }
}

#[cold]
#[inline(never)]
#[no_mangle]
pub extern "C" fn fail(code: u8) -> ! {
    print_char(b'!');
    print_char(code);
    loop {
        hlt()
    }
}

fn hlt() {
    unsafe {
        asm!("hlt");
    }
}

#[panic_handler]
#[cfg(not(test))]
pub fn panic(_info: &core::panic::PanicInfo) -> ! {
    fail(b'P');
}
