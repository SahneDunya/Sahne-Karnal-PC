#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz, çekirdek alanında çalışırız

// Geliştirme sırasında kullanılmayan kod veya argümanlar için izinler
#![allow(dead_code)]
#![allow(unused_variables)]

// Çekirdek içinde kullanılacak temel Karnal64 tipleri ve API fonksiyonları
// Bu modüllere ve fonksiyonlara `karnal64::` öneki ile erişildiğini varsayalım.
// Örnek olarak, daha önceki tanımımızdaki kresource, ktask, handle_syscall gibi.
// Gerçek implementasyonda buraya ilgili 'use' ifadeleri eklenecektir.
 use crate::karnal64::{KError, KHandle, KTaskId, handle_syscall}; // Örnek kullanım

// --- PowerPC'ye Özel Yapılar ve Sabitler ---

/// PowerPC CPU durumunu (kayıtçılarını) istisna/kesme anında kaydetmek için kullanılan yapı.
/// Assembly giriş noktasında doldurulur ve Rust handler'ına geçirilir.
#[repr(C)] // C uyumluluğu, assembly'den erişim için önemli
pub struct TrapFrame {
    // Genel Amaçlı Kayıtçılar (General Purpose Registers - GPRs)
    // PowerPC'de r0-r31 vardır. Syscall argümanları ve dönüş değerleri buradadır.
    pub r0: u64,
    pub r1: u64,  // Genellikle yığın işaretçisi (Stack Pointer)
    pub r2: u64,  // Genellikle TOC (Table of Contents) pointer'ı
    pub r3: u64,  // Fonksiyon argümanı 1 / Dönüş değeri
    pub r4: u64,  // Fonksiyon argümanı 2
    pub r5: u64,  // Fonksiyon argümanı 3
    pub r6: u64,  // Fonksiyon argümanı 4
    pub r7: u64,  // Fonksiyon argümanı 5
    pub r8: u64,
    pub r9: u64,
    pub r10: u64, // Fonksiyon argümanı 8 (varsa)
    pub r11: u64, // Genellikle IChain
    pub r12: u64, // Genellikle TChain
    // ... r13'ten r31'e kadar diğer GPR'ler ...
    pub r13_r31: [u64; 31 - 13 + 1], // Basitlik için dizi olarak tutalım

    // Özel Amaçlı Kayıtçılar (Special Purpose Registers - SPRs)
    pub cr: u64,  // Koşul Kayıtçısı (Condition Register)
    pub xer: u64, // Sabit Nokta İstisna Kayıtçısı (Fixed-point Exception Register)
    pub lr: u64,  // Bağlantı Kayıtçısı (Link Register) - Fonksiyon dönüş adresi
    pub ctr: u64, // Sayım Kayıtçısı (Count Register)
    pub srr0: u64,// İstisna/Kesme anındaki Komut İşaretçisi (Instruction Pointer)
    pub srr1: u64,// İstisna/Kesme anındaki Makine Durum Kayıtçısı (Machine State Register - MSR)
    // Diğer SPR'ler (örneğin, esid, dear, sprg0-3, trap gibi istisnalara özel olanlar)
    // Tam bir TrapFrame, kullanılan PowerPC alt ailesine ve kernel'in detay seviyesine bağlıdır.
    // Basitlik için bazı önemli olanları ekledik.
}

/// PowerPC İstisna Vektörleri (Kavramsal Numaralar/Ofsetler)
/// Gerçek değerler PowerPC referans kılavuzlarına göre belirlenir.
/// Bunlar, `srr1`'deki durum veya istisna tipi bilgisinden veya atlanılan adresten çıkarılır.
pub const POWERPC_VECTOR_SYSTEM_CALL: u64            = 0x0C00; // Genellikle 0xC00 adresine atlar (System Call handler'ı)
pub const POWERPC_VECTOR_DATA_STORAGE: u64           = 0x0300; // Veri Depolama İstisnası (Page Fault, Alignment, vb.)
pub const POWERPC_VECTOR_INSTRUCTION_STORAGE: u64    = 0x0400; // Komut Depolama İstisnası (Page Fault, Illegal Instruction, vb.)
pub const POWERPC_VECTOR_EXTERNAL_INTERRUPT: u64     = 0x0500; // Harici Kesme (Donanım kesmeleri)
pub const POWERPC_VECTOR_ALIGNMENT: u64              = 0x0600; // Hizalama Hatası
pub const POWERPC_VECTOR_PROGRAM: u64                = 0x0700; // Program İstisnası (Illegal Instruction, Privileged Instruction, vb.)
pub const POWERPC_VECTOR_FLOATING_POINT: u64         = 0x0800; // Kayan Nokta İstisnası
pub const POWERPC_VECTOR_DECREMENTER: u64            = 0x0900; // Azaltıcı (Timer) Kesmesi
pub const POWERPC_VECTOR_TRAP: u64                   = 0x0D00; // TRAP İstisnası (kullanıcı tanımlı breakpoint vb.)
// ... diğer PowerPC istisna vektörleri ...

// --- Çekirdek İstisna İşleyicileri ---

/// Assembly giriş noktasından çağrılan genel Rust istisna işleyicisi.
/// CPU durumunu içeren TrapFrame ve istisna/vektör bilgisini alır.
#[no_mangle] // Assembly tarafından çağrılabilmesi için isim düzenlemesi yapılmaz
pub extern "C" fn exception_handler(tf: &mut TrapFrame, vector: u64) {
    // İstisna anındaki Makine Durum Kayıtçısı (MSR) önemli bilgiler içerir.
    // Özellikle PR biti (Ayrıcalık Seviyesi - 0: Supervisor, 1: User)
    // EE biti (Harici Kesme Etkin - External Enable)
    let msr = tf.srr1;
    let is_from_user = (msr & (1 << 1)) != 0; // MSR'nin 1. biti (PR) ayrıcalık seviyesidir.

    // Hata ayıklama için temel bilgi yazdırma
     println!("Karnal64: İstisna alındı! Vektör: {:#x}, User: {}", vector, is_from_user);

    // İstisna vektörüne göre uygun alt işleyiciye yönlendirme (dispatch).
    // Bu, Karnal64'ün ilgili yöneticilerine (Task, Memory, Resource, vb.) ulaşmanın yoludur.
    match vector {
        POWERPC_VECTOR_SYSTEM_CALL => {
            // Sistem çağrısı - Karnal64'ün handle_syscall'unu çağıracağız
            handle_syscall_trap(tf, is_from_user);
        }
        POWERPC_VECTOR_DATA_STORAGE => {
            // Veri depolama istisnası - Bellek yöneticisi ilgilenir (Sayfa hatası, vb.)
            handle_data_storage_trap(tf, is_from_user);
        }
        POWERPC_VECTOR_INSTRUCTION_STORAGE => {
            // Komut depolama istisnası - Bellek yöneticisi veya görev yöneticisi ilgilenir (Sayfa hatası, illegal komut vb.)
            handle_instruction_storage_trap(tf, is_from_user);
        }
        POWERPC_VECTOR_EXTERNAL_INTERRUPT => {
            // Harici kesme - Kesme denetleyicisi ve sürücüler ilgilenir
            handle_external_interrupt(tf, is_from_user);
        }
        POWERPC_VECTOR_DECREMENTER => {
             // Zamanlayıcı kesmesi - Görev zamanlayıcısı ilgilenir
            handle_decrementer_interrupt(tf, is_from_user);
        }
        // TODO: Diğer istisna vektörleri için eşleşmeler eklenecek
        _ => {
            // Bilinmeyen veya desteklenmeyen istisna
            handle_unknown_exception(tf, vector, is_from_user);
        }
    }

    // İstisna işleyiciden dönüş (assembly tarafından TrapFrame kullanılarak yapılır)
    // tf.srr0, tf.srr1 gibi kaydedilmiş kayıtçılar, assembly'nin doğru yere (genellikle istisnaya neden olan komutun hemen sonrasına)
    // ve doğru duruma (kullanıcı/süpervizör, kesmeler etkin/devre dışı) dönmesini sağlar.
}

/// Sistem Çağrısı (SYSCALL) istisnasını işleyen fonksiyon.
/// Karnal64'ün genel sistem çağrısı işleyicisine (handle_syscall) yönlendirir.
fn handle_syscall_trap(tf: &mut TrapFrame, is_from_user: bool) {
    // Güvenlik: Sistem çağrısının kullanıcı alanından geldiğinden emin olun.
    // Çekirdek içinden yapılan 'sc' komutları farklı şekilde ele alınabilir veya yasaklanabilir.
    if !is_from_user {
         println!("Karnal64: Çekirdekten yasa dışı SYSCALL! Panikliyor...");
        // TODO: Çekirdek içi hatayı işle, panik yap veya görevi sonlandır
        panic!("Illegal SYSCALL from kernel mode!");
    }

    // PowerPC ABI'sine göre sistem çağrısı argümanlarını TrapFrame'den al.
    // Genellikle r0 = syscall numarası, r3-r10 = argümanlar.
    let syscall_number = tf.r0;
    let arg1 = tf.r3;
    let arg2 = tf.r4;
    let arg3 = tf.r5;
    let arg4 = tf.r6;
    let arg5 = tf.r7;
    // Karnal64'ün handle_syscall'u 5 argüman alacak şekilde tasarlandı, buraya kadarını alıyoruz.
    // Daha fazla argüman gerekirse TrapFrame'den alınmalı (r8, r9, r10...).

    // Güvenlik Notu: Burada arg1, arg2 vb. kullanıcı alanı pointer'ları olabilir.
    // Bu pointer'ların geçerliliği ve erişilebilirliği *handle_syscall* veya onun
    // çağırdığı Karnal64 API fonksiyonları tarafından *doğrulanmalıdır*.
    // Örneğin, user_buffer_ptr gibi argümanlar kontrol edilmelidir.

    // Karnal64'ün genel sistem çağrısı işleyicisini çağır.
    // Bu fonksiyon, sistem çağrısı numarasına göre uygun Karnal64 API fonksiyonunu (resource_read, task_spawn vb.)
    // çağıracak ve sonucu (i64) dönecektir.
    let result = karnal64::handle_syscall(syscall_number, arg1, arg2, arg3, arg4, arg5);

    // Karnal64'ten dönen sonucu (pozitif/sıfır başarı, negatif hata kodu)
    // kullanıcı alanına dönüş değeri olarak ayarla.
    // PowerPC ABI'de dönüş değeri genellikle r3 kayıtçısına konur.
    tf.r3 = result as u64; // i64'ten u64'e dönüşüm - negatif değerler tamamlayıcı formunda saklanır.

    // SYSCALL komutundan sonra yürütmeye devam etmek için SRR0'ı ayarla.
    // PowerPC'de 'sc' komutu genellikle 4 byte uzunluğundadır.
    // Bu, işlemcinin SYSCALL komutunun *sonraki* komutla yürütmeye devam etmesini sağlar.
    tf.srr0 += 4; // SYSCALL komutunun uzunluğunu ekle (mimariye/komut setine bağlı olabilir)
}

/// Veri Depolama İstisnası (Data Storage Interrupt - DSI) işleyicisi.
/// Sayfa hataları, hizalama hataları vb. durumları ele alır. Bellek yöneticisine yönlendirir.
fn handle_data_storage_trap(tf: &mut TrapFrame, is_from_user: bool) {
    // TODO: DSISR (Data Storage Interrupt Status Register) ve DAR (Data Address Register) oku.
     unsafe {
        let dsisr: u64 = core::arch::asm!("mfspr {0}, 0x3F0", out(reg) _); // DSISR SPR numarası PowerPC alt ailesine göre değişir (örn. 0x3F0 PowerPC 64-bit)
        let dar: u64 = core::arch::asm!("mfspr {0}, 0x3F1", out(reg) _); // DAR SPR numarası
     }
     println!("Karnal64: Veri Depolama İstisnası alındı! DAR: {:#x}, DSISR: {:#x}", dar, dsisr);

    // TODO: Hatayı analiz et (DSISR'ye bakarak okuma/yazma/yürütme, sayfa hatası türü vb.).
    // TODO: Karnal64 bellek yöneticisine (kmemory) page_fault veya benzeri bir fonksiyonla haber ver.
     let result = kmemory::handle_data_storage_exception(tf, dar, dsisr, is_from_user);

    // İşlem başarılı olursa (örn. sayfa map edildi), srr0'ı istisnaya neden olan komuta ayarlayarak
    // o komutun yeniden denenmesini sağla.
     tf.srr0 = srr0_at_exception; // İstisna anındaki SRR0 değeri (TrapFrame'de mevcut)

    // İşlem başarısız olursa (geçersiz erişim), görevi sonlandır veya sinyal gönder.
     if result.is_err() {
        println!("Karnal64: Geçersiz bellek erişimi! Görev sonlandırılıyor...");
        ktask::terminate_task(ktask::get_current_task_id(), -KError::BadAddress as i32); // Örnek kullanım
    //    // terminate_task geri dönmez
     }

    // Placeholder: Şimdilik sadece hata yazıp panik yapalım.
    println!("Karnal64: İşlenmemiş Veri Depolama İstisnası! Görev sonlandırılıyor.");
    // Gerçek kodda burada panic yerine daha kontrollü bir görev sonlandırma olur.
    if is_from_user {
       // Kullanıcı görevini sonlandır
        ktask::terminate_current_task(KError::BadAddress as i32);
    } else {
       // Çekirdek hatası, panik genellikle kaçınılmazdır
       panic!("Kernel Data Storage Exception!");
    }
}

/// Komut Depolama İstisnası (Instruction Storage Interrupt - ISI) işleyicisi.
/// Komut sayfa hataları, yasadışı komutlar vb. durumları ele alır. Bellek/görev yöneticisine yönlendirir.
fn handle_instruction_storage_trap(tf: &mut TrapFrame, is_from_user: bool) {
    // TODO: SRR0'daki adresteki komutu veya durumu analiz et.
    // Yasadışı komut, ayrıcalıklı komut hatası, komut sayfa hatası vb. olabilir.
     println!("Karnal64: Komut Depolama İstisnası alındı! SRR0: {:#x}, SRR1: {:#x}", tf.srr0, tf.srr1);

    // TODO: Hatayı analiz et.
    // TODO: Karnal64 bellek yöneticisine (kmemory) instruction_page_fault veya görev yöneticisine (ktask) illegal_instruction gibi haber ver.
     let result = kmemory::handle_instruction_storage_exception(tf, is_from_user);

     // İşlem başarılı olursa (örn. sayfa map edildi), srr0'ı istisnaya neden olan komuta ayarlayarak yeniden dene.
      tf.srr0 = srr0_at_exception;

     // İşlem başarısız olursa, görevi sonlandır.

     // Placeholder
    println!("Karnal64: İşlenmemiş Komut Depolama İstisnası! Görev sonlandırılıyor.");
    if is_from_user {
        ktask::terminate_current_task(KError::InvalidArgument as i32); // Yasadışı komut için
    } else {
       panic!("Kernel Instruction Storage Exception!");
    }
}

/// Harici Kesme (External Interrupt) işleyicisi.
/// Donanım kesmelerini (disk, ağ, klavye vb.) ele alır. Kesme denetleyicisine ve sürücülere yönlendirir.
fn handle_external_interrupt(tf: &mut TrapFrame, is_from_user: bool) {
    // TODO: Kesme denetleyicisinden (örn. PIC, APIC benzeri donanım) kesme numarasını (IRQ) oku.
     let irq = pic::get_pending_irq(); // Varsayımsal PIC modülü

    // TODO: Karnal64 kesme yöneticisine (kinterrupt?) IRQ numarasını ileterek ilgili sürücünün kesme işleyicisini (ISR) çalıştır.
     kinterrupt::dispatch_irq(irq);

    // TODO: Kesme denetleyicisine End-Of-Interrupt (EOI) sinyali gönder (donanıma bağlı).
     pic::send_eoi(irq);

    // Zamanlayıcı kesmesi (decrementer) ayrı olarak ele alınabilir, ancak genel harici kesme de buradan geçebilir.

    // Placeholder
     println!("Karnal64: Harici Kesme alındı! IRQ: (Bilinmiyor, placeholder)");
}

/// Azaltıcı (Decrementer - Timer) Kesmesi işleyicisi.
/// Periyodik zamanlayıcı kesmelerini ele alır. Görev zamanlayıcısına yönlendirir.
fn handle_decrementer_interrupt(tf: &mut TrapFrame, is_from_user: bool) {
     // println!("Karnal64: Zamanlayıcı Kesmesi alındı!");

     // TODO: Karnal64 görev zamanlayıcısına (ktask) bir zamanlayıcı tikinin gerçekleştiğini bildir.
      ktask::timer_tick(tf);

     // Zamanlayıcı genellikle görev değiştirmeyi (preemption) tetikler.
     // ktask::timer_tick fonksiyonu, eğer bir görev değişimi gerekiyorsa,
     // TrapFrame'i güncelleyebilir (örneğin srr0/srr1 ve yığın işaretçisi r1'i)
     // böylece istisnadan dönüldüğünde farklı bir göreve geçilir.

     // PowerPC'de azaltıcıyı yeniden yüklemek gerekebilir.
      unsafe {
           let new_decrementer_value = calculate_next_tick_value(); // Sonraki kesme zamanını hesapla
           core::arch::asm!("mtspr 0x00E, {0}", in(reg) new_decrementer_value); // Decrementer SPR numarası (0x00E)
      }
}


/// Bilinmeyen veya işlenmemiş istisnalar için genel işleyici.
/// Genellikle görevin sonlandırılmasına veya çekirdek panikine yol açar.
fn handle_unknown_exception(tf: &mut TrapFrame, vector: u64, is_from_user: bool) {
    println!("Karnal64: !!! İşlenmemiş İstisna !!!");
    println!("Vektör: {:#x}", vector);
    println!("SRR0 (IP): {:#x}", tf.srr0);
    println!("SRR1 (MSR): {:#x}", tf.srr1);
    println!("GPR3 (r3): {:#x}", tf.r3); // Genellikle hata kodu/bilgisi içerebilir
    println!("LR: {:#x}", tf.lr);
    println!("CR: {:#x}", tf.cr);
    println!("Kaynak: {}", if is_from_user { "Kullanıcı Alanı" } else { "Çekirdek Alanı" });

    // İşlenmemiş istisna ciddi bir durumdur.
    if is_from_user {
        // Kullanıcı alanındaki bir görevin bilinmeyen bir istisnaya neden olması.
        // Görevi sonlandırmak en güvenli yoldur.
        println!("Karnal64: Kullanıcı görevi sonlandırılıyor...");
        // ktask::terminate_current_task(KError::InternalError as i32); // Veya KError::NotSupported
        // terminate_current_task geri dönmez.
        // Eğer burada panik yaparsak, kullanıcı görevini değil, tüm çekirdeği panikletiriz, bu istenmez.
        // Ancak yer tutucu olarak panik kullanıyoruz:
        panic!("Unhandled exception in user task!");

    } else {
        // Çekirdek alanındaki bir hatanın işlenmemiş bir istisnaya neden olması.
        // Bu genellikle çekirdek hatasıdır ve kurtarma mümkün olmayabilir.
        println!("Karnal64: Çekirdek hatası! Panik.");
        panic!("Unhandled exception in kernel mode!");
    }
}


// --- Çekirdek Boot Sırasındaki Kurulum (Örnek) ---

// Bu fonksiyon, çekirdek başlangıcında istisna vektör tablosunu ayarlamak için kullanılır.
// Gerçek implementasyon, donanımın nasıl yapılandırıldığına bağlıdır.
pub fn init_exception_handling() {
    // TODO: PowerPC'deki istisna vektör tablosu pointer'ını ayarla veya gerekli başlangıç vektörlerine zıplama komutlarını yerleştir.
    // Bu genellikle çekirdek boot loader veya başlangıç kodunda assembly ile yapılır.
    // Örneğin, IVPR (Interrupt Vector Prefix Register) ayarlanabilir.
     unsafe {
        let exception_table_base: u64 = get_exception_table_address(); // Vektör tablosunun bellekteki adresi
        core::arch::asm!("mtspr 0x13F, {0}", in(reg) exception_table_base); // IVPR SPR numarası
     }

    // Her istisna vektör adresine, context save yapacak ve exception_handler'ı çağıracak
    // küçük assembly "trampoline" kod parçacıkları yerleştirilmelidir.
    // exception_table_base + POWERPC_VECTOR_SYSTEM_CALL adresine syscall_trampoline_assembly yerleştir gibi.

    println!("Karnal64: PowerPC İstisna İşleme Başlatıldı (Yer Tutucu).");
}

// --- İhtiyaç Duyulabilecek Diğer Yardımcı Fonksiyonlar ---

// Örnek: İstisna anındaki SRR0 değerinden komutun adresini döndüren fonksiyon (basit, dallanmaları dikkate almaz)
 fn get_instruction_pointer(tf: &TrapFrame) -> u64 {
    tf.srr0
 }

// Örnek: MSR'den ayrıcalık seviyesini kontrol etme
 fn is_supervisor_mode(tf: &TrapFrame) -> bool {
    (tf.srr1 & (1 << 1)) == 0 // MSR'nin PR biti 0 ise supervisor
 }


// --- Kavramsal olarak Karnal64 API'sına çağrılar ---
// Bu dosya bu fonksiyonları implemente etmez, sadece çağırır.
mod karnal64 {
    use super::*; // Üst kapsamdaki KError, KHandle vb. tiplerini kullan
    // Diğer Karnal64 API fonksiyonlarının dummy veya gerçek tanımları buraya gelecek
    // (veya ilgili crate/modülden use edilecek).

    // Örnek olarak, daha önceki handle_syscall fonksiyonunun imzası:
    #[no_mangle] // Sistem çağrısı işleyici tarafından çağrılabilmesi için
    pub extern "C" fn handle_syscall(
        number: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
        arg4: u64,
        arg5: u64
    ) -> i64 {
        // Gerçek Karnal64 handle_syscall implementasyonu burada değil,
        // o kendi modülündedir. Burası sadece çağırdığımız yerin imzasını temsil eder.
        // Dummy implementasyon:
         println!("Karnal64 Dummy Syscall Handler: num={}, arg1={}", number, arg1);
        match number {
             5 => { // SYSCALL_RESOURCE_ACQUIRE (Dummy)
                 // arg1: id_ptr, arg2: id_len, arg3: mode
                 // Gerçek fonksiyona yönlendirme: kresource::resource_acquire(...)
                 println!("Dummy: resource_acquire called");
                 Ok(123_u64).map(|v| v as i64).unwrap_or_else(|e| e as i64) // Dummy handle 123
             },
             6 => { // SYSCALL_RESOURCE_READ (Dummy)
                  // arg1: handle_value, arg2: user_buffer_ptr, arg3: user_buffer_len
                  println!("Dummy: resource_read called with handle {}", arg1);
                  // Güvenlik: user_buffer_ptr/len doğrulanmalı!
                  // Gerçek fonksiyona yönlendirme: kresource::resource_read(...)
                  Ok(5_usize).map(|v| v as i64).unwrap_or_else(|e| e as i64) // Dummy 5 byte okundu
             },
             // Diğer SYSCALL numaraları...
             _ => {
                 println!("Dummy: Unknown syscall {}", number);
                 -38 // KError::NotSupported
             }
        }
    }

    // // Örnek: KTask modülüne yapılan çağrılar (dummies)
     pub mod ktask {
         use super::*;
         pub fn terminate_current_task(exit_code: i32) -> ! {
             println!("Dummy ktask: Terminating current task with code {}", exit_code);
             loop {} // Gerçekte bu fonksiyon geri dönmez
         }
    //     // Diğer ktask fonksiyonları...
     }

    // // Örnek: KMemory modülüne yapılan çağrılar (dummies)
     pub mod kmemory {
         use super::*;
         pub fn handle_data_storage_exception(tf: &mut TrapFrame, dar: u64, dsisr: u64, is_from_user: bool) -> Result<(), KError> {
             println!("Dummy kmemory: Handling DSI...");
    //         // Bellek yönetim mantığı buraya gelmez, bu sadece çağrı arayüzü
             Err(KError::BadAddress) // Dummy hata
         }
    //      // Diğer kmemory fonksiyonları...
     }

    // // Örnek: KInterrupt modülüne yapılan çağrılar (dummies)
     pub mod kinterrupt {
          use super::*;
          pub fn dispatch_irq(irq: u64) {
              println!("Dummy kinterrupt: Dispatching IRQ {}", irq);
              // Gerçekte ilgili ISR'ler çalıştırılır
          }
    //      // Diğer kinterrupt fonksiyonları...
     }

    // Kernel Error Enum'u da burada veya ayrı bir modülde tanımlı olmalı
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(i64)]
    pub enum KError {
     PermissionDenied = -1,
     NotFound = -2,
     InvalidArgument = -3,
     Interrupted = -4,
     BadHandle = -9,
     Busy = -11,
     OutOfMemory = -12,
     BadAddress = -14,
     AlreadyExists = -17,
     NotSupported = -38,
     NoMessage = -61,
     InternalError = -255,
    }
}
