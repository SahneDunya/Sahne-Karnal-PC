#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz, çekirdek alanında çalışırız

use core::panic::PanicInfo;
// core::fmt::Write traitini kullanabilmek için ihtiyaç duyulabilir,
// ancak çekirdek içinde basic bir seri port yazıcısı veya konsol sağlayıcı
// tarafından implemente edilmesi gerekir. Bu örnekte basitleştirme yapabiliriz.
 use core::fmt::Write;

// Karnal64 temel tipleri ve API fonksiyonları için use bildirimleri
// karnal64.rs dosyasındaki public öğeleri buradan çağıracağız.
// struct, enum gibi tipleri kullanmak için tam yolu belirtmemiz gerekebilir
// veya karnal64 modülünü use etmemiz gerekebilir.
use karnal64::{KError, KHandle};
// Varsayımsal olarak kresource modülündeki public fonksiyonlar (veya karnal64'ün kendi pub fonksiyonları)
use karnal64::kresource; // Eğer bu modüller dışarıya public fonksiyonlar sunuyorsa
use karnal64::{resource_acquire, resource_write}; // Karnal64'ün public API fonksiyonları

// PowerPC'ye özgü düşük seviye fonksiyonlar için placeholder'lar
// Bu fonksiyonlar gerçek donanım etkileşimini içerecektir.
mod arch_powerpc {
    /// Tüm kesmeleri devre dışı bırakır.
    #[inline(always)]
    pub fn disable_interrupts() {
        // TODO: PowerPC'ye özgü kesme devre dışı bırakma komutları (örn. mtspr)
        // Bu sadece bir yer tutucudur.
        unsafe {
            // Gerçek bir PowerPC çekirdeğinde buraya uygun assembly veya
            // mimariye özgü Rust intrinsic fonksiyonları gelecektir.
            core::arch::asm!("// disable interrupts (PowerPC specific)", options(nostack, nomem));
        }
    }

    /// CPU'yu durdurur (sonsuz döngü veya halt komutu).
    #[inline(always)]
    pub fn halt() -> ! {
        // TODO: PowerPC'ye özgü durdurma komutları veya sonsuz döngü.
        // Bu sadece bir yer tutucudur.
        unsafe {
            // Gerçek bir PowerPC çekirdeğinde buraya uygun assembly veya
            // mimariye özgü Rust intrinsic fonksiyonları gelecektir.
             core::arch::asm!("1: b 1b", options(noreturn)); // Sonsuz döngü
        }
    }
    
    /// Kaydedilmiş CPU registerlarını döker (debugging için).
    /// Genellikle panic anındaki register değerleri, trap çerçevesinden alınır.
    /// Bu fonksiyon, trap çerçevesinin yapısına ve nasıl erişileceğine bağlıdır.
    /// Burada sadece kavramsal bir yer tutucu.
    pub fn dump_registers(panic_info: &PanicInfo) {
        // TODO: PowerPC registerlarını dump etme mantığı
        // Trap/kesme işleyicisinin panik anında kaydettiği registerlara erişim gerekir.
        let _ = panic_info; // Kullanılmadığı uyarısını engelle
         println!("PowerPC Registers: ..."); // Çekirdek içi print! gerektirir
    }
}


/// Çekirdek panik işleyicisi.
/// `#[panic_handler]` özniteliği ile işaretlenir.
/// Bir panik oluştuğunda Rust çalışma zamanı tarafından çağrılır.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 1. Kesmeleri devre dışı bırak:
    // Bu, panik sırasında başka kesmelerin veya bağlam değişimlerinin olmasını engeller.
    arch_powerpc::disable_interrupts();

    // 2. Panik mesajını ve konumunu almak için Karnal64 konsolunu kullanmayı dene:
    // Konsol kaynağına bir handle edinmeyi deneyeceğiz.
    // Varsayımsal olarak konsol kaynağının adı "karnal://device/console" olsun
    // ve acquire için yazma izni MOD_WRITE gerektirsin.
    // MODE_WRITE değeri, karnal64::kresource::MODE_WRITE gibi bir yerden gelmeli,
    // ancak kresource modülü public değilse, karnal64'ün kendisinden public bir sabit veya fonksiyon
    // aracılığıyla alınmalıdır veya doğrudan buraya kopyalanabilir (ABI uyumluluğu için).
    // Örnek olarak, karnal64::MODE_WRITE public olduğunu varsayalım:
    let console_handle_result = resource_acquire(
        "karnal://device/console".as_ptr(), // Kaynak ID (pointer)
        "karnal://device/console".len(),   // Kaynak ID uzunluğu
        karnal64::MODE_WRITE // Talep edilen modlar (write izni)
    );

    // Eğer konsol handle'ı başarılı bir şekilde edindiysek, panik bilgilerini yaz.
    if let Ok(console_handle) = console_handle_result {
        // Panik mesajını ve konumunu bir string'e veya byte slice'a formatla
        // Çekirdek alanında fmt::Write implementasyonu veya basit bir yazma fonksiyonu gerekir.
        // Basitlik için, panik mesajını ve konumunu string olarak alıp byte slice'a çevirelim.
        // Gerçek bir çekirdekte daha sofistike formatlama ve yazma mantığı olur.

        let mut message_buffer: [u8; 256] = [0; 256]; // Geçici buffer
        let mut cursor = 0;

        // "PANIC: " yazısı
        let prefix = b"PANIC: ";
        if cursor + prefix.len() < message_buffer.len() {
            message_buffer[cursor..cursor + prefix.len()].copy_from_slice(prefix);
            cursor += prefix.len();
        }

        // Panik mesajı
        if let Some(message) = info.message() {
            let message_str = message.as_str().unwrap_or("<no message>"); // Option<&'a fmt::Arguments> -> &str
            let message_bytes = message_str.as_bytes();
            let bytes_to_copy = core::cmp::min(message_bytes.len(), message_buffer.len() - cursor);
            if bytes_to_copy > 0 {
                message_buffer[cursor..cursor + bytes_to_copy].copy_from_slice(&message_bytes[..bytes_to_copy]);
                cursor += bytes_to_copy;
            }
        }

        // Panik konumu
        if let Some(location) = info.location() {
            let location_str = format!(" at {}:{}", location.file(), location.line()); // format! çekirdek içi alloc gerektirebilir veya stack üzerinde çalışmalıdır
            let location_bytes = location_str.as_bytes();
             let bytes_to_copy = core::cmp::min(location_bytes.len(), message_buffer.len() - cursor);
            if bytes_to_copy > 0 {
                 message_buffer[cursor..cursor + bytes_to_copy].copy_from_slice(&location_bytes[..bytes_to_copy]);
                 cursor += bytes_to_copy;
            }
        }

        // Yeni satır
        if cursor < message_buffer.len() {
            message_buffer[cursor] = b'\n';
            cursor += 1;
        }

        // Topladığımız mesajı konsola yaz
        // resource_write fonksiyonu KHandle'ın ham u64 değerini bekliyor.
        let _ = resource_write(
            console_handle.0, // KHandle'ın ham u64 değeri
            message_buffer.as_ptr(), // Yazılacak veri pointer'ı
            cursor // Yazılacak veri uzunluğu
        );

        // TODO: Başka hata ayıklama bilgileri yazılabilir (görev ID, registerlar vb.)
        // Varsayımsal olarak bir görev ID alma fonksiyonu olsun:
         if let Ok(task_id) = karnal64::get_current_task_id() {
            let task_id_msg = format!("Task ID: {}\n", task_id.0);
            let _ = resource_write(console_handle.0, task_id_msg.as_bytes().as_ptr(), task_id_msg.len());
         }

        // PowerPC registerlarını dump et (yer tutucu)
        arch_powerpc::dump_registers(info);

        // TODO: Konsol handle'ını panik durumunda serbest bırakmak güvenli olmayabilir.
        // Kaynak yöneticisinin panik anında istikrarlı kalması gerekir.
        // Eğer serbest bırakma denenecekse: resource_release(console_handle.0);
    } else {
        // Konsol handle'ı edinilemediyse (örn. konsol servisi başlamadı),
        // panik bilgisini yazamıyoruz. Düşük seviyeli bir hata göstergesi yapabiliriz.
        // Örneğin, bir hata LED'ini yakmak veya çok basit bir mesaj göndermek.
        // Bu örnekte sadece devam edip halt edeceğiz.
        // TODO: Düşük seviyeli hata göstergesi (varsa)
    }


    // 3. Sistemi durdur (halt):
    // Çekirdek paniği geri dönülemez bir durumdur. Sistemi durdurmalıyız.
    arch_powerpc::halt();
}

// Varsayımsal olarak karnal64 crate'inde public olması gereken sabit
// Aslında bu Karnal64'ün public API yüzeyine ait olmalı.
// Bu örnek için burada tanımlıyoruz:
#[allow(dead_code)] // Kullanılmadığı uyarısını engelle
const MODE_WRITE: u32 = 1 << 1; // karnal64::MODE_WRITE'ı taklit ediyoruz
