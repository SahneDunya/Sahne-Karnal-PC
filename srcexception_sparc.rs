#![no_std]

use core::arch::asm;
use core::fmt;
use core::panic::PanicInfo;

// İstisna Sebepleri (SPARC V8 mimarisi için - basitleştirilmiş liste)
#[repr(u32)] // Temsil için u32, gerçekte SPARC trap numaraları daha küçük olabilir
enum ExceptionCause {
    InstructionAccessException = 0x01, // Talimat erişim istisnası
    DataAccessException = 0x02,        // Veri erişim istisnası
    IllegalInstruction = 0x0A,          // Yasadışı talimat
    PrivilegedInstruction = 0x0B,       // İmtiyazlı talimat
    DivisionByZero = 0x24,             // Sıfıra bölme
    TrapInstruction = 0x28,            // TA (Trap Always) talimatı
    FloatingPointException = 0x30,      // Kayan nokta istisnası
    Reset = 0x80,                       // Reset (Sıfırlama)
    Unknown = 0xFFFF,                   // Bilinmeyen sebepler için
}

impl fmt::Display for ExceptionCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExceptionCause::InstructionAccessException => write!(f, "Talimat erişim istisnası"),
            ExceptionCause::DataAccessException => write!(f, "Veri erişim istisnası"),
            ExceptionCause::IllegalInstruction => write!(f, "Geçersiz komut"),
            ExceptionCause::PrivilegedInstruction => write!(f, "İmtiyazlı talimat"),
            ExceptionCause::DivisionByZero => write!(f, "Sıfıra bölme hatası"),
            ExceptionCause::TrapInstruction => write!(f, "TA (Trap Always) talimatı"),
            ExceptionCause::FloatingPointException => write!(f, "Kayan nokta istisnası"),
            ExceptionCause::Reset => write!(f, "Sıfırlama"),
            ExceptionCause::Unknown => write!(f, "Bilinmeyen sebep"),
        }
    }
}

// ExceptionContext yapısı (SPARC V8 için basitleştirilmiş)
#[derive(Debug)]
#[repr(C)]
pub struct ExceptionContext {
    pub pc: u32,        // Program Sayacı (İstisna anında)
    pub psr: u32,       // İşlemci Durum Kaydı (Processor State Register)
    pub y: u32,         // Y Kaydı
    pub wim: u32,       // Pencere İnme Maskesi (Window Invalidate Mask)
    pub tbr: u32,       // Tuzak Tabanı Kaydı (Trap Base Register)
    pub cause: u32,     // İstisna Sebebi (Trap Numarası veya Kodu)
    // ... diğer ilgili kayıtlar eklenebilir ...
}

// println! makrosu için basit bir uygulama (MMIO tabanlı örnek)
mod io {
    use core::fmt::Write;

    // MMIO adresleri (ÖRNEK ADRESLER - GERÇEK DONANIMA GÖRE DEĞİŞİR)
    const UART_DATA: u32 = 0x20000000; // Örnek UART Veri Kaydı Adresi
    const UART_STATUS: u32 = 0x20000004; // Örnek UART Durum Kaydı Adresi

    struct Stdout;

    impl Write for Stdout {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for byte in s.bytes() {
                unsafe {
                    // Basit MMIO simülasyonu (gerçekte daha karmaşık olabilir)
                    core::ptr::write_volatile(UART_DATA as *mut u8, byte);
                }
            }
            Ok(())
        }
    }

    pub fn println(s: &str) {
        let mut stdout = Stdout;
        core::fmt::write!(&mut stdout, "{}\n", s).unwrap();
    }
}

#[no_mangle]
extern "C" fn exception_handler(context: &ExceptionContext) {
    let cause = match context.cause {
        0..=0xFF => unsafe { core::mem::transmute::<u32, ExceptionCause>(context.cause) }, // Trap numaraları genellikle 0-255 aralığında
        _ => ExceptionCause::Unknown,
    };

    io::println!("İSTİSNA OLUŞTU (SPARC)!");
    io::println!("PC: {:#x}", context.pc);
    io::println!("PSR: {:#x}", context.psr);
    io::println!("Y Kaydı: {:#x}", context.y);
    io::println!("WIM: {:#x}", context.wim);
    io::println!("TBR: {:#x}", context.tbr);
    io::println!("Sebep (Trap Kodu): {}", cause);

    // Bazı genel amaçlı kayıtların değerlerini al ve yazdır (örnek olarak - SPARC'ta genel amaçlı kayıtlar r0-r31)
    let r0;  // her zaman 0 olmalı
    let r1;
    let r2;
    let r3;
    let r4;
    let r5;
    let r6;
    let r7;

    unsafe {
        asm!("mov %r0, {}", out(reg) r0); // r0 her zaman 0
        asm!("mov %r1, {}", out(reg) r1);
        asm!("mov %r2, {}", out(reg) r2);
        asm!("mov %r3, {}", out(reg) r3);
        asm!("mov %r4, {}", out(reg) r4);
        asm!("mov %r5, {}", out(reg) r5);
        asm!("mov %r6, {}", out(reg) r6);
        asm!("mov %r7, {}", out(reg) r7);
    }

    io::println!("Kayıtlar (r0-r7):");
    io::println!("  r0: {:#x}", r0); // r0 her zaman 0 olmalı
    io::println!("  r1: {:#x}", r1);
    io::println!("  r2: {:#x}", r2);
    io::println!("  r3: {:#x}", r3);
    io::println!("  r4: {:#x}", r4);
    io::println!("  r5: {:#x}", r5);
    io::println!("  r6: {:#x}", r6);
    io::println!("  r7: {:#x}", r7);

    panic!("İstisna İşlenemedi (SPARC): {}", cause);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    io::println!("PANIC! (SPARC)");
    if let Some(location) = info.location() {
        io::println!("Dosya: {}, Satır: {}", location.file(), location.line());
    }
    if let Some(message) = info.message() {
        io::println!("Mesaj: {}", message);
    }

    loop {} // Sonsuz döngüde kal
}

pub fn init() {
    // TBR (Tuzak Tabanı Kaydı) ayarı
    // SPARC mimarisinde, tuzak vektör tablosunun başlangıç adresini TBR kaydı tutar.
    // Aşağıdaki örnek, tuzak işleyici fonksiyonumuzun adresini TBR'ye yüklemeyi gösterir.
    // **SPARC mimarisine ve boot sürecine bağlı olarak bu ayar değişiklik gösterebilir.**
    // **Ayrıca, TBR sadece taban adresi tutar. Gerçek vektör adresine ulaşmak için trap numarası ile ofsetleme yapılır.**
    unsafe {
        // **BU KOD TAMAMEN ÖRNEK VE BASİTLEŞTİRİLMİŞTİR. GERÇEK SPARC SISTEMLERİ İÇİN DOĞRU olmayabilir.**
        // **Gerçek bir SPARC sisteminde, tuzak vektör tablosunu daha detaylı kurmanız gerekebilir.**
        // **Örneğin, her bir tuzak türü için farklı işleyiciler ayarlamak, tuzak tablosunu bellekte doğru yere yerleştirmek vb.**

        // Örnek olarak, exception_handler fonksiyonunun adresini TBR'ye doğrudan yazıyoruz.
        // **BU GENELLİKLE DOĞRU YAKLAŞIM DEĞİLDİR. SADECE BASİT BİR ÖRNEK İÇİN KULLANILMIŞTIR.**
        asm!("wr %g0, {}, %tbr", in(reg) exception_handler as u32);

        // **GERÇEK UYGULAMALARDA TBR'YE TUZAK VEKTÖR TABLOSUNUN BAŞLANGIÇ ADRESİ YAZILMALIDIR.**
        // **VE HER TUZAK NUMARASI İÇİN TABLO İÇİNDE AYRI BİR GİRİŞ (İŞLEYİCİ ADRESİ) OLMALIDIR.**

    }

    io::println!("TBR ayarlandı ve istisna işleyicisi (basit örnek) kuruldu. (SPARC)");
}