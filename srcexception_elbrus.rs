#![no_std]

// Karnal64 API ve tipleri için harici crate (veya modül) bildirimi
// Varsayım: karnal64 crate'i veya modülü derleme ağacınızda mevcut
extern crate karnal64;

use core::arch::asm;
use karnal64::{KError, KHandle, KTaskId}; // Karnal64'ten gerekli tipleri import et
use karnal64::handle_syscall; // Sistem çağrısı işleyicisini import et

// Bu modül, Elbrus mimarisine özel kesme/istisna işleme detaylarını içerecektir.
// Düşük seviyeli istisna vektörleri, bağlam kaydetme/yükleme, CPU özel register erişimleri vb.

// --- Mimariye Özel Yapılar ve Fonksiyonlar ---

/// İstisna veya kesme anında CPU durumunu (kayıtları) kaydetmek için kullanılan yapı.
/// Bu yapı, kullanıcı alanına geri dönerken veya bağlam değiştirirken kullanılır.
/// Alanların isimleri Elbrus'a özel olmalıdır; burada genel isimler kullanılmıştır.
#[repr(C)] // C uyumluluğu, assembly kodundan kolay erişim için
#[derive(Debug, Default, Copy, Clone)]
pub struct TrapFrame {
    // Genel amaçlı kayıtlar (GPRs) - Mimariye göre sayıda ve isimde olmalı
    // Örnek: Elbrus E2K mimarisinde çok sayıda register var.
    // Basit bir örnek için genel kullanım registerlarını simüle edelim:
    // r0-r30 veya x0-x30 gibi + stack pointer + frame pointer
    pub gpr: [u64; 32], // Yer tutucu: Elbrus GPR sayısına göre ayarlanmalı
    pub sp: u64,        // Stack Pointer
    pub fp: u64,        // Frame Pointer
    pub elr: u64,       // Exception Link Register (İstisna sonrası dönülecek adres)
    pub spsr: u64,      // Saved Process Status Register (İstisna anındaki bayraklar/durum)
    pub syscall_num: u64, // Sistem çağrısı numarası (Genellikle belirli bir registerda bulunur)
    // Sistem çağrısı argümanları (Genellikle ilk birkaç GPR'da bulunur)
    pub arg1: u64,
    pub arg2: u64,
    pub arg3: u64,
    pub arg4: u64,
    pub arg5: u64,
    // ... Elbrus'a özel diğer durum kayıtları (fpu durumu, vector durumu vb.) eklenebilir
}

/// Düşük seviyeli istisna giriş noktası.
/// Bu fonksiyon/assembly kodu, CPU istisnaya girdiğinde ilk çalışan kısımdır.
/// Görevi:
/// 1. CPU durumunu (kayıtları) stack'e kaydetmek.
/// 2. Bir TrapFrame yapısı oluşturup bu durumu içine kopyalamak.
/// 3. Yüksek seviyeli Rust işleyicisine (`handle_exception`) bu TrapFrame'in pointer'ını vererek çağırmak.
/// 4. `handle_exception`'dan döndükten sonra (eğer dönüyorsa), TrapFrame'den CPU durumunu geri yüklemek.
/// 5. İstisnaya uğrayan iş parçacığına (genellikle kullanıcı alanına) geri dönmek.
///
/// Bu kısım **kesinlikle** mimariye özel assembly veya inline assembly gerektirir.
/// Aşağıdaki sadece kavramsal bir Rust fonksiyon bildirimidir, gerçek implementasyon *assembly* olacaktır.
#[no_mangle] // Düşük seviyeli vektör tablosundan çağrılabilmesi için isim düzenlemesi yapılmaz
pub extern "C" fn elbrus_exception_entry(trap_frame_ptr: *mut TrapFrame) {
    // GERÇEK KOD: Bu fonksiyon asla Rust'ta doğrudan çağrılmaz.
    // Assembly giriş noktasının, kayıtları kaydettikten sonra
    // bu fonksiyonu çağırması beklenir.
    //
     let trap_frame = unsafe { &mut *trap_frame_ptr };
     handle_exception(trap_frame);
    // Assembly kodu buradan döndüğünde, trap_frame'deki durumu yükleyip dönecektir.
    panic!("elbrus_exception_entry should not be called directly in Rust"); // Sadece konsepti belirtmek için
}

// --- Yüksek Seviyeli Rust İstisna İşleyicisi ---

/// Çekirdek istisna ve kesmeleri için genel Rust işleyicisi.
/// Düşük seviyeli mimariye özel giriş noktasından çağrılır.
/// İstisna türünü çözer ve uygun özel işleyiciye (syscall, page fault, IRQ vb.) yönlendirir.
#[no_mangle] // Düşük seviyeli assembly tarafından çağrılacağı için
pub extern "C" fn handle_exception(trap_frame: &mut TrapFrame) {
    // İstisna nedenini belirle. Bu bilgi SPSR veya başka mimariye özel bir registerda bulunur.
    // Elbrus'a özel istisna sınıflandırma mantığı buraya gelmeli.
    let exception_class: u64 = (trap_frame.spsr >> 26) & 0x3F; // Örnek: AArch64'teki EC alanı gibi

    match exception_class {
        // Örnek İstisna Sınıfları (Bunlar Elbrus'a özel değerler olmalıdır!)
        0x09 => { // Örnek: Sistem Çağrısı (SVC/SYSCALL)
            handle_syscall_exception(trap_frame);
        }
        0x05 => { // Örnek: Sayfa Hatası (Data Abort)
            handle_page_fault(trap_frame);
        }
        0x06 => { // Örnek: Talimat Hatası (Instruction Abort)
             handle_page_fault(trap_frame); // Genellikle aynı işlenir, ama ayrılabilir
        }
        _ => { // Diğer istisnalar (Illegal instruction, alignment error, vb.)
            handle_other_exception(trap_frame, exception_class);
        }
    }

    // Eğer istisna işlendi ve dönülmesi gerekiyorsa (syscall gibi),
    // handle_syscall_exception veya diğer işleyiciler trap_frame'i uygun şekilde günceller.
    // Assembly kodu buradan döndüğünde, trap_frame'deki ELR ve SPSR'ye göre dönecektir.
}


/// Sistem Çağrısı istisnasını işler.
/// Kullanıcı alanından gelen sistem çağrısı isteğini ayrıştırır, argümanları doğrular
/// ve Karnal64'ün handle_syscall fonksiyonunu çağırır.
fn handle_syscall_exception(trap_frame: &mut TrapFrame) {
    // 1. Sistem Çağrısı Numarasını ve Argümanları Al
    // Varsayım: Syscall numarası ve argümanlar trap_frame'e düşük seviyeli giriş tarafından
    // veya mimariye özgü register okuma ile yerleştirildi.
    let syscall_number = trap_frame.syscall_num;
    let arg1 = trap_frame.arg1;
    let arg2 = trap_frame.arg2;
    let arg3 = trap_frame.arg3;
    let arg4 = trap_frame.arg4;
    let arg5 = trap_frame.arg5;

    // 2. KULLANICI ALANI POINTER DOĞRULAMASI!
    // BURASI KRİTİK GÜVENLİK NOKTASIDIR.
    // Sistem çağrısı argümanlarından herhangi biri kullanıcı alanı pointer'ı ise (örn. read/write/send/receive buffer pointerları),
    // BU POINTERLARIN ÇEKİRDEK TARAFINDAN KULLANILMADAN ÖNCE MUTLAKA DOĞRULANMASI GEREKİR!
    // Doğrulama şunları içermelidir:
    // - Pointer'ın kullanıcının sanal adres alanında geçerli bir adresi gösterdiğini.
    // - İstenen bellek bloğunun (pointer + uzunluk) kullanıcının adres alanında olduğunu.
    // - İstenen işlemin (okuma/yazma) o bellek bloğu için izinli olduğunu.
    // - Pointer'ın çekirdek alanını işaret etmediğini.
    //
    // Bu doğrulama, çekirdeğin bellek yönetim birimi (MMU) ve o anki görevin sayfa tabloları
    // ile etkileşim kurularak yapılır.
    //
    // TODO: Sistem çağrısı numarasına bakarak hangi argümanların pointer olduğunu belirle
    // ve kmemory modülündeki doğrulama fonksiyonlarını kullan (örneğin `kmemory::validate_user_pointer`).
    // Eğer doğrulama başarısız olursa, bir KError döndürülmeli (InvalidAddress, PermissionDenied gibi)
    // ve sistem çağrısı işleyicisine iletilmelidir.

    // Örnek: resource_read syscall'ı için arg2 kullanıcı pointerıdır.
     if syscall_number == SYSCALL_RESOURCE_READ { // SYSCALL numaraları karnal64 veya başka bir ortak yerde tanımlı olmalı
         let user_buffer_ptr = arg2 as *mut u8;
         let user_buffer_len = arg3 as usize;
    //     // TODO: kmemory::validate_user_writable_buffer(user_buffer_ptr, user_buffer_len) çağrısı yap
    //     // Hata durumunda KError::BadAddress veya KError::PermissionDenied döndür ve işlemi sonlandır
     }
    // Benzer doğrulama tüm pointer argümanları için yapılmalı.

    // 3. Karnal64 handle_syscall Fonksiyonunu Çağır
    // Doğrulama başarılıysa, sistem çağrısını Karnal64'ün ana işleyicisine yönlendir.
    let syscall_result: i64 = handle_syscall(
        syscall_number,
        arg1,
        arg2,
        arg3,
        arg4,
        arg5,
    );

    // 4. Sonucu Kullanıcı Alanına Döndürmek İçin TrapFrame'e Yaz
    // Sistem çağrısı sonucu (i64), genellikle kullanıcı alanındaki çağıran fonksiyonun
    // beklediği register'a (örneğin R0 veya X0) yazılır.
    // TrapFrame yapısındaki ilgili alana sonucu kaydet.
    // Eğer sonuç pozitif/sıfır ise başarı, negatif ise KError kodudur.
    // Elbrus'a özel sonuç register'ını belirle.
    trap_frame.gpr[0] = syscall_result as u64; // Varsayım: Sonuç GPR[0]'a yazılır

    // Not: task_exit gibi bazı sistem çağrıları geri dönmez.
    // handle_syscall içinde task_exit çağrıldığında, zamanlayıcıya bağlam değiştirmesini söyler
    // ve bu fonksiyonun geri kalan kısmı (TrapFrame güncelleme ve dönüş) atlanabilir.
    // Bu mantık ktask::task_exit içinde ele alınmalıdır.
}

/// Sayfa Hatası (Page Fault) istisnasını işler.
/// Bellek yönetim birimi (MMU) tarafından tetiklenen hataları (örneğin, var olmayan adrese erişim) işler.
fn handle_page_fault(trap_frame: &mut TrapFrame) {
    // 1. Hata Adresini ve Nedenini Al
    // Hata adresi (Fault Address Register - FAR) ve hata nedeni (Fault Status Register - FSR)
    // gibi mimariye özel registerlardan bilgi alınır.
    // TODO: Mimariye özel registerları oku (asm! kullanarak veya helper fonksiyonlarla)
     let fault_address = read_far_register();
     let fault_status = read_fsr_register();

    println!("PANIC: Page Fault!"); // Geçici hata mesajı (kernel içi print gerektirir)
    // TODO: fault_address ve fault_status bilgilerini yazdır

    // 2. Bellek Yönetim Modülüne Yönlendir
    // kmemory modülü sayfa tablosunu kontrol edebilir, copy-on-write, lazy allocation
    // gibi durumları işleyebilir veya segmentasyon hatası olduğunu belirleyebilir.
     let handled = kmemory::handle_fault(fault_address, fault_status, trap_frame);

    // 3. Sonucu İşle
     if handled {
    //     // Hata başarıyla işlendi, istisnaya uğrayan işleme geri dönülebilir.
    //     // TrapFrame zaten dönüş adresini (ELR) içerir.
     } else {
        // Hata işlenemedi (geçersiz adres, izin yok vb.)
        // Bu genellikle o anki görevi sonlandırmak veya kernel panik etmek anlamına gelir.
        println!("Unhandled Page Fault at address: {:x}", 0); // Yer tutucu adres
        // TODO: Karnal64 ktask::terminate_current_task() veya kernel_panic() çağrısı yap
        loop {} // Panik veya terminal döngüsü
     }
}

/// Diğer istisnaları işler (Illegal instruction, alignment error, vb.).
fn handle_other_exception(trap_frame: &mut TrapFrame, exception_class: u64) {
    println!("PANIC: Unhandled Exception!"); // Geçici hata mesajı
    // TODO: TrapFrame ve exception_class bilgilerini yazdır
    println!("Exception Class: {:x}", exception_class);
    println!("ELR: {:x}", trap_frame.elr);
    println!("SPSR: {:x}", trap_frame.spsr);
    // TODO: GPR kayıtlarını yazdır

    // İşlenemeyen istisnalar genellikle ciddi hatalardır.
    // O anki görevi sonlandırmak veya kernel panik etmek gerekir.
    // TODO: Karnal64 ktask::terminate_current_task() veya kernel_panic() çağrısı yap
    loop {} // Panik veya terminal döngüsü
}

/// Donanım Kesmelerini (IRQ) işler.
/// Zamanlayıcı, disk, ağ kartı gibi donanımlardan gelen kesmeleri yönetir.
#[no_mangle] // Genellikle ayrı bir IRQ giriş noktası olur
pub extern "C" fn handle_irq(trap_frame: &mut TrapFrame) {
    // 1. Kesme Kaynağını Belirle
    // Kesme denetleyicisinden (PIC, APIC, GIC gibi) hangi kesmenin geldiğini oku.
    // TODO: Mimariye/Donanıma özel kesme denetleyicisi registerlarını oku.
     let irq_number = read_irq_controller_status();

    println!("IRQ Received!"); // Geçici mesaj
    // TODO: irq_number bilgisini yazdır

    // 2. Karnal64'ün IRQ Yönetim Modülüne Yönlendir
    // kresource veya ayrı bir kirq modülü, kayıtlı kesme işleyicilerini tutar.
    // TODO: kirq::dispatch_irq(irq_number, trap_frame); çağrısı yap

    // 3. Kesmeyi Onayla (Acknowledge)
    // Kesme denetleyicisine kesmenin işlendiğini bildir. Bu, aynı kesmenin tekrar tetiklenmesini önler.
    // TODO: Mimariye/Donanıma özel kesme denetleyicisini onayla.
     write_irq_controller_eoi(irq_number);

    // Not: Zamanlayıcı kesmeleri burada yakalanır ve ktask::schedule() çağrısı yapılarak
    // bağlam değiştirmeye neden olabilir. Bu durumda handle_irq'dan dönüş, istisnaya
    // uğrayan görev yerine başka bir göreve olacaktır (trap_frame güncellenerek).
}

// --- Geliştirme/Hata Ayıklama Yardımcıları ---

// Kernel içinde print! makrosu gerektirir (genellikle konsol kaynağını kullanarak implemente edilir)
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ({
        // TODO: Konsol kaynağına yazma mantığı
         let console_handle = karnal64::kresource::get_console_handle(); // Örnek
         let message = format!($($arg)*);
         karnal64::kresource::write(console_handle.0, message.as_ptr(), message.len());
         // Şimdilik sadece yer tutucu
         unsafe {
             // Gerçek bir kernelde bu işe yaramaz veya tanımsız davranışa yol açar.
             // Sadece derleme hatası vermemesi için.
             core::ptr::write_volatile(0x1000_0000 as *mut u8, b'\n');
         }
    });
}
