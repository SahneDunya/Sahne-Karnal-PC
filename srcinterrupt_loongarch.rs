#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz

// Gerekli core modüllerini içeri aktar
use core::arch::asm; // Inline assembly (CSR erişimi için)
use core::ptr;      // Pointer işlemleri için
use core::mem;      // Gerekirse bellek işlemleri için (Örn: size_of, align_of)

// `volatile` crate'inden Volatile sarmalayıcıyı içeri aktar (cargo.toml'da tanımlı olmalı)
use volatile::Volatile;

// Konsol çıktı makrolarını kullanabilmek için (hata durumlarında loglama veya debug çıktısı için)
// Bu makrolar Sahne64 crate'i tarafından sağlanır ve resource API'sini kullanır.
// Bu crate'te kullanılabilir olmaları için uygun kurulum (örn. #[macro_use]) gereklidir.
// Bu örnekte, #[cfg] ile std/no_std çıktısını ayarlayarak makroların
// uygun ortamda kullanılabilir olduğunu varsayıyoruz.
// use sahne64::eprintln; // Örnek import eğer macro publicse

// LoongArch mimarisine özgü ve donanıma bağımlı sabitler
// ** GERÇEK DEĞERLERİ İŞLEMCİ VE ÇEVRE BİRİMİ REFERANS KILAVUZLARINDAN ALIN! **
// Bu değerler çip modeline, çevre birimi instancelarına ve kesme kontrolcüsüne göre değişir.

// LoongArch Standart CSR'lar (Control and Status Registers) ve Bitleri
const CSR_MTVEC: usize = 0x7c; // MTVEC CSR adresi (Trap Vector Base Address)
const CSR_MIE: usize = 0x7e;   // MIE CSR adresi (Machine Interrupt Enable)
const CSR_MSTATUS: usize = 0x7c0; // MSTATUS CSR adresi (Machine Status)

// MIE (Machine Interrupt Enable): Genel kesme etkinleştirme biti
// MTIE (Machine Timer Interrupt Enable): Makine zamanlayıcı kesmesi etkinleştirme biti
const MIE_BIT: usize = 1 << 3; // Örnek: MSTATUS'taki MIE biti pozisyonu (LoongArch'a göre kontrol edilmeli)
const MTIE_BIT: usize = 1 << 7; // Örnek: MIE'deki Makine Zamanlayıcı Kesmesi biti pozisyonu (LoongArch'a göre kontrol edilmeli)

// USB ile ilgili kesme biti (DONANIMA ÖZGÜ! Loongson 3A5000 veya başka bir çip ÖRNEĞİ)
// USB_IRQ_BIT: USB kesmesini temsil eden bit.
// **DİKKAT:** Bu değer, kullandığınız LoongArch çipine göre DEĞİŞİR.
// Doğru değer için çipinizin REFERANS KILAVUZUNA bakmanız ZORUNLUDUR.
const USB_IRQ_BIT: usize = 1 << 12; // ÖRNEK DEĞER! Kılavuza bakınız! Genellikle MIE'deki bir bit veya harici kesme kontrolcüsüne bağlı

extern "C" {
    // Kesme Vektör Tablosu (Interrupt Vector Table) başlangıç adresi
    // MTVEC CSR'si bu adresin başlangıcını işaret eder.
    // Bu sembol, linker script veya trap/exception handling kodunda tanımlanmalıdır.
   pub static __exception_vector_table: u8; // Linker script veya trap handling'den gelen sembol adı
}

// USB ile ilgili donanım adresleri ve register offset'leri (DONANIMA ÖZGÜ! Loongson 3A5000 veya başka bir çip ÖRNEĞİ)
// USB_BASE_ADDRESS: USB kontrolcüsünün bellek eşlemeli (memory-mapped) temel adresi.
// USB_STATUS_REGISTER_OFFSET: USB durum register'ının temel adrese göre offset'i.
// USB_DATA_REGISTER_OFFSET: USB veri register'ının temel adrese göre offset'i.
// USB_IRQ_CLEAR_BIT: USB kesme bayrağını temizlemek için kullanılacak bit maskesi veya değer.
// **DİKKAT:** Bu değerler ve adresler, kullandığınız LoongArch çipine göre DEĞİŞİR.
// Doğru değerler için çipinizin REFERANS KILAVUZUNA bakmanız ŞARTTIR.
const USB_BASE_ADDRESS: usize = 0x8000_1000; // ÖRNEK ADRES! Kılavuza bakınız!
const USB_STATUS_REGISTER_OFFSET: usize = 0x04; // ÖRNEK OFFSET! Kılavuza bakınız!
const USB_DATA_REGISTER_OFFSET: usize = 0x08; // ÖRNEK OFFSET! Kılavuza bakınız! (Orijinalde yoktu, eklenirse gerekir)
const USB_IRQ_CLEAR_BIT: usize = 1 << 0;    // ÖRNEK BIT! Kılavuza bakınız! (Durum register'ında temizlenecek bit)

// USB Durum Register'ındaki Örnek Bitler (usb_interrupt_handler içinde kullanılmıştı, burada tanımlanmalı)
const USB_STATUS_DATA_RECEIVED: u32 = 0x01; // ÖRNEK DEĞER: Veri alındı durum biti
const USB_STATUS_DATA_SENT: u32 = 0x02;    // ÖRNEK DEĞER: Veri gönderildi durum biti
const USB_STATUS_CLEAR_MASK: u32 = USB_STATUS_DATA_RECEIVED | USB_STATUS_DATA_SENT; // Temizlenecek bit maskesi


/// LoongArch mimarisi için kesme ve trap altyapısını başlatır.
/// MTVEC, MIE ve MSTATUS gibi ilgili CSR'ları ayarlar.
/// Bu fonksiyon, sistem başlangıcında (kernel init sürecinde) çağrılmalıdır.
pub fn init() {
    // Bu fonksiyon, M mode (Machine mode) gibi yüksek privilege seviyesinde çalışmalıdır.
    unsafe {
        // 1. MTVEC (Machine Trap Vector Base Address) Register'ını Ayarlama
        // MTVEC register'ı, kesme/istisna vektör tablosunun (IVT) başlangıç adresini tutar.
        // Kesme/istisna oluştuğunda işlemci, MTVEC + ofset adresine dallanır (MODE'a bağlı).
        // LoongArch'ta MTVEC formatı ve ofset hesaplaması mimariye özgüdür.
        // Elbrus örneğindeki gibi, burada __exception_vector_table sembolünün adresini kullanıyoruz.
        let ivt_address = &__exception_vector_table as *const u8 as usize; // IVT'nin adresini al

        // CSR_MTVEC register'ına IVT adresini yazmak için 'csrwr' kullanılır.
        // options(nostack) burada uygun olabilir.
        asm!(
            "csrwr {0}, {1}", // csrwr rd, csr, rs (destination register, csr address, source register)
            in(reg) ivt_address,  // Kaynak register (rS) olarak IVT adresi
            const CSR_MTVEC,      // Hedef CSR adresi
            options(nostack)      // Stack manipulation olmadığını belirtir
        );
        // ** AÇIKLAMA: MTVEC adresini sisteme bildirme işlemi DONANIMA ÖZGÜDÜR! **
        // Doğru CSR adresi ve yazma yönergesi için LoongArch ISA ve çip kılavuzuna bakın.


        // 2. İlgili Çevre Birimi Kesmelerini Etkinleştirme (MIE register - Machine Interrupt Enable)
        // Zamanlayıcı (MTIE) ve USB (USB_IRQ_BIT) kesmelerini MIE CSR'sinde etkinleştir.
        // 'csrrs rd, csr, rs': CSR Register Set. CSR'yi okur, rs ile OR'lar ve sonucu hem rd'ye hem csr'ye yazar.
        // rs=zero (x0) ile kullanıldığında, sadece rd'ye CSR'nin eski değerini okur.
        // Buradaki kullanım: CSR'yi oku, maske ile OR'la, sonucu CSR'ye yaz. (rd = eski_csr, csr = eski_csr | maske)
        // Maske = MTIE_BIT | USB_IRQ_BIT şeklinde birden fazla biti aynı anda set edebiliriz.
        let irqs_to_enable = MTIE_BIT | USB_IRQ_BIT;

        asm!(
            "csrrs {0}, {1}, {2}", // csrrs rd, csr, rs
            out(reg) _,          // eski MIE değeri (kullanmıyoruz, _ ile ignore et)
            const CSR_MIE,       // MIE CSR adresi
            in(reg) irqs_to_enable, // Set edilecek bit maskesi (kaynak register)
            options(nostack)     // Stack manipulation olmadığını belirtir
        );
        // **DİKKAT:** USB kesme bitinin değeri (USB_IRQ_BIT) çipten çipe farklılık gösterir.
        // MIE register'ı ve bit pozisyonları için LoongArch ISA ve çip kılavuzuna bakın.


        // 3. Genel Kesmeleri Etkinleştirme (MSTATUS register - Machine Status, MIE biti)
        // MSTATUS CSR'sindeki MIE bitini (Machine Interrupt Enable) set et.
        // Bu bit olmadan, bireysel kesme etkinleştirme bitleri (MIE CSR içindekiler) tek başına yeterli DEĞİLDİR.
        // 'csrrs rd, csr, rs' yönergesini MSTATUS ve MIE_BIT maskesi ile kullanıyoruz.
        asm!(
            "csrrs {0}, {1}, {2}", // csrrs rd, csr, rs
            out(reg) _,          // eski MSTATUS değeri (kullanmıyoruz)
            const CSR_MSTATUS,   // MSTATUS CSR adresi
            in(reg) MIE_BIT,     // Set edilecek MIE biti maskesi (kaynak register)
            options(nostack)     // Stack manipulation olmadığını belirtir
        );
        // **DİKKAT:** MSTATUS register'ı farklı amaçlar için başka bitler de içerebilir.
        // MSTATUS ve MIE bit pozisyonu için LoongArch ISA kılavuzuna bakın.

        // NOT: Bu noktada, zamanlayıcı ve USB kesmeleri (ve MIE'de etkinleştirilen diğerleri)
        // gerçekleştiğinde işlemci kontrolü MTVEC tarafından işaret edilen trap entry noktasına devredecektir.
        // Trap entry kodu mcause CSR'sini okuyup uygun işleyiciye dallanmalıdır.

        // Diğer platforma özgü başlatma adımları buraya eklenebilir (örn. diğer çevre birimleri init)
    }
    // init fonksiyonu başarıyla tamamlanırsa geri döner.
}

// USB Kesme İşleyicisi (DONANIMA ÖZGÜ UYGULAMA GEREKLİ! - ÖRNEK YAPI)
// Bu fonksiyon, trap entry noktasından çağrılır.
// Linker scriptte .trap.interrupt_handlers gibi bir bölüme yerleştirilir.
#[no_mangle] // Linker script veya trap entry tarafından çağrılabilir
#[link_section = ".trap.interrupt_handlers"] // Kesme işleyicileri için ayrı bir bölüm (Örnek bölüm adı)
// Kesme işleyici fonksiyonları genellikle 'unsafe extern "C"' olarak tanımlanır.
pub unsafe extern "C" fn usb_interrupt_handler() {
    // ** Güvenlik: Bu işleyici unsafe'dir çünkü kesme bağlamında çalışır **
    // ve donanımla doğrudan etkileşime girer. Yarış durumları ve side effect'lere dikkat!

    // ** DİKKAT: Bu bölüm DONANIMA ÖZGÜDÜR ve KULLANILAN ÇİPİN VE USB KONTROLCÜSÜNÜN KILAVUZUNA GÖRE KODLANMALIDIR! **
    // Bu işleyici çekirdek içinde çalışır, kullanıcı alanındaki Sahne64 API'sini doğrudan çağırmaz.
    // Gelen USB verisini alır ve çekirdekteki USB sürücüsü koduna iletir.
    // Ardından, bu veriyi bekleyen kullanıcı görevini Sahne64 çekirdek zamanlama mekanizması
    // aracılığıyla uyandırır (örn. resource::read için bekleyen görev).

    // 1. Kesme kaynağını belirle (USB Durum Register'ını oku)
    // Hangi kesme bitlerinin set olduğuna bakılır.
    let usb_status_register_address = USB_BASE_ADDRESS.wrapping_add(USB_STATUS_REGISTER_OFFSET); // Güvenli adres hesaplama
    let status = ptr::read_volatile(usb_status_register_address as *const u32);


    // 2. İşlenen kesme bitlerini (flag'larını) temizle (ÇOK ÖNEMLİ!)
    // Bu, kesmenin tekrar tetiklenmesini önler. Temizleme yöntemi donanıma özgüdür.
    // Bu örnekte, işlenen bitleri 0 yaparak statüs kaydına yazıyoruz.
    // SADECE işlenen bitleri değiştirdiğimizden emin olmak için dikkatli olunmalıdır.
    let status_to_clear = status & USB_STATUS_CLEAR_MASK; // Temizlenecek bitleri ayıkla
    if status_to_clear != 0 {
         // Sadece temizlenecek bitler varsa yazma yap
         let mut status_reg = Volatile::new(usb_status_register_address as *mut u32);
         // Okunan değerdeki sadece temizlenecek bitleri 0 yapıp geri yaz.
         // Alternatif olarak, bazı donanımlar temizlemek için 1 yazılmasını bekler, veya farklı bir temizleme register'ı kullanır.
         // Kılavuza bakın!
         status_reg.write(status & !status_to_clear); // Örnek: Bitleri temizle
    }


    // 3. Kesme nedenine göre işlem yap (Veri geldi, Veri gönderildi vb.)
    // Bu kısım, USB sürücüsünün temel logic'idir ve çekirdek içinde yer alır.
    if (status & USB_STATUS_DATA_RECEIVED as u32) != 0 { // Bit maskelerini u32 yap (status u32)
        // Veri geldi kesmesi oluştu
        // Veriyi USB kontrolcüsünden oku (hardware-specific)
         let usb_data_register_address = USB_BASE_ADDRESS.wrapping_add(USB_DATA_REGISTER_OFFSET); // Güvenli adres hesaplama
         let received_data = ptr::read_volatile(usb_data_register_address as *const u32); // Örnek okuma (32-bit)
        // TODO: Okunan veriyi çekirdekteki USB sürücüsü tamponuna yaz.
        // TODO: Resource'u (USB console kaynağı) bekleyen görevleri uyandır (örn. resource::read yapan görev).
         // Debug çıktıları için Sahne64 konsol makrolarını kullan
         #[cfg(feature = "std")] std::println!("USB Veri Alındı: 0x{:x}", received_data);
         #[cfg(not(feature = "std"))] println!("USB Veri Alındı: 0x{:x}", received_data);
    }

    if (status & USB_STATUS_DATA_SENT as u32) != 0 { // Bit maskelerini u32 yap
         // Veri gönderildi kesmesi oluştu
         // TODO: Çekirdek tamponundan bir sonraki veriyi USB kontrolcüsüne yaz (eğer gönderilecek veri varsa).
         // TODO: Resource'u (USB console kaynağı) bekleyen görevleri uyandır (örn. resource::write'ın tamamlanmasını bekleyen görev).
         // Debug çıktıları için Sahne64 konsol makrolarını kullan
         #[cfg(feature = "std")] std::println!("USB Veri Gönderildi (işleyici içinde).");
         #[cfg(not(feature = "std"))] println!("USB Veri Gönderildi (işleyici içinde).");
    }

    // TODO: Diğer kesme nedenleri (hata kesmeleri, bağlantı durum değişiklikleri vb.)


    // NOTE: Kesme işleyiciden çıkış, LoongArch trap mekanizması tarafından yönetilir.
    // Genellikle trap entry noktasında durum geri yüklenir ve EPC + 4 ile MRET/ERET benzeri bir yönerge kullanılır.
    // Bu işleyici fonksiyonunun kendisi normal bir fonksiyon gibi geri döner (çekirdek trap entry noktasına).
}
