#![no_std]

// Karnal64 API'sına erişim için crate root'unu kullanıyoruz.
// Çekirdek crate'inizin adı 'karnal' ise:
 extern crate karnal;
 use karnal::{handle_syscall, KError}; // Veya çekirdek crate'inizin yapısına göre

// Varsayımsal olarak Karnal64 API'mızın path'i bu şekilde olsun
use crate::karnal64::{handle_syscall, KError}; // Kernel crate'inizdeki Karnal64 modülünü kullanın

// ARM Cortex-A/R mimarileri için özel yazmaçlara (register) erişim sağlayan crate'ler.
// Gerçek bir implementasyonda cpu/register erişimi için `cortex-a` veya benzeri crate'ler
// kullanılabilir veya düşük seviye assembly/intrinsics gerekebilir.
// Örnek olarak sadece gerekli register isimlerini kullanıyoruz.
 use cortex_a::registers::*; // Veya benzeri register erişim kütüphaneleri

// --- Trap Frame / Bağlam Yapısı ---
// İstisna anında kullanıcının veya kesilen işin register durumunu kaydetmek için kullanılır.
// AArch64'ün genel amaçlı register'ları (x0-x30), yığın işaretçisi (SP_EL0),
// program sayacı (ELR_EL1), işlemci durumu (SPSR_EL1) gibi bilgileri içerir.
// Bu yapı, genellikle istisna girişi yapan assembly kodu tarafından yığına itilir.
#[repr(C)] // C uyumlu bellek yerleşimi
pub struct TrapFrame {
    pub x: [u64; 31], // x0 - x30
    pub sp_el0: u64,   // Kullanıcı yığın işaretçisi
    pub elr_el1: u64,  // Dönüş adresi
    pub spsr_el1: u64, // İşlemci durumu register'ı (program status register)
    // Diğer register'lar eklenebilir (örn: Q register'ları, FP/SIMD state)
    // FP/SIMD state genellikle ayrı kaydedilir.
}

// --- İstisna Vektör İşleyicileri ---
// Bu fonksiyonlar, istisna geldiğinde assembly glue code'u tarafından çağrılır.
// Her fonksiyon, istisna tipi ve geldiği Exception Level'a (EL) göre ayrılır.
// SVC (Sistem Çağrısı) genellikle EL0 -> EL1 senkron istisnadır.

/// EL0'dan EL1'e senkron istisnalar için genel işleyici.
/// SVC çağrıları, Data/Instruction Abort'lar buraya düşer.
#[no_mangle]
pub extern "C" fn handle_sync_exception_from_el0(tf: &mut TrapFrame) {
    // ESR_EL1 (Exception Syndrome Register) okunarak istisnanın tipi belirlenir.
    let esr_el1_val: u64;
    unsafe {
        // ESR_EL1'i oku (Gerçek kodda uygun register erişim makroları/fonksiyonları kullanılmalı)
         asm!("mrs {}, esr_el1", out(reg) esr_el1_val);
        // Şimdilik mock değer kullanalım
        esr_el1_val = 0x54000000; // Örnek: EC=0b010101 (SVC64), ISS=0 (SVC numarasını başka yerden alacağız)
    }

    // ESR'nin Exception Class (EC) alanını kontrol et
    let ec = (esr_el1_val >> 26) & 0b111111;

    match ec {
        0b010101 => { // EC = 0b010101: SVC instruction execution (AArch64)
            handle_svc_exception(tf);
        }
        0b100000 => { // EC = 0b100000: Instruction Abort from lower EL
            handle_instruction_abort_from_el0(tf, esr_el1_val);
        }
        0b100001 => { // EC = 0b100001: Instruction Abort from same EL
             handle_instruction_abort_from_el1(tf, esr_el1_val); // Sadece EL0'dan geliyorsa çağrılmaz, ama yapıyı gösterelim
        }
        0b100010 => { // EC = 0b100010: Data Abort from lower EL
            handle_data_abort_from_el0(tf, esr_el1_val);
        }
         0b100011 => { // EC = 0b100011: Data Abort from same EL
             handle_data_abort_from_el1(tf, esr_el1_val); // Sadece EL0'dan geliyorsa çağrılmaz
        }
        // TODO: Diğer senkron istisna tiplerini ele al (breakpoint, step, alignment check vb.)
        _ => {
            // Bilinmeyen veya beklenmeyen senkron istisna
            panic!("Bilinmeyen Senkron İstisna EL0->EL1! ESR_EL1: {:#x}, ELR_EL1: {:#x}", esr_el1_val, tf.elr_el1);
        }
    }
}

/// EL0'dan gelen bir SVC (Sistem Çağrısı) istisnasını işler.
fn handle_svc_exception(tf: &mut TrapFrame) {
    // AArch64 ABI'sinde sistem çağrısı numarası genellikle x8'de,
    // argümanlar x0-x7'de taşınır. Karnal64'ün handle_syscall
    // fonksiyonu 5 argüman bekliyor (syscall_number, arg1..arg5).
    // handle_syscall tanımına bakarsak: number, arg1..arg5 şeklindeydi.
    // Genellikle syscall_number ilk argüman DEĞİLDİR, ayrıdır.
    // Karnal64 tanımını yeniden yorumlayalım: arg1..arg5 sistem çağrısının
    // kendi argümanları olsun, number ise SYSCALL numarası olsun.
    // SYSCALL numarasını x8'den, argümanları x0-x4'ten alalım (Karnal64 5 arg bekliyor).

    let syscall_number = tf.x[8]; // x8'deki sistem çağrısı numarası
    let arg1 = tf.x[0]; // x0
    let arg2 = tf.x[1]; // x1
    let arg3 = tf.x[2]; // x2
    let arg4 = tf.x[3]; // x3
    let arg5 = tf.x[4]; // x4 (Karnal64 5 arg bekliyor, x0-x4'ten 5 tane alalım)

    // Kullanıcı pointer'larının doğrulama notu:
    // Buraya gelen arg1..arg5 değerleri kullanıcı alanındaki adresler olabilir.
    // handle_syscall içine ham pointer'ları göndermeden önce veya handle_syscall
    // fonksiyonunun en başında, bu pointer'ların mevcut görevin adres alanı içinde
    // geçerli ve erişilebilir (okuma/yazma izni) olup olmadığını kontrol etmek ÇOK ÖNEMLİDİR.
    // Bu taslakta bu doğrulama atlanmıştır ancak gerçek bir çekirdekte yapılmalıdır.
    // Örneğin: memory_allocate argümanı size, resource_read argümanı user_buffer_ptr gibi.
    // kresource::resource_acquire'daki resource_id_ptr gibi string pointer'ları da doğrulanmalı.

    // Karnal64 API'sındaki sistem çağrısı işleyiciyi çağır
    let result = handle_syscall(syscall_number, arg1, arg2, arg3, arg4, arg5);

    // Karnal64'ten dönen sonucu (i64), kullanıcının beklediği register'a (genellikle x0) yaz.
    tf.x[0] = result as u64; // i64 -> u64 dönüşümü, negatif değerler olduğu gibi korunur.
                             // ARM ABI'sinde dönüş değerleri genellikle x0'a konur.
}

/// EL0'dan gelen bir Veri Abort (Data Abort) istisnasını işler.
/// (Örn: Geçersiz bellek erişimi - sayfa hatası dahil)
fn handle_data_abort_from_el0(tf: &mut TrapFrame, esr_el1_val: u64) {
    let far_el1_val: u64; // Fault Address Register
    unsafe {
        // FAR_EL1'i oku (Hatanın meydana geldiği bellek adresi)
         asm!("mrs {}, far_el1", out(reg) far_el1_val);
        far_el1_val = 0; // Mock değer
    }

    // ESR'den Fault Status Code (FSC) ve diğer bilgileri çıkar.
    let fsc = esr_el1_val & 0b111111;

    // TODO: FSC'ye göre hatayı daha detaylı işle.
    // Örneğin: 0b111100 (TLB Miss - sayfa hatası) -> Bellek yöneticisine yönlendir.
    // 0b001101 (Permission Fault) -> İzin hatası, genellikle görevi sonlandır.

    // Çok temel durumda sadece panik yapalım
    panic!("Veri Abort EL0->EL1! ESR_EL1: {:#x}, FAR_EL1: {:#x}, ELR_EL1: {:#x}", esr_el1_val, far_el1_val, tf.elr_el1);
}

/// EL0'dan gelen bir Komut Abort (Instruction Abort) istisnasını işler.
/// (Örn: Geçersiz komut çalıştırma, komut getirme hatası)
fn handle_instruction_abort_from_el0(tf: &mut TrapFrame, esr_el1_val: u64) {
    // FAR_EL1 burda da alakalı olabilir (Komut getirme hatasıysa)
     let far_el1_val: u64;
    unsafe {
         asm!("mrs {}, far_el1", out(reg) far_el1_val); // Eğer geçerliyse
        far_el1_val = 0; // Mock değer
    }

    let fsc = esr_el1_val & 0b111111;

     // TODO: FSC'ye göre hatayı işle.

    // Çok temel durumda sadece panik yapalım
    panic!("Komut Abort EL0->EL1! ESR_EL1: {:#x}, FAR_EL1: {:#x}, ELR_EL1: {:#x}", esr_el1_val, far_el1_val, tf.elr_el1);
}


// --- Kesme (IRQ/FIQ) İşleyicileri ---
// Asenkron olayları (donanım kesmeleri) işler.

/// EL0'dan EL1'e IRQ (Interrupt ReQuest) işleyici.
#[no_mangle]
pub extern "C" fn handle_irq_from_el0(tf: &mut TrapFrame) {
    // TODO: Kesme denetleyicisini (GIC gibi) okuyarak kesme kaynağını belirle.
    // TODO: İlgili aygıt sürücüsünün kesme işleyicisini çağır.
    // TODO: Kesme denetleyicisinde kesmeyi onaylayıp temizle.

    // Şimdilik sadece logla veya panik yap
     println!("IRQ EL0->EL1 geldi!"); // Çekirdek içi loglama/print sistemi gerektirir
    panic!("IRQ EL0->EL1 geldi!"); // Hata ayıklama aşamasında iyi olabilir
}

// TODO: Diğer istisna tipleri için işleyicileri ekle:
// - handle_sync_exception_from_el1 (EL1'den EL1'e senkron istisnalar)
// - handle_irq_from_el1 (EL1'den EL1'e IRQ)
// - handle_fiq_from_el0/el1 (FIQ işleyicileri)
// - handle_serror (Sistem hatası işleyicisi)
// - handle_sync_exception_from_el2 (Hypervisor istisnaları - eğer kullanılıyorsa)
// - handle_irq_from_el2, vb.

// --- İstisna Vektör Tablosu (Kavramsal) ---
// Bu, genellikle assembly'de tanımlanan ve CPU'nun VBAR_EL1 (Vector Base Address Register)
// register'ına yazılan bir tablodur. Her istisna tipi ve EL geçişi için (eşzamanlı EL0->EL1,
// IRQ EL0->EL1, Eşzamanlı EL1->EL1, IRQ EL1->EL1 vb.) bir giriş noktası içerir.
// Her giriş noktası, CPU bağlamını kaydeden, yığın değiştiren ve yukarıdaki
// `handle_*` fonksiyonlarından birini çağıran assembly koduna atlar.

// Örnek bir AArch64 İstisna Vektör Tablosu yapısı (Rust'ta temsil edilirse):
#[repr(C)]
#[link_section = ".vectors"] // Bağlayıcıya belirli bir bölüme yerleştirmesini söyler
pub struct VectorTable {
    // Current EL with SP_EL0
    pub current_el_sp0_sync: extern "C" fn(),
    pub current_el_sp0_irq: extern "C" fn(),
    pub current_el_sp0_fiq: extern "C" fn(),
    pub current_el_sp0_serror: extern "C" fn(),

    // Current EL with SP_ELx
    pub current_el_spx_sync: extern "C" fn(),
    pub current_el_spx_irq: extern "C" fn(),
    pub current_el_spx_fiq: extern "C" fn(),
    pub current_el_spx_serror: extern "C" fn(),

    // Lower EL with AArch64
    pub lower_el_aarch64_sync: extern "C" fn(), // EL0->EL1 Sync (Buraya handle_sync_exception_from_el0 bağlanır)
    pub lower_el_aarch64_irq: extern "C" fn(),   // EL0->EL1 IRQ (Buraya handle_irq_from_el0 bağlanır)
    pub lower_el_aarch64_fiq: extern "C" fn(),
    pub lower_el_aarch64_serror: extern "C" fn(),

    // Lower EL with AArch32
    pub lower_el_aarch32_sync: extern "C" fn(),
    pub lower_el_aarch32_irq: extern "C" fn(),
    pub lower_el_aarch32_fiq: extern "C" fn(),
    pub lower_el_aarch32_serror: extern "C" fn(),
}

// Gerçekte bu tablo Rust'ta değil, assembly'de oluşturulur ve Rust fonksiyonlarına atlar.
// Örneğin, assembly kodu trap frame'i hazırlar ve `handle_sync_exception_from_el0(&mut tf)` çağrısını yapar.

// Statik vektör tablosu (genellikle assembly'de oluşturulur)
 #[no_mangle]
 static vector_table: VectorTable = ...; // Assembly sembollerine veya fonksiyon işaretçilerine atanır
