#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz.
#![allow(dead_code)] // Geliştirme aşamasında bazı fonksiyonlar doğrudan çağrılmayabilir.
#![allow(unused_variables)] // Benzer şekilde kullanılmayan argümanlar olabilir.

// Rust'ın panik işleyicisi için gerekli trait
use core::panic::PanicInfo;
// x86_64 mimarisine özel intrinsics (CLI, HLT, I/O port erişimi)
use core::arch::x86_64;
// Konsola yazı yazmak için formatlama trait'i
use core::fmt::Write;

// Karnal64'den kullanılabilecek (veya panik-safe versiyonuna ihtiyaç duyulacak) tipler
// Bunları doğrudan import ediyoruz ama panik handler içinde kullanımları güvenlik gerektirir.
 use crate::karnal64::{KError, KHandle, KTaskId}; // Karnal64.rs'den import edilecekler

// --- Panik Anı Çıktısı İçin Temel Seri Port Yazıcısı (Mimariden Bağımsız Olmayan Kısım) ---
// Panik anında, kernelin daha yüksek seviye kaynak yöneticisi (kresource) veya sürücüleri
// tutarsız bir durumda olabilir. Bu yüzden, panik çıktısı için en temel, doğrudan donanım
// erişimi sağlayan bir yöntem tercih edilir. Burada COM1 seri portuna doğrudan yazma kullanılıyor.
// Bu, Karnal64'ün ResourceProvider traitini implement eden bir konsol sürücüsünden DAHA GÜVENLİDİR
// panik sırasında.

// Standart COM1 port adresleri
const COM1_PORT: u16 = 0x3F8;

// Panik çıktısı için seri porta yazan basit bir yapı
struct PanicSerialWriter;

impl PanicSerialWriter {
    // Seri porta tek bir byte gönderir.
    // GÜVENLİK NOTU: Bu fonksiyon, doğrudan I/O port erişimi yaptığı için güvensizdir (`unsafe`).
    // Sadece panik işleyicisi gibi kontrollü ve kritik ortamlarda kullanılmalıdır.
    #[inline(always)] // Küçük ve kritik olduğu için inline yapabiliriz.
    unsafe fn send(&mut self, byte: u8) {
        // Vericinin boşalmasını bekle (Transmit Holding Register Empty)
        // Status Port (COM1_PORT + 5), Bit 5 (0x20) THR Boş ise 1'dir.
        while x86_64::inb(COM1_PORT + 5) & 0x20 == 0 {
            // Çok sık kontrol etmemek için kısa bir duraklama eklenebilir
            // veya bu while döngüsü çok hızlı olacağı için gerek olmayabilir.
        }
        // Byte'ı Data Port'a gönder
        x86_64::outb(COM1_PORT, byte);
    }
}

// `core::fmt::Write` traitini implement ederek `write!` makrosunu kullanabilmeyi sağlarız.
impl Write for PanicSerialWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            // Linux/Unix tarzı \n yeni satır karakterini, Windows tarzı \r\n'e çevir.
            // Bu, çoğu seri terminalde çıktının doğru görünmesini sağlar.
            match byte {
                b'\n' => unsafe {
                    self.send(b'\r'); // Carriage Return
                    self.send(b'\n'); // Line Feed
                }
                _ => unsafe {
                    self.send(byte);
                }
            }
        }
        Ok(()) // Hata yönetimi panik handlerında genellikle basittir, yazma hatasını yok sayabiliriz.
    }
}

// Panik sırasında kolayca seri porta yazı yazmak için makro
macro_rules! panic_print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let mut writer = PanicSerialWriter;
        // write! hata dönebilir, panik sırasında bunu ele almak yerine yoksayıyoruz.
        let _ = write!(writer, $($arg)*);
    });
}

// Panik sırasında yeni satır yazmak için makro
macro_rules! panic_println {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        let mut writer = PanicSerialWriter;
        let _ = writeln!(writer, $($arg)*);
    });
}


// --- CPU Durumu Dökümü (Mimariden Bağımsız Olmayan Kısım - x86_64) ---
// Panik anında hata ayıklamaya yardımcı olması için temel CPU registerlarını (RIP, RSP)
// okumaya çalışalım. Bu, in-line assembly gerektirir ve panik'in tam olarak nerede olduğuna
// bağlı olarak hassas olabilir.

// Bu fonksiyonun inlining'ini önleyerek RIP değerini okuma denemesini biraz daha güvenilir yapabiliriz.
#[inline(never)]
unsafe fn get_basic_cpu_state() -> (u64, u64) {
    let rip: u64; // Instruction Pointer - Hatanın olduğu yer civarı
    let rsp: u64; // Stack Pointer - Mevcut yığıtın tepesi

    // In-line assembly kullanarak RSP ve RIP değerlerini al.
    // RIP okuma şekli, 'lea [rip]' kullanarak mevcut komutun adresini almaktır.
    // Bu, çağrı noktasından sonraki komutun adresini verir, panik yerine yakın bir değer sağlar.
    core::arch::asm!(
        "mov {0}, rsp",     // RSP değerini reg0'a taşı
        "lea {1}, [rip]",   // RIP değerini (bu komuttan sonraki adres) reg1'e taşı
        out(reg) rsp,       // reg0 RSP'ye eşlenecek
        out(reg) rip,       // reg1 RIP'ye eşlenecek
        options(nostack, nomem) // Bu assembly kodu yığıt veya bellek kullanmaz
    );
    (rip, rsp)
}


// --- Çekirdek Panik İşleyicisi ---

// Bu fonksiyon, Rust çalışma zamanı bir panik tespit ettiğinde çağrılır.
// `!` dönüş tipi, bu fonksiyonun asla geri dönmeyeceğini belirtir.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 1. Tüm kesmeleri (interrupts) anında devre dışı bırak.
    // Bu, panik sırasında daha fazla kesme veya yarış durumu oluşmasını engeller.
    unsafe { x86_64::cli(); }

    // 2. Panik mesajını ve detaylarını panik-safe konsol çıktısına yaz.
    panic_println!("\n--- KERNEL PANIC ---");

    // Panik'in olduğu dosya, satır ve sütun bilgisini yazdır.
    if let Some(location) = info.location() {
        panic_println!("Location: {}:{}:{}", location.file(), location.line(), location.column());
    } else {
        panic_println!("Location information unavailable.");
    }

    // Panik mesajını (varsa) yazdır.
    if let Some(message) = info.message() {
        panic_println!("Message: {}", message);
    } else {
        panic_println!("No panic message provided.");
    }

    // Panik nedeni (Payload) hakkında bilgi yazdır (debug buildlerde daha faydalı olabilir).
     if let Some(payload) = info.payload().downcast_ref::<&str>() {
         panic_println!("Payload: {}", payload);
     }

    // 3. Temel CPU durumunu (RIP, RSP) yazdırmaya çalış.
    unsafe {
        let (rip, rsp) = get_basic_cpu_state();
        panic_println!("Basic CPU State:");
        panic_println!("  RIP: {:#x}", rip); // Hexadecimal format
        panic_println!("  RSP: {:#x}", rsp);
        // Daha fazla register (RAX, RBX, RCX, RDX, RBP, RSI, RDI, R8-R15, segment yazmaçları, bayraklar, CR3 vb.)
        // yazdırmak mümkündür ancak assembly kodu daha karmaşıklaşır.
        // Gerçek bir kernelde, istisna işleyicileri panik öncesinde bir Register Dökümünü kaydetmelidir.
    }

    // 4. Karnal64 API'si ile İlgili Olası Bilgiler (Eğer Panik-Safe İse)
    // NOT: Panik sırasında ktask veya kresource gibi daha karmaşık sistemlere
    // güvenmek genellikle ÇOK RİSKLİDİR, çünkü panik bu sistemlerin tutarsız
    // duruma gelmesinden kaynaklanmış olabilir. Aşağıdaki kısımlar SADECE panik-safe
    // olduğu garanti edilen (çok dikkatli implement edilmiş) alt sistemler için
    // KAVRAMSAL olarak buraya eklenebilir.

    // Örnek: Panik olan görevin ID'sini almaya çalışmak (Eğer ktask::get_current_task_id panik-safe ise)
     use crate::karnal64::ktask; // ktask modülünü yukarıda import etmeniz gerekir.
     if let Some(task_id) = ktask::get_current_task_id_panic_safe() { // Hipotetik panik-safe fonksiyon
        panic_println!("Panicking Task ID: {}", task_id.0);
     }

    // Örnek: Sistemdeki diğer görevlerin durumunu dökmeye çalışmak (Eğer ktask panik-safe dump fonksiyonu sağlarsa)
     if ktask::is_panic_safe_dump_available() { // Hipotetik kontrol
         panic_println!("Dumping state of other tasks...");
         ktask::dump_all_task_states_panic_safe(); // Hipotetik fonksiyon
     }
    
    // Panik sırasında kresource::resource_write gibi API çağrıları DAHA AZ güvenlidir
    // çünkü handle tablosu veya alttaki sürücü tutarsız olabilir. Bu yüzden
    // panik çıktısı için doğrudan seri port yazıcı tercih edildi.

    panic_println!("--- END OF KERNEL PANIC ---");

    // 5. Sistemi durdur veya sonsuz döngüye sok.
    // Hata ayıklama için genellikle sonsuz döngü tercih edilir.
    // `hlt` (halt) komutu, sistemin duraklamasını sağlar ve bir kesme bekler (kesmeler kapalı olduğu için gelmeyecektir).
    // Bu, CPU'yu boş yere döndürmekten (spin) daha enerji verimlidir.
    loop {
        unsafe {
            x86_64::_hlt(); // CPU'yu duraklat
        }
    }
}