use core::panic::PanicInfo;
use shared::{println, instructions};

#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    println!("[Panic] {}", info);

    loop {
        instructions::hlt()
    }
}
