#![no_std]
use core::fmt::Write;
use core::panic::PanicInfo;

// Assuming you have a io module like this (adapt to your actual io):
mod io {
    // **SAHNE64 ÖZEL:** VGA tamponu yerine Sahne64'ün konsol genişliği ve yüksekliği
    const SAHNE64_CONSOLE_WIDTH: usize = 80; // Örnek değer, Sahne64'e göre ayarlanmalı
    const SAHNE64_CONSOLE_HEIGHT: usize = 25; // Örnek değer, Sahne64'e göre ayarlanmalı

    // **SAHNE64 ÖZEL:** Sahne64'ün düşük seviyeli yazdırma fonksiyonu (string için)
    extern "C" {
        fn sahne64_console_put_string(s: *const u8, len: usize);
    }

    // **SAHNE64 ÖZEL:** Sahne64'ün düşük seviyeli yazdırma fonksiyonu (tek karakter için)
    extern "C" {
        fn sahne64_console_put_char(c: u8);
    }

    pub fn puts(s: &str) {
        // **SAHNE64 ÖZEL:** Sahne64'ün string yazdırma fonksiyonunu kullan
        unsafe {
            sahne64_console_put_string(s.as_ptr(), s.len());
        }
    }

    pub fn puti(i: isize) {
        let mut buffer = [0u8; 20];
        let s = itoa::itoa(i, &mut buffer);
        puts(s);
    }

    pub struct Writer;

    impl core::fmt::Write for Writer {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            // **SAHNE64 ÖZEL:** Her karakteri tek tek yazdırmak yerine string'i kullan
            puts(s);
            Ok(())
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut writer = io::Writer;

    // Use core::fmt::Write for formatted output
    let _ = write!(writer, "Kernel Panic:\n");

    if let Some(location) = info.location() {
        let _ = write!(writer, "  File: {}\n", location.file());
        let _ = write!(writer, "  Line: {}\n", location.line());
    }

    if let Some(message) = info.message() {
        let _ = write!(writer, "  Message: {}\n", message); // Direct formatting
    } else if let Some(payload) = info.payload().downcast_ref::<&'static str>() {
        let _ = write!(writer, "  Payload (static str): {}\n", payload);
    } else if let Some(payload) = info.payload().downcast_ref::<String>() {
        let _ = write!(writer, "  Payload (String): {}\n", payload);
    } else {
        let _ = write!(writer, "  No panic message or payload available.\n");
    }

    loop {}
}