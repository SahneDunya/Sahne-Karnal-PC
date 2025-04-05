#![no_std]
#![crate_type = "enclave"]

extern crate sgx_types;
extern crate sgx_urts;
#[cfg(feature = "backtrace")]
extern crate backtrace;

use sgx_types::*;
use core::str;
use core::slice;

#[no_mangle]
pub extern "C" fn ecall_hello_world() -> sgx_status_t {
    let message = "Merhaba, SGX Enclave!\n";

    // Mesajı güvenli enclave içinde yazdır
    let mut io = super::user_space::Stdout {};
    let _result = io.write_str(message);

    sgx_status_t::SGX_SUCCESS
}


pub mod user_space {
    use core::fmt;
    use core::fmt::Write;
    use core::result::Result;
    use sgx_types::*;

    pub struct Stdout;

    impl Write for Stdout {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            let c_str = format!("{}\0", s);
            let ptr = c_str.as_ptr() as * const c_char;
            let ret = unsafe { ocall_print_string(ptr) };
            if ret != sgx_status_t::SGX_SUCCESS {
                return Err(fmt::Error);
            }
            Ok(())
        }
    }
}

extern {
    pub fn ocall_print_string(
        string: * const c_char
    ) -> sgx_status_t;
}

#[panic_handler]
#[no_mangle]
pub fn panic(info: &core::panic::PanicInfo) -> ! {
    #[cfg(feature = "backtrace")]
    eprintln!("{:?}", backtrace::Backtrace::new());
    println!("panic in enclave: {}", info);
    loop {}
}