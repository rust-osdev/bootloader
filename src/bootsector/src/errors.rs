use shared::utils;
use super::console::real_mode_println;

#[no_mangle]
extern "C" fn dap_load_failed() -> ! {
    real_mode_println(b"[!] DAP Load Failed");
    loop {
        utils::hlt()
    }
}

#[no_mangle]
extern "C" fn no_int13h_extensions() -> ! {
    real_mode_println(b"[!] No int13h Extensions");
    loop {
        utils::hlt()
    }
}
