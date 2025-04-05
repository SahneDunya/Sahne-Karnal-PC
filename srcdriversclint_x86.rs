#![no_std]
use core::arch::asm;

// x86 üzerinde CLINT benzeri bir yapı veya doğrudan zamanlayıcı donanımı olmayabilir.
// Bu örnek, yerel APIC zamanlayıcısını (Local APIC Timer) kullanacaktır.
// x86 mimarisinde zamanlayıcılar ve kesmeler platforma ve donanıma büyük ölçüde bağımlıdır.
// Bu kod bir örnek teşkil eder ve gerçek bir sistemde platforma özgü detaylara göre ayarlanmalıdır.

mod platform_config_x86 {
    // Yerel APIC taban adresi. Bu adres sistem kurulumuna göre değişebilir.
    // Genellikle yüksek bellekte (örn., 0xFEE00000) bir yere eşlenir.
    pub const LOCAL_APIC_BASE_ADDRESS: usize = 0xFEE00000;

    // Yerel APIC zamanlayıcı kayıtlarının göreli konumları (offsetleri)
    pub const LOCAL_APIC_TIMER_DIVIDE_CONFIG_OFFSET: usize = 0x3E0; // Bölme yapılandırma kaydı
    pub const LOCAL_APIC_LVT_TIMER_OFFSET: usize = 0x320;          // Zamanlayıcı LVT (Yerel Vektör Tablosu) kaydı
    pub const LOCAL_APIC_INITIAL_COUNT_OFFSET: usize = 0x380;      // İlk sayım değeri kaydı
    pub const LOCAL_APIC_CURRENT_COUNT_OFFSET: usize = 0x390;      // Şu anki sayım değeri kaydı

    pub const CLOCK_FREQ: u64 = 10_000_000; // Örnek değer, işlemci veya zamanlayıcı frekansınıza göre değiştirin
    pub const TIMER_VECTOR: u8 = 32;        // Zamanlayıcı kesmesi için vektör numarası (örn., 32, kullanıcı tanımlı aralık)
}

/// x86 Yerel APIC zamanlayıcısını kullanarak zamanlayıcıyı ayarlamak için fonksiyon.
///
/// # Arguments
///
/// * `delay_ms`: Gecikme süresi milisaniye cinsinden.
///
/// # Safety
///
/// Bu fonksiyon `unsafe` bloklar içerir çünkü doğrudan donanım kayıtlarına erişir.
/// Yanlış `delay_ms` değerleri veya platform yapılandırması sistemde beklenmedik davranışlara yol açabilir.
/// Bu fonksiyonun doğru platform yapılandırması (`platform_config_x86` modülü) ile kullanıldığından emin olun.
/// Ayrıca, bu fonksiyon sadece **Yerel APIC zamanlayıcısı** olan x86 sistemlerinde çalışır.
pub fn set_timer_x86(delay_ms: u64) {
    // Yerel APIC adreslerini platform yapılandırmasından al
    let lapic_base = platform_config_x86::LOCAL_APIC_BASE_ADDRESS;
    let timer_divide_config_reg = lapic_base + platform_config_x86::LOCAL_APIC_TIMER_DIVIDE_CONFIG_OFFSET;
    let lvt_timer_reg = lapic_base + platform_config_x86::LOCAL_APIC_LVT_TIMER_OFFSET;
    let initial_count_reg = lapic_base + platform_config_x86::LOCAL_APIC_INITIAL_COUNT_OFFSET;
    let clock_frequency = platform_config_x86::CLOCK_FREQ;
    let timer_vector = platform_config_x86::TIMER_VECTOR;

    // Gecikme süresinin sıfır olup olmadığını kontrol et
    if delay_ms == 0 {
        return; // Sıfır gecikme, hemen geri dön
    }

    unsafe {
        // **1. Zamanlayıcı Bölme Yapılandırmasını Ayarla:**
        //    - Zamanlayıcı frekansını belirlemek için bölme değerini yapılandır.
        //    - Örnek olarak, bölme değerini '1' (bölme yok) olarak ayarlıyoruz, yani CPU saat frekansını kullanacağız.
        //    - Referans kılavuzlarına göre, bölme değeri 11-8 bit aralığında (4 bit).
        //    - 0b10110000 (0x3B) -> Bölme 1 (saat frekansını olduğu gibi kullan)
        asm!("mov eax, {divisor}", divisor = const 0b10110000, options(nostack));
        asm!("mov [{reg}], eax", reg = in(reg) timer_divide_config_reg, options(nostack));


        // **2. Zamanlayıcı LVT Girişini Ayarla:**
        //    - Zamanlayıcı modunu ve kesme vektörünü yapılandır.
        //    - Kesme vektörünü ve zamanlayıcının maskelenmediğinden emin oluyoruz.
        //    - 0b000xxxxx -> Vektör (xxxxx kısmı vektör numarasını içerir)
        //    - 0b0100000000000000 (0x40000) -> Maskelenmemiş (kesmeler etkin)
        //    - 0b0000000000010000 (0x10)    -> Tetikleme modu (seviye tetiklemeli değil, kenar tetiklemeli) - Gerekli olmayabilir, varsayılan kenar tetiklemeli olabilir.
        //    - 0b0000000000100000 (0x20)    -> Fiziksel hedef mod (fiziksel hedefleme) - Gerekli olmayabilir, varsayılan olabilir.

        let lvt_value = (timer_vector as u32) | (0 << 8) | (0 << 15) ; // Vektör | Sabit Teslim Modu | Maskelenmemiş
        asm!("mov eax, {lvt_val}", lvt_val = in(reg) lvt_value, options(nostack));
        asm!("mov [{reg}], eax", reg = in(reg) lvt_timer_reg, options(nostack));


        // **3. İlk Sayım Değerini Hesapla ve Ayarla:**
        //    - Gecikme süresini saat döngüsüne çevir.
        //    - x86'da `rdtsc` (Read Time-Stamp Counter) komutu saat döngülerini okuyabilir,
        //      ancak frekans değişiklikleri ve çekirdekler arası senkronizasyon sorunları olabilir.
        //    - Bu örnekte, basitlik adına `CLOCK_FREQ` sabitini kullanıyoruz.
        //    - Gerçek uygulamalarda, daha güvenilir zamanlama yöntemleri (örn., HPET, ACPI zamanlayıcıları)
        //      veya `rdtsc` kullanılıyorsa frekans kalibrasyonu ve çekirdek senkronizasyonu dikkate alınmalıdır.

        let delay_cycles = delay_ms.saturating_mul(clock_frequency) / 1000;
        // x86 Yerel APIC zamanlayıcısı 32-bitlik bir geri sayım sayacıdır.
        // u32'ye cast ederek taşmayı kontrol altında tutuyoruz.
        let initial_count = delay_cycles as u32;

        asm!("mov eax, {count}", count = in(reg) initial_count, options(nostack));
        asm!("mov [{reg}], eax", reg = in(reg) initial_count_reg, options(nostack));


        // **4. Zamanlayıcıyı Başlat:**
        //    - İlk sayım değeri ayarlandıktan sonra zamanlayıcı otomatik olarak geri saymaya başlar.
        //    - Zamanlayıcı, ilk sayım değerinden 0'a doğru geri sayar.
        //    - 0'a ulaştığında, LVT kaydında belirtilen vektör numarasıyla bir kesme üretir.
        //    - Bu kodda kesme işleme rutini (interrupt handler) bulunmamaktadır.

        // İsteğe bağlı: Şu anki sayım değerini okuyarak zamanlayıcının çalışmaya başladığını doğrulayabilirsiniz.
        // let current_count: u32;
        // asm!("mov eax, [{reg}]", reg = in(reg) current_count_reg, out("eax") current_count, options(nostack));
        // println!("Şu anki sayım değeri: {}", current_count); // Eğer standart kütüphane kullanılsaydı (no_std değilse)


    }
}


// Örnek kullanım (test veya ana fonksiyon içinde)
// Dikkat: Bu örnek kod çıplak metal ortamda veya işletim sistemi çekirdeği bağlamında çalıştırılmak üzere tasarlanmıştır.
// Standart bir işletim sistemi üzerinde çalıştırılırsa, Yerel APIC zamanlayıcısına doğrudan erişim işletim sistemi tarafından engellenebilir
// veya farklı sonuçlar verebilir.
fn main() {
    // Gecikme süresini ayarlayın
    let delay_ms = 100; // 100 milisaniye gecikme

    set_timer_x86(delay_ms);

    // ... diğer işlemler ...
    // Bu noktada, Yerel APIC zamanlayıcısı ayarlanmış ve geri saymaya başlamıştır.
    // Belirtilen gecikme süresi sonunda, TIMER_VECTOR (32) numaralı kesme üretilecektir.
    // Bir kesme işleme rutini (interrupt handler) bu kesmeyi yakalamalı ve gerekli işlemleri yapmalıdır.
    // Bu örnek kodda kesme işleme rutini bulunmamaktadır.
}