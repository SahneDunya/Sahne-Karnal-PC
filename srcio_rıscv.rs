#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz, çekirdek alanında çalışırız
#![allow(dead_code)] // Geliştirme sırasında kullanılmayan kodlara izin ver

// Karnal64 API'sından gerekli tipleri ve fonksiyonları içeri aktaralım.
// handle_syscall fonksiyonu, raw sistem çağrısı argümanlarını alıp genel API'ye iletecektir.
use karnal64::{KError, KHandle, handle_syscall};

// RISC-V mimarisine özel tanımlar (gerçek implementasyonlar ayrı modüllerde olabilir)
// Örneğin, CSR (Control and Status Register) erişimi için fonksiyonlar veya register tanımları

/// Bir trap (kesme veya istisna) meydana geldiğinde görev bağlamının kaydedildiği yapı.
/// Bu yapı, RISC-V ABI'sına ve trap işleme kurulumuna göre değişir.
/// Burası sistem çağrısı argümanları ve dönüş değeri için ilgili registerları içerir.
/// Tam bir TrapFrame tüm 32/64 genel amaçlı registerı içermelidir.
#[repr(C)] // Assembly kodu ile etkileşim için C uyumlu bellek düzeni sağlar
pub struct TrapFrame {
    // x0 (zero register) kaydedilmez
    // x1-x9: Ra, Sp, Gp, Tp, T0-T2 (Kaydedilmeli)
    pub ra: u64, // x1
    pub sp: u64, // x2
    pub gp: u64, // x3
    pub tp: u64, // x4
    pub t0: u64, // x5
    pub t1: u64, // x6
    pub t2: u64, // x7

    // x10-x17: A0-A7 (Argüman/Dönüş Değeri ve Sistem Çağrısı Numarası - Kaydedilmeli)
    pub a0: u64, // x10 - Argüman 0 / Dönüş Değeri
    pub a1: u64, // x11 - Argüman 1
    pub a2: u64, // x12 - Argüman 2
    pub a3: u64, // x13 - Argüman 3
    pub a4: u64, // x14 - Argüman 4
    pub a5: u64, // x15 - Argüman 5
    pub a6: u64, // x16 - Argüman 6
    pub a7: u64, // x17 - Sistem Çağrısı Numarası

    // x18-x27: S2-S11 (Kaydedilmeli)
    pub s2: u64, // x18
    pub s3: u64, // x19
    pub s4: u64, // x20
    pub s5: u64, // x21
    pub s6: u64, // x22
    pub s7: u64, // x23
    pub s8: u64, // x24
    pub s9: u64, // x25
    pub s10: u64, // x26
    pub s11: u64, // x27

    // x28-x31: T3-T6 (Kaydedilmeli)
    pub t3: u64, // x28
    pub t4: u64, // x29
    pub t5: u64, // x30
    pub t6: u64, // x31

    // Kontrol ve Durum Registerları (CSRs) - Trap sırasında kaydedilmeli
    pub sepc: u64,  // Supervisor Exception Program Counter (Trap'e neden olan komutun adresi)
    pub sstatus: u64, // Supervisor Status Register
    pub scause: u64, // Trap'in Nedeni
    pub stval: u64, // Trap Value (örn: geçersiz adres)
    // Diğer CSR'lar da duruma göre kaydedilebilir (sscratch, satp vb.)
}

// RISC-V Trap Nedenleri (scause register değerleri)
const CAUSE_INTERRUPT_SUPERVISOR_TIMER: u64 = 0x8000000000000005; // Supervisor Timer Interrupt
const CAUSE_ECALL_U_MODE: u64 = 8; // User mode'dan Environment Call (Sistem Çağrısı)
const CAUSE_ECALL_S_MODE: u64 = 9; // Supervisor mode'dan Environment Call (Sistem Çağrısı)
const CAUSE_PAGE_FAULT_LOAD: u64 = 13; // Bellekten okuma sırasında sayfa hatası
const CAUSE_PAGE_FAULT_STORE: u64 = 15; // Belleğe yazma sırasında sayfa hatası
// ... diğer nedenler (illegal instruction, breakpoint vb.)

/// Bir kullanıcı alanı pointer'ının geçerli ve izinlere sahip olup olmadığını doğrular.
/// Bu, kritik bir güvenlik fonksiyonudur ve GERÇEK MMU (Bellek Yönetim Birimi)
/// ve sayfa tablosu etkileşimi gerektirir.
/// DİKKAT: Buradaki implementasyon bir yer tutucudur ve GÜVENSİZDİR.
/// Validasyon, mevcut görev/iş parçacığının sanal adres alanına göre yapılmalıdır.
unsafe fn validate_user_pointer(ptr: *const u8, len: usize, writeable: bool) -> Result<(), KError> {
    // TODO: GERÇEK MMU TABANLI DOĞRULAMAYI BURAYA EKLEYİN!
    // 1. ptr + len değerinin taşma yapıp yapmadığını kontrol edin.
    // 2. [ptr, ptr + len) adres aralığının mevcut görev'in sanal adres alanında MAP'li olup olmadığını kontrol edin.
    // 3. MAP'li bellek bölgesinin READ iznine sahip olup olmadığını kontrol edin.
    // 4. Eğer `writeable` true ise, WRITE iznine de sahip olup olmadığını kontrol edin.
    // 5. len > 0 iken ptr'nin null olmadığından emin olun.

    // Bu yer tutucu implementasyon, herhangi bir gerçek doğrulama yapmaz.
    // Sadece null pointer ve sıfır uzunluk durumu gibi temel hataları kontrol eder.
    if ptr.is_null() && len > 0 {
         println!("ERROR: validate_user_pointer received null pointer with non-zero length"); // Çekirdek içi print! gerektirir
        return Err(KError::BadAddress);
    }

    // GERÇEK KERNEL'DE BU KISIM MMU SORGUSU YAPMALIDIR:
     if !current_task_mmu.is_valid_and_accessible(ptr, len, read_permission=true, write_permission) {
        return Err(KError::BadAddress);
     }

    // Yer tutucu başarılı varsayım (GÜVENSİZ!)
    Ok(())
}


/// RISC-V mimarisi için ana trap işleyici giriş noktası.
/// Düşük seviyeli assembly trap vektörü tarafından çağrılır.
/// Kaydedilmiş registerları içeren TrapFrame'i alır.
#[no_mangle] // Assembly kodu tarafından çağrılabilmesi için isim düzenlemesi yapılmaz
pub extern "C" fn riscv_trap_handler(trap_frame: *mut TrapFrame) {
    // Güvenlik: Assembly kodunun geçerli bir trap_frame pointer'ı sağladığına güvenmeliyiz.
    // Ham pointer'a erişim unsafe blok içinde.
    unsafe {
        let tf = &mut *trap_frame;
        let cause = tf.scause;

        // Trap nedenini kontrol et
        match cause {
            CAUSE_ECALL_U_MODE | CAUSE_ECALL_S_MODE => {
                // --- Sistem Çağrısı (ECALL) İşleme ---

                // Sistem çağrısı numarasını ve argümanları TrapFrame'den al
                // RISC-V ABI'sına göre:
                // syscall numarası: a7 (x17)
                // argümanlar: a0-a5 (x10-x15)
                // dönüş değeri: a0 (x10)
                let syscall_num = tf.a7;
                let arg1 = tf.a0;
                let arg2 = tf.a1;
                let arg3 = tf.a2;
                let arg4 = tf.a3;
                let arg5 = tf.a4;
                // Karnal64'ün handle_syscall'ı 5 argüman alıyor, RISC-V'nin ilk 5 argümanını kullanıyoruz (a0-a4).
                // Bu eşleşme, kullanıcı alanındaki sistem çağrısı stubları ile tutarlı olmalıdır.

                // --- Güvenlik: Kullanıcı Pointer Doğrulama ---
                // Karnal64 API fonksiyonlarına kullanıcı alanından gelen pointer'lar geçirilmeden ÖNCE,
                // bu pointer'ların geçerli ve erişilebilir olduğundan emin olmalıyız.
                // Bu, `validate_user_pointer` fonksiyonu ile yapılır.
                // Hangi argümanların pointer olduğu ve hangi izinlere ihtiyaç duyulduğu, sistem çağrısı numarasına bağlıdır.
                // Gerçek bir trap işleyici, syscall numarasına göre pointer argümanlarını bilmeli ve buna göre doğrulamalıdır.

                let syscall_result: i64;

                // Burada her sistem çağrısı için pointer argümanlarının doğrulama gereksinimleri
                // tek tek veya bir lookup tablosu aracılığıyla ele alınmalıdır.
                // Örnek olarak RESOURCE_READ ve RESOURCE_WRITE için pointer doğrulama ekleyelim:
                match syscall_num {
                     SYSCALL_RESOURCE_READ = 6 ( handle, buffer_ptr, buffer_len, ... )
                    6 => {
                        let user_buffer_ptr = arg2 as *mut u8; // arg2 user buffer pointer'ı
                        let user_buffer_len = arg3 as usize; // arg3 buffer uzunluğu
                        // RESOURCE_READ durumunda çekirdek, kullanıcı tamponuna yazacağı için buffer'ın YAZILABİLİR olduğunu doğrula.
                        if let Err(err) = validate_user_pointer(user_buffer_ptr as *const u8, user_buffer_len, true) {
                            syscall_result = err as i64; // Doğrulama başarısızsa hata döndür
                        } else {
                            // Doğrulama başarılı, Karnal64 API'sini çağır
                            syscall_result = handle_syscall(syscall_num, arg1, arg2, arg3, arg4, arg5);
                        }
                    }
                     SYSCALL_RESOURCE_WRITE = 7 ( handle, buffer_ptr, buffer_len, ... )
                    7 => {
                        let user_buffer_ptr = arg2 as *const u8; // arg2 user buffer pointer'ı
                        let user_buffer_len = arg3 as usize; // arg3 buffer uzunluğu
                        // RESOURCE_WRITE durumunda çekirdek, kullanıcı tamponundan okuyacağı için buffer'ın OKUNABİLİR olduğunu doğrula.
                        if let Err(err) = validate_user_pointer(user_buffer_ptr, user_buffer_len, false) {
                            syscall_result = err as i64; // Doğrulama başarısızsa hata döndür
                        } else {
                            // Doğrulama başarılı, Karnal64 API'sini çağır
                            syscall_result = handle_syscall(syscall_num, arg1, arg2, arg3, arg4, arg5);
                        }
                    }
                    // TODO: Diğer pointer alan sistem çağrıları için de benzer doğrulama mantığı ekleyin:
                     SYSCALL_TASK_SPAWN (arg2: args_ptr) -> OKUNABİLİR
                     SYSCALL_MESSAGE_SEND (arg2: message_ptr) -> OKUNABİLİR
                     SYSCALL_MESSAGE_RECEIVE (arg1: buffer_ptr) -> YAZILABİLİR
                     SYSCALL_RESOURCE_ACQUIRE (arg1: resource_id_ptr) -> OKUNABİLİR

                    _ => {
                        // Pointer argümanı almayan veya henüz doğrulaması eklenmemiş sistem çağrıları için
                        // doğrudan Karnal64 API'sini çağır.
                        // UYARI: Eğer bu syscall'lar validate edilmemiş pointer alıyorsa bu GÜVENSİZDİR.
                        syscall_result = handle_syscall(syscall_num, arg1, arg2, arg3, arg4, arg5);
                    }
                }

                // Karnal64 API'sinden dönen sonucu (i64) dönüş değeri registerına (a0) yaz
                tf.a0 = syscall_result as u64; // i64'ü u64'e dönüştür (negatifler de doğru temsil edilmeli)

                // sepc (Exception Program Counter) registerını sistem çağrısı komutundan sonrasına taşı
                // RISC-V'de ECALL komutu genellikle 4 byte uzunluğundadır.
                tf.sepc += 4;
            }
            CAUSE_INTERRUPT_SUPERVISOR_TIMER => {
                 // Timer Kesmesini İşle
                 // TODO: Timer kesme işleyicisini çağır
                 // Muhtemelen zamanlayıcıyı resetle ve zamanlayıcı (scheduler) mantığını tetikle
                  println!("Timer Interrupt!"); // Çekirdek içi print! gerektirir
                 // Zamanlayıcı kesmesini işledikten sonra sbi_rt::time::clear_sip() gibi bir çağrı gerekebilir
                 // Trap dönüşünde sepc'yi ilerletmeye gerek yok, interrupted komut tekrar çalışacak.
            }
            CAUSE_PAGE_FAULT_LOAD | CAUSE_PAGE_FAULT_STORE => {
                // Sayfa Hatasını İşle
                // TODO: Bellek yöneticisini (kmemory) çağırarak sayfa hatasını çözmeye çalış
                // tf.stval geçersiz adresi içerir. scause hatanın tipini (okuma/yazma) belirtir.
                // Eğer çözülemezse, hataya neden olan görevi sonlandır.
                 println!("Page Fault at {:x}, Cause: {:x}", tf.stval, cause); // Çekirdek içi print!
                 tf.a0 = KError::BadAddress as i64 as u64; // Hata kodu döndür
                 tf.sepc += 4; // sepc'yi ilerlet (Görev sonlandırılacaksa buna gerek kalmaz)
            }
            // TODO: Diğer interrupt ve exception nedenlerini (Illegal Instruction, Breakpoint vb.) ele al

            _ => {
                // Bilinmeyen veya işlenmemiş trap nedeni
                // Hata mesajı yazdır ve hataya neden olan görevi sonlandır veya sistemi durdur
                 println!("Unhandled Trap! Cause: {:x}, SEPC: {:x}, STVAL: {:x}", cause, tf.sepc, tf.stval); // Çekirdek içi print!
                 tf.a0 = KError::InternalError as i64 as u64; // Genel hata kodu döndür
                 tf.sepc += 4; // sepc'yi ilerlet (Görev sonlandırılacaksa buna gerek kalmaz)
                 // TODO: Görevi sonlandır veya panic yap
            }
        }
        // İşlemeden sonra, kontrol assembly trap dönüş koduna geri döner.
        // Assembly kodu kaydedilmiş registerları TrapFrame'den yükler ve `sret` komutu ile
        // tuzaktan önceki bağlama (genellikle kullanıcı moduna) geri döner.
    }
}

// TODO: RISC-V özelindeki diğer düşük seviye I/O veya donanım etkileşim kodlarını buraya ekleyin.
// Örneğin:
// - UART (seri port) sürücüsü implementasyonu (çekirdek içi print! veya konsol kaynağı için)
// - Zamanlayıcı (timer) kurulumu ve kesme işleyicisi bağlantısı
// - MMU (paging) kurulumu ve yönetimi fonksiyonları (kmemory modülü tarafından kullanılır)
// - Kesme denetleyicisi (PLIC veya CLINT) yönetimi

// Örnek olarak bir dummy çekirdek içi print fonksiyonu (geliştirme için faydalı)

#[cfg(feature = "enable_kernel_debug_print")]
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ($crate::riscv_uart::print_fmt(format_args!($($arg)*)));
}

// src/riscv_uart.rs içinde implemente edilecek
 pub fn print_fmt(args: core::fmt::Arguments);
