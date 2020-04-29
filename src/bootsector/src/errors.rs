use shared::utils;
use super::console::println;

#[no_mangle]
pub extern "C" fn dap_load_failed() -> ! {
    println(b"[!] DAP Load Failed");
    loop {
        utils::hlt()
    }
}

#[no_mangle]
pub extern "C" fn no_int13h_extensions() -> ! {
    println(b"[!] No int13h Extensions");
    loop {
        utils::hlt()
    }
}