#![no_std] // Standart kütüphane yok, çekirdek alanındayız

// PowerPC mimarisine özgü importlar veya modüller (gerektiğinde eklenecek)
 mod cpu;
 mod mmu;
 mod trap;
 mod timer;
 mod uart; // Konsol çıkışı için basit seri port sürücüsü

// Karnal64 çekirdek API'sına erişim izni ver
// 'super' anahtar kelimesi, bir üst seviyedeki (yani src/lib.rs veya src/main.rs gibi
// ana çekirdek kodu dosyası) scope'a bakar ve oradan karnal64 modülünü bulur.
// Gerçek bir projede, karnal64 modülü muhtemelen ana çekirdek kraterinin içinde
// yer alır ve bu şekilde erişilir.
extern crate alloc; // Karnal64 içinde Box veya Vec gibi türler kullanılıyorsa gerekebilir.
                    // Eğer alloc da yoksa, statik veya arena tabanlı tahsis kullanılmalı.

// Kernel.rs dosyasından Karnal64 API'sını import et (Eğer karnal64 src/lib.rs içindeyse)
// Ya da doğrudan karnal64 kraterinden import etme şekli projenizin yapısına göre değişir.
// Varsayım: ana çekirdek kraterinde 'karnal64' modülü var.
#[path = "../../karnal64.rs"] // Geçici olarak dosya yolunu belirtiyorum, gerçekte modül import edilir.
mod karnal64;

use karnal64::{KError, KHandle}; // Karnal64 API'sından hata ve handle tiplerini kullanıyoruz

// --- PowerPC Platformuna Özgü Başlatma ---

/// Çekirdeğin PowerPC platformuna özgü başlatma fonksiyonu.
/// Bu, çekirdek bootloader'ı tarafından çağrılan ilk Rust fonksiyonlarından biri olmalıdır.
/// CPU, MMU, kesme denetleyicisi gibi donanımları başlatır ve ardından
/// genel Karnal64 başlatma fonksiyonunu çağırır.
#[no_mangle] // Bootloader'ın bu fonksiyonu bulabilmesi için isminin değişmemesi gerekir
pub extern "C" fn platform_init() {
    // Güvenlik: Bu fonksiyon çağrıldığında çok temel bir ortamın (stack pointer ayarlı vb.)
    // bootloader tarafından kurulduğu varsayılır.

    // TODO: PowerPC mimarisine özgü erken donanım başlatma adımları
    // - CPU çekirdek durumu ayarları ( privileged mode, fpu vb.)
    // - Erken konsol (UART) başlatma, böylece debug mesajları yazılabilir
    // - Kesme denetleyicisini başlatma ve temel kesme vektörlerini kurma
    // - MMU'yu başlatma (temel kernel bellek haritasını kurma)
    // - Zamanlayıcıyı başlatma

    // Örnek: Çok temel bir boot mesajı (UART sürücüsü implemente edilirse)
    // platform_uart::print("Karnal64 PowerPC Platformu Başlatılıyor...\n");
    println!("Karnal64 PowerPC Platformu Başlatılıyor..."); // Eğer global bir print! makrosu tanımlıysa

    // Genel Karnal64 çekirdek başlatma fonksiyonunu çağır.
    // Bu fonksiyon, resource manager, task manager gibi Karnal64 iç modüllerini başlatır.
    karnal64::init();

    // TODO: Daha sonraki başlatma adımları
    // - Cihaz sürücülerini kaydetme (platforma özgü cihazlar için ResourceProvider implementasyonları)
    // - İlk kullanıcı görevi (init/shell) oluşturma ve zamanlayıcıya ekleme
    // - Kesmeleri etkinleştirme (çekirdek başlatma tamamlandıktan sonra)

    // Karnal64 init'ten sonra, sistem çağrıları ve kesmeler işlenmeye hazır hale gelir.
    println!("Karnal64 Genel Başlatma Tamamlandı.");

    // TODO: Eğer ilk görev başlatılmadıysa veya bootloader otomatik çalıştırmıyorsa
    // buradan ilk kullanıcı görevine (genellikle assembly trampolin koduna) geçiş yapılabilir.
    // Bu, bağlam değiştirme (context switch) mantığı gerektirir.
     ktask::start_first_user_task(); // Karnal64 task manager API'sından bir fonksiyon
}

// --- PowerPC Sistem Çağrısı İşleyici ---

/// PowerPC sistem çağrısı tuzağını (trap) yakalayan düşük seviyeli işleyici.
/// Donanım bu fonksiyona dallandığında CPU durumu (registerlar) çekirdek yığıtına
/// kaydedilmiş olmalıdır (genellikle assembly boot kodunda veya trap vektöründe yapılır).
/// Bu fonksiyon, kullanıcı alanından gelen ham argümanları toplar, genel
/// karnal64::handle_syscall fonksiyonunu çağırır ve sonucu kullanıcıya döndürmek üzere
/// uygun registerlara yerleştirir.
///
/// Güvenlik Notu: Kullanıcı registerlarının güvenli bir şekilde kaydedilip geri yüklenmesi
/// ve yığıt güvenliği kritik öneme sahiptir. Buradaki kod sadece kavramsal bir yapıdır.
///
/// Argümanların ve sistem çağrısı numarasının hangi PowerPC registerlarında
/// olduğuna dair PowerPC ABI (Application Binary Interface) bilgisine ihtiyaç vardır.
/// Varsayım: R3 (syscall number), R4-R8 (argümanlar), sonuç R3'e konur.
#[no_mangle] // Kesme/trap vektör tablosu tarafından çağrılacağı için isminin değişmemesi gerekir
pub extern "C" fn powerpc_syscall_handler(
    // CPU registerlarının kaydedildiği yerin pointer'ı (örn. trap frame yapısı)
    // Bu yapı mimariye ve kullanılan bağlam değiştirme/trap işleme yöntemine göre değişir.
    // Örnek olarak sadece argümanları ve syscall numarasını alalım, gerçekte trap frame kullanılır.
    syscall_number: u64, // Varsayımsal olarak R3'ten geldi
    arg1: u64,         // Varsayımsal olarak R4'ten geldi
    arg2: u64,         // Varsayımsal olarak R5'ten geldi
    arg3: u64,         // Varsayımsal olarak R6'ten geldi
    arg4: u64,         // Varsayımsal olarak R7'den geldi
    arg5: u64,         // Varsayımsal olarak R8'den geldi
    // ... Diğer registerlar ve durum bilgileri de trap frame içinde olabilir ...
) -> i64 { // Kullanıcı alanına döndürülecek sonuç (R3'e konulur)

    // TODO: Gelen argümanların (özellikle pointer olanların) kullanıcı alanında geçerli ve
    // erişilebilir olduğunu DOĞRULAMA mekanizması buraya veya handle_syscall içine eklenmelidir.
    // Bu, MMU ve sayfa tabloları ile etkileşimi gerektirir.

    // Genel Karnal64 sistem çağrısı işleyicisini çağır.
    // Bu fonksiyon, sistem çağrısı numarasına göre ilgili Karnal64 API fonksiyonunu
    // (örn. karnal64::resource_read) dispatch eder.
    let result = unsafe { // User pointer doğrulaması yapılacağı varsayılırsa safe olabilir
        karnal64::handle_syscall(syscall_number, arg1, arg2, arg3, arg4, arg5)
    };

    // handle_syscall fonksiyonu sonucu zaten i64 formatında döndürür
    // (başarı için >= 0, hata için negatif KError değeri).
    // Bu sonuç, PowerPC'nin sistem çağrısı dönüş registerına (varsayım: R3)
    // trap'ten çıkış sırasında yerleştirilecektir.
    result
}

// --- PowerPC MMU Etkileşim Fonksiyonları (kmemory modülü tarafından çağrılır) ---
// Karnal64'ün genel bellek yöneticisi (kmemory), donanımdan bağımsız mantığı içerir.
// Ancak sayfa tablosu manipülasyonu, TLB temizleme gibi mimariye özgü işlemler
// için bu platform modülünü çağırır.

mod mmu_powerpc {
    // TODO: PowerPC MMU registerları ve komutları ile etkileşim için güvenli (veya unsafe) sarmalayıcılar
    // Örn: Sayfa tablosu girdisi ekleme, TLB temizleme/geçersiz kılma, MMU'yu etkinleştirme/devre dışı bırakma.
}

// TODO: Ktask modülünün çağıracağı bağlam değiştirme (context switch) fonksiyonları
// Bu, PowerPC'nin register setini kaydetme/geri yükleme assembly kodunu çağırır.
 pub unsafe fn switch_context(old_ctx: *mut TaskContext, new_ctx: *const TaskContext);

// TODO: Kesme işleme alt sistemi
// PowerPC'nin kesme vektörlerini kurma ve gelen kesmeleri doğru işleyicilere yönlendirme mantığı.
 pub fn handle_interrupt(irq_number: u32, trap_frame: *mut TrapFrame);

// TODO: Diğer platforma özgü donanım etkileşimleri (cihaz sürücüleri için temel arayüzler)
 pub fn read_uart_register(addr: usize) -> u8;
 pub fn write_uart_register(addr: usize, value: u8);

// --- Yer Tutucu Print Fonksiyonu ---
// Erken boot aşamasında veya UART sürücüsü tam implemente olmadan debug için kullanılabilir.
// Gerçekte donanıma yazan bir UART sürücüsü tarafından desteklenmelidir.
#[cfg(feature = "enable_uart_debug_print")] // Build feature ile kontrol edilebilir
mod debug_print {
    // TODO: PowerPC UART donanım adresine yazan düşük seviye implementasyon
     #[no_mangle] // Eğer assembly'den çağrılıyorsa
     pub extern "C" fn _putchar(c: u8) { /* write c to UART data register */ }

    // TODO: Formatlı çıktı için basit bir mekanizma (eğer Rust'ın format! makrosu kullanılabiliyorsa)
}

// Basit bir println! makrosu yer tutucusu (debug_print modülü aktifse)
// Gerçek bir çekirdekte daha gelişmiş logging/print! makroları kullanılır.
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ({
        // TODO: Format string'i al ve debug_print modülünü kullanarak yazdır
         debug_print::_print(format_args!($($arg)*)); // 'alloc' veya farklı bir formatlama gerekir
    });
}
