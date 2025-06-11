#![no_std] // Standart kütüphane yok

// Gerekli Rust özellikleri (inline assembly gibi)
#![feature(asm_sym)]
#![feature(naked_functions)]
#![feature(global_asm)]
#![allow(dead_code)] // Henüz kullanılmayan fonksiyonlar olabilir

// Karnal64 API'nıza erişim için (karnal64.rs dosyanızdaki tipleri/fonksiyonları kullanacak)
// Bu, karnal64.rs'nin bu modül tarafından erişilebilir olması gerektiğini varsayar.
use crate::karnal64::{handle_syscall, KError}; // handle_syscall'ı ve KError'u import edin

// --- İstisna Bağlamını Saklamak İçin Yapı (Trap Frame) ---
// İstisna/sistem çağrısı gerçekleştiğinde CPU'nun durumunu (register'ları)
// bu yapıda saklayacağız. Assembly kodu bu yapıyı dolduracak ve boşaltacaktır.
// Minimum olarak kullanıcı alanındaki register'ları kaydetmemiz gerekir.
// Tam bir bağlam için ELR_EL1, SPSR_EL1, SP_EL0 gibi register'lar da gereklidir.
#[repr(C)] // C uyumlu bellek düzeni (assembly ile etkileşim için)
#[derive(Debug, Default)]
pub struct TrapFrame {
    // Genel amaçlı register'lar (x0 - x30)
    x: [u64; 31],
    // Stack Pointer (user)
    sp_el0: u64,
    // Exception Link Register (dönülecek adres)
    elr_el1: u64,
    // Saved Program Status Register (istisna anındaki PSTATE)
    spsr_el1: u64,
    // Exception Syndrome Register (istisnanın nedeni hakkında bilgi)
    esr_el1: u64,
    // Far Sync Exception Register (bellek hatası adresleri için)
    far_el1: u64,
}

// --- İstisna Vektör Tablosu (Assembly) ---
// AArch64 istisna vektör tablosu, VBAR_EL1 register'ında belirtilen
// 2048 byte'lık hizalanmış bir adreste bulunur.
// Her istisna tipi için 8 adet giriş vardır (4 EL'den, SP0/SPx kullanarak).
// Her giriş 0x80 (128) byte uzunluğundadır.
// Vektörler:
// 0: Current EL, SP0
// 1: Current EL, SPx
// 2: Lower EL, AArch64
// 3: Lower EL, AArch32

// Her alt vektörde (0, 1, 2, 3) 4 farklı istisna tipi:
// 0: Synchronous (SVC, Data Abort, Instruction Abort, etc.)
// 1: IRQ (Interrupt Request)
// 2: FIQ (Fast Interrupt Request)
// 3: SError (System Error)

// Bizim için en önemli olanlar:
// - Lower EL, AArch64 -> Synchronous (Sistem Çağrıları buradan gelir)
// - Lower EL, AArch64 -> IRQ (Donanım Kesmeleri buradan gelir)

global_asm!(r#"
.align 11 // 2048 byte hizalama (VBAR_EL1 için gerekli)

// İstisna Vektör Tablosunun başlangıcı
.global vector_table

vector_table:

    // Current EL, SP0
    .align 7 // 128 byte hizalama
    b handle_sync_curr_el_sp0
    .align 7
    b handle_irq_curr_el_sp0
    .align 7
    b handle_fiq_curr_el_sp0
    .align 7
    b handle_serror_curr_el_sp0

    // Current EL, SPx (SP_EL1)
    .align 7
    b handle_sync_curr_el_spx
    .align 7
    b handle_irq_curr_el_spx
    .align 7
    b handle_fiq_curr_el_spx
    .align 7
    b handle_serror_curr_el_spx

    // Lower EL, AArch64 (Kullanıcı Alanından İstisnalar, Sistem Çağrıları Buradan Gelir)
    .align 7
    b handle_sync_lower_el_aarch64 // <--- Sistem Çağrıları (SVC) Buraya Gelir
    .align 7
    b handle_irq_lower_el_aarch64   // <--- Donanım Kesmeleri (IRQ) Buraya Gelir
    .align 7
    b handle_fiq_lower_el_aarch64
    .align 7
    b handle_serror_lower_el_aarch64

    // Lower EL, AArch32 (Eğer AArch32 kullanıcı alanı destekleniyorsa)
    .align 7
    b handle_sync_lower_el_aarch32
    .align 7
    b handle_irq_lower_el_aarch32
    .align 7
    b handle_fiq_lower_el_aarch32
    .align 7
    b handle_serror_lower_el_aarch32

// --- Genel İstisna İşleyicisi Şablonu (Assembly) ---
// Bu makro, her istisna vektör girişi için bağlamı kaydeder, Rust işleyicisini çağırır ve bağlamı geri yükler.
// $handler: Çağrılacak Rust fonksiyonunun sembol adı (örn: handle_sync_lower_el_aarch64_rust)
.macro push_and_call_rust_handler handler
    // İstisna sırasında kullanılmayan temp register'ları sakla (x16, x17)
    // Stack Pointer EL1 (sp_el1) kullanılıyor.
    stp x16, x17, [sp, #-16]!

    // TrapFrame için yer ayır ve hizala
    // TrapFrame boyutu: 31 * 8 (x) + 8 (sp_el0) + 8 (elr_el1) + 8 (spsr_el1) + 8 (esr_el1) + 8 (far_el1) = 360 byte
    // En yakın 16'ya katı hizalama: 368 (örn)
    // sp -= 368
    mov x16, sp
    sub sp, sp, #(368) // TrapFrame için stack üzerinde yer aç

    // Genel amaçlı register'ları TrapFrame'e kaydet (x0-x30)
    // x0-x15 (16 register)
    stp x0, x1, [sp, #(8*0)]
    stp x2, x3, [sp, #(8*2)]
    stp x4, x5, [sp, #(8*4)]
    stp x6, x7, [sp, #(8*6)]
    stp x8, x9, [sp, #(8*8)]
    stp x10, x11, [sp, #(8*10)]
    stp x12, x13, [sp, #(8*12)]
    stp x14, x15, [sp, #(8*14)]

    // x16-x30 (15 register) - x16, x17 zaten saklandı, geri yüklenip tekrar saklanması gerekir mi?
    // Basitlik için, x16, x17'yi stack'ten geri alıp TrapFrame'e saklayalım.
    ldp x16, x17, [x16] // x16 ve x17'yi orijinal yerinden geri al
    stp x16, x17, [sp, #(8*16)] // TrapFrame'de x16, x17'nin yerine kaydet
    stp x18, x19, [sp, #(8*18)]
    stp x20, x21, [sp, #(8*20)]
    stp x22, x23, [sp, #(8*22)]
    stp x24, x25, [sp, #(8*24)]
    stp x26, x27, [sp, #(8*26)]
    stp x28, x29, [sp, #(8*28)]
    str x30, [sp, #(8*30)] // x30 (LR)

    // Özel register'ları TrapFrame'e kaydet
    mrs x16, sp_el0     // SP_EL0 oku
    str x16, [sp, #(8*31)] // TrapFrame.sp_el0
    mrs x16, elr_el1    // ELR_EL1 oku
    str x16, [sp, #(8*32)] // TrapFrame.elr_el1
    mrs x16, spsr_el1   // SPSR_EL1 oku
    str x16, [sp, #(8*33)] // TrapFrame.spsr_el1
    mrs x16, esr_el1    // ESR_EL1 oku
    str x16, [sp, #(8*34)] // TrapFrame.esr_el1
    mrs x16, far_el1    // FAR_EL1 oku
    str x16, [sp, #(8*35)] // TrapFrame.far_el1


    // İlk argüman (x0) olarak TrapFrame pointer'ını ayarla
    mov x0, sp

    // Rust işleyicisini çağır (BL: Branch with Link)
    bl \handler

    // Rust işleyicisinden döndükten sonra, TrapFrame'den register'ları geri yükle
    // Özel register'ları yükle
    ldr x16, [sp, #(8*31)] // TrapFrame.sp_el0
    msr sp_el0, x16
    ldr x16, [sp, #(8*32)] // TrapFrame.elr_el1
    msr elr_el1, x16
    ldr x16, [sp, #(8*33)] // TrapFrame.spsr_el1
    msr spsr_el1, x16
    // ESR_EL1 ve FAR_EL1 genellikle geri yüklenmez

    // Genel amaçlı register'ları geri yükle (x0-x30)
    ldr x30, [sp, #(8*30)] // x30 (LR)
    ldp x28, x29, [sp, #(8*28)]
    ldp x26, x27, [sp, #(8*26)]
    ldp x24, x25, [sp, #(8*24)]
    ldp x22, x23, [sp, #(8*22)]
    ldp x20, x21, [sp, #(8*20)]
    ldp x18, x19, [sp, #(8*18)]
    // x16, x17'yi TrapFrame'den yükle
    ldp x16, x17, [sp, #(8*16)]

    // x0-x15'i yükle (x0, Rust fonksiyonunun dönüş değerini içeriyor olabilir!)
    // Sistem çağrısı işleyicisi dönüş değerini x0'a yazmış olmalı.
    // Diğer x1-x15 register'larını TrapFrame'den yükleyebiliriz.
    ldp x14, x15, [sp, #(8*14)]
    ldp x12, x13, [sp, #(8*12)]
    ldp x10, x11, [sp, #(8*10)]
    ldp x8, x9, [sp, #(8*8)]
    ldp x6, x7, [sp, #(8*6)]
    ldp x4, x5, [sp, #(8*4)]
    ldp x2, x3, [sp, #(8*2)]
    // x0 ve x1'i en son yükle, böylece x0 dönüş değerini tutmaya devam eder.
    // Eğer x0'ın orijinal değerine ihtiyaç varsa, onu da yüklemeliyiz.
    // Sistem çağrısında x0 dönüş değeri olduğu için, x0'ın orijinal değerini kurtarmadan önce
    // Rust fonksiyonunun döndürdüğü değeri x0'a yazmış olması beklenir.
    // Varsayım: Rust işleyicisi TrapFrame.x[0]'ı günceller.
    ldp x0, x1, [sp, #(8*0)]

    // TrapFrame için ayrılan stack alanını geri al
    add sp, sp, #(368)

    // İstisna sırasında saklanan temp register'ları geri yükle (x16, x17)
    ldp x16, x17, [sp], #16

    // İstisnadan dön (Exception Return)
    eret
.endmacro

// --- İstisna Vektör Tablosu Hedefleri (Assembly) ---
// Bu etiketler, vector_table'daki dallanmaların hedefleridir.
// Buradan yukarıdaki push_and_call_rust_handler makrosu ile
// ilgili Rust işleyicilerine dallanacağız.

handle_sync_curr_el_sp0:
    // SP0 kullanılmaz, buraya gelmek bir hata olabilir.
    push_and_call_rust_handler handle_sync_curr_el_sp0_rust

handle_irq_curr_el_sp0:
    push_and_call_rust_handler handle_irq_curr_el_sp0_rust

handle_fiq_curr_el_sp0:
    push_and_call_rust_handler handle_fiq_curr_el_sp0_rust

handle_serror_curr_el_sp0:
    push_and_call_rust_handler handle_serror_curr_el_sp0_rust


handle_sync_curr_el_spx:
    // Çekirdek içinde senkron istisna (örn: sayfa hatası, geçersiz talimat)
    push_and_call_rust_handler handle_sync_curr_el_spx_rust

handle_irq_curr_el_spx:
    // Çekirdek içinde kesme
    push_and_call_rust_handler handle_irq_curr_el_spx_rust

handle_fiq_curr_el_spx:
    push_and_call_rust_handler handle_fiq_curr_el_spx_rust

handle_serror_curr_el_spx:
    push_and_call_rust_handler handle_serror_curr_el_spx_rust


handle_sync_lower_el_aarch64:
    // Kullanıcı alanından senkron istisna (SVC, Data Abort, etc.)
    push_and_call_rust_handler handle_sync_lower_el_aarch64_rust // <--- Sistem Çağrısı İşleyicimiz

handle_irq_lower_el_aarch64:
    // Kullanıcı alanından kesme (donanım kesmeleri)
    push_and_call_rust_handler handle_irq_lower_el_aarch64_rust // <--- IRQ İşleyicimiz

handle_fiq_lower_el_aarch64:
    push_and_call_rust_handler handle_fiq_lower_el_aarch64_rust

handle_serror_lower_el_aarch64:
    push_and_call_rust_handler handle_serror_lower_el_aarch64_rust

// AArch32 işleyicileri (şimdilik boş bırakılabilir veya panic ile doldurulabilir)
handle_sync_lower_el_aarch32:
    push_and_call_rust_handler handle_aarch32_exception

handle_irq_lower_el_aarch32:
    push_and_call_rust_handler handle_aarch32_exception

handle_fiq_lower_el_aarch32:
    push_and_call_rust_handler handle_aarch32_exception

handle_serror_lower_el_aarch32:
    push_and_call_rust_handler handle_aarch32_exception

"#);

// --- Rust İstisna İşleyicileri ---
// Bu fonksiyonlar assembly şablonu tarafından çağrılır.
// Parametre olarak TrapFrame'in mutable bir referansını alırlar.

/// Geçersiz bir istisna tipi için genel işleyici (şimdilik panik yapabiliriz)
#[no_mangle] // Assembly'den çağrılacak
extern "C" fn handle_invalid_exception(frame: &mut TrapFrame, message: &str) {
    // TODO: Daha sofistike hata işleme veya loglama
    println!("Çekirdek Hatası: Geçersiz istisna veya unimplemented handler: {}", message);
    println!("Trap Frame: {:?}", frame);
    // Çekirdek panikledi!
    loop {} // Sonsuz döngüde kal
}

#[no_mangle]
extern "C" fn handle_sync_curr_el_sp0_rust(frame: &mut TrapFrame) {
    handle_invalid_exception(frame, "Senkron (Current EL, SP0)");
}

#[no_mangle]
extern "C" fn handle_irq_curr_el_sp0_rust(frame: &mut TrapFrame) {
     handle_invalid_exception(frame, "IRQ (Current EL, SP0)");
}

#[no_mangle]
extern "C" fn handle_fiq_curr_el_sp0_rust(frame: &mut TrapFrame) {
     handle_invalid_exception(frame, "FIQ (Current EL, SP0)");
}

#[no_mangle]
extern "C" fn handle_serror_curr_el_sp0_rust(frame: &mut TrapFrame) {
     handle_invalid_exception(frame, "SError (Current EL, SP0)");
}


#[no_mangle]
extern "C" fn handle_sync_curr_el_spx_rust(frame: &mut TrapFrame) {
    // TODO: Çekirdek içinde oluşan senkron istisnaları işle
    // Örneğin: Sayfa hatası (Data/Instruction Abort), Geçersiz talimat
    // ESR_EL1 register'ına bakarak hatanın tipini belirleyin.
    let esr = frame.esr_el1;
    let ec = (esr >> 26) & 0x3f; // Exception Class

    match ec {
        0b100000 | 0b100001 => { // Instruction Abort from Current EL
            println!("Çekirdek Hatası: Çekirdek içinde talimat hatası!");
            println!("ESR: {:#x}, ELR: {:#x}", esr, frame.elr_el1);
             // TODO: Sayfa hatası işleyicisini çağır veya panik
            loop {}
        }
         0b100100 | 0b100101 => { // Data Abort from Current EL
            println!("Çekirdek Hatası: Çekirdek içinde veri hatası (Sayfa Hatası?)!");
            println!("ESR: {:#x}, FAR: {:#x}, ELR: {:#x}", esr, frame.far_el1, frame.elr_el1);
             // TODO: Sayfa hatası işleyicisini çağır veya panik
            loop {}
         }
        // TODO: Diğer EC değerlerini işle
        _ => handle_invalid_exception(frame, &format!("Senkron (Current EL, SPx), EC: {:#b}", ec)),
    }
}

#[no_mangle]
extern "C" fn handle_irq_curr_el_spx_rust(frame: &mut TrapFrame) {
    // TODO: Çekirdek içinde oluşan kesmeleri işle (zamanlayıcı kesmesi vb.)
     // Genellikle GIC (Generic Interrupt Controller) ile etkileşime girilir.
    println!("Çekirdek İçi IRQ alındı! (Yer Tutucu)");
    // TODO: GIC'ten kesme ID'sini oku, ilgili işleyiciye yönlendir, kesmeyi onayla.
}

#[no_mangle]
extern "C" fn handle_fiq_curr_el_spx_rust(frame: &mut TrapFrame) {
     handle_invalid_exception(frame, "FIQ (Current EL, SPx)");
}

#[no_mangle]
extern "C" fn handle_serror_curr_el_spx_rust(frame: &mut TrapFrame) {
     handle_invalid_exception(frame, "SError (Current EL, SPx)");
}


/// --- Kullanıcı Alanından Gelen Senkron İstisnaları İşleyici (Sistem Çağrıları Buradan Geçer) ---
#[no_mangle]
extern "C" fn handle_sync_lower_el_aarch64_rust(frame: &mut TrapFrame) {
    let esr = frame.esr_el1;
    let ec = (esr >> 26) & 0x3f; // Exception Class

    match ec {
        0b010101 => { // EC: 0b010101 -> SVC instruction (SVC64)
            // Sistem Çağrısı!
            // Varsayım: Sistem çağrısı numarası x8 register'ında,
            // argümanlar x0, x1, x2, x3, x4, x5 register'larında.
            // Bu bir ABI (Application Binary Interface) kuralıdır ve kullanıcı
            // alanı kodunuzun buna uyması gerekir.

            let syscall_number = frame.x[8]; // x8 = syscall numarası
            let arg1 = frame.x[0];           // x0 = arg1
            let arg2 = frame.x[1];           // x1 = arg2
            let arg3 = frame.x[2];           // x2 = arg3
            let arg4 = frame.x[3];           // x3 = arg4
            let arg5 = frame.x[4];           // x4 = arg5
            // not: arg5 aslında x5'te olmalı, ama kodda x4'e kadar kullanılmış.
            // ABI'nıza göre bunu düzeltin. Varsayımsal olarak x0-x5 argümanlar.
            let arg6 = frame.x[5]; // x5 = arg6 (Eğer 6 argüman kullanılıyorsa)

            // Karnal64 API'sındaki sistem çağrısı işleyicisini çağır
            // handle_syscall fonksiyonu i64 döndürüyor (başarı için >= 0, hata için negatif KError değeri)
            let result = handle_syscall(syscall_number, arg1, arg2, arg3, arg4, arg5); // Sadece 5 argüman geçelim API tanımına göre

            // Sistem çağrısının dönüş değerini kullanıcı alanının göreceği x0 register'ına yaz.
            // assembly kodu geri dönerken bu x0'ı kullanıcı stack'ine yazacaktır.
            frame.x[0] = result as u64; // i64'ü u64'e çevirirken dikkatli olun (negatif değerler için)
                                        // KError değerleri zaten negatif i64 olduğu için bu dönüşüm doğrudur.
        }
        0b100000 | 0b100001 => { // Instruction Abort from Lower EL
            // Kullanıcı alanında geçersiz talimat
             println!("Kullanıcı Alanı Hatası: Geçersiz talimat! ELR: {:#x}", frame.elr_el1);
            // TODO: Süreci sonlandır veya sinyal gönder
            loop{}
        }
         0b100100 | 0b100101 => { // Data Abort from Lower EL
            // Kullanıcı alanında veri hatası (genellikle sayfa hatası)
            println!("Kullanıcı Alanı Hatası: Sayfa hatası! FAR: {:#x}, ELR: {:#x}", frame.far_el1, frame.elr_el1);
            // TODO: Sayfa hatasını işle (örneğin: copy-on-write, stack büyümesi, mmap),
            // veya süreci sonlandır
            loop {}
         }
        // TODO: Diğer EC değerlerini işle (FIQ, SError, vb.)
        _ => {
            // Bilinmeyen veya beklenmeyen senkron istisna
            handle_invalid_exception(frame, &format!("Senkron (Lower EL, AArch64), EC: {:#b}", ec));
        }
    }
}


/// --- Kullanıcı Alanından Gelen Kesmeleri İşleyici (Donanım Kesmeleri Buradan Geçer) ---
#[no_mangle]
extern "C" fn handle_irq_lower_el_aarch64_rust(frame: &mut TrapFrame) {
    // Donanım kesmesi!
    // Genellikle GIC (Generic Interrupt Controller) ile etkileşime girilir.
    println!("IRQ alındı! (Yer Tutucu)");

    // TODO:
    // 1. GIC'ten hangi kesmenin geldiğini öğren (örneğin, GIC CPU arayüzünden).
    // 2. Kesmenin ID'sine göre ilgili aygıt sürücüsünün işleyicisini çağır.
    // 3. GIC'e kesmenin işlendiğini bildir (acknowledge).
}

#[no_mangle]
extern "C" fn handle_fiq_lower_el_aarch64_rust(frame: &mut TrapFrame) {
     handle_invalid_exception(frame, "FIQ (Lower EL, AArch64)");
}

#[no_mangle]
extern "C" fn handle_serror_lower_el_aarch64_rust(frame: &mut TrapFrame) {
     handle_invalid_exception(frame, "SError (Lower EL, AArch64)");
}

// AArch32 kullanıcı alanı destekleniyorsa bu işleyicileri doldurmanız gerekir.
// Şimdilik panik yapıyorlar.
#[no_mangle]
extern "C" fn handle_aarch32_exception(frame: &mut TrapFrame) {
     handle_invalid_exception(frame, "AArch32 istisnası (desteklenmiyor)");
}


// --- İstisna Sistemini Başlatma ---

/// İstisna vektör tablosunu kurar. Çekirdek başlangıcında çağrılmalıdır.
pub fn init() {
    // `vector_table` sembolünün adresini al (global_asm ile tanımlanan etiket)
    let vector_table_addr = vector_table as *const () as u64;

    unsafe {
        // VBAR_EL1 register'ına vektör tablosunun adresini yaz.
        // Bu, CPU'ya istisna/kesme olduğunda nereye dallanacağını söyler.
        core::arch::asm!(
            "msr vbar_el1, {}",
            in(reg) vector_table_addr,
            options(nostack, nomem) // Bu talimatın stack veya belleği etkilemediğini belirtir
        );
    }

    println!("ARM İstisna Vektör Tablosu kuruldu. Adres: {:#x}", vector_table_addr);

    // TODO: Donanım kesmeleri için GIC (Generic Interrupt Controller) gibi
    // kesme denetleyicisini de burada başlatmanız gerekebilir.
    // Kesmeleri global olarak etkinleştirmek (PSTATE.I ve PSTATE.F flagları)
    // genellikle GIC kurulumu veya zamanlayıcı başlangıcı sırasında yapılır.
    // Basitçe kesmeleri etkinleştirmek için:
     unsafe { core::arch::asm!("msr daifclr, #2") }; // DAIF I bitini temizle (kesmeleri etkinleştir)
}

// --- Assembly Sembolünün Harici Tanımı ---
// global_asm tarafından tanımlanan 'vector_table' sembolünü Rust tarafında
// kullanabilmek için harici (extern) olarak bildirmemiz gerekir.
extern "C" {
    fn vector_table();
}

// TODO: Yardımcı print! macro'su veya loglama mekanizması gereklidir.
// no_std ortamında std::println doğrudan kullanılamaz.
// Kendi konsol/UART sürücünüz üzerinden yazan bir macro implemente etmelisiniz.
macro_rules! println {
    ($($arg:tt)*) => ({
        // Buraya konsol/UART sürücünüzü kullanarak yazdırma mantığını ekleyin.
        // Şimdilik boş bırakılabilir veya dummy bir implementasyon konabilir.
         crate::driver::uart::print_fmt(format_args!($($arg)*));
    });
}

// TODO: panic handler implementasyonu gereklidir (`panic_handler` özelliği).
// Bir panik durumunda ne yapılacağını tanımlar.

// TODO: ResourceProvider traitini implemente eden dummy bir konsol sürücüsü
// (println! macro'su için gerekli olabilir) veya çekirdek içi bir loglama yapısı.

// TODO: GIC (Generic Interrupt Controller) etkileşimi için kodlar (IRQ işleyicisi için).
// Kesmeleri kaydetme, etkinleştirme, devre dışı bırakma, onayla vb. fonksiyonlar.
