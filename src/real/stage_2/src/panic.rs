use core::panic::PanicInfo;
use shared::println;
use shared::instructions;

#[panic_handler]
pub fn panic(info: &PanicInfo) -> ! {
    println!("[Panic] {}", info);

    loop {
        instructions::hlt()
    }
}
