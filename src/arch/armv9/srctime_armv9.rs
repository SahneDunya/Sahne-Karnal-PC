#![no_std]

use core::sync::atomic::{AtomicU64, Ordering};

// **Açıklama**: ARM Generic Timer register adresleri.
// **Önemli**: Bu adresler **örnektir** ve kullandığınız ARM işlemciye ve platforma göre **değişkenlik gösterebilir**.
// Veri sayfalarından ve teknik referans kılavuzlarından doğrulanmalıdır.
const CNTP_TVAL_EL0: usize = 0xC3FFE020; // Timer Value Register
const CNTP_CTL_EL0: usize = 0xC3FFE028;  // Timer Control Register
const CNTP_CVAL_EL0: usize = 0xC3FFE030; // Timer Counter-timer Value Register (salt okunur)
const CNTV_CTL_EL0: usize = 0xC3FFE048; // Sanal Timer Control Register (örnek olarak eklenmiştir, kullanılmayabilir)

// **Açıklama**: Kesme numarası.
// **Önemli**: Bu numara da platforma özgüdür. Genellikle kesme kontrolcüsü (örn. GIC) yapılandırmasına bağlıdır.
// Bu değer de platformun kesme haritasından alınmalıdır.
const TIMER_INTERRUPT_NUMBER: usize = 30;

static TICKS: AtomicU64 = AtomicU64::new(0);

// **Açıklama**: Zamanlayıcı kesme işleyicisi (handler) fonksiyonu.
// `#[no_mangle]` ve `extern "C"` öznitelikleri, bu fonksiyonun C çağrı kurallarına uygun olarak
// dışarıdan (örn. kesme vektör tablosundan) erişilebilir olmasını sağlar.
#[no_mangle]
extern "C" fn timer_interrupt_handler() {
    TICKS.fetch_add(1, Ordering::SeqCst); // Tick sayacını artır. `SeqCst` en güçlü hafıza sıralamasıdır.
    clear_timer_interrupt(); // Kesme durumunu temizle (platforma özgü)
    set_timer(10_000_000); // Zamanlayıcıyı yeniden ayarla (bir sonraki kesme için)
                                  // **Dikkat**: 10_000_000 değeri frekansa bağlıdır.
                                  // 10ms aralık için frekansa göre ayarlanmalıdır.
}

// **Açıklama**: Zamanlayıcı modülünü başlatma fonksiyonu.
pub fn init() {
    set_interrupt_handler(TIMER_INTERRUPT_NUMBER, timer_interrupt_handler); // Kesme işleyicisini ayarla
    enable_timer_interrupt(); // Zamanlayıcı kesmesini etkinleştir (platforma özgü)
    set_timer(10_000_000); // İlk kesmeyi ayarla
    enable_timer(); // Zamanlayıcıyı başlat
}

// **Açıklama**: Geçen tick sayısını döndüren fonksiyon.
pub fn ticks() -> u64 {
    TICKS.load(Ordering::SeqCst) // Tick sayısını güvenli bir şekilde oku.
}

// **Açıklama**: Belirtilen milisaniye kadar bekleyen (gecikme) fonksiyonu.
pub fn delay(ms: u64) {
    let target_ticks = ticks() + ms; // Hedef tick sayısını hesapla.
    while ticks() < target_ticks {} // Hedef tick sayısına ulaşılana kadar bekle.
                                     // **Dikkat**: Bu bloklayıcı (blocking) bir gecikmedir.
                                     // Gerçek zamanlı sistemlerde dikkatli kullanılmalıdır.
}

// **Açıklama**: Zamanlayıcı değerini ayarlayan fonksiyon.
fn set_timer(value: u64) {
    unsafe {
        // **Güvensiz Blok**: `asm!` makrosu ile doğrudan assembly komutu kullanılıyor.
        // `msr cntp_tval_el0, {}`: `value` değişkenini `cntp_tval_el0` register'ına yazar.
        asm!("msr cntp_tval_el0, {}", in(reg) value);
    }
}

// **Açıklama**: Zamanlayıcıyı etkinleştiren fonksiyon.
fn enable_timer() {
    unsafe {
        // `msr cntp_ctl_el0, {}`: `1` değerini `cntp_ctl_el0` register'ına yazar.
        // Bu, zamanlayıcıyı etkinleştirir (genellikle kontrol register'ının 0. biti etkinleştirme bitidir).
        asm!("msr cntp_ctl_el0, {}", in(reg) 1);
    }
}

// **Açıklama**: Zamanlayıcıyı devre dışı bırakan fonksiyon.
fn disable_timer() {
    unsafe {
        // `msr cntp_ctl_el0, {}`: `0` değerini `cntp_ctl_el0` register'ına yazar.
        // Bu, zamanlayıcıyı devre dışı bırakır.
        asm!("msr cntp_ctl_el0, {}", in(reg) 0);
    }
}

// **Açıklama**: Zamanlayıcı kesmesini etkinleştiren fonksiyon (PLATFORMA ÖZGÜ).
// **Önemli**: Bu fonksiyonun içeriği, hedef platformun kesme kontrolcüsüne (örn. GIC) ve
// kesme yönlendirme mekanizmasına göre **kesinlikle** değiştirilmelidir.
// Aşağıda **örnek** bir GIC (Generic Interrupt Controller) yaklaşımı gösterilmiştir.
fn enable_timer_interrupt() {
    // **PLATFORMA ÖZGÜ KOD BAŞLANGICI - GIC ÖRNEĞİ**
    // Varsayımlar:
    // 1. GICv2 kullanılıyor.
    // 2. Zamanlayıcı kesmesi SPI (Shared Peripheral Interrupt) olarak yapılandırılmış.
    // 3. TIMER_INTERRUPT_NUMBER, GIC'deki SPI kesme numarasına karşılık geliyor.

    let gicd_base: usize = 0x...; // GIC Dağıtıcı (Distributor) arayüzünün base adresi (platforma göre değişir!)
    let gicc_base: usize = 0x...; // GIC CPU arayüzünün base adresi (platforma göre değişir!)

    let interrupt_id = TIMER_INTERRUPT_NUMBER as u32; // Kesme numarasını u32'ye dönüştür.
    let target_cpu_list: u32 = 0x01; // Kesmeyi hedef CPU 0'a yönlendir (bitmask).

    unsafe {
        // 1. Kesmeyi hedef CPU'lara yönlendir (GICD_IROUTER - Interrupt Routing Register).
        let gicd_irouter_offset = 0x8000 + (interrupt_id / 4) * 4; // Örnek offset hesaplama (GICv2'ye göre değişebilir)
        let gicd_irouter_addr = gicd_base + gicd_irouter_offset as usize;
        core::ptr::write_volatile(gicd_irouter_addr as *mut u32, target_cpu_list);

        // 2. Kesmeyi etkinleştir (GICD_ISENABLER - Interrupt Set-Enable Register).
        let gicd_isenabler_offset = 0x100 + (interrupt_id / 32) * 4; // Örnek offset hesaplama
        let gicd_isenabler_addr = gicd_base + gicd_isenabler_offset as usize;
        let enable_bit_mask: u32 = 1 << (interrupt_id % 32);
        core::ptr::write_volatile(gicd_isenabler_addr as *mut u32, enable_bit_mask);

        // 3. CPU arayüzünde kesmeleri etkinleştir (GICC_CTLR - CPU Interface Control Register).
        let gicc_ctlr_addr = gicc_base + 0x0 as usize; // GICC_CTLR offset'i genellikle 0'dır.
        let current_gicc_ctlr = core::ptr::read_volatile(gicc_ctlr_addr as *mut u32);
        core::ptr::write_volatile(gicc_ctlr_addr as *mut u32, current_gicc_ctlr | 0x01); // Bit 0'ı set et (Enable bit)
    }
    // **PLATFORMA ÖZGÜ KOD SONU**
}


// **Açıklama**: Zamanlayıcı kesmesini devre dışı bırakan fonksiyon (PLATFORMA ÖZGÜ).
// **Önemli**: `enable_timer_interrupt` fonksiyonuna benzer şekilde, bu fonksiyonun içeriği de
// platforma ve kullanılan kesme kontrolcüsüne göre **kesinlikle** uyarlanmalıdır.
fn disable_timer_interrupt() {
    // **PLATFORMA ÖZGÜ KOD BAŞLANGICI - GIC ÖRNEĞİ** (GICv2 varsayımıyla)

    let gicd_base: usize = 0x...; // GIC Dağıtıcı base adresi (platforma göre değişir!)

    let interrupt_id = TIMER_INTERRUPT_NUMBER as u32;

    unsafe {
        // Kesmeyi devre dışı bırak (GICD_ICENABLER - Interrupt Clear-Enable Register).
        let gicd_icenabler_offset = 0x180 + (interrupt_id / 32) * 4; // Örnek offset hesaplama
        let gicd_icenabler_addr = gicd_base + gicd_icenabler_offset as usize;
        let disable_bit_mask: u32 = 1 << (interrupt_id % 32);
        core::ptr::write_volatile(gicd_icenabler_addr as *mut u32, disable_bit_mask);
    }
    // **PLATFORMA ÖZGÜ KOD SONU**
}


// **Açıklama**: Zamanlayıcı kesmesini temizleyen fonksiyon (PLATFORMA ÖZGÜ).
// **Önemli**: Kesmenin nasıl temizleneceği platforma ve kesme kontrolcüsüne bağlıdır.
// ARM Generic Timer kesmeleri genellikle otomatik olarak temizlenir (kenar tetiklemeli değilse).
// Ancak, bazı platformlarda kesme durumunu manuel olarak temizlemek gerekebilir.
// Aşağıdaki örnek, GIC (Generic Interrupt Controller) için kesme durumunu temizleme adımlarını **varsayımsal** olarak göstermektedir.
// **Dikkat**: Bu kod, **gerçek GIC temizleme mekanizmasını yansıtmayabilir**. Platformunuzun dokümantasyonunu kontrol edin!
fn clear_timer_interrupt() {
    // **PLATFORMA ÖZGÜ KOD BAŞLANGICI - ÖRNEK TEMİZLEME KODU (GIC için varsayımsal)**
    // **ÖNEMLİ**: Bu kod sadece **örnek** amaçlıdır ve doğru olmayabilir!

    let gicc_base: usize = 0x...; // GIC CPU arayüzü base adresi (platforma göre değişir!)
    let interrupt_id = TIMER_INTERRUPT_NUMBER as u32;

    unsafe {
        // Kesme durumunu temizle (GICC_EOIR - End of Interrupt Register).
        // GICv2'de kesme temizleme genellikle EOI register'ına kesme ID'sini yazarak yapılır.
        let gicc_eoir_addr = gicc_base + 0x10 as usize; // GICC_EOIR offset'i genellikle 0x10'dur.
        core::ptr::write_volatile(gicc_eoir_addr as *mut u32, interrupt_id);

        // **DİKKAT**: Bazı platformlarda ek olarak interrupt acknowledge (kesme onayı - GICC_IAR)
        // register'ından okuma yapmak da gerekebilir. Bu örnekte bu adım **atlandı**.
        // Platformunuzun GIC dokümantasyonunu dikkatlice inceleyin.
    }
    // **PLATFORMA ÖZGÜ KOD SONU - ÖRNEK TEMİZLEME KODU**
}


// **Açıklama**: Kesme işleyicisini ayarlayan fonksiyon (PLATFORMA ÖZGÜ).
// **Önemli**: Kesme işleyicisinin nasıl ayarlanacağı platforma ve kullanılan kesme yönetim mekanizmasına bağlıdır.
// En yaygın yöntemlerden biri **vektör tablosunu** güncellemektir.
// Diğer bir yöntem ise, bazı gelişmiş kesme kontrolcülerinde (örn. GIC) kesme işleyicilerini
// doğrudan kontrolcü register'larına yazmaktır (bu örnekte GIC için bu yaklaşım **gösterilmemiştir**, vektör tablosu yaklaşımı daha geneldir).
fn set_interrupt_handler(interrupt_number: usize, handler: extern "C" fn()) {
    // **PLATFORMA ÖZGÜ KOD BAŞLANGICI - VEKTÖR TABLOSU ÖRNEĞİ**
    // Varsayım: Vektör tablosu, `__vectors_start` sembolünde başlıyor ve her giriş 4 byte (veya 8 byte - platforma göre)

    extern "C" {
        static __vectors_start: u32; // Vektör tablosunun başlangıç adresi (linker script'te tanımlanmalı)
    }

    let vectors_start_addr = &__vectors_start as *const u32 as usize; // Vektör tablosu başlangıç adresini al.
    let handler_ptr = handler as usize; // İşleyici fonksiyonun adresini al.
    let interrupt_index = interrupt_number; // Kesme numarası (vektör tablosundaki index)

    unsafe {
        // Vektör tablosuna kesme işleyicisi adresini yaz.
        // **Dikkat**: Vektör tablosu giriş boyutu (4 byte mı, 8 byte mı?) platforma göre değişebilir.
        // Bu örnek 4 byte varsayar. 8 byte ise, offset `interrupt_index * 8` olmalıdır.
        let vector_entry_addr = vectors_start_addr + interrupt_index * 4;
        core::ptr::write_volatile(vector_entry_addr as *mut usize, handler_ptr);
    }
    // **PLATFORMA ÖZGÜ KOD SONU - VEKTÖR TABLOSU ÖRNEĞİ**

    // **Alternatif GIC Yaklaşımı (örnek olarak - vektör tablosu yerine, bazı GIC'lerde işleyici doğrudan ayarlanabilir):**
    // **Bu örnek kod **tamamen varsayımsaldır** ve gerçek GIC'lerde bu şekilde işleyici ayarlamak mümkün olmayabilir.**
    // **Vektör tablosu yaklaşımı daha genel ve yaygın bir yöntemdir.**
    
    fn set_interrupt_handler_gic_example(interrupt_number: usize, handler: extern "C" fn()) {
        let gicc_base: usize = 0x...; // GIC CPU arayüzü base adresi (platforma göre değişir!)
        let interrupt_id = interrupt_number as u32;
        let handler_address = handler as usize as u32;

        unsafe {
            // **UYARI: BU KOD TAMAMEN VARSAYIMSALDIR VE ÇALIŞMAYABİLİR! GIC DOKÜMANTASYONUNA BAKIN!**
            let gicc_ihr_offset = 0x...; // Varsayımsal Interrupt Handler Register offset'i
            let gicc_ihr_addr = gicc_base + gicc_ihr_offset as usize + interrupt_id as usize * 4; // Her kesme için ayrı register varsayımı
            core::ptr::write_volatile(gicc_ihr_addr as *mut u32, handler_address);
        }
    }   
}
