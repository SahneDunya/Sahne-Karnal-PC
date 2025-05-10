#![no_std] // Standart kütüphaneye ihtiyaç yok
#![allow(dead_code)] // Geliştirme aşamasında kullanılmayan kodları görmezden gel
#![allow(unused_variables)] // Kullanılmayan değişkenleri görmezden gel

// Karnal64 çekirdek API'sını import et. handle_syscall fonksiyonunu kullanacağız.
// karnal64 crate'i veya modülü projenizde erişilebilir olmalı.
use karnal64::{self, KError};

// TODO: Mimariye özgü register tanımları ve trap frame layout'u
// Elbrus'un register kümesi ve trap/exception sırasında donanımın veya düşük seviye assembly'nin
// hangi registerları hangi sırayla kaydettiğine bağlı olarak bu yapı değişir.
// Bu sadece genel bir örnektir.
#[repr(C)] // C uyumlu layout sağlamak genellikle önemlidir
#[derive(Debug, Default)] // Debug yazdırma ve varsayılan değer alabilme (geliştirme için)
pub struct TrapFrame {
    // Genel Amaçlı Registerlar (r0, r1, ..., rn gibi)
    // Sıra, düşük seviye assembly'deki kaydetme sırası ile eşleşmeli!
    // Elbrus'un kaç registerı var ve hangileri trap'te kaydediliyor varsayalım.
    // Bu bir varsayımdır, gerçek mimariye göre düzeltilmelidir.
    pub regs: [u64; 32], // Örnek: 32 adet 64-bit register
    pub sp: u64,       // Stack Pointer
    pub pc: u64,       // Program Counter (Tuzağın olduğu veya dönecek adres)
    pub flags: u64,    // Durum/Bayrak registerı (PSW veya benzeri)

    // Tuzağın nedeni ve ilgili ekstra bilgiler
    pub trap_cause: u64,   // Tuzağın nedeni (syscall, page fault kodu vb.)
    pub trap_value: u64,   // Tuzağa özgü ek bilgi (örn: page fault adresi)
    pub syscall_nr: u64,   // Eğer trap_cause sistem çağrısı ise, bu syscall numarasıdır

    // TODO: Elbrus mimarisine özgü ek durum registerları veya bilgiler
    // Örneğin: Geçerli Görev/İş Parçacığı ID'si, kullanılan memory map bilgisi (TLB durumunu yeniden kurmak için)
}

// TODO: Elbrus mimarisine özgü trap/kesme neden kodları
// Bunlar donanım tarafından belirlenir.
const TRAP_CAUSE_SYSCALL: u64 = 1; // Varsayım: Sistem çağrısı nedeni kodu 1
const TRAP_CAUSE_PAGE_FAULT_LOAD: u64 = 2; // Varsayım: Yükleme (Load) hatası
const TRAP_CAUSE_PAGE_FAULT_STORE: u64 = 3; // Varsayım: Yazma (Store) hatası
const TRAP_CAUSE_ILLEGAL_INSTRUCTION: u64 = 4; // Varsayım: Geçersiz komut
const TRAP_CAUSE_TIMER_INTERRUPT: u64 = 5; // Varsayım: Zamanlayıcı kesmesi
// TODO: Diğer nedenler: Cihaz kesmeleri, hizalama hataları, breakpoint'ler vb.

// TODO: Sistem çağrısı argümanlarının ve dönüş değerinin hangi registerlarda taşındığına dair ABI (Application Binary Interface) bilgisi
// Bu da Elbrus mimarisine özgü bir konudur. Genellikle ilk birkaç register argümanlar için,
// birinci register (veya belirli bir register) ise dönüş değeri içindir.
// Bu indeksler TrapFrame'deki `regs` dizisinin indeksleri ile eşleşmelidir.
const SYSCALL_ARG1_REG: usize = 0; // Varsayım: 1. argüman regs[0]'da
const SYSCALL_ARG2_REG: usize = 1; // Varsayım: 2. argüman regs[1]'de
const SYSCALL_ARG3_REG: usize = 2; // Varsayım: 3. argüman regs[2]'de
const SYSCALL_ARG4_REG: usize = 3; // Varsayım: 4. argüman regs[3]'te
const SYSCALL_ARG5_REG: usize = 4; // Varsayım: 5. argüman regs[4]'te
const SYSCALL_RETURN_REG: usize = 0; // Varsayım: Dönüş değeri regs[0]'a yazılır

// --- Çekirdek İçi Hata İşleme (Placeholder) ---
// Gerçek bir çekirdekte düzgün bir hata işleme ve panik mekanizması olmalıdır.
// Bu sadece bir yer tutucudur.
fn kernel_panic(info: &str, frame: &TrapFrame) -> ! {
    // TODO: Konsola hata mesajını, trap frame içeriğini vb. yazdır
    // Bunun için Karnal64'ün ResourceProvider arayüzünü kullanan bir konsol sürücüsü gereklidir.
    // Şu an sadece döngüye girelim veya sistemi durduralım.
     println!("KERNEL PANIC: {}", info); // Eğer console ResourceProvider varsa kullanılabilir

    // Panik durumunda sistemin durması veya yeniden başlaması beklenir.
    loop {
        // Güvenli bir şekilde sonsuz döngüde kal
        core::arch::asm!("wfi", options(nomem, nostack)); // Wait For Interrupt (eğer mimari destekliyorsa)
    }
}


// --- Elbrus Düşük Seviye Tuzak Giriş Noktası (Rust Tarafı) ---
//
// Bu fonksiyon, genellikle düşük seviye Assembly kodundan çağrılır.
// Assembly kodu, tuzağı yakalar, kullanıcı registerlarını kaydeder (TrapFrame'e),
// çekirdek stack'ine geçer ve bu fonksiyonu çağırır.
// Fonksiyon işini bitirince Assembly kodu devam eder, registerları geri yükler ve kullanıcıya döner.
//
// # Safety
//
// Bu fonksiyon, ham pointerlarla çalıştığı ve çekirdeğin kritik bir giriş noktası olduğu için
// oldukça güvenli olmayan (unsafe) bir arayüzdür. Dikkatli kullanılmalıdır.
#[no_mangle] // Linker'ın bu fonksiyonun adını değiştirmemesini sağlar
pub extern "C" fn elbrus_trap_vector_entry(frame_ptr: *mut TrapFrame) {
    // Gelen pointer'ın NULL olup olmadığını temel bir kontrol yapabiliriz.
    // Daha gelişmiş kontroller (pointer'ın geçerli bir adres alanında olup olmadığı)
    // genellikle MMU/bellek yönetim kodu tarafından sağlanır veya gerekiyorsa burada yapılır.
    if frame_ptr.is_null() {
        kernel_panic("NULL trap frame pointer!", unsafe { core::ptr::null_mut::<TrapFrame>().as_ref().unwrap() }); // Dummy panik çağrısı
    }

    // Ham pointer'dan güvenli (ancak 'unsafe' blok içinde) mutable referans oluştur
    let frame = unsafe {
        &mut *frame_ptr
    };

    // Yüksek seviye işleyiciye yönlendir
    handle_trap(frame);

    // Not: Bu fonksiyon geri döndüğünde, düşük seviye Assembly kodu devam eder,
    // TrapFrame'den kaydedilen registerları geri yükler ve kullanıcı alanına döner.
}

// --- Yüksek Seviye Tuzak İşleyici ve Dağıtıcı ---
//
// Bu fonksiyon, tuzak nedenini analiz eder ve uygun çekirdek alt sistemine yönlendirir.
fn handle_trap(frame: &mut TrapFrame) {
    match frame.trap_cause {
        TRAP_CAUSE_SYSCALL => {
            // Sistem çağrısı ise, syscall işleyicisine yönlendir
            handle_syscall_trap(frame);
        }
        TRAP_CAUSE_PAGE_FAULT_LOAD | TRAP_CAUSE_PAGE_FAULT_STORE => {
            // Sayfa hatası ise, bellek yöneticisinin sayfa hatası işleyicisine yönlendir
            // TODO: Memory management module'den sayfa hatası işleyicisini çağır
             kmemory::handle_page_fault(frame);
            kernel_panic("Page Fault Not Implemented!", frame); // Geçici
        }
        TRAP_CAUSE_TIMER_INTERRUPT => {
            // Zamanlayıcı kesmesi ise, zamanlayıcı ve zamanlayıcıya yönlendir
            // TODO: Scheduler veya zamanlayıcı modülünden kesme işleyicisini çağır
             ktask::handle_timer_interrupt(frame);
             kernel_panic("Timer Interrupt Not Implemented!", frame); // Geçici
        }
        TRAP_CAUSE_ILLEGAL_INSTRUCTION => {
            // Geçersiz komut ise, görev sonlandırma veya sinyal gönderme (kullanıcı alanına)
            // TODO: Görev yöneticisinden ilgili görevi sonlandırma/sinyalleme fonksiyonunu çağır
             ktask::handle_illegal_instruction(frame);
             kernel_panic("Illegal Instruction Not Implemented!", frame); // Geçici
        }
        // TODO: Diğer trap/kesme nedenlerini buraya ekle ve ilgili işleyicilere yönlendir

        _ => {
            // Tanımlanmamış veya işlenmeyen tuzak nedeni
            // Bu ciddi bir hatadır, genellikle çekirdek paniğine neden olur.
            kernel_panic("Unknown or Unhandled Trap Cause!", frame);
        }
    }
}

/// Sistem çağrısı tuzaklarını işler.
///
/// TrapFrame'den sistem çağrısı numarasını ve argümanlarını çıkarır,
/// Karnal64'ün `handle_syscall` fonksiyonunu çağırır ve sonucu
/// TrapFrame'in dönüş değeri registerına yerleştirir.
fn handle_syscall_trap(frame: &mut TrapFrame) {
    // Sistem çağrısı numarasını al
    let syscall_number = frame.syscall_nr; // Veya hangi register/alandaysa oradan

    // Argümanları TrapFrame'deki registerlardan al
    // Register indeksleri mimariye özgü ABI'ya göre belirlenmelidir.
    let arg1 = frame.regs[SYSCALL_ARG1_REG];
    let arg2 = frame.regs[SYSCALL_ARG2_REG];
    let arg3 = frame.regs[SYSCALL_ARG3_REG];
    let arg4 = frame.regs[SYSCALL_ARG4_REG];
    let arg5 = frame.regs[SYSCALL_ARG5_REG];

    // Karnal64 sistem çağrısı dağıtıcısını çağır.
    // Karnal64::handle_syscall fonksiyonu, kullanıcı pointerlarını doğrulama
    // ve çekirdek içi işlemleri yapma sorumluluğuna sahiptir.
    let result: i64 = karnal64::handle_syscall(
        syscall_number,
        arg1,
        arg2,
        arg3,
        arg4,
        arg5,
    );

    // Karnal64'ten dönen sonucu (i64), kullanıcı alanının beklediği
    // dönüş değeri registerına (genellikle ilk argüman registerı) yaz.
    // i64 değeri u64 registerına yazılırken, negatif değerler (hata kodları)
    // büyük pozitif sayılar olarak temsil edilecektir, bu beklenen bir davranıştır.
    frame.regs[SYSCALL_RETURN_REG] = result as u64;
}


// TODO: Sayfa hatası işleyicisi için temel bir taslak (memory modülü implementasyonu gerektirir)

fn handle_page_fault(frame: &mut TrapFrame) {
    // Sayfa hatası nedeni (okuma/yazma), adres (trap_value), izinler vb. bilgileri frame'den al
    let fault_addr = frame.trap_value; // Varsayım
    let is_write = frame.trap_cause == TRAP_CAUSE_PAGE_FAULT_STORE; // Varsayım

    // TODO: Bellek yöneticisine (kmemory modülü) hatayı bildir
     kmemory::handle_page_fault(fault_addr, is_write, frame);

    // Bellek yöneticisi hatayı çözemezse (geçersiz erişim vb.), görevi sonlandır veya panik yap
     kernel_panic("Page Fault Unresolved!", frame); // Örnek
}


// TODO: Zamanlayıcı kesmesi işleyicisi için temel bir taslak (task/scheduler modülü gerektirir)

fn handle_timer_interrupt(frame: &mut TrapFrame) {
    // TODO: Zamanlayıcı donanımını sıfırla veya bir sonraki kesmeyi ayarla
    // TODO: Zamanlayıcı tick sayacını artır
    // TODO: Görev zamanlayıcısını (scheduler) çalıştır (eğer zamanlayıcı önleyici ise)
     ktask::schedule(frame); // frame zamanlayıcı için bağlamı sağlar
}
