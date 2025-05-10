#![no_std] // Standart kütüphane yok, çekirdek alanındayız

// SPARC mimarisine özgü importlar veya modüller (gerektiğinde eklenecek)
 mod cpu;      // CPU özgü registerlar, talimatlar
 mod mmu;      // Bellek Yönetim Birimi (MMU) etkileşimi
 mod trap;     // Tuzak (trap) ve kesme (interrupt) yönetimi
 mod regs;     // SPARC register tanımları ve Register Window yönetimi
 mod timer;    // Donanımsal zamanlayıcı
 mod uart;     // Konsol çıkışı için basit seri port sürücüsü (örneğin, SBus UART)

// Karnal64 çekirdek API'sına erişim izni ver
// Varsayım: ana çekirdek kraterinde 'karnal64' modülü var.
extern crate alloc; // Eğer Box veya Vec gibi türler kullanılacaksa

#[path = "../../karnal64.rs"] // Geçici dosya yolu, gerçekte modül import edilir
mod karnal64;

use karnal64::{KError, KHandle}; // Karnal64 API'sından hata ve handle tiplerini kullanıyoruz

// --- SPARC Platformuna Özgü Başlatma ---

/// Çekirdeğin SPARC platformuna özgü başlatma fonksiyonu.
/// Bu, çekirdek bootloader'ı tarafından çağrılan ilk Rust fonksiyonlarından biri olmalıdır.
/// SPARC CPU, MMU, kesme denetleyicisi, trap tablosu gibi donanımları başlatır
/// ve ardından genel Karnal64 başlatma fonksiyonunu çağırır.
#[no_mangle] // Bootloader'ın bu fonksiyonu bulabilmesi için isminin değişmemesi gerekir
pub extern "C" fn platform_init() {
    // Güvenlik: Bu fonksiyon çağrıldığında çok temel bir ortamın (stack pointer ayarlı vb.)
    // bootloader tarafından kurulduğu varsayılır.

    // TODO: SPARC mimarisine özgü erken donanım başlatma adımları
    // - CPU registerlarının başlangıç ayarları
    // - Trap tablosu adres registerını (TBA) ayarlama ve trap işleyicilerin adreslerini girme
    // - Erken konsol (UART) başlatma (örn. SBus UART)
    // - Kesme denetleyicisini başlatma (örn. I/O MMU/Interrupt Controller)
    // - MMU'yu başlatma (temel kernel bellek haritasını kurma, TSBase, TSB_Ptr vb.)
    // - Zamanlayıcıyı başlatma

    // Örnek: Çok temel bir boot mesajı (UART sürücüsü implemente edilirse)
     platform_uart::print("Karnal64 SPARC Platformu Başlatılıyor...\n");
    println!("Karnal64 SPARC Platformu Başlatılıyor..."); // Eğer global bir print! makrosu tanımlıysa

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
    // Bu, bağlam değiştirme (context switch) mantığı ve Register Window yönetimi gerektirir.
     ktask::start_first_user_task(); // Karnal64 task manager API'sından bir fonksiyon
}


// --- SPARC Sistem Çağrısı İşleyici ---

// SPARC Trap Frame yapısı (Örnek)
// Bir trap (sistem çağrısı dahil) oluştuğunda donanım tarafından kaydedilen veya
// trap işleyici assembly kodu tarafından yığına itilen CPU durumu.
// Register Window'lar nedeniyle bu yapı karmaşık olabilir. Burada sadece temel registerlar ve
// Register Window'lar arasındaki ilişkiyi gösterecek yer tutucular var.
#[repr(C)] // C uyumlu bellek düzeni
pub struct SparcTrapFrame {
    // Global registerlar (g0-g7) - g0 her zaman 0'dır
    g_regs: [u64; 8],
    // Out registerlar (o0-o7) - trap sırasında bunlar yeni window'un in registerları olur
    // Syscall numarası ve argümanlar genellikle o registerlarında bulunur.
    o_regs: [u64; 8],
    // Local registerlar (l0-l7)
    l_regs: [u64; 8],
    // In registerlar (i0-i7) - trap sırasında bunlar önceki window'un out registerları olur
    i_regs: [u64; 8],
    // Program Counter (nPC - next PC)
    tpc: u64,
    // Trap Program Counter (PC where trap occurred)
    ttd: u64, // Trap Type Descriptor (SPARC v9) veya benzeri
    // Processor State Register (PSR) veya State Register (STATE) (SPARC v9)
    state_reg: u64,
    // Window State Register (WSTATE) (SPARC v9)
    wstate: u64,
    // ... Diğer durum registerları ve FP durumu ...
}


/// SPARC sistem çağrısı tuzağını (trap) yakalayan düşük seviyeli işleyici.
/// Donanım bir sistem çağrısı tuzağı ürettiğinde, kontrol buraya dallanır.
/// Trap işleyici assembly kodu, genellikle SparcTrapFrame yapısını yığıta kaydeder
/// ve bu Rust fonksiyonunu çağırır.
///
/// Güvenlik Notu: Kullanıcı registerlarının güvenli bir şekilde kaydedilip geri yüklenmesi,
/// Register Window taşmalarının/alt akışlarının (overflow/underflow) yönetilmesi
/// ve yığıt güvenliği kritik öneme sahiptir. Buradaki kod sadece kavramsal bir yapıdır.
///
/// SPARC sistem çağrısı konvansiyonuna göre:
/// - Sistem çağrısı numarası genellikle %o0 registerında bulunur (SparcTrapFrame::o_regs[0]).
/// - Argümanlar %o1 - %o5 veya %o6 registerlarında bulunur (SparcTrapFrame::o_regs[1..6]).
/// - Sonuç %o0 registerına konulur (SparcTrapFrame::o_regs[0]).
#[no_mangle] // Trap vektör tablosu tarafından çağrılacağı için isminin değişmemesi gerekir
pub extern "C" fn sparc_syscall_handler(
    // Trap sırasında yığıta kaydedilen trap frame'in pointer'ı
    // Bu pointer, kullanıcının yığıtında DEĞİL, kernel yığıtında olmalıdır.
    trap_frame: *mut SparcTrapFrame,
) {
    // Güvenlik: trap_frame pointer'ının geçerli ve güvenli bir kernel yığıtı adresini
    // gösterdiği varsayılır. Bu, düşük seviye trap işleyicinin sorumluluğundadır.
    let frame = unsafe { &mut *trap_frame };

    // SPARC ABI'sine göre sistem çağrısı numarası ve argümanları trap frame'den al
    let syscall_number = frame.o_regs[0]; // %o0
    let arg1 = frame.o_regs[1]; // %o1
    let arg2 = frame.o_regs[2]; // %o2
    let arg3 = frame.o_regs[3]; // %o3
    let arg4 = frame.o_regs[4]; // %o4
    let arg5 = frame.o_regs[5]; // %o5

    // TODO: Gelen argümanların (özellikle pointer olanların - arg1, arg2 vb.)
    // kullanıcı alanında geçerli ve erişilebilir olduğunu DOĞRULAMA mekanizması
    // buraya veya handle_syscall içine eklenmelidir. Bu, SPARC MMU'su ile etkileşimi gerektirir.
    // Şu an için doğrulama yapılmadığı varsayılarak unsafe bloğu içinde Karnal64 çağrılıyor.

    // Genel Karnal64 sistem çağrısı işleyicisini çağır.
    // Bu fonksiyon, sistem çağrısı numarasına göre ilgili Karnal64 API fonksiyonunu
    // (örn. karnal64::resource_read) dispatch eder.
    let result = unsafe {
        karnal64::handle_syscall(syscall_number, arg1, arg2, arg3, arg4, arg5)
    };

    // Karnal64'ten dönen i64 sonucu, SPARC ABI'sine göre %o0 registerına konulur.
    frame.o_regs[0] = result as u64; // i64 -> u64 dönüşümü (negatif değerler korunur)

    // Not: Register Window'ların geri yüklenmesi ve trap'ten dönüş (eret)
    // genellikle bu fonksiyon döndükten sonraki assembly kodunda yapılır.
    // Register Window underflow/overflow tuzaklarının da ayrı ele alınması gerekir.
}


// --- SPARC MMU Etkileşim Fonksiyonları (kmemory modülü tarafından çağrılır) ---
// Karnal64'ün genel bellek yöneticisi (kmemory), donanımdan bağımsız mantığı içerir.
// Ancak SPARC sayfa tablosu (TSB - Translation Storage Buffer) manipülasyonu,
// TLB (Translation Lookaside Buffer) yönetimi gibi mimariye özgü işlemler
// için bu platform modülünü çağırır.

mod mmu_sparc {
    use super::*; // Üst scope'taki tipleri ve karnal64'ü kullan

    // TODO: SPARC MMU (örn. UltraSPARC MMU) registerları ve komutları ile etkileşim için güvenli (veya unsafe) sarmalayıcılar
    // Örn: TSB girdisi ekleme, TLB temizleme/geçersiz kılma, MMU'yu etkinleştirme/devre dışı bırakma.
     pub unsafe fn map_page(vpn: u64, ppn: u64, flags: u64); // Sanal -> Fiziksel eşleme
     pub unsafe fn unmap_page(vpn: u64); // Eşlemeyi kaldır
     pub unsafe fn flush_tlb_page(vpn: u64); // Belirli bir sayfa için TLB girdisini geçersiz kıl
     pub unsafe fn flush_tlb_all(); // Tüm TLB'yi temizle
}

// TODO: Ktask modülünün çağıracağı bağlam değiştirme (context switch) fonksiyonları
// Bu, SPARC'ın Register Window setini ve diğer registerları kaydetme/geri yükleme
// assembly kodunu çağırır veya doğrudan implemente eder.
 pub unsafe fn switch_context(old_ctx: *mut TaskContext, new_ctx: *const TaskContext);

// TODO: Kesme işleme alt sistemi
// SPARC'ın kesme vektörlerini kurma ve gelen kesmeleri doğru işleyicilere yönlendirme mantığı.
 pub fn handle_interrupt(trap_type: u64, trap_frame: *mut SparcTrapFrame);

// TODO: Diğer platforma özgü donanım etkileşimleri (cihaz sürücüleri için temel arayüzler)
 pub unsafe fn read_uart_register(addr: usize) -> u8;
 pub unsafe fn write_uart_register(addr: usize, value: u8);


// --- Yer Tutucu Print Fonksiyonu ---
// Erken boot aşamasında veya UART sürücüsü tam implemente olmadan debug için kullanılabilir.
// Genellikle belirli bir seri port donanım adresine doğrudan yazma şeklinde olur.
#[cfg(feature = "enable_uart_debug_print")] // Build feature ile kontrol edilebilir
mod debug_print {
    use core::fmt::{Write, Arguments}; // Formatlama için

    // Örnek SPARC SBus UART veri register adresi (varsayımsal, donanıma göre değişir)
    const UART_DATA: usize = 0xFFFFFFFFE0000000usize;
    // Örnek SPARC SBus UART durum register adresi (varsayımsal)
     const UART_STATUS: usize = 0xFFFFFFFFE0000004usize;
    // TODO: Status register kontrolü (hazır mı?)

    struct SparcUart;

    impl Write for SparcUart {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for byte in s.bytes() {
                // TODO: Durum registerını kontrol ederek göndermeye hazır olana kadar bekle
                unsafe {
                    core::ptr::write_volatile(UART_DATA as *mut u8, byte);
                }
            }
            Ok(())
        }
    }

    // Formatlı çıktı için kullanılan düşük seviye fonksiyon
    #[no_mangle] // Eğer farklı bir yerden çağrılması gerekiyorsa
    pub extern "C" fn _print(args: Arguments) {
        let mut uart = SparcUart;
        uart.write_fmt(args).unwrap(); // Hata yönetimi eklenebilir
    }
}

// Basit bir println! makrosu yer tutucusu (debug_print modülü aktifse)
// Gerçek bir çekirdekte daha gelişmiş logging/print! makroları kullanılır.
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ({
        #[cfg(feature = "enable_uart_debug_print")] // Feature flag'e bağlı
        $crate::platform::mod_sparc::debug_print::_print(format_args!($($arg)*)); // TODO: $crate::platform::mod_sparc yerine path düzeltilmeli
        #[cfg(not(feature = "enable_uart_debug_print"))] // Debug kapalıysa bir şey yapma
        { /* Compile time no-op */ }
    });
}

// Global println! makrosunu çekirdek genelinde kullanılabilir yap
 #[macro_use] extern crate <çekirdek_adı>; // Ana krate dosyasında yapılabilir
