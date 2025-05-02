#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz

// Gerekli core modüllerini içeri aktar
use core::{arch::asm, ptr, mem}; // Added mem for size_of

// `volatile` crate'inden Volatile sarmalayıcıyı içeri aktar (cargo.toml'da tanımlı olmalı)
use volatile::Volatile; // <-- Added import

// Konsol çıktı makrolarını kullanabilmek için (hata durumlarında loglama veya debug çıktısı için)
// Bu makrolar Sahne64 crate'i tarafından sağlanır ve resource API'sini kullanır.
// Bu crate'te kullanılabilir olmaları için uygun kurulum (örn. #[macro_use]) gereklidir.
// Bu örnekte, #[cfg] ile std/no_std çıktısını ayarlayarak makroların
// uygun ortamda kullanılabilir olduğunu varsayıyoruz.
 use sahne64::eprintln; // Örnek import eğer macro publicse


// PowerPC mimarisine özgü ve donanıma bağımlı sabitler
// ** GERÇEK DEĞERLERİ İŞLEMCİ VE ÇEVRE BİRİMİ REFERANS KILAVUZLARINDAN ALIN! **
// Bu değerler çip modeline, çevre birimi instancelarına ve kesme kontrolcüsüne göre değişir.

// Kesme vektör tablosu (IVT) için adres ve boyut tanımları
// Bu, donanımın exception/interrupt oluştuğunda zıpladığı bellekteki tablodur.
// Adres ve format işlemci modeline (Book E, Book S vb.) ve konfigürasyona bağlıdır.
const IVT_BASE: usize = 0x1000; // Örnek adres: Gerçek donanıma göre ayarlanmalı (Genellikle 0x100, 0x1000, 0xFFF00000 vb.)
const IVT_SIZE: usize = 256 * mem::size_of::<usize>(); // Kesme vektör tablosu boyutu (Örnek: 256 vektör girişi * pointer boyutu)
// Her girişin boyutu ve anlamı PowerPC modeli ve IVT formatına bağlıdır (komut veya işleyici adresi).
// Bu örnekte her girişin bir işleyici adresini tutan 'usize' olduğunu varsayıyoruz.


// Kesme numaraları (IRQ numaraları)
// Bu numaralar, genellikle harici bir Interrupt Controller (örn. PIC/VIC benzeri) tarafından atanır.
// Bu numaralar, IVT_BASE'e göre ofset olarak kullanılır (IRQ_NUMBER * entry_size).
const TIMER_IRQ: usize = 0; // Örnek zamanlayıcı kesme numarası (Offset = 0 * size_of<usize>)
const USB_IRQ: usize = 1; // Örnek USB kesme numarası (Offset = 1 * size_of<usize>)
// Diğer vektörler (System Call, Alignment, Data/Instruction Access vb.) farklı ofsetlerde bulunur.


// Zamanlayıcı ile ilgili donanım adresleri ve register offset'leri (DONANIMA ÖZGÜ!)
// TIMER_CONTROL_REGISTER_OFFSET: Zamanlayıcı kontrol register'ının temel adrese göre offset'i.
// TIMER_IRQ_CLEAR_VALUE: Zamanlayıcı kesme bayrağını temizlemek için yazılacak değer.
// **DİKKAT:** Bu değerler ve adresler, kullandığınız PowerPC çipine ve zamanlayıcı donanımına göre DEĞİŞİR.
// Doğru değerler için çipinizin REFERANS KILAVUZUNA bakmanız ŞARTTIR.
const TIMER_BASE_ADDRESS: usize = 0xE000_0000; // ÖRNEK ADRES! Kılavuza bakınız!
const TIMER_CONTROL_REGISTER_OFFSET: usize = 0x00; // ÖRNEK OFFSET! Kılavuza bakınız!
const TIMER_IRQ_CLEAR_VALUE: u32 = 1; // ÖRNEK DEĞER! Kılavuza bakınız! (32-bit yazma varsayımı)


// USB ile ilgili donanım adresleri ve register offset'leri (DONANIMA ÖZGÜ!)
// USB_BASE_ADDRESS: USB kontrolcüsünün bellek eşlemeli (memory-mapped) temel adresi.
// USB_IRQ_CLEAR_REGISTER_OFFSET: USB kesme bayrağını temizlemek için yazılacak register ofseti.
// USB_IRQ_CLEAR_VALUE: USB kesme bayrağını temizlemek için bu register'a yazılacak değer.
// **DİKKAT:** Bu değerler ve adresler, kullandığınız PowerPC çipine ve USB kontrolcüsüne göre DEĞİŞİR.
// Doğru değerler için çipinizin REFERANS KILAVUZUNA bakmanız ŞARTTIR.
const USB_BASE_ADDRESS: usize = 0xE000_1000; // ÖRNEK ADRES! Kılavuza bakınız!
const USB_IRQ_CLEAR_REGISTER_OFFSET: usize = 0x14; // ÖRNEK OFFSET! Kılavuza bakınız!
const USB_IRQ_CLEAR_VALUE: u32 = 1; // ÖRNEK DEĞER! Kılavuza bakınız!


/// PowerPC mimarisi için kesme ve trap altyapısını başlatır.
/// IVT'yi kurar, işleyici adreslerini yazar ve genel kesmeleri etkinleştirir.
/// Bu fonksiyon, sistem başlangıcında (kernel init sürecinde) çağrılmalıdır.
pub fn init() {
    // Bu fonksiyon, Supervisor mode gibi yüksek privilege seviyesinde çalışmalıdır.
    unsafe {
        // 1. Kesme Vektör Tablosunu (IVT) Başlatma ve İşleyici Adreslerini Yazma
        // IVT, IVT_BASE adresinde bulunur. Her giriş bir 'usize' (pointer) boyutundadır.
        let ivt_ptr = IVT_BASE as *mut usize;

        // Başlangıçta tüm IVT girişlerini sıfırlayalım (iyi bir pratiktir)
        let num_entries = IVT_SIZE / mem::size_of::<usize>();
        for i in 0..num_entries {
             // Volatile yazma kullanarak derleyicinin optimizasyon yapmasını engelle
            ptr::write_volatile(ivt_ptr.add(i), 0);
        }

        // Zamanlayıcı kesme işleyicisinin adresini IVT'deki yerine yaz
        let timer_handler_addr = timer_interrupt_handler as unsafe extern "C" fn() as usize; // İşleyici adresi al
        // Güvenli olmayan (unsafe) volatile yazma ile adresi IVT_BASE + (TIMER_IRQ * size_of<usize>)'ye yaz.
        ptr::write_volatile(ivt_ptr.add(TIMER_IRQ), timer_handler_addr);

        // USB kesme işleyicisinin adresini IVT'deki yerine yaz
        let usb_handler_addr = usb_interrupt_handler as unsafe extern "C" fn() as usize; // İşleyici adresi al
        ptr::write_volatile(ivt_ptr.add(USB_IRQ), usb_handler_addr);

        // Diğer exception handler adresleri (System Call, Alignment, vs.) de buraya yazılmalıdır.


        // TODO: Harici Kesme Denetleyicisini (PIC/VIC benzeri) Yapılandırma
        // Eğer PowerPC'niz harici bir kesme denetleyicisi kullanıyorsa,
        // bu denetleyicinin registerlarını ayarlayarak ilgili IRQ'ları etkinleştirmeniz gerekir.
        // Bu kısım DONANIMA ÖZGÜDÜR.


        // 2. Genel Kesmeleri Etkinleştirme (MSR - Machine State Register)
        // MSR register'ındaki IE (Interrupt Enable) bitini (veya ilgili bitleri) set edin.
        // MSR'ın numarası ve IE bitinin pozisyonu PowerPC modeline bağlıdır.
        // Örnek: MSR CP0 register 1, IE bit 15 (Bazı modellerde farklı olabilir)
        const SPR_MSR: usize = 1; // Örnek MSR SPR numarası (Kılavuza bakın!)
        const MSR_IE_BIT: usize = 1 << 15; // Örnek IE bit pozisyonu (Kılavuza bakın!)

        unsafe {
             // MSR'ı oku: mfspr rt, spr (rt: dest reg, spr: spr num)
             let mut msr_value: usize;
             asm!("mfspr {0}, {1}", out(reg) msr_value, const SPR_MSR, options(nostack));

             // Okunan MSR değerine IE bitini OR'la
             msr_value |= MSR_IE_BIT;

             // Yeni MSR değerini geri yaz: mtspr spr, rs (spr: spr num, rs: source reg)
             asm!("mtspr {0}, {1}", const SPR_MSR, in(reg) msr_value, options(nostack));

             // **DİKKAT:** SPR_MSR, MSR_IE_BIT ve etkinleştirme yöntemi DONANIMA GÖRE DEĞİŞİR.
             // Doğru değerler için PowerPC ISA ve çip kılavuzuna bakın.
        }


        // Diğer platforma özgü başlatma adımları buraya eklenebilir.
    }
    // init fonksiyonu başarıyla tamamlanırsa geri döner.
}

// Zamanlayıcı kesme işleyicisi (DONANIMA ÖZGÜ UYGULAMA GEREKLİ! - ÖRNEK YAPI)
// Bu fonksiyon, IVT'den veya bir merkezi exception dispatcher'ından çağrılır.
#[no_mangle] // Linker script veya IVT/dispatcher tarafından çağrılabilir
// Kesme işleyici fonksiyonları genellikle 'unsafe extern "C"' olarak tanımlanır.
pub unsafe extern "C" fn timer_interrupt_handler() {
    // ** Güvenlik: Bu işleyici unsafe'dir çünkü kesme bağlamında çalışır **
    // ve donanımla doğrudan etkileşime girer. Yarış durumları ve side effect'lere dikkat!
    // TODO: Kesme bağlamında registerları kaydet!

    // ** DİKKAT: Bu bölüm DONANIMA ÖZGÜDÜR ve ZAMANLAYICI KILAVUZUNA GÖRE KODLANMALIDIR! **
    // Bu işleyici çekirdek içinde çalışır, kullanıcı alanındaki Sahne64 API'sini doğrudan çağırmaz.
    // Periyodik zamanlayıcı olayını çekirdek zamanlayıcıya bildirir.

    // 1. Zamanlayıcı ile ilgili işlemleri gerçekleştir
    // Örnek: Zamanlayıcı sayacını resetleme veya bir sonraki kesme zamanını ayarlama
    // Bu genellikle donanım registerlarına yazarak yapılır (volatile kullanarak).
     let timer_control_register_address = TIMER_BASE_ADDRESS.wrapping_add(TIMER_CONTROL_REGISTER_OFFSET); // Güvenli adres hesaplama
     let mut timer_control_register = Volatile::new(timer_control_register_address as *mut u32); // Örnek: 32-bit register
     // timer_control_register.write(...); // Sayacı resetleme veya ayar değeri yazma

     // Sahne64 çekirdek zamanlayıcısını güncelle (çekirdek içindeki bir fonksiyona çağrı)
      kernel_timer_tick(); // Varsayımsal çekirdek fonksiyonu

     // Debug çıktıları için Sahne64 konsol makrolarını kullan
     #[cfg(feature = "std")] std::println!("Zamanlayıcı kesmesi işleniyor.");
     #[cfg(not(feature = "std"))] println!("Zamanlayıcı kesmesi işleniyor."); // Sahne64 macro varsayımı


    // 2. Kesme bayrağını temizle (donanıma özgü). BU ÇOK ÖNEMLİDİR!
    // Kesme işlendikten sonra, kesme bayrağının TEMİZLENMESİ ZORUNLUDUR.
    // Bu genellikle zamanlayıcı kontrol register'ındaki bir biti yazarak/temizleyerek yapılır.
    unsafe {
         let timer_irq_clear_register_address = TIMER_BASE_ADDRESS.wrapping_add(TIMER_CONTROL_REGISTER_OFFSET); // Genellikle aynı register veya farklı bir offset
         let mut timer_irq_clear_register = Volatile::new(timer_irq_clear_register_address as *mut u32);
         timer_irq_clear_register.write(TIMER_IRQ_CLEAR_VALUE as u32); // Temizleme değerini yaz (u32 varsayımı)
         // **UYARI:** Temizleme register adresi/ofseti ve temizleme değeri DONANIMA GÖRE DEĞİŞİR.
    }

    // TODO: Kaydedilen registerları geri yükle.
    // TODO: Kesme işleyiciden çıkış yönergesini kullan (örn. rfi - return from interrupt).
     asm!("rfi"); // Dönüş yönergesi (unsafe asm)
}

// USB kesme işleyicisi (DONANIMA ÖZGÜ UYGULAMA GEREKLİ! - ÖRNEK YAPI)
// Bu fonksiyon, IVT'den veya bir merkezi exception dispatcher'ından çağrılır.
#[no_mangle] // Linker script veya IVT/dispatcher tarafından çağrılabilir
// Kesme işleyicisi fonksiyonları genellikle 'unsafe extern "C"' olarak tanımlanır.
pub unsafe extern "C" fn usb_interrupt_handler() {
    // ** Güvenlik: Bu işleyici unsafe'dir çünkü kesme bağlamında çalışır **
    // ve donanımla doğrudan etkileşime girer. Yarış durumları ve side effect'lere dikkat!
    // TODO: Kesme bağlamında registerları kaydet!

    // ** DİKKAT: Bu bölüm DONANIMA ÖZGÜDÜR ve USB KONTROLCÜSÜ KILAVUZUNA GÖRE KODLANMALIDIR! **
    // Bu işleyici çekirdek içinde çalışır, kullanıcı alanındaki Sahne64 API'sini doğrudan çağırmaz.
    // Gelen USB verisini alır ve çekirdekteki USB sürücüsü koduna iletir.
    // Ardından, bu veriyi bekleyen kullanıcı görevini Sahne64 çekirdek zamanlama mekanizması
    // aracılığıyla uyandırır (örn. resource::read için bekleyen görev).

    // 1. USB ile ilgili işlemleri gerçekleştir
    // Örnek: Veri okuma, veri gönderme, durum kontrolü vb.
    // Bu genellikle donanım registerlarına volatile okuma/yazma ile yapılır.
     // TODO: USB durum register'ını oku, nedeni belirle (veri geldi mi, TX bitti mi vb.)
     // TODO: Veriyi USB kontrolcüsünden oku veya gönderilecek veriyi kontrolcüye yaz.

     // Debug çıktıları için Sahne64 konsol makrolarını kullan
     #[cfg(feature = "std")] std::println!("USB kesmesi işleniyor.");
     #[cfg(not(feature = "std"))] println!("USB kesmesi işleniyor."); // Sahne64 macro varsayımı

    // 2. Kesme bayrağını temizle (donanıma özgü). BU ÇOK ÖNEMLİDİR!
    // Kesme işlendikten sonra, kesme bayrağının TEMİZLENMESİ ZORUNLUDUR.
    // Bu genellikle USB kontrolcüsünün kendi register'ındaki bir biti yazarak/temizleyerek yapılır.
    unsafe {
         let usb_irq_clear_register_address = USB_BASE_ADDRESS.wrapping_add(USB_IRQ_CLEAR_REGISTER_OFFSET); // Güvenli adres hesaplama
         let mut usb_irq_clear_register = Volatile::new(usb_irq_clear_register_address as *mut u32); // Örnek: 32-bit register
         usb_irq_clear_register.write(USB_IRQ_CLEAR_VALUE as u32); // Temizleme değerini yaz (u32 varsayımı)
         // **UYARI:** Temizleme register adresi/ofseti ve temizleme değeri DONANIMA GÖRE DEĞİŞİR.
    }

    // TODO: Kaydedilen registerları geri yükle.
    // TODO: Kesme işleyiciden çıkış yönergesini kullan (örn. rfi - return from interrupt).
     asm!("rfi"); // Dönüş yönergesi (unsafe asm)
}
