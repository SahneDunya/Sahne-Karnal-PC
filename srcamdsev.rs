#![no_std]

use core::arch::asm;
use core::fmt::Write; // format! makrosu için gerekli

// Konsola karakter yazdırmak için (srcpoweropensbi.rs'deki gibi)
fn console_putchar(c: char) {
    // Bu fonksiyonun gerçek implementasyonu platforma özel olmalıdır.
    // Örneğin, UART veya belirli bir konsol sürücüsü kullanılabilir.
    // Aşağıdaki basit implementasyon sadece örnek amaçlıdır ve çalışmayabilir.
    unsafe {
        asm!(
            "/* Konsola karakter yazdırma platforma özel implementasyonu */",
            "nop", // Gerçek bir implementasyon buraya gelmeli
            in("a0") c as usize,
        );
    }
}

// Yazma trait'ini uygulamak (format! makrosu için)
struct ConsoleWriter;

impl Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for c in s.chars() {
            console_putchar(c);
        }
        Ok(())
    }
}

// Sabit bir ConsoleWriter örneği
static CONSOLE_WRITER: ConsoleWriter = ConsoleWriter;

// CPUID komutunu çalıştırmak için yardımcı fonksiyon
#[inline(always)]
fn cpuid(leaf: u32, sub_leaf: u32) -> (u32, u32, u32, u32) {
    let mut eax: u32;
    let mut ebx: u32;
    let mut ecx: u32;
    let mut edx: u32;

    unsafe {
        asm!(
            "cpuid",
            inout("eax") leaf => eax,
            inout("ecx") sub_leaf => ecx,
            out("ebx") ebx,
            out("edx") edx,
        );
    }
    (eax, ebx, ecx, edx)
}

// SEV özelliklerini kontrol et
fn check_sev_features() {
    let message = "AMD SEV Özellikleri Kontrolü:\n";
    for c in message.chars() {
        console_putchar(c);
    }

    // CPUID fonksiyonu 0x8000000A'yı çağırarak SEV bilgilerini al
    let (_, _, _, edx) = cpuid(0x8000000Au32, 0x0u32);

    // EDX register'ının 16. bitini kontrol et (SEV desteği için)
    let sev_supported = (edx >> 16) & 1;

    if sev_supported == 1 {
        let message_sev_supported = "SEV Destekleniyor!\n";
        for c in message_sev_supported.chars() {
            console_putchar(c);
        }
    } else {
        let message_sev_not_supported = "SEV Desteklenmiyor.\n";
        for c in message_sev_not_supported.chars() {
            console_putchar(c);
        }
    }

    // Daha detaylı SEV bilgilerini almak için CPUID leaf 0x8000001F ve sonrası kullanılabilir.
    // Bu örnek sadece temel SEV desteği algılamayı gösterir.

    let message_done = "Kontrol Tamamlandı.\n";
    for c in message_done.chars() {
        console_putchar(c);
    }
}


#[no_mangle]
pub extern "C" fn _start() -> ! {
    check_sev_features();

    loop {} // Sonsuz döngü
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}