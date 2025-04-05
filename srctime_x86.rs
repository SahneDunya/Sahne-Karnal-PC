#![no_std]
use core::sync::atomic::{AtomicU64, Ordering};

// PIT frekansı (yaklaşık 1.193182 MHz)
const PIT_FREQUENCY: u32 = 1193182;

// PIT Kanal 0 veri portu
const PIT_CHANNEL0_DATA_PORT: u16 = 0x40;
// PIT komut portu
const PIT_COMMAND_PORT: u16 = 0x43;

// PIC komut portu (Ana PIC)
const PIC_COMMAND_PORT: u16 = 0x20;

// PIT Kanal 0'ı ayarlamak için komut değeri:
// Kanal 0, Kare dalga modu (mod 3), binary sayıcı
const PIT_CHANNEL0_SET_COMMAND: u8 = 0x36;

// Kesme Sonunu Bildirme (End of Interrupt - EOI) komutu için değer
const PIC_EOI_COMMAND: u8 = 0x20;

// Donanım kesmelerinin (IRQ'ler) yazılım kesme vektörlerine (0-31 ayrılmış) ofseti.
// IRQ 0, vektör 32 (0x20) ile eşlenir.
const INTERRUPT_VECTOR_OFFSET: u8 = 32;
// Timer (PIT Kanal 0) için IRQ numarası
const TIMER_IRQ_NUMBER: u8 = 0;
// Timer kesmesinin tam vektör numarası
const TIMER_INTERRUPT_VECTOR: u8 = INTERRUPT_VECTOR_OFFSET + TIMER_IRQ_NUMBER;


static TICKS: AtomicU64 = AtomicU64::new(0);

// PIT'i belirtilen frekansta başlatır ve zamanlayıcı kesme işleyicisini ayarlar.
//
// # Parametreler
//
// * `frequency`: Zamanlayıcı kesmesinin frekansı Hertz cinsinden.
pub fn init(frequency: u32) {
    let divisor = PIT_FREQUENCY / frequency;

    unsafe {
        // **Güvensiz İşlem**: Doğrudan donanım portlarına yazılıyor.
        // PIT kontrol portuna komut göndererek Kanal 0'ı ayarlar.
        out8(PIT_COMMAND_PORT, PIT_CHANNEL0_SET_COMMAND);
        // Bölücünün düşük baytını Kanal 0 veri portuna gönderir.
        out8(PIT_CHANNEL0_DATA_PORT, (divisor & 0xFF) as u8);
        // Bölücünün yüksek baytını Kanal 0 veri portuna gönderir.
        out8(PIT_CHANNEL0_DATA_PORT, ((divisor >> 8) & 0xFF) as u8);

        // **Güvensiz İşlem**: Kesme işleyicisi tablosuna doğrudan yazılıyor.
        // Zamanlayıcı kesmesi için kesme işleyicisini ayarlar.
        // Burada, IRQ 0 için, kesme vektörü 32 (0x20) olarak ayarlanır.
        // `interrupt` modülünün `set_interrupt_handler` fonksiyonunun
        // sisteminize uygun şekilde uygulandığını varsayar.
        interrupt::set_interrupt_handler(TIMER_INTERRUPT_VECTOR as u8, timer_interrupt_handler);
    }
}

// Zamanlayıcı kesme işleyicisi fonksiyonu.
//
// Bu fonksiyon her zamanlayıcı kesmesinde çağrılır.
#[no_mangle]
extern "C" fn timer_interrupt_handler() {
    // Global tik sayacını artırır.
    TICKS.fetch_add(1, Ordering::SeqCst);

    unsafe {
        // **Güvensiz İşlem**: Doğrudan donanım portuna yazılıyor.
        // Kesme denetleyicisine (PIC) Kesme Sonunu Bildirme (EOI) komutu gönderir.
        // Bu, PIC'e kesmenin işlendiğini ve başka kesmeleri kabul etmeye hazır olduğunu bildirir.
        out8(PIC_COMMAND_PORT, PIC_EOI_COMMAND); // Ana PIC için EOI komutu (0x20)
    }
}

// Geçen tik sayısını döndürür.
//
// Tik sayısı, `init` fonksiyonunda ayarlanan zamanlayıcı frekansına bağlıdır.
pub fn ticks() -> u64 {
    TICKS.load(Ordering::SeqCst)
}

// Belirtilen milisaniye kadar bekler (aktif bekleyerek - busy-waiting).
//
// # Uyarı
//
// Bu fonksiyon aktif bekleyerek çalıştığı için işlemciyi meşgul eder ve güç tüketimini artırır.
// Mümkünse, daha verimli bekleme yöntemleri (örneğin, olay tabanlı veya uyku modları) tercih edilmelidir.
//
// # Parametreler
//
// * `ms`: Beklenecek milisaniye sayısı.
pub fn delay(ms: u64) {
    // Hedef tik sayısını hesaplar.
    // Her milisaniyede yaklaşık olarak `PIT_FREQUENCY / 1000` tik oluşur.
    let target_ticks = ticks() + (ms * (PIT_FREQUENCY as u64 / 1000) );
    // Geçerli tik sayısı hedef tik sayısına ulaşana kadar bekler.
    while ticks() < target_ticks {}
}

// Belirtilen porta bir bayt yazar.
//
// # Güvensizlik
//
// Bu fonksiyon `unsafe` olarak işaretlenmiştir çünkü doğrudan donanım portlarına erişir.
// Yanlış port veya değer yazmak donanımın kararsız çalışmasına veya zarar görmesine neden olabilir.
//
// # Parametreler
//
// * `port`: Yazılacak donanım portunun adresi.
// * `value`: Porta yazılacak bayt değeri.
unsafe fn out8(port: u16, value: u8) {
    // **Güvensiz İşlem**: `out` assembly komutu doğrudan donanım portuna yazıyor.
    asm!("out dx, al", in("dx") port, in("al") value);
}

// Harici `interrupt` modülünün varsayılan tanımı.
// Bu modülün sisteminize ve kesme işleme mekanizmanıza göre
// uygun şekilde uygulanması gerekmektedir.
mod interrupt {
    pub unsafe fn set_interrupt_handler(interrupt_vector: u8, handler: extern "C" fn()) {
        // Gerçek donanım seviyesi kesme işleyici ayar mekanizması
        // sisteme özgüdür ve burada örnek bir uygulama gösterilmemiştir.
        // Bu fonksiyonun içeriği hedef sisteme göre uyarlanmalıdır.
        // Örneğin, IDT (Interrupt Descriptor Table) manipülasyonu içerebilir.
        unimplemented!(); // Sisteminiz için uygun uygulamayı buraya ekleyin.
    }
}