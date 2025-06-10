#![no_std] // Standart kütüphane yok
#![feature(abi_x86_interrupt)] // x86 kesme ABI'sini kullanmak için

// Kullanılacak dış crate'ler ve modüller
extern crate x86_64; // x86_64 mimarisine özel fonksiyonlar ve yapılar için

// Karnal64 API'sından gerekli tipler ve fonksiyonlar
// Varsayım: karnal64 modülü aynı dizinde veya bağımlılıklarda var.
use karnal64::{KError, KHandle}; // İhtiyaca göre diğer tipler de eklenebilir
use karnal64::handle_syscall; // Sistem çağrısı dağıtıcısı
use karnal64::init as karnal_init; // Karnal64 başlatma fonksiyonu

// --- Sabitler ---
// Sistem çağrısı için kullanılacak kesme numarası (örnek olarak 0x80)
const SYSCALL_INT_VECTOR: u8 = 0x80;

// --- Global Durum ---
// Kesme Yönlendirme Tablosu (IDT)
// 'static olarak tanımlanmalı ki ömrü tüm kernel boyunca sürsün
static mut IDT: x86_64::structures::idt::InterruptDescriptorTable =
    x86_64::structures::idt::InterruptDescriptorTable::new();

// Temel bir spinlock ile kernel çıktısını senkronize etmek için (isteğe bağlı ama iyi uygulama)
 extern crate spin; // Spinlock crate'ini eklemeniz gerekebilir
 use spin::Mutex;
 static WRITER: Mutex<Option<SerialPort>> = Mutex::new(None); // Örnek Seri Port çıktısı

// --- Kernel Giriş Noktası ---
// Önyükleyici (bootloader) tarafından çağrılan ana fonksiyon
// Genellikle 'C' ABI'si kullanılır ve isim düzenlemesi yapılmaz.
#[no_mangle]
pub extern "C" fn kernel_main(boot_info: *const ()) -> ! { // 'boot_info' önyükleyiciye özel olabilir
    // --- 1. Başlangıç Güvenlik ve Kurulum ---
    // Donanım kesmelerini devre dışı bırak (kurulum sırasında rahatsız edilmemek için)
    x86_64::instructions::interrupts::disable();

    // TODO: Temel bellek yönetimi kurulumu
    // - Boot_info'dan bellek haritasını oku
    // - Fiziksel bellek ayırıcısını başlat
    // - Kernel kod/veri segmentleri için sayfa tablolarını kur/doğrula
    // - Heap için bellek ayır ve haritala (eğer kullanılacaksa)
     kmemory::init_early(...); // Yer tutucu fonksiyon çağrısı

    // TODO: GDT (Global Descriptor Table) kurulumu (eğer önyükleyici tarafından yapılmadıysa veya yenileniyorsa)
     x86_64::structures::gdt::Gdt::new(); // Yer tutucu

    // --- 2. Kesme Sistemi ve Hata İşleyicileri Kurulumu ---
    init_idt(); // IDT'yi kur

    // TODO: PIC (Programmable Interrupt Controller) veya APIC (Advanced PIC) başlatma
    // Eğer PIC kullanılıyorsa, kesmeleri yeniden eşle (remap) ve devre dışı bırak/maskele
    // Eğer APIC kullanılıyorsa, başlat ve yapılandır.
     x86_64::instructions::port::Port::new(0x20).write(0x11); // Örnek PIC komutu (detaylı implementasyon gerekli)

    // Seri port gibi temel cihazları başlat (eğer çıktı için kullanılacaksa)
     init_serial_port(); // Yer tutucu fonksiyon çağrısı
     WRITER.lock().replace(SerialPort::new(0x3F8)); // Örnek: COM1 portu

    println!("Karnal64 çekirdeği başlatılıyor..."); // Kernel log çıktısı

    // --- 3. Karnal64 API'sını Başlat ---
    karnal_init();

    println!("Karnal64 API'sı başlatıldı.");

    // --- 4. Donanım Kesmelerini Yeniden Etkinleştir (Opsiyonel ama genellikle gerekli) ---
    // IDT ve PIC/APIC kurulduktan sonra kesmeleri tekrar açabiliriz.
    x86_64::instructions::interrupts::enable();

    println!("Donanım kesmeleri etkinleştirildi.");

    // --- 5. İlk Görevi Başlat veya Boşta Döngüye Gir ---
    // TODO: İlk kullanıcı alanı görevinin yüklenmesi ve başlatılması
    // Bu genellikle bir dosya sisteminden (kaynak handle'ı ile) çalıştırılabilir bir dosyanın
    // okunması, yeni bir adres alanı oluşturulması, kodun oraya kopyalanması ve
    // zamanlayıcıya eklenmesi adımlarını içerir.
     ktask::spawn_initial_task("karnal://disk/app/init").expect("Failed to spawn init task"); // Yer tutucu

    // Eğer görev başlatma mantığı yoksa veya tüm görevler bittiyse
    // Çekirdek burada boşta döngüye girebilir.
    println!("Çekirdek boşta döngüye giriyor.");
    loop {
        x86_64::instructions::hlt(); // CPU'yu düşük güç moduna al, kesme gelene kadar bekle
    }
}

// --- Kesme Yönlendirme Tablosu (IDT) Kurulumu ---
fn init_idt() {
    unsafe {
        // Hata İşleyicileri (Exceptions)
        IDT.divide_by_zero.set_handler_fn(divide_by_zero_handler);
        IDT.breakpoint.set_handler_fn(breakpoint_handler);
        IDT.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        IDT.page_fault.set_handler_fn(page_fault_handler);
        // TODO: Diğer önemli hata işleyicilerini ekle (stack segment fault, double fault vb.)

        // Donanım Kesmeleri (Interrupts) - PIC veya APIC tarafından tetiklenenler
        // Varsayılan olarak PIC kesmeleri 32-47 arasına maplenir.
         IDT[32] -= Timer, IDT[33] -= Klavye vb.
        // TODO: PIC/APIC'e göre donanım kesme işleyicilerini ayarla
         IDT[32].set_handler_fn(timer_interrupt_handler);
         IDT[33].set_handler_fn(keyboard_interrupt_handler);

        // Sistem Çağrısı İşleyicisi
        // Kullanıcı alanından SYSCALL_INT_VECTOR (0x80) kesmesi geldiğinde çalışacak işleyici
        IDT[SYSCALL_INT_VECTOR as usize].set_handler_fn(syscall_interrupt_handler)
            // Sistem çağrısı işleyicisi kullanıcı modundan çağrılacağı için DPL (Descriptor Privilege Level) 3 olmalı
            .set_present(true)
            .set_privilege_level(x86_64::PrivilegeLevel::Ring3);

        // IDT'yi yüklüyoruz
        IDT.load();
    }
    println!("IDT kuruldu.");
}

// --- Hata İşleyici Fonksiyonları (Örnekler) ---
// Hata işleyicileri 'x86_interrupt' ABI'sini kullanmalıdır.
// Genellikle hata türüne göre ek bir 'error_code' argümanı alırlar.

extern "x86-interrupt" fn divide_by_zero_handler(
    stack_frame: x86_64::structures::idt::InterruptStackFrame,
) {
    println!("PANIC: Divide by zero!");
    println!("Stack Frame: {:?}", stack_frame);
    loop { x86_64::instructions::hlt(); } // Kurtarılamaz hata, CPU'yu durdur
}

extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: x86_64::structures::idt::InterruptStackFrame,
) {
    println!("DEBUG: Breakpoint hit!");
    println!("Stack Frame: {:?}", stack_frame);
    // Hata sonrası devam etmek mümkün olabilir, iretq ile döner.
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: x86_64::structures::idt::InterruptStackFrame,
    error_code: u64,
) {
    println!("PANIC: General Protection Fault!");
    println!("Error Code: {}", error_code);
    println!("Stack Frame: {:?}", stack_frame);
    loop { x86_64::instructions::hlt(); }
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: x86_64::structures::idt::InterruptStackFrame,
    error_code: x86_64::structures::idt::PageFaultErrorCode,
) {
    println!("PANIC: Page Fault!");
    println!("Access Address: {:?}", x86_64::registers::control::Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("Stack Frame: {:?}", stack_frame);
    // TODO: Bellek yöneticisine danışarak sayfayı haritalama veya hatayı işleme
    loop { x86_64::instructions::hlt(); }
}

// --- Sistem Çağrısı İşleyici (Rust Wrapper) ---
// IDT'deki SYSCALL_INT_VECTOR girişi bu fonksiyona yönlendirilir.
// x86_interrupt ABI'si, kesme geldikten sonra CPU tarafından
// otomatik olarak stack'e yazılan InterruptStackFrame'i argüman olarak verir.
// Sistem çağrısı argümanları ise çağıran görevin (user space) register'larında bulunur.
// Bu fonksiyonun görevi, stack frame ve register durumundan argümanları çıkarıp
// Karnal64'ün `handle_syscall` fonksiyonunu çağırmaktır.

extern "x86-interrupt" fn syscall_interrupt_handler(
    stack_frame: x86_64::structures::idt::InterruptStackFrame,
) {
    // Güvenlik Notu: Gerçek bir implementasyonda, stack frame içindeki register değerlerinin
    // (örn. Instruction Pointer) ve kullanıcı tarafından sağlanan pointer argümanlarının
    // (örn. syscall_number, arg1..arg5) geçerli ve izin verilen bellek alanlarını gösterdiği
    // titizlikle doğrulanmalıdır. Bu örnekte bu doğrulama basitleştirilmiştir.

    // System V AMD64 ABI'ye göre registerlardaki varsayılan sistem çağrısı argümanları:
    // RAX: Sistem çağrısı numarası
    // RDI: Argüman 1
    // RSI: Argüman 2
    // RDX: Argüman 3
    // R10: Argüman 4 (RCX yerine R10 kullanılır syscall ABI'de)
    // R8:  Argüman 5
    // R9:  Argüman 6 (Eğer 6. argüman varsa, Karnal64'te 5 argüman tanımlı)

    let syscall_number = stack_frame.preserved_registers.rax;
    let arg1 = stack_frame.argument_registers.rdi;
    let arg2 = stack_frame.argument_registers.rsi;
    let arg3 = stack_frame.argument_registers.rdx;
    let arg4 = stack_frame.argument_registers.r10; // SYSCALL ABI'ye göre R10
    let arg5 = stack_frame.argument_registers.r8;  // SYSCALL ABI'ye göre R8

    // TODO: Kullanıcı alanı register'larından alınan pointer argümanlarının
    // (örn. resource_acquire'daki id_ptr, read/write'daki buffer_ptr)
    // gerçekten kullanıcının adres alanında geçerli, erişilebilir ve doğru izinlere sahip
    // olduğunu doğrula. Bu genellikle sanal bellek yöneticisi fonksiyonları ile yapılır.
    // Geçersiz ise KError::BadAddress veya KError::PermissionDenied döndürülmelidir.

    // Karnal64'ün ana sistem çağrısı dağıtıcısını çağır
    let result = handle_syscall(syscall_number, arg1, arg2, arg3, arg4, arg5);

    // Sonucu, sistem çağrısını çağıran kullanıcı alanındaki görevin RAX register'ına yazıyoruz.
    // x86-interrupt ABI'si, stack frame üzerinde değişiklik yapılmasına ve
    // iretq talimatı ile geri dönüldüğünde bu değişikliklerin uygulanmasına olanak tanır.
    // Başarı durumunda pozitif/sıfır, hata durumunda negatif i64 dönülür.
    // TODO: handle_syscall i64 döndürüyor. stack_frame.preserved_registers.rax u64.
    // i64'ten u64'e dönüşümde işaret bitine dikkat etmek gerekebilir veya
    // stack_frame yapısı i64 yazmaya izin verecek şekilde ayarlanmalıdır.
    // Varsayım: i64 değeri doğrudan u64'e dönüştürülerek RAX'e yazılır.
    stack_frame.preserved_registers.rax = result as u64;


    // Kesmeden başarıyla döndüğümüzde, CPU iretq talimatını çalıştırır (ABI tarafından halledilir)
    // ve kullanıcı alanına, RAX'te sistem çağrısı sonucu ile geri döner.
    println!("Sistem çağrısı {} işlendi, sonuç: {}", syscall_number, result); // Debug çıktısı
}

// --- Donanım Kesme İşleyicileri (Örnekler) ---
// TODO: PIC/APIC başlatıldıktan sonra ilgili kesmeler için bu fonksiyonlar yazılmalı.
// Örneğin, bir timer kesmesi geldiğinde zamanlayıcıyı güncellemek gibi.

extern "x86-interrupt" fn timer_interrupt_handler(
    stack_frame: x86_64::structures::idt::InterruptStackFrame,
) {
    // TODO: Zamanlayıcı sayacını güncelle
    // TODO: Görev zamanlayıcıyı çalıştır (eğer zaman aşımı olduysa)
    // TODO: PIC veya APIC'e EOI (End of Interrupt) sinyali gönder (çok önemli!)
     println!("Timer interrupt"); // Debug çıktısı, sık olabilir dikkat!
    unsafe {
         // PIC EOI örneği (eğer PIC kullanılıyorsa)
          x86_64::instructions::port::Port::new(0x20).write(0x20);
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(
    stack_frame: x86_64::structures::idt::InterruptStackFrame,
) {
    // TODO: Klavye portundan (genellikle 0x60) scan code oku
    // TODO: Scan code'u bir klavye sürücüsüne ilet
    // TODO: PIC veya APIC'e EOI sinyali gönder
     println!("Keyboard interrupt"); // Debug çıktısı
    unsafe {
         // PIC EOI örneği
          x86_64::instructions::port::Port::new(0x20).write(0x20);
    }
}

// --- Basic Logging (Yer Tutucu) ---
// Kernel çıktısı için basit bir mekanizma. Gerçekte bir seri port veya VGA sürücüsü
// kullanılarak implemente edilir.
// `println!` makrosu için bir `_print` fonksiyonu sağlar.

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
}

// Kernel içinde format! kullanabilmek için alloc crate'ine veya özel bir formatlama implementasyonuna ihtiyacınız olabilir.
// Eğer `alloc` crate'ini kullanıyorsanız, bir global ayırıcı (global allocator) tanımlamanız gerekir.
// Alternatif olarak, basit string formatting implementasyonu yazabilirsiniz.
// Bu örnekte format_args! kullanılıp bir Yazıcı trait'ine gönderildiği varsayılır.

use core::fmt::{self, Write};

// Varsayım: Kernelin bir yerinde konsola yazan bir Writer implementasyonunuz var
 struct ConsoleWriter;
 impl fmt::Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
//        // TODO: Implement actual writing to serial/VGA text mode
        Ok(())
    }
 }
 static CONSOLE: Mutex<ConsoleWriter> = Mutex::new(ConsoleWriter);

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    // TODO: Buraya gerçek çıktı mekanizması implemente edilmeli
    // Örneğin seri porta yazmak veya VGA tamponuna yazmak gibi.
    // Kesme devre dışı veya spinlock kullanarak senkronizasyon sağlamak önemlidir.
     if let Some(ref mut serial) = *WRITER.lock() { // Örnek seri port kullanımı
         serial.write_fmt(args).expect("Serial print failed");
     } else {
        // Fallback: Eğer seri port yoksa veya init olmadıysa çok temel bir şey yapabilir
        // Örneğin VGA text mode'a doğrudan yazmak (çok dikkatli kullanılmalı!)
     }

    // Şimdilik sadece debug için qemu konsoluna yazdıralım (qemu ayarı gerektirir)
    // veya varsayılan seri port çıktısını kullanalım.
    // BU KISIM GERÇEK DONANIM İÇİN ÇEKİRDEK SÜRÜCÜSÜ YAZILMASINI GEREKTİRİR.
    // Aşağıdaki satır sadece bir placeholder'dır ve doğrudan çalışmayabilir.
     x86_64::instructions::port::Port::new(0x3F8).write(b' '); // Dummy yazma
     let mut writer = DummyKernelWriter;
     writer.write_fmt(args).expect("Dummy write failed");
}

// Debug amaçlı çok basit yer tutucu Writer
struct DummyKernelWriter;
impl fmt::Write for DummyKernelWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Bu kısım gerçekte seri porta veya ekrana byte yazmalıdır.
        // QEMU debug çıktısı için basit bir port yazma denemesi:
        // Port 0xe9 genellikle QEMU debug portudur.
        let mut port = x86_64::instructions::port::Port::new(0xe9);
        for byte in s.as_bytes() {
            unsafe { port.write(*byte); }
        }
        Ok(())
    }
}


// --- Panic İşleyici ---
// Kernelde kurtarılamaz bir hata oluştuğunda burası çağrılır.
// İmplementasyonu 'libcore' tarafından beklenir.
#[cfg(not(test))] // Testlerde kullanılmasın
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {}", info);
    // Hata sonrası sistemi durdur
    loop {
        x86_64::instructions::hlt(); // CPU'yu durdur
    }
}
