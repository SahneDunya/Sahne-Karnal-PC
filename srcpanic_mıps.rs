#![no_std]
// Geliştirme sırasında kullanılmayan kod veya argümanlar için izinler (şimdilik)
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)] // Karnal64 API'sından kullanılacak öğeleri import edeceğiz.

// `core` kütüphanesinden panik bilgisi ve formatlama yetenekleri
use core::panic::PanicInfo;
use core::fmt::Write; // Karnal64 kaynağına yazmak için Write trait'ini kullanacağız.

// Karnal64 API'sından ihtiyaç duyulan tipleri ve fonksiyonları içe aktar.
// Karnal64 modülünün/crate'inin burada erişilebilir olduğunu varsayıyoruz.
use karnal64::{
    KError,
    KHandle,
    // Karnal64 API'sının public fonksiyonları:
    resource_acquire,
    resource_write,
    // Karnal64'ün içindeki public sabitler veya tipler (mode bayrakları gibi)
    kresource::MODE_WRITE, // kresource modülü içindeki MODE_WRITE sabitini kullanacağız.
};

// --- MIPS Mimariye Özgü Fonksiyonlar (Placeholder) ---
// Bu fonksiyonlar, gerçek MIPS çekirdek kodunda implemente edilmelidir.

/// MIPS işlemcisi için kesmeleri devre dışı bırakır.
/// Panik sırasında, daha fazla kesme veya zamanlama olmaması için kesmeler kapatılmalıdır.
fn mips_disable_interrupts() {
    // TODO: Gerçek MIPS kesme devre dışı bırakma kodunu buraya ekleyin.
    // Bu genellikle MIPS CP0 Status Register'ı manipüle ederek yapılır.
    unsafe {
         core::arch::asm!("... MIPS assembly for disabling interrupts ...", options(nomem, nostack, preserves_flags));
        // Güvenlik: Gerçek assembly kodu dikkatli yazılmalıdır.
        core::arch::asm!("/* MIPS Interrupt Disable Placeholder */");
    }
}

/// MIPS işlemcisini durdurur veya sistemin donanım seviyesinde durmasını sağlar.
/// Panik sonrası sistem çalışmaya devam etmemelidir.
fn mips_halt_cpu() -> ! {
    // TODO: Gerçek MIPS CPU durdurma veya sonsuz döngü kodunu buraya ekleyin.
    // Genellikle bir sonsuz döngü veya özel bir düşük güç durumu komutu kullanılır.
    unsafe {
         core::arch::asm!("... MIPS assembly for halting/spinning ...", options(noreturn));
        // Güvenlik: Gerçek assembly kodu dikkatli yazılmalıdır.
        core::arch::asm!("/* MIPS CPU Halt Placeholder */");
    }
    // Bu fonksiyon asla geri dönmez.
    loop {}
}

/// Panik anındaki MIPS işlemcisinin kayıt defteri durumunu yakalar ve yazar.
/// Hata ayıklama için çok faydalıdır.
/// Karnal64'ün Yazıcı traitini (Write) kullanan bir nesneye yazar.
fn mips_print_registers<W: Write>(writer: &mut W) {
    // TODO: Gerçek MIPS kayıt defterlerini okuma ve bunları 'writer' nesnesine
    // formatlı bir şekilde yazma kodunu buraya ekleyin.
    // Panik handler'da unsafe bloklar içinde CP0 registerlarına erişim gerekebilir.
    writeln!(writer, "--- MIPS Register State ---").ok(); // .ok() hata durumunda paniklemeyi önler
    writeln!(writer, "  General Purpose Registers:").ok();
    writeln!(writer, "    $zero: ..., $at: ..., $v0: ..., $v1: ...").ok();
    // ... diğer GPR'ler ...
    writeln!(writer, "    $sp: ..., $fp: ..., $ra: ...").ok();
    writeln!(writer, "  Special Registers:").ok();
    writeln!(writer, "    PC: ..., Cause: ..., Status: ...").ok();
    // ... diğer özel registerlar (BadVAddr, Context, vb.) ...
    writeln!(writer, "---------------------------").ok();
}

// --- Karnal64 Konsol Yazıcısı Yardımcısı ---

/// Karnal64 Kaynak Yöneticisi üzerinden konsol kaynağına yazmak için `core::fmt::Write` traitini implemente eden yardımcı struct.
/// Bu sayede `write!` ve `writeln!` makrolarını kullanabiliriz.
struct KarnalConsoleWriter {
    handle: KHandle, // Yazma işlemleri için edindiğimiz konsol kaynağı handle'ı
}

// `core::fmt::Write` traitini implemente ediyoruz
impl Write for KarnalConsoleWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        // String slice'ını byte slice'a dönüştürüyoruz
        let buffer = s.as_bytes();
        // Karnal64 API'sının `resource_write` fonksiyonunu çağırıyoruz.
        // Konsol gibi akış odaklı kaynaklarda ofset genellikle dikkate alınmaz veya 0'dır.
        // Başarılı yazılan byte sayısını veya KError dönecektir.
        let write_result = resource_write(
            self.handle.0,          // Handle'ın ham değeri
            buffer.as_ptr(),        // Yazılacak verinin pointer'ı (çekirdek alanında erişilebilir olmalı)
            buffer.len()            // Yazılacak verinin uzunluğu
        );

        match write_result {
            Ok(bytes_written) => {
                // Konsola yazarken genellikle tamamının yazılmasını bekleriz.
                // Kısmi yazma bir sorun olduğunu gösterebilir, formatlama hatası döndürebiliriz.
                if bytes_written == buffer.len() {
                    Ok(()) // Başarı
                } else {
                    // Kısmi yazma durumu, fmt hatası olarak bildirilir
                    Err(core::fmt::Error)
                }
            },
            Err(_kerror) => {
                // Karnal64 API'sından bir hata döndü (örn: BadHandle, PermissionDenied).
                // Panik anında bu hataları loglamak zor, fmt hatası olarak dönüyoruz.
                Err(core::fmt::Error)
            }
        }
    }
}


// --- Panik İşleyicisi Implementasyonu ---

/// Bu fonksiyon, Rust çalışma zamanı tarafından bir panik oluştuğunda çağrılır.
/// #[panic_handler] attribute'ü gereklidir.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 1. İlk ve en önemli adım: Kesmeleri devre dışı bırakarak sistemi stabilize et.
    mips_disable_interrupts();

    // 2. Panik bilgisini çıkarmak için bir konsol kaynağı edinmeye çalışın.
    // Bu işlem panik anında bileşenlerin kararlı olmasını gerektirir.
    // Başarısız olursa, panik mesajını yazamayız.
    let console_handle_result = resource_acquire(
        "karnal://device/console".as_ptr(), // Konsol kaynağının Karnal64 ID'si
        "karnal://device/console".len(),
        MODE_WRITE // Yazma izni talep et
    );

    match console_handle_result {
        Ok(handle) => {
            // Konsol handle'ını başarıyla aldık, şimdi yazabiliriz.
            let mut writer = KarnalConsoleWriter { handle };

            // Panik başlığını yazdır
            writeln!(writer, "\n--- KERNEL PANIC (MIPS) ---").ok();

            // Panik nerede oldu (dosya/satır) bilgisini yazdır
            if let Some(location) = info.location() {
                writeln!(writer, "Location: {}:{}", location.file(), location.line()).ok();
            } else {
                 writeln!(writer, "Location: Unknown").ok();
            }

            // Panik mesajını yazdır (eğer varsa)
            if let Some(message) = info.message() {
                // Message bir `core::fmt::Arguments` olabilir, doğrudan yazdırılır.
                writeln!(writer, "Message: {}", message).ok();
            } else {
                 writeln!(writer, "Message: No message provided.").ok();
            }

            // 3. MIPS mimariye özgü kayıt defteri durumunu yazdır.
            mips_print_registers(&mut writer);

            writeln!(writer, "---------------------------\n").ok();

            // Panik anında handle'ın serbest bırakılması genellikle yapılmaz veya gerekmez.
            // Sistem zaten duracaktır.

        },
        Err(_acquire_err) => {
            // Konsol handle'ını alamadık. Panik mesajını yazamıyoruz.
            // Bu çok kötü bir durumdur. İdeal olarak, burada düşük seviye donanım
            // (örn. doğrudan UART portu adresine yazma) kullanarak minimal bir hata
            // göstergesi sağlamak gerekebilir.
             unsafe {
                 // Yer Tutucu: Düşük seviye yolla panik sinyali (eğer mümkünse)
                  core::ptr::write_volatile(0xDEADBEEF as *mut u8, 0xFA); // Örnek adres ve değer
             }
        }
    }

    // 4. Sistemi veya ilgili CPU'yu durdur. Bu fonksiyon geri dönmemelidir.
    mips_halt_cpu();
}

// --- Ek Yardımcı Fonksiyonlar (Opsiyonel) ---

// Geliştirme veya test sırasında panik tetiklemek için kullanılabilir.
 #[allow(dead_code)] // Doğrudan çağrılmayabilir.
 pub fn trigger_test_panic(message: &'static str) -> ! {
    panic!("{}", message);
 }