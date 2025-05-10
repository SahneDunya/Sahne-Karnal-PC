#![no_std] // Standart kütüphane yok, çekirdek alanındayız

// RISC-V mimarisine özgü kütüphaneleri veya modülleri import et
// 'riscv' krateri, CSR'lara erişim gibi düşük seviye işlemler için yaygın olarak kullanılır.
extern crate riscv;
extern crate alloc; // Karnal64 içinde veya platformda Box/Vec kullanılıyorsa gerekebilir.

// Karnal64 çekirdek API'sına erişim izni ver
// Varsayım: ana çekirdek kraterinde 'karnal64' modülü var.
#[path = "../../karnal64.rs"] // Geçici olarak dosya yolunu belirtiyorum, gerçekte modül import edilir.
mod karnal64;

use karnal64::{KError, KHandle}; // Karnal64 API'sından temel tipleri kullanıyoruz
use riscv::register::{
    mcause, // Trap nedenini (cause) okumak için
    mtvec,  // Trap vektör tablosu adresini ayarlamak için
    mepc,   // Trap sonrası devam edilecek PC'yi (Program Counter) okumak/yazmak için
    mstatus, // Makine durumu (Machine status) registerı
    // ... Diğer M-mode veya S-mode CSR'ları (sstatus, sepc, stvec, satp, vb.)
};
// Supervisor modu (S-mode) kullanılıyorsasstatus, sepc, stvec, satp kullanılır.
// Genellikle çekirdek S-mode'da, bootloader M-mode'da çalışır.
// Bu örnek S-mode varsayımıyla ilerleyebilir.
use riscv::register::{sstatus, sepc, stvec, satp}; // Supervisor mode registerları

// --- RISC-V Platformuna Özgü Başlatma ---

/// Çekirdeğin RISC-V platformuna özgü başlatma fonksiyonu.
/// Bu, çekirdek bootloader'ı tarafından M-mode'da temel kurulum yapıldıktan sonra
/// S-mode'a geçiş öncesi veya sonrası çağrılabilir.
/// MMU, kesme denetleyicisi (PLIC), zamanlayıcı (CLINT/HPET), trap vektör tablosu
/// gibi donanımları başlatır ve ardından genel Karnal64 başlatma fonksiyonunu çağırır.
#[no_mangle] // Bootloader'ın bu fonksiyonu bulabilmesi için isminin değişmemesi gerekir
pub extern "C" fn platform_init() {
    // Güvenlik: Bu fonksiyon çağrıldığında çok temel bir ortamın (S-mode'a geçilmiş,
    // stack pointer ayarlı vb.) bootloader tarafından kurulduğu varsayılır.

    // TODO: RISC-V mimarisine özgü erken donanım başlatma adımları
    // - S-mode'a geçiş ayarları (eğer M-mode'dan başlandıysa)
    // - sstatus registerını ayarlama (FS alanı vb.)
    // - Trap vektör tablosunu ayarlama: stvec = &trap_entry_point
    unsafe {
        // Trap vektörünü ayarlayın (assembly veya Rust fonksiyonuna işaret edebilir)
        // Buradaki 'trap_handler' fonksiyonu, trap entry assembly kodundan çağrılacaktır.
        stvec::write(trap_entry_point as usize); // veya trap_handler fonksiyonu doğrudan stvec'e yazılabilir
    }

    // - PLIC (Platform-Level Interrupt Controller) başlatma ve temel öncelikleri ayarlama
    // - CLINT (Core Local Interruptor) veya diğer zamanlayıcıları ayarlama
    // - MMU'yu başlatma (satp registerını ve başlangıç sayfa tablolarını ayarlama)

    // Örnek: Çok temel bir boot mesajı (UART sürücüsü implemente edilirse)
     platform_uart::print("Karnal64 RISC-V Platformu Başlatılıyor...\n");
     println!("Karnal64 RISC-V Platformu Başlatılıyor..."); // Eğer global bir print! makrosu tanımlıysa

    // Genel Karnal64 çekirdek başlatma fonksiyonunu çağır.
    // Bu fonksiyon, resource manager, task manager gibi Karnal64 iç modüllerini başlatır.
    karnal64::init();

    // TODO: Daha sonraki başlatma adımları
    // - Cihaz sürücülerini kaydetme (platforma özgü cihazlar için ResourceProvider implementasyonları)
    // - İlk kullanıcı görevi (init/shell) oluşturma ve zamanlayıcıya ekleme
    // - Kesmeleri etkinleştirme (RISC-V'de sstatus::set_sie() veya mie/mstatus ayarları)

    println!("Karnal64 Genel Başlatma Tamamlandı.");

    // TODO: Eğer ilk görev başlatılmadıysa veya bootloader otomatik çalıştırmıyorsa
    // buradan ilk kullanıcı görevine geçiş yapılabilir.
     unsafe { ktask::start_first_user_task(); } // Karnal64 task manager API'sından
}


// --- RISC-V Trap (Sistem Çağrısı, Kesme, Hata) İşleyici ---

/// RISC-V'de tüm trap'lerin (senkron - ecall, hatalar; asenkron - kesmeler) giriş noktasıdır.
/// Bu fonksiyon, `stvec` registerına yazılan adres olmalıdır.
/// Genellikle bu fonksiyona girmeden önce assembly kodunda kullanıcı/görev registerları
/// çekirdek yığıtına kaydedilir (trap frame oluşturulur).
///
/// `trap_frame` pointer'ı, kaydedilmiş kullanıcı registerlarının bulunduğu yapıyı gösterir.
/// Bu yapı, RISC-V'nin genel amaçlı registerlarını (x0-x31) ve bazı CSR'ları içermelidir.
///
/// Güvenlik Notu: Trap frame'in doğru kaydedilmesi ve geri yüklenmesi, yığıt güvenliği
/// ve kullanıcı pointerlarının doğrulanması (handle_syscall içinde) kritik öneme sahiptir.
///
/// Bu fonksiyonun doğru çalışması, çağıran assembly kodunun (trap entry)
/// RISC-V Bağlamı (Context) ve ABI kurallarına uymasına bağlıdır.
#[no_mangle] // stvec registerına yazılacak veya assembly'den çağrılacak
pub extern "C" fn trap_entry_point(trap_frame: *mut TrapFrame) {
    // Trap frame yapısı mimariye ve kullanılan bağlam değiştirme yöntemine göre değişir.
    // Basit bir örnek TrapFrame yapısı aşağıda gösterilmiştir.

    let tf = unsafe { &mut *trap_frame }; // Güvenli olmayan (unsafe) blok içinde pointer'ı dereference et

    // Hatanın nedenini (cause) ve adresini (tval) oku
    let cause = mcause::read().cause(); // Eğer S-mode'daysak, scause::read() kullanılır
    let tval = riscv::register::mtval::read(); // veya stval::read() S-mode için

    match cause {
        riscv::register::mcause::Trap::Exception(riscv::register::mcause::Exception::UserEnvCall) => {
            // Kullanıcı alanından 'ecall' (sistem çağrısı) geldi.
            // mepc (veya sepc) registerı, ecall talimatının adresini tutar.
            // ecall talimatının uzunluğu genellikle 4 byte'tır, bu yüzden
            // bir sonraki talimattan devam etmek için mepc'yi ilerletmek gerekir.
            unsafe {
                sepc::write(sepc::read().wrapping_add(4)); // S-mode için sepc kullanılıyor
            }

            // Sistem çağrısı numarasını ve argümanları trap frame'den al (RISC-V calling convention'a göre a0-a5 registerları)
            let syscall_number = tf.a0; // Varsayılan RISC-V ABI'sine göre a0 sistem çağrısı numarasını tutar
            let arg1 = tf.a1;
            let arg2 = tf.a2;
            let arg3 = tf.a3;
            let arg4 = tf.a4;
            let arg5 = tf.a5;

            // Genel Karnal64 sistem çağrısı işleyicisini çağır.
            // Bu fonksiyon, sistem çağrısı numarasına göre ilgili Karnal64 API fonksiyonunu dispatch eder.
            let result = unsafe { // handle_syscall içinde user pointer doğrulaması yapılacağı varsayılır
                karnal64::handle_syscall(syscall_number, arg1, arg2, arg3, arg4, arg5)
            };

            // handle_syscall'dan dönen sonucu (i64) kullanıcı alanına döndürmek üzere
            // uygun registera (varsayım: a0) yaz.
            tf.a0 = result as u64; // RISC-V ABI'sine göre a0 dönüş değerini tutar
            if result < 0 { // Hata durumunda a1'e hata kodunu da yazmak yaygın bir ABI kuralı olabilir
               tf.a1 = result as u64; // Veya doğrudan a0 negatif değer olarak kullanılır
            }

            // Trap'ten 'sret' talimatı ile dönüldüğünde, sepc'deki adres ve sstatus'taki
            // önceki mod (user) ve PIE (önceki kesme etkin durumu) kullanılacaktır.
        }
        riscv::register::mcause::Trap::Interrupt(interrupt_cause) => {
            // Asenkron trap: Kesme geldi.
            // TODO: Kesme nedenini (interrupt_cause) belirle ve ilgili kesme işleyicisini çağır.
            // PLIC (Platform-Level Interrupt Controller) ile etkileşim burada olur.
            // Önceliklendirme ve kesme sonlandırma (EOI - End Of Interrupt) işlemleri yapılır.
            println!("RISC-V Interrupt received: {:?}", interrupt_cause); // Debug mesajı
            // Örneğin: platform_plic::handle_interrupt(interrupt_cause, trap_frame);
        }
        _ => {
            // Beklenmeyen hata (Exception) veya bilinmeyen trap
            // TODO: Bu tür hataları işle: Görevi sonlandır, hata raporla vb.
            println!("Beklenmeyen RISC-V Trap! Cause: {:?}, Tval: 0x{:x}", cause, tval);
            // Örnek: KTask modülünü kullanarak mevcut görevi sonlandır
             unsafe { karnal64::ktask::terminate_current_task(KError::InternalError); }
            loop {} // Sistem kilitlenirse sonsuz döngü (debug için)
        }
    }
}

/// Temel RISC-V Trap Frame yapısı yer tutucusu.
/// Trap entry assembly kodu, kullanıcı modundan çekirdek moduna geçerken
/// genel amaçlı registerları (x0-x31) ve gerekli bazı CSR'ları (sstatus, sepc, satp vb.)
/// bu yapıya kaydetmelidir.
#[repr(C)] // C uyumlu bellek düzeni
#[derive(Debug, Default, Copy, Clone)]
pub struct TrapFrame {
    // RISC-V Genel Amaçlı Registerlar (x0-x31) - Sıralama ABI'ye ve assembly koduna bağlıdır!
    // Genellikle x1 (ra), x2 (sp), x3 (gp), x4 (tp), x5-x7 (t0-t2), x8 (s0/fp), x9 (s1),
    // x10-x17 (a0-a7), x18-x27 (s2-s11), x28-x31 (t3-t6) kaydedilir.
    // x0 (zero) genellikle kaydedilmez.
    pub regs: [u64; 32], // x0 hariç 31 register + belki bir dummy entry veya tüm 32'si

    // Kaydedilmesi gerekebilecek bazı önemli CSR'lar
    pub sstatus: u64,
    pub sepc: u64,
    pub satp: u64,
    pub scause: u64,
    pub stval: u64,
    // ... diğer gerekli CSR'lar veya FPU durumu
}

// RISC-V ABI'sinde sistem çağrısı argümanları genellikle a0-a5 (x10-x15) registerlarındadır.
// Dönüş değeri ise a0 (x10) registerındadır.
// Bu nedenle TrapFrame içindeki regs dizisinden bu registerlara erişim sağlamak gerekir.
// Örnek olarak regs[10] a0'a, regs[11] a1'e denk gelir gibi.
// Yukarıdaki powerpc_syscall_handler'daki gibi doğrudan arg1..arg5 almak yerine
// trap_frame pointer'ından erişmek daha doğru bir yaklaşımdır.

impl TrapFrame {
    // Kolay erişim için helper fonksiyonlar (örnek)
    #[inline]
    pub fn syscall_number(&self) -> u64 { self.regs[10] } // a0
    #[inline]
    pub fn arg1(&self) -> u64 { self.regs[11] } // a1
    #[inline]
    pub fn arg2(&self) -> u64 { self.regs[12] } // a2
    #[inline]
    pub fn arg3(&self) -> u64 { self.regs[13] } // a3
    #[inline]
    pub fn arg4(&self) -> u64 { self.regs[14] } // a4
    #[inline]
    pub fn arg5(&self) -> u64 { self.regs[15] } // a5

    #[inline]
    pub fn set_return_value(&mut self, value: u64) { self.regs[10] = value; } // a0
    #[inline]
    pub fn set_error_code(&mut self, error_code: i64) {
         self.regs[10] = error_code as u64; // Hata durumunda a0 negatif kod
         // Bazı ABI'ler a1'i de kullanır, duruma göre ayarlanır:
          self.regs[11] = ...; // a1
    }

}


// --- RISC-V MMU Etkileşim Fonksiyonları (kmemory modülü tarafından çağrılır) ---
// Karnal64'ün genel bellek yöneticisi (kmemory), donanımdan bağımsız mantığı içerir.
// RISC-V'nin Sv39/Sv48 gibi sayfa tablosu formatlarına özgü manipülasyonlar ve
// TLB yönetimi (SFENCE.VMA) için bu platform modülünü çağırır.

pub mod mmu_riscv {
    use super::*;
    // TODO: RISC-V MMU (satp, sayfa tablosu formatları) ile etkileşim için sarmalayıcılar
    // Örn: satp registerına yazma, SFENCE.VMA talimatını çağırma (riscv::asm::sfence_vma),
    // Sayfa tablosu girdisi oluşturma/güncelleme.
    pub unsafe fn set_user_pagetables(satp_value: usize) {
        // satp registerına yazar ve TLB'yi temizler (genellikle bağlam değiştirmede kullanılır)
        satp::write(satp_value);
        riscv::asm::sfence_vma_all(); // Tüm adres alanları için TLB temizleme
    }
    // TODO: map_page, unmap_page fonksiyonları (kmemory tarafından kullanılır)
}


// TODO: Ktask modülünün çağıracağı bağlam değiştirme (context switch) fonksiyonları
// Bu, RISC-V'nin register setini (genel amaçlı, sstatus, sepc, satp vb.) kaydetme/geri yükleme
// assembly kodunu çağırır.
 pub unsafe fn switch_context(old_ctx: *mut TaskContext, new_ctx: *const TaskContext);


// TODO: Kesme işleme alt sistemi (PLIC ve CLINT ile etkileşim)
// Gelen kesmeleri belirleme ve ilgili işleyicilere yönlendirme mantığı.
 pub fn handle_plic_interrupt(interrupt_id: u32);
 pub fn handle_timer_interrupt();


// TODO: Diğer platforma özgü donanım etkileşimleri (cihaz sürücüleri için temel arayüzler)
// RISC-V platformlarında yaygın olan SiFive UART veya VirtIO gibi cihazlar için temel I/O fonksiyonları.
 pub fn read_uart_register(addr: usize) -> u8;
 pub fn write_uart_register(addr: usize, value: u8);

// --- Yer Tutucu Print Fonksiyonu ---
// Erken boot aşamasında veya UART sürücüsü tam implemente olmadan debug için kullanılabilir.
// Genellikle memory-mapped I/O ile bir UART donanımına yazar.
mod debug_print_riscv {
    // TODO: RISC-V UART donanım adresine yazan düşük seviye implementasyon
     #[no_mangle] // Eğer assembly'den çağrılıyorsa
     pub extern "C" fn _putchar(c: u8) { /* write c to UART data register */ }

    // TODO: Formatlı çıktı için basit bir mekanizma (format! makrosu kullanılabiliyorsa)
}

// Basit bir println! makrosu yer tutucusu (debug_print_riscv modülü aktifse)
// Gerçek bir çekirdekte daha gelişmiş logging/print! makroları kullanılır.
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ({
        // TODO: Format string'i al ve debug_print_riscv modülünü kullanarak yazdır
         debug_print_riscv::_print(format_args!($($arg)*)); // 'alloc' veya farklı bir formatlama gerekir
    });
}
