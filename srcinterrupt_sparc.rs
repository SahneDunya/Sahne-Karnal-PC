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
// use sahne64::eprintln; // Örnek import eğer macro publicse


// SPARC mimarisine özgü ve donanıma bağımlı sabitler
// ** GERÇEK DEĞERLERİ İŞLEMCİ VE ÇEVRE BİRİMİ REFERANS KILAVUZLARINDAN ALIN! **
// Bu değerler çip modeline, çevre birimi instancelarına ve kesme kontrolcüsüne göre değişir.

// **Örnek: SPARC Kontrol Registerları (SPR'ler)**
// PSR (Processor State Register): Genel durum ve kesme etkinleştirme bitleri (örn. ET, PS, S, EF, EC)
// WIM (Window Invalid Mask): Geçerli olmayan register pencerelerini maskeleme
// TBR (Trap Base Register): Kesme/İstisna Vektör Tablosu (IVT) başlangıç adresi
// **Bu SPR'lere erişim genellikle `rd` ve `wr` yönergeleri ile veya memory-mapped erişimle yapılır. Kılavuza bakın!**

// Kesme vektör tablosu (IVT) için adres ve boyut tanımları
// Bu, donanımın exception/interrupt oluştuğunda zıpladığı bellekteki tablodur.
// Adres ve format işlemci modeline (Book E, Book S vb.) ve konfigürasyona bağlıdır.
// TBR register'ı bu adresin başlangıcını işaret eder.
const IVT_BASE: usize = 0x1000; // Örnek adres: Gerçek donanıma göre ayarlanmalı (Genellikle 0x0, 0x1000, 0xFFF00000 vb.)
const IVT_SIZE: usize = 256 * mem::size_of::<usize>(); // Kesme vektör tablosu boyutu (Örnek: 256 vektör girişi * pointer boyutu)
// Her girişin boyutu ve anlamı PowerPC modeli ve IVT formatına bağlıdır (komut veya işleyici adresi).
// Bu örnekte her girişin bir işleyici adresini tutan 'usize' olduğunu varsayıyoruz.


// Kesme numaraları (IRQ numaraları)
// Bu numaralar, genellikle harici bir Interrupt Controller (örn. PIC/VIC benzeri) tarafından atanır.
// Bu numaralar, IVT_BASE'e göre ofset olarak kullanılır (IRQ_NUMBER * entry_size).
const TIMER_IRQ: usize = 0; // Örnek zamanlayıcı kesme numarası (Offset = 0 * size_of<usize>)
const USB_IRQ: usize = 1; // Örnek USB kesme numarası (Offset = 1 * size_of<usize>)
// Diğer vektörler (System Call, Alignment, Data/Instruction Access vb.) farklı ofsetlerde bulunur. Trap Base Register (TBR) offsetleri SPARC'ta standarttır.


// SPARC Processor State Register (PSR) Bitleri (Örnek)
const PSR_ET_BIT: usize = 1 << 4; // Enable Traps (Dahili/Harici kesmeleri/istisnaları etkinleştirir)
const PSR_PS_BIT: usize = 1 << 2; // Previous Supervisor (Önceki privilege seviyesi)
const PSR_S_BIT: usize = 1 << 1; // Supervisor (Current privilege seviyesi)
// Diğer PSR bitleri (EC, EF, CWP vb.)


// Zamanlayıcı ile ilgili donanım adresleri ve register offset'leri (DONANIMA ÖZGÜ!)
// TIMER_BASE_ADDRESS: Zamanlayıcı kontrolcüsünün temel adresi.
// TIMER_CONTROL_REGISTER_OFFSET: Zamanlayıcı kontrol register'ının temel adrese göre offset'i.
// TIMER_IRQ_CLEAR_VALUE: Zamanlayıcı kesme bayrağını temizlemek için yazılacak değer.
// **DİKKAT:** Bu değerler ve adresler, kullandığınız SPARC çipine ve zamanlayıcı donanımına göre DEĞİŞİR.
// Doğru değerler için çipinizin REFERANS KILAVUZUNA bakmanız ŞARTTIR.
const TIMER_BASE_ADDRESS: usize = 0xD000_0000; // ÖRNEK ADRES! Kılavuza bakınız!
const TIMER_CONTROL_REGISTER_OFFSET: usize = 0x00; // ÖRNEK OFFSET! Kılavuza bakınız!
const TIMER_IRQ_CLEAR_VALUE: u32 = 1; // ÖRNEK DEĞER! Kılavuza bakınız! (32-bit yazma varsayımı)


// USB ile ilgili donanım adresleri ve register offset'leri (DONANIMA ÖZGÜ!)
// USB_BASE_ADDRESS: USB kontrolcüsünün bellek eşlemeli (memory-mapped) temel adresi.
// USB_IRQ_CLEAR_REGISTER_OFFSET: USB kesme bayrağını temizlemek için yazılacak register ofseti.
// USB_IRQ_CLEAR_VALUE: USB kesme bayrağını temizlemek için bu register'a yazılacak değer.
// **DİKKAT:** Bu değerler ve adresler, kullandığınız SPARC çipine ve USB kontrolcüsüne göre DEĞİŞİR.
// Doğru değerler için çipinizin REFERANS KILAVUZUNA bakmanız ŞARTTIR.
const USB_BASE_ADDRESS: usize = 0xD000_1000; // ÖRNEK ADRES! Kılavuza bakınız!
const USB_IRQ_CLEAR_REGISTER_OFFSET: usize = 0x14; // ÖRNEK OFFSET! Kılavuza bakınız!
const USB_IRQ_CLEAR_VALUE: u32 = 1; // ÖRNEK DEĞER! Kılavuza bakınız!


// Kesme İşleyici Fonksiyonları (Prototip tanımları gereksiz, fonksiyonlar aşağıda)
 extern "C" { fn timer_interrupt_handler(); fn usb_interrupt_handler(); } // <-- Kaldırıldı

// Boş Kesme İşleyicisi (Varsayılan)
// IVT'de boş bırakılan girişlere atanan varsayılan işleyici.
// Güvenli olmayan (unsafe) extern "C" fn olarak tanımlanmalıdır.
unsafe extern "C" fn empty_handler() {
    // ** Güvenlik: Bu işleyici unsafe'dir çünkü kesme bağlamında çalışır **
    // ve muhtemelen donanım veya paylaşılan bellekle etkileşime girecektir.

    // Beklenmeyen bir kesme/istisna durumu. Sahne64 konsol makrolarını kullanarak logla.
    // Kesme işleyicileri hassas bir bağlamda çalışır, loglama dikkatli yapılmalıdır.
    // Loglama fonksiyonu interrupt-safe olmalıdır.
    #[cfg(feature = "std")] std::eprintln!("UYARI: SPARC Beklenmeyen kesme/istisna!");
    #[cfg(not(feature = "std"))] eprintln!("UYARI: SPARC Beklenmeyen kesme/istisna!"); // Sahne64 macro varsayımı

    // TODO: Hata durumunu logla veya sistemi güvenli bir duruma getir.
    // Panik, debug amaçlı kullanılabilir ancak kesme bağlamında güvenliği düşünülmelidir.
     panic!("Beklenmeyen kesme/istisna!"); // Eğer panik güvenliyse

    // TODO: Bu işleyici exception entry noktasından çağrılıyorsa, buradan exception entry'ye dönmesi gerekebilir.
    // Eğer doğrudan IVT'den çağrılıyorsa, registerları kurtarıp rfe ile dönmelidir (bu karmaşıktır).
    // Basitlik adına şimdilik boş bırakılıyor, trap entry noktasının burayı çağırdığı varsayımıyla.
}


/// SPARC mimarisi için kesme ve trap altyapısını başlatır.
/// TBR ve PSR gibi ilgili SPR'ları ayarlar, IVT'yi kurar ve kesmeleri etkinleştirir.
/// Bu fonksiyon, sistem başlangıcında (kernel init sürecinde) Supervisor mode'da çağrılmalıdır.
pub fn init() {
    // Bu fonksiyon, Supervisor mode (S mode) privilege seviyesinde çalışmalıdır.
    unsafe {
        // 1. Kesme Vektör Tablosunu (IVT) Başlatma ve İşleyici Adreslerini Yazma
        // IVT, `static mut IVT` dizisi olarak bellekte tanımlanmıştır.
        // TBR register'ı bu dizinin adresini işaret etmelidir.
        let ivt_ptr = IVT.as_mut_ptr(); // statik mutable dizinin pointer'ı

        // Başlangıçta tüm IVT girişlerini `empty_handler` olarak başlat
        let num_entries = IVT.len(); // IVT_SIZE / mem::size_of::<usize>(); // static mut dizinin boyutu doğrudan alınabilir
        for i in 0..num_entries {
             // Volatile yazma kullanarak derleyicinin optimizasyon yapmasını engelle
            ptr::write_volatile(ivt_ptr.add(i), empty_handler); // Varsayılan handler'ı yaz
        }

        // Zamanlayıcı kesme işleyicisinin adresini IVT'deki yerine yaz
        let timer_handler_fn_ptr = timer_interrupt_handler as unsafe extern "C" fn(); // İşleyici fonksiyon pointer'ı
        // Güvenli olmayan (unsafe) volatile yazma ile adresi IVT_BASE + (TIMER_IRQ * entry_size)'ye yaz.
        // Eğer IVT'nin her girişi bir pointer ise, ofset TIMER_IRQ'dur.
         if TIMER_IRQ < num_entries {
            ptr::write_volatile(ivt_ptr.add(TIMER_IRQ), timer_handler_fn_ptr);
         } else {
             #[cfg(feature = "std")] std::eprintln!("KRİTİK HATA: SPARC Timer IRQ numarası ({}) IVT boyutundan ({}) büyük!", TIMER_IRQ, num_entries);
             #[cfg(not(feature = "std"))] eprintln!("KRİTİK HATA: SPARC Timer IRQ numarası ({}) IVT boyutundan ({}) büyük!", TIMER_IRQ, num_entries);
              loop { core::hint::spin_loop(); } // Veya halt_system();
         }


        // USB kesme işleyicisinin adresini IVT'deki yerine yaz
        let usb_handler_fn_ptr = usb_interrupt_handler as unsafe extern "C" fn(); // İşleyici fonksiyon pointer'ı
         if USB_IRQ < num_entries {
            ptr::write_volatile(ivt_ptr.add(USB_IRQ), usb_handler_fn_ptr);
         } else {
             #[cfg(feature = "std")] std::eprintln!("KRİTİK HATA: SPARC USB IRQ numarası ({}) IVT boyutundan ({}) büyük!", USB_IRQ, num_entries);
             #[cfg(not(feature = "std"))] eprintln!("KRİTİK HATA: SPARC USB IRQ numarası ({}) IVT boyutundan ({}) büyük!", USB_IRQ, num_entries);
              loop { core::hint::spin_loop(); } // Veya halt_system();
         }

        // Diğer exception handler adresleri (System Call, Alignment, vs.) de buraya yazılmalıdır.
        // TBR offsetleri SPARC'ta sabittir (örn. Syscall = TBR + 0x80, TLB Miss = TBR + 0x40 vb.).


        // 2. TBR (Trap Base Register) Register'ını Ayarlama
        // TBR register'ına IVT dizisinin adresini yaz.
        // SPARC'ta TBR'ye yazmak için özel bir yönerge kullanılır (örn. `wr %psr, %g0, TBR_value` veya özel bir SPR yazma).
        // TBR'ye yazma genellikle `wr TBR, %g0, ivt_address` gibi görünür.
        // TBR adresi/numarası SPARC modeline bağlıdır.
        const SPR_TBR: usize = 0x00; // Örnek TBR SPR adresi (Kılavuza bakın!)

        asm!(
            "wr {0}, %g0, {1}", // wr spr, %g0, value (value'yi spr'ye yaz)
            const SPR_TBR,      // Hedef SPR (TBR)
            in(reg) ivt_ptr,  // Yazılacak değer (IVT adresi)
            options(nostack)      // Stack manipulation olmadığını belirtir
        );
        // ** AÇIKLAMA: TBR adresini sisteme bildirme işlemi DONANIMA ÖZGÜDÜR! **
        // Doğru SPR adresi, yazma yönergesi ve %g0 kullanımı için SPARC ISA ve çip kılavuzuna bakın.


        // 3. Genel Kesmeleri Etkinleştirme (PSR - Processor State Register)
        // PSR register'ındaki ET (Enable Traps) bitini set edin.
        // PSR'ı okuyun (`rd %psr, register`), ET bitini set edin, geri yazın (`wr register, %g0, %psr`).
        const SPR_PSR: usize = 0x10; // Örnek PSR SPR adresi (Kılavuza bakın!)

        // PSR'ı oku: rd rd, spr (rd: dest reg, spr: spr num)
        let mut psr_value: usize;
        asm!("rd {0}, {1}", out(reg) psr_value, const SPR_PSR, options(nostack));

        // Okunan PSR değerine ET bitini OR'la
        psr_value |= PSR_ET_BIT;

        // Yeni PSR değerini geri yaz: wr spr, %g0, rs (spr: spr num, %g0: zero, rs: source reg)
        asm!("wr {0}, %g0, {1}", const SPR_PSR, in(reg) psr_value, options(nostack));

        // **DİKKAT:** SPR_PSR, PSR_ET_BIT ve etkinleştirme yöntemi DONANIMA GÖRE DEĞİŞİR.
        // Doğru değerler için SPARC ISA ve çip kılavuzuna bakın.


        // TODO: Belirli Kesme Kaynaklarını Etkinleştirme (Eğer PSR.ET yeterli değilse)
        // Bazı SPARC sistemlerinde, PSR.ET sadece genel trap işleme mekanizmasını etkinleştirir.
        // Bireysel IRQ kaynaklarını (zamanlayıcı, USB) etkinleştirmek için
        // harici bir kesme denetleyicisinin registerlarını (örn. PIC/VIC benzeri)
        // ayarlamanız gerekebilir.
        // Bu kısım DONANIMA ÖZGÜDÜR.


        // Diğer platforma özgü başlatma adımları buraya eklenebilir.
    }

    // İstisna işleyicisini başlat. exception::init() SPARC'a özgü
    // istisna işleme mekanizmalarını kurmalıdır (örneğin, register penceresi yönetimi).
     exception::init(); // Çekirdek exception init'ini çağır

     // init fonksiyonu başarıyla tamamlanırsa geri döner.
}


// Zamanlayıcı kesme işleyicisi (DONANIMA ÖZGÜ UYGULAMA GEREKLİ! - ÖRNEK YAPI)
// Bu fonksiyon, IVT'den veya bir merkezi trap dispatcher'ından çağrılır.
#[no_mangle] // Linker script veya IVT/dispatcher tarafından çağrılabilir
// Kesme işleyici fonksiyonları genellikle 'unsafe extern "C"' olarak tanımlanır.
pub unsafe extern "C" fn timer_interrupt_handler() {
    // ** Güvenlik: Bu işleyici unsafe'dir çünkü kesme bağlamında çalışır **
    // ve donanımla doğrudan etkileşime girer. Yarış durumları ve side effect'lere dikkat!
    // ** ÖNEMLİ: SPARC'ta trap işleyicileri register pencerelerini kaydetmelidir! **
    // save/restore veya trap entry noktasında otomatik kaydetme (tstate.cwp) yapılmalıdır.

    // ** DİKKAT: Bu bölüm DONANIMA ÖZGÜDÜR ve ZAMANLAYICI KILAVUZUNA GÖRE KODLANMALIDIR! **
    // Bu işleyici çekirdek içinde çalışır, kullanıcı alanındaki Sahne64 API'sini doğrudan çağırmaz.
    // Periyodik zamanlayıcı olayını çekirdek zamanlayıcıya bildirir.

    // 1. Zamanlayıcı ile ilgili işlemleri gerçekleştir
    // Örnek: Zamanlayıcı sayacını resetleme veya bir sonraki kesme zamanını ayarlama
    // Bu genellikle donanım registerlarına yazarak yapılır (volatile kullanarak).
     let timer_control_register_address = TIMER_BASE_ADDRESS.wrapping_add(TIMER_CONTROL_REGISTER_OFFSET); // Güvenli adres hesaplama
     let mut timer_control_register = Volatile::new(timer_control_register_address as *mut u32); // Örnek: 32-bit register
      timer_control_register.write(...); // Sayacı resetleme veya ayar değeri yazma

     // Sahne64 çekirdek zamanlayıcısını güncelle (çekirdek içindeki bir fonksiyona çağrı)
      kernel_timer_tick(); // Varsayımsal çekirdek fonksiyonu

     // Debug çıktıları için Sahne64 konsol makrolarını kullan
     #[cfg(feature = "std")] std::println!("SPARC Zamanlayıcı kesmesi işleniyor.");
     #[cfg(not(feature = "std"))] println!("SPARC Zamanlayıcı kesmesi işleniyor."); // Sahne64 macro varsayımı


    // 2. Kesme bayrağını temizle (donanıma özgü). BU ÇOK ÖNEMLİDİR!
    // Kesme işlendikten sonra, kesme bayrağının TEMİZLENMESİ ZORUNLUDUR.
    // Bu genellikle zamanlayıcı kontrol register'ındaki bir biti yazarak/temizleyerek yapılır.
    unsafe {
         let timer_irq_clear_register_address = TIMER_BASE_ADDRESS.wrapping_add(TIMER_CONTROL_REGISTER_OFFSET); // Genellikle aynı register veya farklı bir offset
         let mut timer_irq_clear_register = Volatile::new(timer_irq_clear_register_address as *mut u32);
         timer_irq_clear_register.write(TIMER_IRQ_CLEAR_VALUE as u32); // Temizleme değerini yaz (u32 varsayımı)
         // **UYARI:** Temizleme register adresi/ofseti ve temizleme değeri DONANIMA GÖRE DEĞİŞİR.
    }

    // TODO: Kesme işleyiciden çıkış. Kaydedilen registerları geri yükle.
    // TODO: rfe (return from exception) yönergesini kullan. Bu, PSR'nin PS/S bitlerini günceller ve CWP'yi önceki pencereye ayarlar.
     asm!("rfe"); // Dönüş yönergesi (unsafe asm)
    // NOT: Eğer exception entry noktası register kaydı ve rfe'yi yapıyorsa,
    // bu handler sadece işini yapıp normal geri dönebilir.
}

// USB kesme işleyicisi (DONANIMA ÖZGÜ UYGULAMA GEREKLİ! - ÖRNEK YAPI)
// Bu fonksiyon, IVT'den veya bir merkezi trap dispatcher'ından çağrılır.
#[no_mangle] // Linker script veya IVT/dispatcher tarafından çağrılabilir
// Kesme işleyicisi fonksiyonları genellikle 'unsafe extern "C"' olarak tanımlanır.
pub unsafe extern "C" fn usb_interrupt_handler() {
    // ** Güvenlik: Bu işleyici unsafe'dir çünkü kesme bağlamında çalışır **
    // ve donanımla doğrudan etkileşime girer. Yarış durumları ve side effect'lere dikkat!
    // ** ÖNEMLİ: SPARC'ta trap işleyicileri register pencerelerini kaydetmelidir! **

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
     #[cfg(feature = "std")] std::println!("SPARC USB kesmesi işleniyor.");
     #[cfg(not(feature = "std"))] println!("SPARC USB kesmesi işleniyor."); // Sahne64 macro varsayımı


    // 2. Kesme bayrağını temizle (donanıma özgü). BU ÇOK ÖNEMLİDİR!
    // Kesme işlendikten sonra, kesme bayrağının TEMİZLENMESİ ZORUNLUDUR.
    // Bu genellikle USB kontrolcüsünün kendi register'ındaki bir biti yazarak/temizleyerek yapılır.
    unsafe {
         let usb_irq_clear_register_address = USB_BASE_ADDRESS.wrapping_add(USB_IRQ_CLEAR_REGISTER_OFFSET); // Güvenli adres hesaplama
         let mut usb_irq_clear_register = Volatile::new(usb_irq_clear_register_address as *mut u32); // Örnek: 32-bit register
         usb_irq_clear_register.write(USB_IRQ_CLEAR_VALUE as u32); // Temizleme değerini yaz (u32 varsayımı)
         // **UYARI:** Temizleme register adresi/ofseti ve temizleme değeri DONANIMA GÖRE DEĞİŞİR.
    }

    // TODO: Kesme işleyiciden çıkış. Kaydedilen registerları geri yükle.
    // TODO: rfe (return from exception) yönergesini kullan.
     asm!("rfe"); // Dönüş yönergesi (unsafe asm)
    // NOT: Eğer exception entry noktası register kaydı ve rfe'yi yapıyorsa,
    // bu handler sadece işini yapıp normal geri dönebilir.
}
