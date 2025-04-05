#![no_std]
#![feature(asm_const)] // Eğer sabit değerleri asm! içinde doğrudan kullanmak istersek

use core::arch::asm;

// Güvenli Dünya Hizmet Çağrı Numaraları (Örnek)
const TZ_CONSOLE_PUTCHAR: usize = 1; // Örnek güvenli konsol çıktı hizmeti numarası

// TrustZone Çağrısı Yapan Yardımcı Fonksiyon
#[inline(always)]
fn tz_call(service_id: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe {
        asm!(
            "smc #0", // Secure Monitor Call (SMC) komutu, #0 genellikle standart SMC kullanımını belirtir.
            inout("r0") service_id => ret, // r0 register'ı hizmet numarası için (girdi ve çıktı)
            in("r1") arg0, // r1 register'ı argüman 0 için (girdi)
            in("r2") arg1, // r2 register'ı argüman 1 için (girdi)
            in("r3") arg2, // r3 register'ı argüman 2 için (girdi)
            options(nomem, nostack), // Hafıza ve yığın etkileşimini belirtmeyen optimizasyon ipuçları
        );
    }
    ret
}

// Güvenli Konsola Karakter Yazdıran Fonksiyon
fn secure_console_putchar(c: char) {
    tz_call(TZ_CONSOLE_PUTCHAR, c as usize, 0, 0);
}

// Test fonksiyonu (main yerine)
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Test mesajı (güvenli konsola yazdırılacak)
    let message = "Güvenli Konsoldan Mesaj: Askıya alınıyor...\n";
    for c in message.chars() {
        secure_console_putchar(c);
    }

    // Hart'ı askıya alma (veya başka bir güvenli işlem) - Örnek olarak sonsuz döngü
    loop {} // Sonsuz döngü (güvenli işlem sonrası veya sistemin askıya alınması beklenebilir)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}