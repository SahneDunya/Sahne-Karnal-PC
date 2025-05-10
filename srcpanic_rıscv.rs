#![no_std] // Bu dosya standart kütüphaneye ihtiyaç duymaz, çekirdek alanında çalışır

use core::panic::PanicInfo;
use core::fmt::Write; // core::fmt::Write trait'ini kullanacağız
use core::ptr; // Pointer işlemleri için

// Karnal64 API fonksiyonlarını ve tiplerini kullanmak için karnal64 modülünü/crate'ini içe aktarın.
// Bu, bu dosyanın `karnal64.rs` dosyasındaki public öğelere erişebilmesi gerektiğini varsayar.
use karnal64::{
    resource_acquire, resource_write, KError, KHandle,
    // karnal64::kresource modülündeki MODE_WRITE sabiti public olmalı
    kresource::MODE_WRITE,
};

// --- Sabit Boyutlu Tampon ve Yazıcı ---
// Panik mesajını formatlamak için yığın ayırma (heap allocation) kullanamayız
// (`no_std` ortamında ve panik anında ayırıcı çalışmayabilir).
// Bunun yerine, sabit boyutlu statik bir tampon ve bu tampuna yazan bir yardımcı yapı kullanacağız.

const PANIC_BUFFER_SIZE: usize = 512; // Panik mesajı için tampon boyutu
static mut PANIC_BUFFER: [u8; PANIC_BUFFER_SIZE] = [0u8; PANIC_BUFFER_SIZE]; // Statik tampon

// core::fmt::Write trait'ini uygulayan basit bir yazıcı yapısı.
// Mesajları PANIC_BUFFER'a yazar.
struct PanicBufferWriter {
    cursor: usize, // Tampondaki mevcut yazma konumu
}

impl PanicBufferWriter {
    // Yeni bir yazıcı oluşturur.
    fn new() -> Self {
        PanicBufferWriter { cursor: 0 }
    }

    // Tampondaki içeriği, yazılan kısma kadar bir byte slice olarak döndürür.
    fn as_slice(&self) -> &[u8] {
        // GÜVENLİK: Bu fonksiyon, imlecin tampon sınırları içinde olduğunu varsayar.
        // write_str metodunda bu kontrol yapılır.
        unsafe {
            &PANIC_BUFFER[..self.cursor]
        }
    }
}

// core::fmt::Write trait implementasyonu. string slice'ları tampona yazar.
impl Write for PanicBufferWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let remaining_space = PANIC_BUFFER_SIZE - self.cursor;
        let bytes_to_write = core::cmp::min(bytes.len(), remaining_space);

        if bytes_to_write > 0 {
            // GÜVENLİK: copy_nonoverlapping çağrısı güvenlidir çünkü:
            // - Kaynak pointer (bytes.as_ptr()) geçerli bir slice'a işaret eder.
            // - Hedef pointer (PANIC_BUFFER.as_mut_ptr().add(self.cursor)) statik tamponumuzun içinde ve yazma izni var.
            // - kopyalanacak boyut (bytes_to_write) hem kaynaktan okunabilecek byte sayısı hem de hedefe yazılabilecek kalan alan kadardır.
            unsafe {
                ptr::copy_nonoverlapping(
                    bytes.as_ptr(),
                    PANIC_BUFFER.as_mut_ptr().add(self.cursor),
                    bytes_to_write,
                );
            }
            self.cursor += bytes_to_write;
        }

        // Eğer yazılacak her byte için yer kalmadıysa, hata döndür.
        if bytes_to_write < bytes.len() {
            Err(core::fmt::Error)
        } else {
            Ok(())
        }
    }
}

// --- Panik İşleyici ---
// Bu fonksiyon, bir panik oluştuğunda Rust çalışma zamanı tarafından çağrılır.
#[panic_handler]
// `#[no_mangle]` panik işleyicisinin sembol adının değiştirilmemesini sağlar,
// böylece çekirdeğin giriş kodu (bootloader/assembly) onu bulabilir.
#[no_mangle]
pub extern "C" fn panic_handler(info: &PanicInfo) -> ! {
    // Panik işleyicisi, senkronizasyon ilkel (primitive) kullanmadan
    // statik değişkenlere erişmek zorunda kalabilir. Bu nedenle 'unsafe' kullanılır.
    // Gerçek bir çekirdekte, panik anında birden fazla çekirdek
    // aynı anda bu koda girerse dikkatli olunmalıdır (örn. NMI panik işleyicisi).
    let mut writer = PanicBufferWriter::new();

    // Mesajı tampuna yazın
    // write! makrosunu kullanabilmek için Write trait'ini uyguladık.
    // Hataları görmezden gelin, çünkü panik anında hata işlemek zordur.
    let _ = writer.write_str("KERNEL PANIC: ");

    // Panik mesajını al (varsa) veya varsayılan bir mesaj kullan
    let message = info.message().and_then(|msg| msg.as_str()).unwrap_or("Unknown panic cause");
    let _ = writer.write_str(message);

    // Panik konumunu al (varsa)
    if let Some(location) = info.location() {
        let _ = writer.write_str(" at ");
        let _ = writer.write_str(location.file());
        let _ = writer.write_str(":");
        // core::fmt::Write implementasyonumuz sayesinde write! makrosunu kullanabiliriz.
        let _ = write!(writer, "{}", location.line());
        let _ = writer.write_str(":");
        let _ = write!(writer, "{}", location.column());
    }

    // Yeni satır ekle
    let _ = writer.write_str("\n");

    // Formatlanmış mesajı byte slice olarak al
    let panic_message_bytes = writer.as_slice();

    // --- Karnal64 API'sini Kullanarak Konsola Yaz ---

    // Konsol kaynağının kimliği (bu, karnal64.rs'deki kresource modülünde
    // kaydedilen kimlikle eşleşmeli).
    let console_resource_id = b"karnal://device/console"; // Byte slice

    // Konsol kaynağı için handle edinmeye çalışın (yazma modunda)
    // Bu, karnal64.rs dosyasındaki `resource_acquire` public fonksiyonunu çağırır.
    let console_handle_result = resource_acquire(
        console_resource_id.as_ptr(), // Kaynak kimliği pointer'ı
        console_resource_id.len(),    // Kaynak kimliği uzunluğu
        MODE_WRITE,                    // Erişim modu (karnal64::kresource::MODE_WRITE)
    );

    if let Ok(console_handle) = console_handle_result {
        // Handle başarıyla alındıysa, panik mesajını kaynağa yazmayı deneyin.
        // Bu, karnal64.rs dosyasındaki `resource_write` public fonksiyonunu çağırır.
        let write_result = resource_write(
            console_handle.0,              // Karnal64 Handle değeri (u64)
            panic_message_bytes.as_ptr(), // Yazılacak veri (tampondaki mesaj) pointer'ı
            panic_message_bytes.len(),    // Yazılacak veri uzunluğu
        );

        // Panik anında yazma hatasıyla başa çıkmak zor. Hatayı görmezden gelin.
        let _ = write_result;

        // Handle'ı serbest bırakmak teorik olarak doğru olsa da, panik anında
        // sistem zaten durmak üzere olduğu için bu gerekli veya güvenli olmayabilir.
         kresource::resource_release(console_handle.0); // İsteğe bağlı, dikkatli kullanılmalı
    }
    // Handle edinilemezse, konsola yazamayız. Bu durumda mesaj sessizce kaybolur.

    // --- Sistemi Durdur ---
    // Panikler kurtarılamaz hatalardır. İşleyici döndürmemelidir.
    // En basit durdurma şekli sonsuz döngüdür.
    loop {}

    // Alternatif olarak, donanıma özgü bir durdurma komutu kullanılabilir
    // (örn. RISC-V `WFI` - Wait For Interrupt olabilir, ancak sistemin
    // bir daha asla kesme almayacağını varsayar). Sonsuz döngü en güvenlisidir.
}
