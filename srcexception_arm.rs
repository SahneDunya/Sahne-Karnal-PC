#![no_std]

use core::arch::asm;
use core::fmt;
use core::panic::PanicInfo;

// İstisna Sebepleri (ARM mimarisine özgü - basitleştirilmiş örnek)
#[repr(u32)]
enum ExceptionCause {
    Reset = 0,
    UndefinedInstruction = 1,
    SoftwareInterrupt = 2,
    PrefetchAbort = 3,
    DataAbort = 4,
    AddressException = 5,
    IRQ = 6,
    FIQ = 7,
    Unknown = 0xFFFF, // Bilinmeyen sebepler için
}

impl fmt::Display for ExceptionCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExceptionCause::Reset => write!(f, "Sıfırlama"),
            ExceptionCause::UndefinedInstruction => write!(f, "Tanımsız talimat"),
            ExceptionCause::SoftwareInterrupt => write!(f, "Yazılım kesmesi (SWI)"),
            ExceptionCause::PrefetchAbort => write!(f, "Önceden getirme iptali"),
            ExceptionCause::DataAbort => write!(f, "Veri iptali"),
            ExceptionCause::AddressException => write!(f, "Adres istisnası"),
            ExceptionCause::IRQ => write!(f, "IRQ kesmesi"),
            ExceptionCause::FIQ => write!(f, "FIQ kesmesi"),
            ExceptionCause::Unknown => write!(f, "Bilinmeyen sebep"),
        }
    }
}

// ExceptionContext yapısı (ARM mimarisine özgü - basitleştirilmiş örnek)
#[derive(Debug)]
#[repr(C)]
pub struct ExceptionContext {
    pub pc: u32,       // Program Sayacı
    pub sp: u32,       // Yığın İşaretçisi
    pub cpsr: u32,     // Mevcut Program Durum Kaydı
    pub lr: u32,       // Bağlantı Kaydı
    pub cause: u32,    // İstisna sebebi (enum değerinin sayısal karşılığı)
    // ... diğer ilgili kayıtlar eklenebilir ...
}

// println! makrosu için basit bir uygulama (UART MMIO adresleri ARM için örnek)
mod io {
    use core::fmt::Write;

    // ARM için MMIO adresleri (örnek olarak, gerçek adresler SoC'ye bağlıdır)
    const UART_DR: u32 = 0x101f1000; // UART Veri Kaydı
    const UART_FR: u32 = 0x101f1018; // UART Flag Kaydı (örneğin, FIFO boş/dolu durumu)

    struct Stdout;

    impl Write for Stdout {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for byte in s.bytes() {
                unsafe {
                    // Basit MMIO simülasyonu (gerçekte daha karmaşık olabilir)
                    // UART Veri Kaydına (UART_DR) byte yazma
                    core::ptr::write_volatile(UART_DR as *mut u8, byte);
                    // ** Gerçek UART için burada FIFO dolu kontrolü vb. gerekebilir.
                    // ** Bu örnek sadece en temel düzeyde bir çıktı simülasyonudur.
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
        0..=7 => unsafe { core::mem::transmute(context.cause) }, // ARM için basitleştirilmiş aralık
        _ => ExceptionCause::Unknown,
    };

    io::println!("İstisna Oluştu (ARM)!");
    io::println!("PC: {:#x}", context.pc);
    io::println!("SP: {:#x}", context.sp);
    io::println!("CPSR: {:#x}", context.cpsr);
    io::println!("LR: {:#x}", context.lr);
    io::println!("Sebep: {}", cause);
    // io::println!("Bağlam: {:?}", context); // İsteğe bağlı olarak tüm bağlamı yazdırabilirsiniz.

    panic!("İstisna İşlenemedi (ARM): {}", cause);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    io::println!("PANIC! (ARM)");
    if let Some(location) = info.location() {
        io::println!("Dosya: {}, Satır: {}", location.file(), location.line());
    }
    if let Some(message) = info.message() {
        io::println!("Mesaj: {}", message);
    }

    loop {} // Sonsuz döngüde kal
}

pub fn init() {
    // Vektör Tablosu Adresi Kaydı (VTOR) ayarı (ARM mimarisine özgü)
    // VTOR, istisna vektör tablosunun başlangıç adresini belirtir.
    // Bu örnekte, vektör tablosunu istisna işleyicimizin adresiyle başlatacağız.
    // **Gerçek bir ARM sisteminde, vektör tablosu genellikle farklı istisna türleri
    // **için ayrı işleyicilerin adreslerini içerir. Bu örnek basitleştirilmiştir.

    // ** ARMv7-M mimarisi ve sonrası için VTOR ayarı genellikle bir kontrol kaydı aracılığıyla yapılır.
    // ** ARM mimarisi ve çekirdek tipine (örn. ARMv7-A, ARM Cortex-M) bağlı olarak VTOR ayarı değişebilir.
    // ** Aşağıdaki örnek, ARM Cortex-M serisi için tipik bir yaklaşımdır (SCB->VTOR).
    unsafe {
        extern "C" {
            static mut __VTOR_RAM: u32; // Vektör tablosu RAM'de (linker script ile tanımlanmalı)
        }
        // Basitçe ilk vektör tablosu girişini (Reset vektörü) istisna işleyicimiz olarak ayarlıyoruz.
        // ** Gerçekte vektör tablosu daha karmaşık olabilir ve farklı vektörler içerebilir.
        let vtor_ptr = &mut __VTOR_RAM as *mut u32;
        *vtor_ptr = exception_handler as u32; // İlk vektör girişine istisna işleyici adresini yaz.

        // ** ARM Cortex-M için VTOR kaydını SCB üzerinden ayarlamak daha yaygındır.
        // ** MMIO adresleri ve SCB yapısı kullanılan ARM çekirdeğine göre değişir.
        // ** Aşağıdaki örnek genel bir gösterimdir ve gerçek adresler kontrol edilmelidir.
        const SCB_VTOR: u32 = 0xE000ED08; // Örnek SCB VTOR adresi (Cortex-M için yaygın)
        core::ptr::write_volatile(SCB_VTOR as *mut u32, &__VTOR_RAM as *const u32 as u32);

    }

    io::println!("VTOR ayarlandı ve istisna işleyicisi kuruldu (ARM).");
}