#![no_std]
use core::arch::asm;
use core::sync::atomic::{AtomicU64, Ordering};

// Zamanlayıcı kesme numarası (LoongArch için genellikle 7 veya 3'tür, kontrol edilmesi gerekir)
const TIMER_INTERRUPT_NUMBER: u64 = 7; // veya 3, mimariye ve yapılandırmaya bağlı

// Zamanlayıcı frekansı (Hz cinsinden, örneğin 1000 Hz, yani 1ms periyot)
const TIMER_FREQUENCY: u64 = 1000;

// Tick sayacını tutan atomik değişken
static TICKS: AtomicU64 = AtomicU64::new(0);

// Zamanlayıcı kesme işleyicisi
#[no_mangle]
extern "C" fn timer_interrupt_handler() {
    // Tick sayacını arttır
    TICKS.fetch_add(1, Ordering::SeqCst);

    // Zamanlayıcı kesme bayrağını temizle (mipi CSR'ını temizleyerek)
    unsafe {
        asm!("csrci $csr_mip, {}", const 1 << TIMER_INTERRUPT_NUMBER); // Doğru bit maskesi ile temizle
    }

    // Bir sonraki kesme için zamanlayıcıyı ayarla
    let current_ticks = get_ticks();
    let next_tick = current_ticks.wrapping_add(TIMER_FREQUENCY / 10); // 100 ms sonra kesme, wrapping_add taşmayı güvenli yönetir
    set_timer(next_tick);
}

// Zamanlayıcıyı başlatma fonksiyonu
pub fn init() {
    // Kesme işleyicisini ayarla (mtvec CSR'ı ve kesme tablosu kurulumu)
    set_interrupt_handler(TIMER_INTERRUPT_NUMBER, timer_interrupt_handler);

    // mstatus CSR'ında global kesmeleri etkinleştir
    unsafe {
        asm!("csrsi $csr_mstatus, {}", const 0x8); // MIE bitini ayarla (Global Interrupt Enable)
    }

    // mie CSR'ında zamanlayıcı kesmesini etkinleştir
    unsafe {
        asm!("csrsi $csr_mie, {}", const 1 << TIMER_INTERRUPT_NUMBER); // Timer interrupt enable bitini ayarla
    }

    // İlk kesmeyi ayarla
    let initial_tick = get_ticks().wrapping_add(TIMER_FREQUENCY / 10);
    set_timer(initial_tick);
}

// Geçen zamanı (ticks) döndüren fonksiyon
pub fn ticks() -> u64 {
    TICKS.load(Ordering::SeqCst)
}

// Belirtilen milisaniye kadar gecikme fonksiyonu
pub fn delay(ms: u64) {
    let target_ticks = ticks().wrapping_add((ms * TIMER_FREQUENCY) / 1000);
    while ticks() < target_ticks {}
}

// Zamanlayıcıyı ayarlayan fonksiyon (mtimecmp CSR'ını kullanır)
fn set_timer(ticks: u64) {
    unsafe {
        asm!("csrw $csr_mtimecmp, {}", in(reg) ticks);
    }
}

// Geçerli zamanı (ticks) okuyan fonksiyon (mtime CSR'ını kullanır)
fn get_ticks() -> u64 {
    let ticks: u64;
    unsafe {
        asm!("csrr {}, $csr_mtime", out(reg) ticks);
    }
    ticks
}

// Kesme işleyicisini ayarlayan fonksiyon (basitleştirilmiş örnek, gerçek uygulamada daha karmaşık olabilir)
fn set_interrupt_handler(interrupt_number: u64, handler: extern "C" fn()) {
    // Not: Bu örnek çok basitleştirilmiştir ve gerçek bir uygulamada kesme vektör tablosunun doğru şekilde ayarlanması gerekir.
    //      LoongArch mimarisi, kesme vektör tablosu için genellikle mtvec CSR'ını kullanır.
    //      Aşağıdaki örnek sadece bir yer tutucudur ve doğru kesme işleme için yeterli DEĞİLDİR.

    // Örnek olarak, sadece handler fonksiyonunun adresini bir yere kaydediyoruz (DOĞRU YÖNTEM DEĞİL).
    // Gerçek bir uygulamada, mtvec CSR'ını ayarlamanız ve kesme vektör tablosunu doğru şekilde yapılandırmanız gerekir.
    // Statik bir değişken veya global bir dizi kullanarak kesme işleyicilerini saklayabilirsiniz.

    // **UYARI:** Aşağıdaki kod **KESİNLİKLE** üretim ortamı için uygun DEĞİLDİR.
    //          Sadece konsepti göstermek amacıyla basitleştirilmiştir.

    static mut INTERRUPT_HANDLERS: [Option<extern "C" fn()>; 16] = [None; 16]; // Örnek olarak 16 kesme için yer ayırdık

    unsafe {
        if (interrupt_number as usize) < INTERRUPT_HANDLERS.len() {
            INTERRUPT_HANDLERS[interrupt_number as usize] = Some(handler);
        } else {
            // Kesme numarası aralık dışında, hata yönetimi gerekebilir
            panic!("Kesme numarası aralık dışında");
        }
    }

    // mtvec CSR'ının AYARLANMASI GEREKİR. Bu örnekte mtvec ayarlanmamıştır!
    // Örnek mtvec ayarı (doğrudan mod için, kesme adresleri mtvec'e göre hesaplanır):
    // unsafe {
    //     asm!("csrw $csr_mtvec, {}", in(reg) &INTERRUPT_VECTOR_TABLE as *const _ as u64);
    // }

    // Kesme vektör tablosu (ÖRNEK, gerçek uygulamaya göre düzenlenmeli)
    // #[link_section = ".trap.vector"] // veya uygun bir bölüm
    // static INTERRUPT_VECTOR_TABLE: [extern "C" fn(); 16] = { // Boyut ve içerik mimariye göre ayarlanmalı
    //     let mut table: [extern "C" fn(); 16] = [default_interrupt_handler; 16]; // Varsayılan işleyici ile doldur
    //     table[interrupt_number as usize] = handler; // İlgili kesme için handler'ı ayarla
    //     table
    // };
}

// Varsayılan kesme işleyicisi (örnek)
#[no_mangle]
extern "C" fn default_interrupt_handler() {
    // Beklenmeyen bir kesme oluştuğunda yapılacak işlemler (örneğin hata ayıklama çıktısı)
    panic!("Beklenmeyen kesme oluştu!");
}


// --- ÖRNEK KULLANIM (main.rs veya benzeri bir dosyada) ---

// extern crate panic_halt; // panic durumunda durdurmak için (no_std ortamında gerekebilir)

// #[no_mangle]
// extern "C" fn _start() -> ! {
//     main();
//     loop {}
// }

// fn main() {
//     init(); // Zamanlayıcıyı başlat

//     loop {
//         println!("Ticks: {}", ticks()); // Tick sayısını yazdır (örnek olarak UART veya benzeri bir yöntemle)
//         delay(1000); // 1 saniye gecikme
//     }
// }


// Panik işleyicisi (no_std ortamı için gereklidir)
// use core::panic::PanicInfo;
// #[panic_handler]
// fn panic(_info: &PanicInfo) -> ! {
//     loop {}
// }