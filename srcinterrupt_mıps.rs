#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz

// Gerekli core modüllerini içeri aktar
use core::arch::asm; // Inline assembly (CP0 register erişimi için)
use core::ptr;      // Pointer işlemleri için
// use core::mem;      // Gerekirse bellek işlemleri için (Örn: size_of, align_of) - şu an kullanılmıyor

// `volatile` crate'inden Volatile sarmalayıcıyı içeri aktar (cargo.toml'da tanımlı olmalı)
use volatile::Volatile; // <-- Added import

// Konsol çıktı makrolarını kullanabilmek için (hata durumlarında loglama veya debug çıktısı için)
// Bu makrolar Sahne64 crate'i tarafından sağlanır ve resource API'sini kullanır.
// Bu crate'te kullanılabilir olmaları için uygun kurulum (örn. #[macro_use]) gereklidir.
// Bu örnekte, #[cfg] ile std/no_std çıktısını ayarlayarak makroların
// uygun ortamda kullanılabilir olduğunu varsayıyoruz.
// use sahne64::eprintln; // Örnek import eğer macro publicse

// MIPS mimarisine özgü ve donanıma bağımlı sabitler
// ** GERÇEK DEĞERLERİ İŞLEMCİ VE ÇEVRE BİRİMİ REFERANS KILAVUZLARINDAN ALIN! **
// Bu değerler çip modeline ve çevre birimi instancelarına göre değişir.

// MIPS CP0 Register Adresleri (Örnek: MIPS32/64)
// Status Register: CP0 register 12, select 0
// Cause Register: CP0 register 13, select 0
// EBase Register: CP0 register 15, select 1 (MIPS32) veya CP0 register 15, select 0 (MIPS64)
// Bu örnek MIPS32 varsayıyor olabilir, EBase 15 select 1 veya Status/Cause 12/13 select 0 yaygındır.
// Kesin adresler ve select değerleri için kullandığınız MIPS CPU kılavuzuna bakın.

// Status register'daki bitler
const STATUS_IE_BIT: u32 = 1 << 0; // Interrupt Enable (Global) bit 0
const STATUS_EXL_BIT: u32 = 1 << 1; // Exception Level (1: Kernel, 0: User) bit 1
// IM (Interrupt Mask) bitleri: Status register'ın 8-15 bitleri (IP0-IP7'ye karşılık gelir)
// Örneğin, IP2 (Cause'taki bit 10) maskesi Status'ta bit 10'dadır.
// cause register'daki IP (Interrupt Pending) bitleri
const CAUSE_IP_SHIFT: u32 = 10; // IP bitlerinin Cause register'daki başlangıç bit pozisyonu (tipik)
const CAUSE_IP_MASK: u32 = 0xFF << CAUSE_IP_SHIFT; // IP bitlerinin maskesi


// USB ile ilgili kesme biti (DONANIMA ÖZGÜ! Çipe göre değişir)
// Cause register'da bir IP[n] biti ve Status register'da buna karşılık gelen IM[n] biti vardır.
// USB_IRQ_IP_BIT: USB kesmesini bekleyen durumunu gösteren Cause register IP bit pozisyonu.
// USB_IRQ_IM_BIT: USB kesmesinin *işlenmesini* etkinleştiren Status register IM bit pozisyonu.
// Genellikle IP[n] Cause register'da bit (10 + n), IM[n] Status register'da bit (8 + n)'dir.
// veya ikisi de (10+n) olabilir, kılavuza bakın.
const USB_IRQ_IP_BIT_POS: u32 = 5; // ÖRNEK DEĞER! Cause register'da IP[5] için bit pozisyonu (Cause bit 10+5 = 15)
const USB_IRQ_IM_BIT_POS: u32 = 13; // ÖRNEK DEĞER! Status register'da IM[5] için bit pozisyonu (Status bit 8+5 = 13)

const USB_IRQ_CAUSE_MASK: u32 = 1 << (CAUSE_IP_SHIFT + USB_IRQ_IP_BIT_POS); // USB IRQ için Cause'taki pending bit maskesi
const USB_IRQ_STATUS_MASK: u32 = 1 << USB_IRQ_IM_BIT_POS; // USB IRQ için Status'taki enable maskesi


// Makine Zamanlayıcı Kesmesi (Cause register'da IP[n], Status register'da IM[n])
// Timer kesmesi genellikle IP[7] veya IP[6] kullanılır.
const TIMER_IRQ_IP_BIT_POS: u32 = 7; // ÖRNEK DEĞER! Cause register'da IP[7] için bit pozisyonu (Cause bit 10+7 = 17)
const TIMER_IRQ_IM_BIT_POS: u32 = 15; // ÖRNEK DEĞER! Status register'da IM[7] için bit pozisyonu (Status bit 8+7 = 15)

const TIMER_IRQ_CAUSE_MASK: u32 = 1 << (CAUSE_IP_SHIFT + TIMER_IRQ_IP_BIT_POS); // Timer IRQ için Cause'taki pending bit maskesi
const TIMER_IRQ_STATUS_MASK: u32 = 1 << TIMER_IRQ_IM_BIT_POS; // Timer IRQ için Status'taki enable maskesi


extern "C" {
    // Kesme Vektör Tablosu (Interrupt Vector Table) başlangıç adresi
    // EBase register'ı bu adresin başlangıcını işaret eder.
    // Bu sembol, linker script veya trap/exception handling kodunda tanımlanmalıdır.
   pub static __exception_vector_table: u32; // Linker script veya trap handling'den gelen sembol adı (32-bit mimari varsayımı)
}

// USB ile ilgili donanım adresleri ve register offset'leri (DONANIMA ÖZGÜ!)
// USB_BASE_ADDRESS: USB kontrolcüsünün bellek eşlemeli (memory-mapped) temel adresi.
// USB_IRQ_CLEAR_REGISTER: USB kesme bayrağını temizlemek için yazılacak register adresi (veya offset).
// USB_IRQ_CLEAR_VALUE: USB kesme bayrağını temizlemek için bu register'a yazılacak değer.
// **DİKKAT:** Bu değerler ve adresler, kullandığınız MIPS çipine ve USB kontrolcüsüne göre DEĞİŞİR.
// Doğru değerler için çipinizin REFERANS KILAVUZUNA bakmanız ŞARTTIR.
const USB_BASE_ADDRESS: usize = 0xBF00_0000; // ÖRNEK ADRES! Kılavuza bakınız!
const USB_IRQ_CLEAR_REGISTER_OFFSET: usize = 0x10; // ÖRNEK OFFSET! Kılavuza bakınız!
const USB_IRQ_CLEAR_VALUE: u32 = 1; // ÖRNEK DEĞER! Kılavuza bakınız!


/// MIPS mimarisi için kesme ve trap altyapısını başlatır.
/// EBase, Status (IM ve IE bitleri) gibi ilgili CP0 registerlarını ayarlar.
/// Bu fonksiyon, sistem başlangıcında (kernel init sürecinde) çağrılmalıdır.
pub fn init() {
    // Bu fonksiyon, Kernel mode gibi yüksek privilege seviyesinde çalışmalıdır.
    unsafe {
        // 1. EBase (Exception Base Address) Register'ını Ayarlama
        // EBase register'ı, kesme/istisna vektör tablosunun (IVT) başlangıç adresini tutar.
        // Kesme/istisna oluştuğunda işlemci, EBase + ofset adresine dallanır.
        // MIPS'te EBase genellikle CP0 register 15, select 1'dir (kontrol ediniz).
        let ivt_address = &__exception_vector_table as *const u32 as u32; // IVT'nin 32-bit adresini al (32-bit mimari varsayımı)

        // EBase register'ına IVT adresini yazmak için 'mtc0' (Move To CP0) kullanılır.
        // $15: EBase register numarası, 1: select numarası (MIPS32 için yaygın).
        asm!(
            "mtc0 {0}, $15, 1", // mtc0 rt, csr, sel (rt: source reg, csr: cp0 reg num, sel: select num)
            in(reg) ivt_address,  // Kaynak register olarak IVT adresi
            options(nostack)      // Stack manipulation olmadığını belirtir
        );
        // ** AÇIKLAMA: EBase adresini sisteme bildirme işlemi DONANIMA ÖZGÜDÜR! **
        // Doğru CP0 register numarası, select değeri ve yazma yönergesi için MIPS ISA ve çip kılavuzuna bakın.


        // 2. İlgili Çevre Birimi Kesmelerini Etkinleştirme (Status register'daki IM bitleri)
        // Zamanlayıcı ve USB kesmelerini Status register'ın IM bitleri (Interrupt Mask) ile etkinleştir.
        // Önce Status register'ı okuyun, IM bitlerini set edin, geri yazın.
        // Status register genellikle CP0 register 12, select 0'dır (kontrol ediniz).
        let interrupt_mask_bits = TIMER_IRQ_STATUS_MASK | USB_IRQ_STATUS_MASK; // Etkinleştirilecek IRQ bit maskesi

        // Status register'ı oku: mfc0 rd, csr, sel (rd: dest reg, csr: cp0 reg num, sel: select num)
        // $t0 (register 8) geçici olarak kullanılabilecek bir register.
        asm!("mfc0 $t0, $12, 0", options(nostack));

        // Okunan Status değerine maskeyi OR'la (IM bitlerini set et)
        asm!(
            "ori $t0, $t0, {0}", // ori rt, rs, immediate (rt = rs | immediate)
            in(reg) interrupt_mask_bits,
            options(nostack)
        );

        // Yeni Status değerini geri yaz: mtc0 rt, csr, sel
        asm!("mtc0 $t0, $12, 0", options(nostack));
        // **DİKKAT:** Status register'ı ve IM bit pozisyonları için MIPS ISA ve çip kılavuzuna bakın.


        // 3. Genel Kesmeleri Etkinleştirme (Status register'daki IE biti)
        // Status register'daki IE bitini (Interrupt Enable - bit 0) set edin.
        // Bu bit olmadan, bireysel kesme maskeleme (IM) yeterli DEĞİLDİR.
        // Status register'ı tekrar okuyun, IE bitini set edin, geri yazın.
         // $t0 register'ı hala kullanılabilir, veya tekrar okuyun.
        asm!("mfc0 $t0, $12, 0", options(nostack));

        asm!(
            "ori $t0, $t0, {0}", // Status değerine IE_BIT'i OR'la
            in(reg) STATUS_IE_BIT, // STATUS_IE_BIT = 1 << 0
            options(nostack)
        );

        asm!("mtc0 $t0, $12, 0", options(nostack));
        // **DİKKAT:** Status register'ı ve IE bit pozisyonu için MIPS ISA kılavuzuna bakın.


        // NOT: Bu noktada, zamanlayıcı ve USB kesmeleri (ve Status.IM'de etkinleştirilen diğerleri)
        // gerçekleştiğinde işlemci kontrolü EBase tarafından işaret edilen exception vector'a devredecektir.
        // Exception vector kodu Cause register'ını okuyup kesme nedenini belirlemeli ve uygun işleyiciye dallanmalıdır.

        // Diğer platforma özgü başlatma adımları buraya eklenebilir (örn. diğer çevre birimleri init)
    }
    // init fonksiyonu başarıyla tamamlanırsa geri döner.
}

// USB Kesme İşleyicisi (DONANIMA ÖZGÜ UYGULAMA GEREKLİ! - ÖRNEK YAPI)
// Bu fonksiyon, exception vector veya bir merkezi trap/kesme dispatcher'ından çağrılır.
// Genellikle ayrı bir kod bölümüne (.text.interrupts gibi) yerleştirilir.
#[no_mangle] // Linker script veya trap entry tarafından çağrılabilir
// Kesme işleyici fonksiyonları genellikle 'unsafe extern "C"' olarak tanımlanır.
pub unsafe extern "C" fn usb_interrupt_handler() {
    // ** Güvenlik: Bu işleyici unsafe'dir çünkü kesme bağlamında çalışır **
    // ve donanımla doğrudan etkileşime girer. Yarış durumları ve side effect'lere dikkat!

    // ** DİKKAT: Bu bölüm DONANIMA ÖZGÜDÜR ve KULLANILAN ÇİPİN VE USB KONTROLCÜSÜNÜN KILAVUZUNA GÖRE KODLANMALIDIR! **
    // Bu işleyici çekirdek içinde çalışır, kullanıcı alanındaki Sahne64 API'sini doğrudan çağırmaz.
    // Gelen USB verisini alır ve çekirdekteki USB sürücüsü koduna iletir.
    // Ardından, bu veriyi bekleyen kullanıcı görevini Sahne64 çekirdek zamanlama mekanizması
    // aracılığıyla uyandırır (örn. resource::read için bekleyen görev).

    // 1. Kesme kaynağını belirle (Genellikle Cause register IP bitlerine ve çevre birimi status registerlarına bakılır)
    // Cause register'ı oku (CP0 register 13, select 0)
    let cause: u32;
    asm!("mfc0 {0}, $13, 0", out(reg) cause, options(nostack));

    // USB kesme pending bitini (Cause.IP[USB_IRQ_IP_BIT_POS]) kontrol et.
    // Bu, bu handler'ın neden çağrıldığının bir göstergesidir (eğer Cause register'ı dispatch öncesi okunuyorsa).
    // Veya, bu handler'ın çağrılması zaten USB kesmesinin pending olduğunu gösteriyordur.
    // Bu kısım, genel exception vector'ın nasıl çalıştığına bağlıdır.
    // Genellikle işleyicinin içine girmek zaten ilgili IRQ'nun aktif olduğunu gösterir.
    // Yine de çevre birimi status register'ına bakmak gerekebilir.

    // USB kontrolcüsünün kendi durum register'ına bakarak spesifik nedeni bul.
    // Örnek: USB durum register'ını oku ve veri var mı, TX bitti mi kontrol et.
    let usb_status_register_address = USB_BASE_ADDRESS.wrapping_add(USB_STATUS_REGISTER_OFFSET); // Güvenli adres hesaplama
    let usb_status = ptr::read_volatile(usb_status_register_address as *const u32); // Örnek okuma (32-bit status)

    // Örnek USB durum bitleri (usb_interrupt_handler içinde kullanılmıştı, burada tanımlanmalı)
    // Bu bitler, durum register'ında, Cause.IP bitleri gibi değil.
    const USB_STATUS_DATA_RECEIVED_BIT: u32 = 1 << 0; // ÖRNEK DEĞER: Durum kaydındaki Veri Alındı biti
    const USB_STATUS_DATA_SENT_BIT: u32 = 1 << 1;    // ÖRNEK DEĞER: Durum kaydındaki Veri Gönderildi biti


    if (usb_status & USB_STATUS_DATA_RECEIVED_BIT) != 0 {
        // Veri geldi kesmesi oluştu (USB kontrolcüsüne göre)
        // TODO: Veriyi USB kontrolcüsünden oku (hardware-specific).
        // TODO: Okunan veriyi çekirdekteki USB sürücüsü tamponuna yaz.
        // TODO: Resource'u (USB console kaynağı) bekleyen görevleri uyandır (örn. resource::read yapan görev).
         // Debug çıktıları için Sahne64 konsol makrolarını kullan
         #[cfg(feature = "std")] std::println!("USB Veri Alındı (işleyici içinde).");
         #[cfg(not(feature = "std"))] println!("USB Veri Alındı (işleyici içinde).");
    }

    if (usb_status & USB_STATUS_DATA_SENT_BIT) != 0 {
         // Veri gönderildi kesmesi oluştu (USB kontrolcüsüne göre)
         // TODO: Çekirdek tamponundan bir sonraki veriyi USB kontrolcüsüne yaz (eğer gönderilecek veri varsa).
         // TODO: Resource'u (USB console kaynağı) bekleyen görevleri uyandır (örn. resource::write'ın tamamlanmasını bekleyen görev).
         // Debug çıktıları için Sahne64 konsol makrolarını kullan
         #[cfg(feature = "std")] std::println!("USB Veri Gönderildi (işleyici içinde).");
         #[cfg(not(feature = "std"))] println!("USB Veri Gönderildi (işleyici içinde).");
    }

    // TODO: Diğer kesme nedenleri (hata kesmeleri, bağlantı durum değişiklikleri vb.)

    // 2. Kesme Bayrağını Temizleme (ÇOK ÖNEMLİ! - DONANIMA ÖZGÜ)**
    // Kesme işlendikten sonra, kesme bayrağının TEMİZLENMESİ ZORUNLUDUR.
    // Aksi takdirde, aynı kesme sürekli olarak tekrar tetiklenir ve sistem kilitlenir.
    // MIPS'te bu genellikle çevre birimi kontrolcüsünün kendi register'ındaki bir biti
    // yazarak/temizleyerek yapılır. Cause register IP bitleri genellikle donanım tarafından temizlenir
    // veya EOI (End of Interrupt) mekanizması varsa onunla etkileşime girilir.
    // **DİKKAT:** Kesme bayrağını temizleme yöntemi DONANIMA ÖZGÜDÜR! Kılavuza bakın!
    unsafe {
         let usb_irq_clear_register_address = USB_BASE_ADDRESS.wrapping_add(USB_IRQ_CLEAR_REGISTER_OFFSET); // Güvenli adres hesaplama
         let mut usb_irq_clear_register = Volatile::new(usb_irq_clear_register_address as *mut u32); // Örnek: 32-bit register
         usb_irq_clear_register.write(USB_IRQ_CLEAR_VALUE); // Örnek: Temizleme değerini yaz
         // **UYARI:** USB_IRQ_CLEAR_REGISTER_OFFSET ve USB_IRQ_CLEAR_VALUE DONANIMA GÖRE DEĞİŞİR.
         // Doğru yöntem için çipinizin ve USB kontrolcünüzün REFERANS KILAVUZUNA BAKIN.
    }

    // 3. (Opsiyonel) EOI (End of Interrupt) İşlemi
    // MIPS'te harici bir Interrupt Controller (örn. PIC veya GIC benzeri) varsa,
    // işleyicinin sonunda o kontrolcüye EOI sinyali göndermek gerekebilir.
    // Bu da donanıma özgüdür.
     write_eoi_register(...); // Donanıma özel

    // NOTE: Kesme işleyiciden çıkış, MIPS exception mekanizması tarafından yönetilir.
    // Genellikle exception vector noktasında durum geri yüklenir ve EPC + 4 ile ERET yönergesi kullanılır.
    // Bu işleyici fonksiyonunun kendisi normal bir fonksiyon gibi geri döner (çekirdek exception vector noktasına).
}
