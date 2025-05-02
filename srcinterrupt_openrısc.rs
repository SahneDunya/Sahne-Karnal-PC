#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz

// Gerekli core modüllerini içeri aktar
use core::arch::asm; // Inline assembly (SPR erişimi için)
use core::ptr;      // Pointer işlemleri için

// `volatile` crate'inden Volatile sarmalayıcıyı içeri aktar (cargo.toml'da tanımlı olmalı)
use volatile::Volatile; // <-- Added import

// Konsol çıktı makrolarını kullanabilmek için (hata durumlarında loglama veya debug çıktısı için)
// Bu makrolar Sahne64 crate'i tarafından sağlanır ve resource API'sini kullanır.
// Bu crate'te kullanılabilir olmaları için uygun kurulum (örn. #[macro_use]) gereklidir.
// Bu örnekte, #[cfg] ile std/no_std çıktısını ayarlayarak makroların
// uygun ortamda kullanılabilir olduğunu varsayıyoruz.
 use sahne64::eprintln; // Örnek import eğer macro publicse


// OpenRISC mimarisine özgü ve donanıma bağımlı sabitler
// ** GERÇEK DEĞERLERİ İŞLEMCİ VE ÇEVRE BİRİMİ REFERANS KILAVUZLARINDAN ALIN! **
// Bu değerler çip modeline ve çevre birimi instancelarına göre değişir.

// OpenRISC SPR'ler (Special Purpose Registers) (Örnek OR1200 veya başka bir çekirdek)
// SPR_IVTBR: Interrupt Vector Table Base Register (Genellikle SPR 0x1B)
// SPR_SR: Status Register (Genellikle SPR 0x11)
// SPR_CSR: Cause Register veya benzeri (Kesme nedenini raporlayan register) - Donanıma özgü
// SPR isimleri ve adresleri için kullandığınız OpenRISC çekirdeği kılavuzuna bakın.

// Kesme Vektör Tablosu (IVT) adresi (spr_ivtbr'ye yazılacak adres)
// Bu adres, bellekteki kesme vektörlerinin başlangıcıdır.
const IVT_ADDRESS: usize = 0x100; // Örnek adres: Donanımınıza göre ayarlayın. Linker script ile belirlenebilir.

// Kesme Nedenleri (Cause Register'dan okunan değerlere karşılık gelen sabitler)
// Bu değerler, spr_csr (veya ilgili cause register) okunduğunda gelen bit maskeleri veya değerlerdir.
// Donanımınıza göre genişletin ve doğru bit maskelerini/değerleri kullanın.
const TIMER_INTERRUPT_CAUSE: usize = 0x01; // Örnek neden: Zamanlayıcı kesmesi biti
const USB_INTERRUPT_CAUSE: usize = 0x02; // Örnek neden: USB kesmesi biti (Donanıma özgü!)
// Diğer nedenler: Bus error, TLB miss, Syscall vb.

// USB ile ilgili donanım adresleri ve register offset'leri (DONANIMA ÖZGÜ!)
// USB_BASE_ADDRESS: USB kontrolcüsünün bellek eşlemeli (memory-mapped) temel adresi.
// USB_IRQ_CLEAR_REGISTER_OFFSET: USB kesme bayrağını temizlemek için yazılacak register ofseti.
// USB_IRQ_CLEAR_VALUE: USB kesme bayrağını temizlemek için bu register'a yazılacak değer.
// **DİKKAT:** Bu değerler ve adresler, kullandığınız OpenRISC çipine ve USB kontrolcüsüne göre DEĞİŞİR.
// Doğru değerler için çipinizin REFERANS KILAVUZUNA bakmanız ŞARTTIR.
const USB_BASE_ADDRESS: usize = 0xD000_0000; // ÖRNEK ADRES! Kılavuza bakınız!
const USB_IRQ_CLEAR_REGISTER_OFFSET: usize = 0x14; // ÖRNEK OFFSET! Kılavuza bakınız!
const USB_IRQ_CLEAR_VALUE: u32 = 1; // ÖRNEK DEĞER! Kılavuza bakınız!


/// Genel kesme işleyicisi.
/// Bu fonksiyon, bir kesme veya istisna (trap) oluştuğunda CPU tarafından çağrılır.
/// Kesme nedenini okuyarak ilgili spesifik işleyiciye dallanır.
///
/// # Güvenlik
/// Bu fonksiyon güvenli değildir (unsafe) çünkü kesme bağlamında çalışır,
/// donanım registerlarına erişir ve potansiyel olarak shared memory kullanabilir.
/// Eş zamanlılık ve reentrancy (yeniden giriş) konularına dikkat edilmelidir.
#[no_mangle] // Linker script veya donanım tarafından çağrılabilir
pub unsafe extern "C" fn interrupt_handler() {
     // ** Güvenlik: Kesme bağlamında çalışıyoruz. Registerları kaydetmek gerekebilir. **

    // 1. Kesme nedenini okumak için SPR_CSR (veya ilgili cause register) kullan.
    let cause = get_interrupt_cause();

    // 2. Kesme nedenine göre ilgili işleyiciye dallan.
    // Bu, daha karmaşık bir dispatch tablosu veya lookup yapısı içerebilir.
    match cause {
        TIMER_INTERRUPT_CAUSE => {
            handle_timer_interrupt();
        }
        USB_INTERRUPT_CAUSE => { // USB kesmesi için yeni branch eklendi
             handle_usb_interrupt();
        }
        _ => {
            // Beklenmeyen kesme durumu (isteğe bağlı işleme)
            // Sahne64 konsol makrolarını kullanarak hata logla.
             #[cfg(feature = "std")] std::eprintln!("UYARI: Beklenmeyen kesme nedeni: {}", cause);
             #[cfg(not(feature = "std"))] eprintln!("UYARI: Beklenmeyen kesme nedeni: {}", cause); // Sahne64 macro varsayımı
            // TODO: Hata ayıklama veya varsayılan işleme (örn. panik veya sistem durdurma)
              halt_system(); // Eğer tanımlıysa
        }
    }
     // TODO: Kaydedilen registerları geri yükle.
     // TODO: Kesme işleyiciden çıkış yönergesini (örn. rfe - return from exception) kullan.
      asm!("rfe"); // Dönüş yönergesi (unsafe asm)
}

// Yardımcı fonksiyon: Kesme nedenini oku
/// SPR_CSR (veya ilgili cause register)'den kesme nedenini okur.
/// # Güvenlik
/// Güvenli değildir çünkü SPR okumak privilege gerektirebilir ve kesme bağlamında çağrılır.
fn get_interrupt_cause() -> usize {
    let mut cause: usize;
    unsafe {
        // SPR_CSR (Kontrol ve Durum Register)'den kesme nedenini oku
        // 'mfspr rd, spr' yönergesi kullanılır.
        // 'spr_csr' yerine ilgili SPR adresi/numarası kullanılmalıdır.
        // ÖRNEK SPR ADI: spr_excause, spr_eesr gibi farklı registerlar olabilir. Kılavuza bakın!
        const SPR_CAUSE_REGISTER: usize = 0x1A; // Örnek SPR adresi (Kılavuza bakın!)
        asm!("mfspr {0}, {1}", out(reg) cause, const SPR_CAUSE_REGISTER, options(nostack)); // options(nostack)
    }
    cause
}

// Yardımcı fonksiyon: Zamanlayıcı kesmesini işle
/// Zamanlayıcı kesmesi gerçekleştiğinde çekirdek içinde yapılacak işlemleri barındırır.
/// # Güvenlik
/// Güvenli değildir, kesme bağlamında çalışır.
fn handle_timer_interrupt() {
    // Zamanlayıcı kesmesi işleme kodu buraya gelecek
    // Örnek: Zamanlayıcı sayacını resetleme, zamanlayıcı olayını çekirdek zamanlayıcıya bildirme, görev uyandırma
     #[cfg(feature = "std")] std::println!("Zamanlayıcı kesmesi işleniyor.");
     #[cfg(not(feature = "std"))] println!("Zamanlayıcı kesmesi işleniyor."); // Sahne64 macro varsayımı

    // TODO: Zamanlayıcı ile ilgili çekirdek işlemleri...

    clear_timer_interrupt_flag(); // Bayrağı temizle
}

// Yardımcı fonksiyon: USB kesmesini işle (Yeni eklendi)
/// USB kesmesi gerçekleştiğinde çekirdek içinde yapılacak işlemleri barındırır.
/// # Güvenlik
/// Güvenli değildir, kesme bağlamında çalışır.
unsafe fn handle_usb_interrupt() { // unsafe eklendi
     #[cfg(feature = "std")] std::println!("USB kesmesi işleniyor.");
     #[cfg(not(feature = "std"))] println!("USB kesmesi işleniyor."); // Sahne64 macro varsayımı

     // TODO: USB sürücü kodu ile etkileşime gir (veri oku/yaz, durum kontrol et).
     // Bu, çekirdekteki USB sürücüsü logic'idir.
     // resource::read veya resource::write çağrısı yapan görevleri uyandırabilir.

     clear_usb_interrupt_flag(); // Bayrağı temizle
}


// Yardımcı fonksiyon: Zamanlayıcı kesme bayrağını temizle (DONANIMA ÖZGÜ)
/// Zamanlayıcı donanımındaki kesme bayrağını temizler.
/// # Güvenlik
/// Güvenli değildir, donanım registerına yazma yapar.
fn clear_timer_interrupt_flag() {
    // TODO: Zamanlayıcı kontrol register'ının adresini ve temizleme yöntemini tanımla.
    // ÖRNEK ADRES VE YÖNTEM: Gerçek donanımınıza uygun register ve biti kullanın.
     const TIMER_CONTROL_REGISTER_ADDRESS: usize = 0x...; // Donanıma özgü adres
     const TIMER_CLEAR_BIT: u32 = 1 << ...; // Donanıma özgü temizleme biti

    unsafe {
         // Örnek: Zamanlayıcı kontrol register'ındaki ilgili biti temizle (volatile yazma)
          volatile_store!(TIMER_CONTROL_REGISTER_ADDRESS, ...); // volatile crate kullanımı

         // Ya da asm ile SPR yazma (eğer zamanlayıcı SPR üzerinden yönetiliyorsa)
          asm!("mtspr spr_timer_control, {0}", in(reg) clear_value); // Örnek SPR
         #[cfg(feature = "std")] std::println!("Zamanlayıcı kesme bayrağı temizlendi (simüle)."); // Debug çıktı
         #[cfg(not(feature = "std"))] println!("Zamanlayıcı kesme bayrağı temizlendi (simüle)."); // Debug çıktı

         // !!! DİKKAT: Aşağıdaki satır SADECE BİR ÖRNEKTİR ve ÇALIŞMAYABİLİR !!!
         // !!! Gerçek donanımınızın kılavuzuna başvurarak doğru yöntemi bulun !!!
          asm!("nop"); // Yer tutucu: Donanıma özgü bayrak temizleme komutu buraya
     }
}

// Yardımcı fonksiyon: USB kesme bayrağını temizle (DONANIMA ÖZGÜ) (Yeni eklendi)
/// USB donanımındaki kesme bayrağını temizler.
/// # Güvenlik
/// Güvenli değildir, donanım registerına yazma yapar.
unsafe fn clear_usb_interrupt_flag() { // unsafe eklendi
    // USB kesme bayrağını temizleme (ÇOK ÖNEMLİ!)
    // Kesme işlendikten sonra, kesme bayrağının TEMİZLENMESİ ZORUNLUDUR.
    // Aksi takdirde, aynı kesme sürekli olarak tekrar tetiklenir ve sistem kilitlenir.
    // **DİKKAT:** Kesme bayrağını temizleme yöntemi DONANIMA ÖZGÜDÜR.
    // Genellikle USB kontrolcüsünün kendi register'ındaki bir biti yazarak/temizleyerek yapılır.

    let usb_irq_clear_register_address = USB_BASE_ADDRESS.wrapping_add(USB_IRQ_CLEAR_REGISTER_OFFSET); // Güvenli adres hesaplama

    unsafe {
         let mut usb_irq_clear_register = Volatile::new(usb_irq_clear_register_address as *mut u32); // Örnek: 32-bit register
         usb_irq_clear_register.write(USB_IRQ_CLEAR_VALUE); // Örnek: Temizleme değerini yaz
         // **UYARI:** USB_IRQ_CLEAR_REGISTER_OFFSET ve USB_IRQ_CLEAR_VALUE DONANIMA GÖRE DEĞİŞİR.
         // Doğru yöntem için çipinizin ve USB kontrolcüsünün REFERANS KILAVUZUNA BAKIN.
         #[cfg(feature = "std")] std::println!("USB kesme bayrağı temizlendi."); // Debug çıktı
         #[cfg(not(feature = "std"))] println!("USB kesme bayrağı temizlendi."); // Debug çıktı
    }
}


/// OpenRISC mimarisi için kesme ve trap altyapısını başlatır.
/// IVTBR ve SR gibi ilgili SPR'ları ayarlar, kesmeleri etkinleştirir.
/// Bu fonksiyon, sistem başlangıcında (kernel init sürecinde) çağrılmalıdır.
pub fn init() {
    // Bu fonksiyon, Supervisor mode (S mode) veya Machine mode (M mode) gibi
    // yüksek privilege seviyesinde çalışmalıdır. OpenRISC'te bu genellikle
    // Supervisor mode'dur (SR[SM]=1).
    unsafe {
        // 1. IVTBR (Interrupt Vector Table Base Register) Register'ını Ayarlama
        // IVTBR register'ı, kesme vektör tablosunun (IVT) başlangıç adresini tutar.
        // Kesme oluştuğunda işlemci, IVTBR + ofset adresine dallanır.
        // SPR_IVTBR register'ına IVT adresini yazmak için 'mtspr' kullanılır.
        // SPR adresi olarak spr_ivtbr (0x1B) veya çipinize özgü bir değer kullanılmalıdır.
        const SPR_IVTBR: usize = 0x1B; // Örnek IVTBR SPR adresi (Kılavuza bakın!)

        asm!(
            "mtspr {0}, {1}", // mtspr spr, rs (spr: spr address, rs: source reg)
            const SPR_IVTBR,      // Hedef SPR adresi
            in(reg) IVT_ADDRESS,  // Kaynak register olarak IVT adresi sabiti
            options(nostack)      // Stack manipulation olmadığını belirtir
        );
        // ** AÇIKLAMA: IVTBR adresini sisteme bildirme işlemi DONANIMA ÖZGÜDÜR! **
        // Doğru SPR adresi ve yazma yönergesi için OpenRISC ISA ve çip kılavuzuna bakın.


        // 2. İlgili Çevre Birimi Kesmelerini Etkinleştirme (Zamanlayıcı ve USB)
        // OpenRISC'te bu genellikle SPR_SR (Status Register) içindeki EE (External Enable)
        // gibi genel bitler veya özel bir kesme denetleyici SPR'ı (örn. PICMR - PIC Mask Register)
        // kullanılarak yapılır. Bu kısım donanıma özgü en çok değişen yerdir.
        // ÖRNEK: Varsayımsal bir PICMR kaydı üzerinden maskeleme.
         const SPR_PICMR: usize = 0x...; // Örnek PIC Mask Register SPR adresi
         const PICMR_TIMER_ENABLE_BIT: usize = ...; // Zamanlayıcı biti maskesi
         const PICMR_USB_ENABLE_BIT: usize = ...; // USB biti maskesi
         let irqs_to_enable_mask = PICMR_TIMER_ENABLE_BIT | PICMR_USB_ENABLE_BIT;
         asm!("mtspr {0}, {1}", const SPR_PICMR, in(reg) irqs_to_enable_mask, options(nostack));

        // Veya Status Register (SR) içindeki ilgili bitler.
        enable_timer_interrupt(); // Zamanlayıcıyı etkinleştirme helper'ı çağır
        // USB etkinleştirme helper'ı eklenebilir.

        // Bu örnekte, enable_timer_interrupt() helper fonksiyonunun kendi içinde
        // ilgili donanım veya SPR ayarını yaptığını varsayalım.
        // USB için de benzer bir helper eklenebilir.
         enable_timer_interrupt(); // Zamanlayıcı kesmesini etkinleştir (helper içinde donanıma özgü kod var)
          enable_usb_interrupt(); // USB kesmesini etkinleştir (eğer helper'ı tanımlarsak)

        // Not: enable_timer_interrupt() ve enable_usb_interrupt() fonksiyonları
        // aslında kesmeleri donanımsal olarak (zamanlayıcı/USB kontrolcüsünde)
        // etkinleştirme logic'ini içermelidir. Bu logic SPR yazmayı veya
        // memory-mapped register yazmayı içerebilir.


        // 3. Genel Kesmeleri Etkinleştirme (Status register'daki EE biti)
        // OpenRISC'te genellikle SR (Status Register) içindeki EE (External Enable)
        // veya IE (Interrupt Enable) gibi bir bit kullanılır.
        // SPR_SR register'ını okuyun, EE bitini set edin, geri yazın.
        const SPR_SR: usize = 0x11; // Örnek SR SPR adresi (Kılavuza bakın!)
        const SR_EE_BIT: usize = 1 << 1; // Örnek EE (External Enable) bit pozisyonu (Kılavuza bakın!)

        // Status register'ı oku: mfspr rd, spr
        // $t0 geçici olarak kullanılabilecek bir register.
        let mut sr_value: usize;
        asm!("mfspr {0}, {1}", out(reg) sr_value, const SPR_SR, options(nostack));

        // Okunan Status değerine EE bitini OR'la
        sr_value |= SR_EE_BIT;

        // Yeni Status değerini geri yaz: mtspr spr, rs
        asm!("mtspr {0}, {1}", const SPR_SR, in(reg) sr_value, options(nostack));

        // **DİKKAT:** SPR_SR, SR_EE_BIT ve etkinleştirme yöntemi DONANIMA GÖRE DEĞİŞİR.
        // Doğru değerler için OpenRISC ISA ve çip kılavuzuna bakın.


        // Diğer platforma özgü başlatma adımları buraya eklenebilir.
    }
    // init fonksiyonu başarıyla tamamlanırsa geri döner.
}

// Yardımcı fonksiyon: USB kesmesini etkinleştir (DONANIMA ÖZGÜ) (Yeni eklendi)
/// USB donanımındaki kesme üretme yeteneğini etkinleştirir.
/// # Güvenlik
/// Güvenli değildir, donanım registerına yazma yapar.

unsafe fn enable_usb_interrupt() {
     #[cfg(feature = "std")] std::println!("USB kesmesi etkinleştiriliyor (simüle).");
     #[cfg(not(feature = "std"))] println!("USB kesmesi etkinleştiriliyor (simüle)."); // Sahne64 macro varsayımı

    // TODO: USB kontrolcüsünün kesme etkinleştirme register'ının adresini ve bitini tanımla.
    // ÖRNEK ADRES VE BİT: Gerçek donanımınıza uygun register ve biti kullanın.
     const USB_CONTROL_REGISTER_ADDRESS: usize = 0x...; // Donanıma özgü adres
     const USB_ENABLE_BIT: u32 = 1 << ...; // Donanıma özgü etkinleştirme biti

    // volatile yazma ile etkinleştirme
     volatile_store!(USB_CONTROL_REGISTER_ADDRESS, ...);

    // Veya asm ile SPR yazma (eğer USB SPR üzerinden yönetiliyorsa)
     asm!("mtspr spr_usb_control, {0}", in(reg) enable_value); // Örnek SPR

     // !!! BU KISIM DONANIMA ÖZGÜ KOD İLE DOLDURULMALIDIR. !!!
}
