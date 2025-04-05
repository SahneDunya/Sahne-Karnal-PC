#![no_std]
use core::arch::asm;
use volatile::Volatile;

// LoongArch standart kesme bitleri (64-bit uyumlu)
// MIE (Machine Interrupt Enable): Genel kesme etkinleştirme biti
// MTIE (Machine Timer Interrupt Enable): Makine zamanlayıcı kesmesi etkinleştirme biti
const MIE_BIT: usize = 1 << 3;
const MTIE_BIT: usize = 1 << 7;

// USB ile ilgili kesme biti (DONANIMA ÖZGÜ! Loongson 3A5000 ÖRNEĞİ)
// USB_IRQ_BIT: USB kesmesini temsil eden bit.
// **DİKKAT:** Bu değer, kullandığınız LoongArch çipine göre DEĞİŞİR.
// Doğru değer için çipinizin REFERANS KILAVUZUNA bakmanız ZORUNLUDUR.
const USB_IRQ_BIT: usize = 1 << 12; // ÖRNEK DEĞER! Kılavuza bakınız!

extern "C" {
    static IVT: u8; // Kesme Vektör Tablosu (Interrupt Vector Table) başlangıç adresi
}

// USB ile ilgili donanım adresleri ve register offset'leri (DONANIMA ÖZGÜ! Loongson 3A5000 ÖRNEĞİ)
// USB_BASE_ADDRESS: USB kontrolcüsünün temel adresi.
// USB_STATUS_REGISTER_OFFSET: USB durum register'ının temel adrese göre offset'i.
// USB_IRQ_CLEAR_BIT: USB kesme bayrağını temizlemek için kullanılacak bit maskesi.
// **DİKKAT:** Bu değerler ve adresler, kullandığınız LoongArch çipine göre DEĞİŞİR.
// Doğru değerler için çipinizin REFERANS KILAVUZUNA bakmanız ŞARTTIR.
const USB_BASE_ADDRESS: usize = 0x8000_1000; // ÖRNEK ADRES! Kılavuza bakınız!
const USB_STATUS_REGISTER_OFFSET: usize = 0x04; // ÖRNEK OFFSET! Kılavuza bakınız!
const USB_IRQ_CLEAR_BIT: usize = 1 << 0;    // ÖRNEK BIT! Kılavuza bakınız!

pub fn init() {
    unsafe {
        // 1. MTVEC (Machine Trap Vector Base Address) Register'ını Ayarlama
        // MTVEC register'ı, kesme vektör tablosunun (IVT) başlangıç adresini tutar.
        // Kesme oluştuğunda işlemci, IVT'deki ilgili adrese dallanır.
        let ivt_address = &IVT as *const u8 as usize; // IVT'nin adresini al
        asm!(
            "csrwr mtvec, {0}", // LoongArch'ta CSR register'larına yazmak için 'csrwr' kullanılır.
            in(reg) ivt_address  // IVT adresini MTVEC'e yaz.
        );

        // 2. Zamanlayıcı Kesmesini Etkinleştirme (MTIE biti)
        // MTIE biti, Makine Modu Zamanlayıcı Kesmesini (Machine Timer Interrupt) etkinleştirir.
        // Bu, periyodik zamanlayıcı kesmelerini almak için gereklidir (örneğin, işletim sistemi zamanlaması için).
        asm!(
            "csrrs mie, {0}, zero", // 'csrrs mie, {0}, zero': MIE register'ına {0} bitini SET ET (geri kalanı değişmez).
            in(reg) MTIE_BIT         // MTIE bitini MIE'ye set ederek zamanlayıcı kesmesini etkinleştir.
        );

        // 3. USB Kesmesini Etkinleştirme (USB_IRQ_BIT biti) - DONANIMA ÖZGÜ DEĞER!
        // USB_IRQ_BIT biti, USB kontrolcüsünden gelen kesmeleri etkinleştirir.
        // **DİKKAT:** USB kesme bitinin değeri çipten çipe farklılık gösterir.
        // Kılavuzdan doğru değeri bulup USB_IRQ_BIT sabitine atamanız GEREKİR.
        asm!(
            "csrrs mie, {0}, zero", // MIE register'ına {0} bitini SET ET.
            in(reg) USB_IRQ_BIT     // USB_IRQ_BIT'i MIE'ye set ederek USB kesmesini etkinleştir.
                                     // **ÖNEMLİ: USB_IRQ_BIT DEĞERİNİ KILAVUZDAN ALIN!**
        );

        // 4. Genel Kesmeleri Etkinleştirme (MIE biti)
        // MIE biti (Machine Interrupt Enable), genel olarak makine modunda kesmelerin işlenmesini etkinleştirir.
        // Bu bit olmadan, bireysel kesme etkinleştirme bitleri (MTIE, USB_IRQ_BIT vb.) tek başına yeterli DEĞİLDİR.
        asm!(
            "csrrs mstatus, {0}, zero", // 'csrrs mstatus, {0}, zero': MSTATUS register'ına {0} bitini SET ET.
            in(reg) MIE_BIT             // MIE bitini MSTATUS'a set ederek genel kesmeleri etkinleştir.
                                         // **DİKKAT:** mstatus register'ı farklı amaçlar için başka bitler de içerebilir.
                                         // Bu örnekte sadece MIE bitini etkiliyoruz.
        );
    }
}

// USB Kesme İşleyicisi (DONANIMA ÖZGÜ UYGULAMA GEREKLİ! - ÖRNEK YAPI)
#[no_mangle]
pub extern "C" fn usb_interrupt_handler() {
    // **1. USB Kesmesi İşlendiğinde YAPILACAKLAR (DONANIMA ÖZGÜ)**
    // Bu kısım, USB kesmesi gerçekleştiğinde yapılması gereken işlemleri içerir.
    // Örnek olarak:
    //  - USB veri alma/gönderme işlemlerini yönetme
    //  - USB durumunu kontrol etme
    //  - Gerekirse daha fazla veri transferi başlatma vb.
    // **UYARI:** Bu kod tamamen DONANIMA ve USB cihazınızın protokolüne ÖZGÜDÜR.
    // Detaylı uygulama için USB kontrolcü ve cihaz dokümantasyonunu İNCELEYİN.
    // ... USB sürücü kodu buraya gelecek ...
    // ... (veri okuma, gönderme, durum kontrolü, vb.) ...


    // 2. Kesme Bayrağını Temizleme (ÇOK ÖNEMLİ! - DONANIMA ÖZGÜ)**
    // Kesme işlendikten sonra, kesme bayrağının TEMİZLENMESİ ZORUNLUDUR.
    // Aksi takdirde, aynı kesme sürekli olarak tekrar tetiklenir ve sistem kilitlenir.
    // **DİKKAT:** Kesme bayrağını temizleme yöntemi DONANIMA ÖZGÜDÜR.
    // Aşağıdaki örnek, USB durum register'ındaki bir biti temizleyerek bayrağı temizler.
    unsafe {
        // USB Durum Register'ına erişim (Volatile ile güvenli erişim)
        let usb_status_register_address = USB_BASE_ADDRESS + USB_STATUS_REGISTER_OFFSET;
        let usb_status_register = Volatile::new(usb_status_register_address as *mut usize);

        // Kesme Bayrağını Temizleme İşlemi (ÖRNEK! Kılavuza bakınız!)
        // Mevcut değeri OKU, kesme temizleme bitini (USB_IRQ_CLEAR_BIT) TEMİZLE (AND NOT işlemi), geri YAZ.
        usb_status_register.write(usb_status_register.read() & !USB_IRQ_CLEAR_BIT);
        // **UYARI:** USB_IRQ_CLEAR_BIT ve bayrak temizleme yöntemi DONANIMA GÖRE DEĞİŞİR.
        // Doğru yöntem için çipinizin ve USB kontrolcünüzün REFERANS KILAVUZUNA BAKIN.
    }

    // **3. (Opsiyonel) Kesme İşlemi Sonrası Yapılacaklar**
    // Kesme işlendikten sonra, gerekirse başka işlemler yapılabilir.
    // Örneğin, bir görevi (task) uyandırma, bir olay (event) sinyali gönderme, vb.
    // Bu örnekte basitlik adına bu kısım boş bırakılmıştır.
}