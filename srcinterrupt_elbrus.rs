#![no_std]
use core::arch::asm;
use core::ptr;
use volatile::Volatile;

// Elbrus çipine özgü USB kesme bitleri ve adresleri (GERÇEK DEĞERLERİ ELBRUS REFERANS KILAVUZUNDAN ALIN!)
const USB_IRQ_BIT: usize = 1 << 10; // ÖRNEK DEĞER: Elbrus kılavuzundan gerçek değeri alın
const USB_STATUS_REGISTER: usize = 0x4000_1000; // ÖRNEK DEĞER: Elbrus kılavuzundan gerçek değeri alın
const USB_DATA_REGISTER: usize = 0x4000_1004; // ÖRNEK DEĞER: Elbrus kılavuzundan gerçek değeri alın
const USB_STATUS_DATA_RECEIVED: u32 = 0x01; // ÖRNEK DEĞER: Veri alındı durum biti
const USB_STATUS_DATA_SENT: u32 = 0x02;    // ÖRNEK DEĞER: Veri gönderildi durum biti
const USB_STATUS_CLEAR_MASK: u32 = USB_STATUS_DATA_RECEIVED | USB_STATUS_DATA_SENT; // Temizlenecek bit maskesi

extern "C" {
    static IVT: u8; // Kesme vektör tablosu (başlangıç adresi) - exception.rs dosyasında tanımlanmalı
}

pub mod exception {
    use core::arch::asm;

    // exception.rs içinde init fonksiyonu ve IVT tanımı olmalı
    extern "C" {
        pub static IVT: u8;
    }

    #[link_section = ".trap.entry"]
    #[naked]
    #[export = "_start"]
    pub unsafe extern "C" fn _start() -> ! {
        asm!("j exception_entry", options(noreturn));
    }

    #[link_section = ".trap.exceptions"]
    #[export = "exception_entry"]
    pub unsafe extern "C" fn exception_entry() {
        // MTVEC ayarlanıyor (kesme vektör tablosu adresi) - _start içinde yapılıyor
        // Kesme nedenini kontrol etme (isteğe bağlı, hata ayıklama için faydalı olabilir)
        // ...
        // Kesme işleyiciye dallan
        asm!("mret":::"volatile"); // Basitçe MRET ile dönüyoruz, gerçek kodda kesme işleyici çağrılmalı
    }


    pub fn init() {
        // MTVEC'i ayarla: _start fonksiyonunda yapılıyor.
        // Genel kesmeleri etkinleştir: init fonksiyonunda yapılıyor.
    }
}


pub fn init() {
    unsafe {
        // MTVEC'i ayarla (Kesme Vektör Tablosu Adresi)
        let ivt_address = &IVT as *const u8 as usize;
        asm!("csrw mtvec, {0}", in(reg) ivt_address);

        // USB kesmesini etkinleştir (mie register - machine interrupt enable)
        asm!("csrrs mie, {0}, zero", in(reg) USB_IRQ_BIT);

        // Genel kesmeleri etkinleştir (mstatus register - machine status, MIE biti - Machine Interrupt Enable)
        asm!("csrrs mstatus, {0}, zero", in(reg) 1 << 3); // MIE biti (3. bit)

        exception::init(); // İstisna işleyicisini başlat (şu anda içi boş, IVT ayarı _start'da yapılıyor)
    }
}

#[no_mangle]
#[link_section = ".trap.interrupt_handlers"] // Kesme işleyicileri için ayrı bir bölüm
pub extern "C" fn usb_interrupt_handler() {
    // USB kesme işleyicisi

    // 1. Kesme kaynağını belirle (Elbrus'a özgü STATUS REGISTER'ı oku)
    let status = unsafe { ptr::read_volatile(USB_STATUS_REGISTER as *const u32) };

    // 2. Kesme nedenine göre işlem yap
    if status & USB_STATUS_DATA_RECEIVED != 0 {
        // Veri geldi kesmesi işleme
        let data = unsafe { ptr::read_volatile(USB_DATA_REGISTER as *const u32) };
        // ... veriyi işle ...
        // Örneğin, veriyi bir tampona yaz veya başka bir fonksiyona ilet
        println!("USB Veri Alındı: {}", data); // Örnek çıktı (println! makrosu için ek yapılandırma gerekebilir)
    }

    if status & USB_STATUS_DATA_SENT != 0 {
        // Veri gönderildi kesmesi işleme
        // ... veri gönder ...
        println!("USB Veri Gönderildi");
    }

    // 3. Kesme bayrağını TEMİZLE (ÇOK ÖNEMLİ!) - SADECE İŞLEDİĞİMİZ KESME BAYRAKLARINI TEMİZLİYORUZ
    unsafe {
        let mut status_reg = Volatile::new(USB_STATUS_REGISTER as *mut u32);
        status_reg.write(status & !USB_STATUS_CLEAR_MASK); // SADECE ilgili bitleri temizle
        // ÖNEMLİ: Tüm status register'ı sıfırlamayın, sadece işlediğiniz kesme bitlerini temizleyin.
        // Aksi takdirde, diğer kesme durumlarını (eğer varsa) yanlışlıkla temizleyebilirsiniz.
    }

    // Kesme işleyiciden çıkış (MRET - Machine Return ile donmeli) - exception_entry'de yapılıyor
    // Gerçek kodda, kesme işleyiciden çıkış için 'mret' assembly komutu kullanılmalıdır.
}


// Örnek println! makrosu (no_std ortamda temel çıktı için - gerçek uygulama için uygun bir UART veya benzeri sürücü gerekebilir)
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        let s = format_args!($($arg)*);
        let _ = write!(crate::stdio::Stdout, "{}", s); // stdio::Stdout'a yaz
    }};
}

// Temel stdio modülü (UART veya benzeri bir arayüz üzerinden çıktı için yapılandırılmalıdır)
pub mod stdio {
    use core::fmt;

    pub struct Stdout;

    impl fmt::Write for Stdout {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            // Gerçek donanıma (örn. UART) çıktı gönderme kodu buraya gelecek
            // Şimdilik basit bir döngü ile karakterleri gönderelim (ÖRNEK KOD, UART sürücüsü yerine geçmez!)
            for byte in s.bytes() {
                unsafe {
                    // UART veri gönderme register'ına byte yazma (ADRES ve DONANIM DETAYLARI ELBRUS'A ÖZGÜ OLACAK!)
                    // ÖRNEK ADRES VE KOD: GERÇEK UART ADRESİ VE GÖNDERME YÖNTEMİ İÇİN ELBRUS KILAVUZUNA BAKIN!
                    const UART_DATA_REGISTER: usize = 0x4000_2000; // ÖRNEK UART DATA REGISTER ADRESİ
                    core::ptr::write_volatile(UART_DATA_REGISTER as *mut u8, byte);
                    // Basit bir gecikme (DONANIM HIZINA GÖRE AYARLANMALI)
                    for _ in 0..1000 {}; // Çok basit gecikme - GERÇEK UYGULAMADA UYGUN GECİKME MEKANİZMASI KULLANIN!
                }
            }
            Ok(())
        }
    }
}
use core::fmt::Write; // fmt::Write trait'ini kullanmak için import

// ... (diğer kodlar ve bağımlılıklar - örneğin `volatile` crate'i Cargo.toml'e eklenmeli) ...