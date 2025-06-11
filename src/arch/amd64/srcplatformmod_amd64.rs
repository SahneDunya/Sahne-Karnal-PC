#![no_std] // Standart kütüphane yok, çekirdek alanındayız

// x86_64 mimarisine özgü helper kraterlerini kullanabiliriz (opsiyonel ama önerilir)
// Örneğin: 'x86_64' krateri GDT, IDT, sayfa tabloları, port G/Ç erişimi gibi konularda yardımcı olur.
// Eğer projenize bu krateri eklerseniz buraya import edebilirsiniz:
 extern crate x86_64;
 use x86_64::instructions::port::Port; // Port G/Ç için
 use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame}; // IDT ve kesme çerçevesi için
 use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor}; // GDT için
 use x86_64::registers::control::{Cr3, Cr4, Cr0}; // Kontrol registerları için

extern crate alloc; // Karnal64 içinde Box veya Vec gibi türler kullanılıyorsa gerekebilir.

// Kernel.rs dosyasından Karnal64 API'sını import et (Eğer karnal64 src/lib.rs içindeyse)
// Ya da doğrudan karnal64 kraterinden import etme şekli projenizin yapısına göre değişir.
// Varsayım: ana çekirdek kraterinde 'karnal64' modülü var.
#[path = "../../karnal64.rs"] // Geçici olarak dosya yolunu belirtiyorum, gerçekte modül import edilir.
mod karnal64;

use karnal64::{KError, KHandle}; // Karnal64 API'sından hata ve handle tiplerini kullanıyoruz

// --- x86_64 Platformuna Özgü Veri Yapıları ---

// Düşük seviye kesme/trap işleyicileri tarafından kaydedilen CPU durumu.
// Bu yapı, bir kesme veya sistem çağrısı olduğunda donanım tarafından veya
// assembly stub kodu tarafından yığına kaydedilen registerları temsil eder.
// SYSCALL/SYSRET veya INT 0x80 kullanımına göre içeriği ve register sırası değişir.
// Bu sadece kavramsal bir örnektir.
#[repr(C)] // C ABI uyumu için
pub struct TrapFrame {
    // SYSCALL/SYSRET ABI'sine göre registerlar (genel kullanım registerları)
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64, // SYSCALL instruction clears this, often used for RCX copy
    pub r10: u64, // SYSCALL argument (rcx copy)
    pub r9: u64,  // SYSCALL argument
    pub r8: u64,  // SYSCALL argument
    pub rsi: u64, // SYSCALL argument
    pub rdi: u64, // SYSCALL argument (often syscall number or arg1 depending on convention)
    pub rbp: u64,
    pub rbx: u64,
    pub rdx: u64, // SYSCALL argument
    pub rax: u64, // SYSCALL number OR return value
    // Donanım tarafından yığına itilen bilgiler
    // Kesme/Trap türüne göre segment selector, stack pointer, RFLAGS, Instruction Pointer (RIP) değişir.
    // SYSRET stack'e bir şey itmez, INT 0x80/kesmeler iter.
    // Örnek: x86_64 interrupt stack frame yapısı
     pub interrupt_stack_frame: InterruptStackFrame,
}

// TODO: GDT (Global Descriptor Table) yapısı ve fonksiyonları
// TODO: IDT (Interrupt Descriptor Table) yapısı ve fonksiyonları
// TODO: Sayfa Tablosu (PML4, PDPT, PD, PT) yapıları (kmemory tarafından kullanılır)

// --- x86_64 Platformuna Özgü Başlatma ---

/// Çekirdeğin x86_64 platformuna özgü başlatma fonksiyonu.
/// Bu, çekirdek bootloader'ı tarafından çağrılan ilk Rust fonksiyonlarından biri olmalıdır.
/// GDT, IDT, erken sayfalama (paging), PIC/APIC, zamanlayıcı gibi donanımları başlatır
/// ve ardından genel Karnal64 başlatma fonksiyonunu çağırır.
#[no_mangle] // Bootloader'ın bu fonksiyonu bulabilmesi için isminin değişmemesi gerekir
pub extern "C" fn platform_init() {
    // Güvenlik: Bu fonksiyon çağrıldığında çok temel bir ortamın (long mode aktif,
    // minimum bellek haritalı, stack pointer ayarlı vb.) bootloader tarafından
    // kurulduğu varsayılır.

    // TODO: x86_64 mimarisine özgü erken donanım başlatma adımları
    // - GDT (Global Descriptor Table) kurulumu ve yüklenmesi
    // - IDT (Interrupt Descriptor Table) kurulumu ve yüklenmesi (kesme/trap vektörleri buraya yönlendirilir)
    // - Erken sayfalama (paging) kurulumu (çekirdek ve bootloader tarafından kullanılan belleği haritalama)
    //   x86_64::structures::paging modülü faydalı olabilir.
    // - PIC (Programmable Interrupt Controller) veya APIC (Advanced PIC) başlatılması
    // - Zamanlayıcı (Timer) başlatılması

    // Örnek: Çok temel bir boot mesajı (VGA text mode veya seri port sürücüsü implemente edilirse)
     platform_vga::print_str("Karnal64 x86_64 Platformu Başlatılıyor...\n");
    println!("Karnal64 x86_64 Platformu Başlatılıyor..."); // Eğer global bir print! makrosu tanımlıysa

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

// --- x86_64 Sistem Çağrısı İşleyici ---

/// x86_64 sistem çağrısı (SYSCALL/SYSRET veya INT 0x80) tuzağını yakalayan
/// Rust tarafındaki işleyici.
/// Bu fonksiyondan önce çalışan düşük seviye assembly kodu, CPU registerlarını
/// bir `TrapFrame` yapısına veya yığıt üzerine kaydetmelidir.
/// Bu fonksiyon, kaydedilen CPU durumundan sistem çağrısı numarasını ve argümanları okur,
/// genel karnal64::handle_syscall fonksiyonunu çağırır ve sonucu TrapFrame'e
/// (veya dönüş registerına) yazarak assembly kodunun kullanıcıya dönmesini sağlar.
///
/// SYSCALL/SYSRET ABI (SysV AMD64 ABI'sine benzer):
/// - Sistem çağrısı numarası: RAX
/// - Argümanlar: RDI, RSI, RDX, R10, R8, R9
/// - Dönüş değeri: RAX
/// - Clobbered (değişen) registerlar: RCX (RIP'e yazılır), R11 (RFLAGS'a yazılır)
/// - Çekirdek Rip: RCX (SYSCALL'dan önce kaydedilir)
/// - Çekirdek Rflags: R11 (SYSCALL'dan önce kaydedilir)
///
/// Güvenlik Notu: Kullanıcı registerlarının güvenli bir şekilde kaydedilip geri yüklenmesi,
/// kullanıcıdan gelen pointer argümanlarının doğrulanması (TrapFrame üzerinden veya ayrı bir fonksiyonla)
/// ve yığıt güvenliği kritik öneme sahiptir. Buradaki kod sadece kavramsal bir yapıdır.
#[no_mangle] // Assembly trap işleyicisinin bu fonksiyonu bulabilmesi için
pub extern "C" fn x86_64_syscall_handler(
    // Assembly tarafından kaydedilen TrapFrame'in pointer'ı.
    // Gerçek trap işleyici assembly kodu, bu struct'ı uygun registerları kullanarak doldurmalı.
    trap_frame: *mut TrapFrame,
) {
    // Güvenlik: trap_frame pointer'ının geçerli bir çekirdek yığıtı adresini gösterdiğini varsayıyoruz.
    // Ancak kullanıcı pointerlarını (argümanların işaret ettiği yerler) doğrulamak GEREKLİDİR.

    let frame = unsafe { &mut *trap_frame }; // TrapFrame içeriğine erişim

    // Sistem çağrısı numarasını ve argümanları TrapFrame'den al
    let syscall_number = frame.rax; // Varsayılan SYSCALL convention
    let arg1 = frame.rdi; // Varsayılan SYSCALL convention
    let arg2 = frame.rsi; // Varsayılan SYSCALL convention
    let arg3 = frame.rdx; // Varsayılan SYSCALL convention
    let arg4 = frame.r10; // Varsayılan SYSCALL convention
    let arg5 = frame.r8;  // Varsayılan SYSCALL convention
    // Not: arg5 için R9 da kullanılabilir, ABİ'ye ve kullanım şekline bağlıdır.

    // TODO: arg1..arg5 içindeki kullanıcı pointerlarının geçerli ve erişilebilir
    // olduğunu DOĞRULAMA mekanizması buraya veya handle_syscall içine eklenmelidir.
    // Bu, kullanıcının sayfa tabloları ile etkileşimi gerektirir.
     validate_user_pointer(arg_ptr, size, Permissions::READ | Permissions::WRITE)?;

    // Genel Karnal64 sistem çağrısı işleyicisini çağır.
    // Bu fonksiyon, sistem çağrısı numarasına göre ilgili Karnal64 API fonksiyonunu
    // (örn. karnal64::resource_read) dispatch eder.
    let result = unsafe { // User pointer doğrulaması yapılacağı varsayılırsa safe olabilir
        karnal64::handle_syscall(syscall_number, arg1, arg2, arg3, arg4, arg5)
    };

    // Karnal64'ten dönen sonucu (i64) kullanıcı alanına döndürülecek register'a yaz.
    // SYSCALL/SYSRET convention'da dönüş değeri RAX'e konur.
    frame.rax = result as u64; // i64 -> u64 dönüşümü (negatif değerler 2's complement olarak temsil edilir, ABİ ile uyumlu olmalı)

    // TODO: Assembly trampoline kodu TrapFrame'deki güncellenmiş registerları kullanarak
    // kullanıcı alanına geri dönmelidir (SYSRET talimatı kullanılır).
}

// --- x86_64 MMU Etkileşim Fonksiyonları (kmemory modülü tarafından çağrılır) ---
// Karnal64'ün genel bellek yöneticisi (kmemory), donanımdan bağımsız mantığı içerir.
// Ancak sayfa tablosu manipülasyonu, TLB temizleme gibi mimariye özgü işlemler
// için bu platform modülünü çağırır.

mod mmu_x86_64 {
    // TODO: x86_64 Sayfa Tablosu yapıları (PML4, PDPT, PD, PT) ile etkileşim
    // TODO: CR3 registerını yükleme (aktif sayfa tablosunu değiştirme)
    // TODO: TLB girdisini geçersiz kılma (invlpg talimatı)
    // TODO: Sayfalama (Paging) özelliğini etkinleştirme (CR0, CR4 registerları)
}

// TODO: Ktask modülünün çağıracağı bağlam değiştirme (context switch) fonksiyonları
// Bu, x86_64'ün register setini (TrapFrame veya özel bir yapı) kaydetme/geri yükleme
// assembly kodunu çağırır.
 pub unsafe fn switch_context(old_ctx: *mut TaskContext, new_ctx: *const TaskContext);

// TODO: Kesme işleme alt sistemi
// x86_64 IDT'sini kullanarak gelen donanım kesmelerini (PIC/APIC'ten) ilgili
// Rust işleyicilerine yönlendirme mantığı.
 pub fn handle_interrupt(interrupt_index: u8, trap_frame: *mut TrapFrame);

// TODO: Diğer platforma özgü donanım etkileşimleri (cihaz sürücüleri için temel arayüzler)
// Seri Port (UART), VGA Text Mode, PIC/APIC, PCI gibi donanımlarla etkileşim için düşük seviye fonksiyonlar.
 pub unsafe fn inb(port: u16) -> u8; // Port G/Ç giriş
 pub unsafe fn outb(port: u16, val: u8); // Port G/Ç çıkış


// --- Yer Tutucu Print Fonksiyonu ---
// Erken boot aşamasında veya UART/VGA sürücüsü tam implemente olmadan debug için kullanılabilir.
// Genellikle VGA text mode veya seri port donanımına doğrudan yazılır.
#[cfg(feature = "enable_debug_print")] // Build feature ile kontrol edilebilir
mod debug_print {
    // TODO: VGA text bufferına veya seri port donanım adresine yazan düşük seviye implementasyon
    // Genellikle 'volatile' yazmalar ve 'unsafe' blokları içerir.
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
