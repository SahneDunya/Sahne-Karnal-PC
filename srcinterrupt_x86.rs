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


use crate::exception; // x86 için istisna işleme modülü (çekirdek içi)

// x86'da kesme numaraları (IRQ)
// IDT indeksi, kesme/istisna vektör numarasıdır.
// Donanım IRQ numaraları genellikle 0-15 arasıdır (PIC) veya daha yüksek (APIC).
// Bu IRQ numaraları, IDT vektör numaralarına maplenir (genellikle 32+IRQ).
// **DİKKAT**: USB_IRQ değeri DONANIMA GÖRE DEĞİŞİR!
// Bu sadece bir ÖRNEK değerdir (örneğin, PIC IRQ 11). Bu, IDT indeksi *değildir*.
// Bu IRQ'nun hangi IDT vektörüne maplendiğini (genellikle 32 + IRQ_numarası) bilmelisiniz.
const USB_IRQ_NUM: u8 = 11; // ÖRNEK USB Donanım IRQ numarası. DONANIMA ÖZGÜ DEĞER!
// Bu IRQ'nun maplendiği IDT vektör numarasını belirleyelim (genellikle IRQ base + IRQ num).
const IRQ_BASE_VECTOR: u8 = 32; // Örnek: İlk 32 vektör istisnalar için, 32 sonrası IRQ'lar için.
const USB_IRQ_VECTOR: u8 = IRQ_BASE_VECTOR + USB_IRQ_NUM; // USB IRQ'sunun maplendiği IDT vektör numarası


// Kesme tanımlama tablosu (IDT) için yapı
// `#[repr(C, packed)]` x86 segment/gate tanımları için doğru bellekle yerleşimini sağlar.
// Bu yapı 32-bit interrupt gate tanımını temsil eder.
#[repr(C, packed)]
#[derive(Clone, Copy)] // init döngüsünde kopyalama için
struct IDTEntry {
    offset_low: u16,        // İşleyici adresinin düşük 16 biti
    selector: u16,          // Kod segment seçicisi (genellikle kernel kod segmenti)
    zero: u8,               // Boş byte (0 olmalı)
    type_attributes: u8,    // Tür ve öznitelikler (DPL, Varlık biti, Kapı türü vb.)
    offset_high: u16,       // İşleyici adresinin yüksek 16 biti
    // 64-bit (x86_64) için bu yapı daha farklıdır (offset_high 32 bit, zero alanı 32 bit daha uzar).
    // Bu kod 32-bit IDT entry formatına uygundur.
}

// IDT'nin kendisi. Sabit boyutlu bir dizi olarak tanımlanır.
// x86 mimarisinde 256 kesme vektörü vardır (0-255).
static mut IDT: [IDTEntry; 256] = [IDTEntry {
    offset_low: 0,
    selector: 0,
    zero: 0,
    type_attributes: 0,
    offset_high: 0,
}; 256]; // static mut IDT: writable olmalı

// Varsayılan Kesme/İstisna İşleyicisi
// IDT'de özel bir işleyici tanımlanmayan tüm vektörler için kullanılır.
// Genellikle basit bir hata raporlama ve sistem durdurma işlemi yapar.
// Güvenli olmayan (unsafe) extern "C" fn olarak tanımlanmalıdır.
// x86 kesme işleyicileri genellikle hiçbir argüman almaz (bazı istisnalar hata kodu pushlar).
unsafe extern "C" fn default_trap_handler() {
     // ** Güvenlik: Bu işleyici unsafe'dir çünkü trap/kesme bağlamında çalışır **
     // ve muhtemelen stack, donanım veya paylaşılan bellekle etkileşime girecektir.

     // Sahne64 konsol makrolarını kullanarak hata logla.
     // Hata ayıklama için hangi vektörün tetiklendiğini bulmak önemlidir.
     // Bu bilgi genellikle stack'te (eğer işlemci hata kodu pushladıysa) veya debug registerlarında bulunur.
     // Basitlik adına sadece genel bir hata loglayalım.
     #[cfg(feature = "std")] std::eprintln!("UYARI: x86 Beklenmeyen kesme/istisna!");
     #[cfg(not(feature = "std"))] eprintln!("UYARI: x86 Beklenmeyen kesme/istisna!"); // Sahne64 macro varsayımı

     // TODO: Hata durumunu logla, vektör numarasını belirle (stack veya debug register'lardan).
     // panic!("Beklenmeyen kesme/istisna!"); // Panik, debug amaçlı kullanılabilir.
     // Kritik hata durumunda sistemi durdur.
     loop { core::hint::spin_loop(); } // Veya halt_system();
}


/// x86 mimarisi için kesme ve trap altyapısını başlatır.
/// IDT'yi kurar, işleyici adreslerini yazar ve genel kesmeleri etkinleştirir.
/// Bu fonksiyon, sistem başlangıcında (kernel init sürecinde) Ring 0 privilege seviyesinde çağrılmalıdır.
pub fn init() {
    // Bu fonksiyon Ring 0 (Kernel mode) privilege seviyesinde çalışmalıdır.
    unsafe {
        // 1. IDT'yi Başlatma
        // Başlangıçta tüm IDT girişlerini varsayılan işleyiciye ayarla.
        let default_handler_addr = default_trap_handler as unsafe extern "C" fn() as usize;

         // Varsayılan IDT girişi yapısını oluştur
        let default_entry = IDTEntry {
             offset_low: (default_handler_addr as u16) & 0xFFFF,
             selector: 0x08, // Varsayım: Kernel kod segment seçici
             zero: 0,
             type_attributes: 0x8E, // Varsayım: 32-bit Interrupt Gate, DPL=0, Present
             offset_high: (default_handler_addr >> 16) as u16,
         };

        // Tüm IDT girişlerini varsayılan işleyici ile doldur
        for i in 0..256 {
             IDT[i] = default_entry; // struct Copy olduğu için doğrudan atama
         }


        // 2. Özel Kesme İşleyicilerini IDT'ye Yerleştirme
        // USB kesme işleyicisi fonksiyonunun adresini al
        let usb_interrupt_handler_addr = usb_interrupt_handler as unsafe extern "C" fn() as usize;

        // IDT'nin USB_IRQ_VECTOR numaralı girişini yapılandır (USB IRQ'sunun maplendiği vektör)
        if USB_IRQ_VECTOR < 256 {
             IDT[USB_IRQ_VECTOR as usize].offset_low = (usb_interrupt_handler_addr as u16) & 0xFFFF;
             IDT[USB_IRQ_VECTOR as usize].selector = 0x08; // Kod segment seçicisi (genellikle 0x08 kernel)
             IDT[USB_IRQ_VECTOR as usize].zero = 0;
             IDT[USB_IRQ_VECTOR as usize].type_attributes = 0x8E; // Varlık biti (Present=1), DPL=0, Kesme kapısı (32-bit Interrupt Gate)
             IDT[USB_IRQ_VECTOR as usize].offset_high = (usb_interrupt_handler_addr >> 16) as u16;
         } else {
              #[cfg(feature = "std")] std::eprintln!("KRİTİK HATA: x86 USB IRQ Vektör numarası ({}) IDT boyutundan ({}) büyük!", USB_IRQ_VECTOR, 256);
              #[cfg(not(feature = "std"))] eprintln!("KRİTİK HATA: x86 USB IRQ Vektör numarası ({}) IDT boyutundan ({}) büyük!", USB_IRQ_VECTOR, 256);
               loop { core::hint::spin_loop(); } // Veya halt_system();
         }


        // Diğer istisna işleyicileri (Page Fault, General Protection Fault, Syscall, vb.) de buraya veya exception modülünde ayarlanmalıdır.
        // Exception modülü genellikle temel istisna işleyicilerini IDT'ye yerleştirir.


        // 3. IDT İşaretçi Yapısını Oluşturma ve Yükleme
        // lidt komutu için gerekli IDT işaretçi yapısı
        #[repr(C, packed)] // Doğru paketleme önemli
        struct IDTPointer {
            limit: u16,       // IDT'nin boyutu - 1
            base: u32,        // IDT'nin başlangıç adresi (32-bit)
            // 64-bit (x86_64) için base 64 bit olmalıdır.
        }

        let idt_pointer = IDTPointer {
            limit: (mem::size_of::<[IDTEntry; 256]>() - 1) as u16,
            base: &IDT as *const _ as u32, // static mut IDT'nin 32-bit adresini al
        };

        // IDT'yi işlemciye yükle: lidt komutu
         asm!("lidt [{0}]", in(reg) &idt_pointer); // Pointer'a referans
         asm!("lidt ({0})", in(reg) &idt_pointer, options(nostack, nomem)); // Offset olarak pointer'ı ver

        // 4. Harici Kesme Denetleyicilerini (PIC/APIC/MSI) Yapılandırma ve Kesmeleri Etkinleştirme
        // Bu kısım TAMAMEN DONANIMA ÖZGÜDÜR ve KULLANILAN CHIPSET/ANAKART/IRQ KONTROLCÜSÜNE BAĞLIDIR.
        // Exception modülü genellikle PIC/APIC başlatma ve temel IRQ yönlendirmesini yapar.
        // Sadece USB IRQ'sunu etkinleştirmek için, bu IRQ'yu kontrol eden denetleyiciyi (PIC/APIC/MSI)
        // doğru şekilde ayarlamanız gerekir.

        // Örnek PIC/APIC/MSI yapılandırmaları (YORUM SATIRI OLARAK BIRAKILDI - GERÇEK DEĞER VE YÖNTEM İÇİN KILAVUZA BAKIN!):
        
        // PIC Yeniden Eşleme (Genellikle IRQ 0-15'i vektör 32-47'ye mapler)
         outb(0x20, 0x11); outb(0xA0, 0x11); // ICW1
         outb(0x21, IRQ_BASE_VECTOR); outb(0xA1, IRQ_BASE_VECTOR + 8); // ICW2 (Offset)
         outb(0x21, 0x04); outb(0xA1, 0x02); // ICW3 (Cascade)
         outb(0x21, 0x01); outb(0xA1, 0x01); // ICW4 (8086 mode)

        // Tüm IRQ'ları maskele (şimdilik)
         outb(0x21, 0xFF); outb(0xA1, 0xFF);

        // APIC/MSI yapılandırması çok daha karmaşıktır.

        // USB IRQ'sunu Etkinleştirme (Kullanılan denetleyiciye göre)
         outb(0x21, inb(0x21) & !(1 << USB_IRQ_NUM)); // PIC Master maskesini aç (USB IRQ 11 = bit 11)

        // APIC veya MSI ile etkinleştirme çok daha karmaşık donanım yazmalarını içerir.
        // APIC'in LVT'sini veya IOAPIC'in Redirection Table'ını ayarlama.
        // MSI için PCI yapılandırma alanını ve bellek eşlemeli kontrolcü registerlarını ayarlama.

        // Genel Kesmeleri Etkinleştirme (EFLAGS register'ındaki IF biti)
        // sti (Set Interrupt Flag) komutu kullanılır.
        asm!("sti", options(nostack, nomem)); // Kesmeleri etkinleştir
        // ** UYARI: Kesmeleri etkinleştirmeden önce TÜM İŞLEYİCİLERİN ve İLGİLİ DONANIMIN
        // doğru yapılandırıldığından EMİN OLUN! **
    }
    // Exception modülünü başlat (bu genellikle genel istisna işleyicilerini kurar)
     exception::init(); // <-- exception::init() burada veya daha önce çağrılabilir.
                         // Genellikle IDT kurulduktan sonra çağrılır.

     // init fonksiyonu başarıyla tamamlanırsa geri döner.
}

// USB kesme işleyicisi (DONANIMA ÖZGÜ UYGULAMA GEREKLİ! - ÖRNEK YAPI)
// Bu fonksiyon, IDT'den çağrılır.
#[no_mangle] // Linker script veya IDT tarafından çağrılabilir
// Kesme işleyicisi fonksiyonları genellikle 'unsafe extern "C"' olarak tanımlanır.
// IDT Gate Type 0x8E (32-bit Interrupt Gate) otomatik olarak IF bitini temizler
// ve hat kodu pushlamaz (sadece bazı istisnalar pushlar).
// Bu nedenle 'extern "C" fn()' imzası IRQ handlerları için uygundur.
pub unsafe extern "C" fn usb_interrupt_handler() {
    // ** Güvenlik: Bu işleyici unsafe'dir çünkü kesme bağlamında çalışır **
    // ve donanımla doğrudan etkileşime girer, stack manipülasyonu gerekebilir.
    // TODO: Gerekirse (örneğin iç içe kesmeler veya task switch öncesi) registerları kaydet!

    // ** DİKKAT: Bu bölüm DONANIMA ÖZGÜDÜR ve KULLANILAN ÇİPİN VE USB KONTROLCÜSÜNÜN KILAVUZUNA GÖRE KODLANMALIDIR! **
    // Bu işleyici çekirdek içinde çalışır, kullanıcı alanındaki Sahne64 API'sini doğrudan çağırmaz.
    // Gelen USB verisini alır ve çekirdekteki USB sürücüsü koduna iletir.
    // Ardından, bu veriyi bekleyen kullanıcı görevini Sahne64 çekirdek zamanlama mekanizması
    // aracılığıyla uyandırır (örn. resource::read için bekleyen görev).

    // 1. USB ile ilgili işlemleri gerçekleştir
    // Örnek: USB durum register'ını okuyarak kesme nedenini belirle (veri geldi, TX bitti vb.)
    // ve ilgili işlemleri yap (veri okuma/yazma).
    // Bu genellikle donanım registerlarına volatile okuma/yazma ile yapılır (memory-mapped I/O)
    // veya port I/O (inb/outb) ile yapılır (legacy donanım).

    // Örnek: USB durum register'ını oku (memory-mapped I/O)
     const USB_STATUS_REGISTER_ADDRESS: usize = 0x...; // DONANIMA ÖZGÜ
     let usb_status = ptr::read_volatile(USB_STATUS_REGISTER_ADDRESS as *const u32);
    // TODO: Durum bitlerine göre işlem yap (veri oku/yaz).

    // Debug çıktıları için Sahne64 konsol makrolarını kullan
     #[cfg(feature = "std")] std::println!("x86 USB kesmesi işleniyor (Vektör {}).", USB_IRQ_VECTOR);
     #[cfg(not(feature = "std"))] println!("x86 USB kesmesi işleniyor (Vektör {}).", USB_IRQ_VECTOR); // Sahne64 macro varsayımı


    // 2. Kesme bayrağını temizle ve/veya EOI (End Of Interrupt) gönder (ÇOK ÖNEMLİ! - DONANIMA ÖZGÜ)**
    // Kesme işlendikten sonra, kesme bayrağının TEMİZLENMESİ veya interrupt denetleyicisine
    // EOI gönderilmesi ZORUNLUDUR. Aksi takdirde, aynı kesme sürekli olarak tekrar tetiklenir.
    // Yöntem kullanılan IRQ denetleyicisine (PIC/APIC/MSI) ve çevre birimine bağlıdır.

    // Örnek: PIC için EOI gönderme (SADECE PIC KULLANILIYORSA!)
     outb(0x20, 0x20); // Master PIC EOI komut portu (0x20), EOI değeri (0x20)
    // Eğer USB IRQ'su Slave PIC'ten geliyorsa, Slave PIC'e EOI (0xA0, 0x20) ve Master PIC'e EOI gönderilmeli.
    // Bu işlem port I/O (outb) gerektirir.

    // Örnek: APIC için EOI gönderme (APIC KULLANILIYORSA!)
    // APIC EOI register'ı memory-mapped'dir.
     const APIC_EOI_REGISTER_ADDRESS: usize = ...; // Memory-mapped APIC base + EOI offset
     ptr::write_volatile(APIC_EOI_REGISTER_ADDRESS as *mut u32, 0); // Genellikle 0 yazılır

    // Örnek: MSI için kesme temizleme (MSI KULLANILIYORSA!)
    // MSI, kesme bilgisini belleğe yazar. Bayrak temizleme donanıma özgüdür,
    // genellikle USB kontrolcüsünün kendi registerına yazılır.
     const USB_MSI_IRQ_CLEAR_REGISTER_ADDRESS: usize = ...; // DONANIMA ÖZGÜ
     ptr::write_volatile(USB_MSI_IRQ_CLEAR_REGISTER_ADDRESS as *mut u32, CLEAR_VALUE);

    // TODO: Kullanılan donanıma uygun kesme temizleme/EOI kodunu buraya ekleyin.

    // TODO: Kesme işleyiciden çıkış. Kaydedilen registerları geri yükle.
    // TODO: iret, iretd veya iretq yönergesini kullan. Bu, EFLAGS'ı (ve x66_64'te RFLAGS), CS, EIP (RIP) ve muhtemelen SS, ESP (RSP) değerlerini stack'ten yükler.
     asm!("iret"); // veya "iretd" (32-bit), "iretq" (64-bit)
    // NOT: Exception entry noktası register kaydı ve iret/iretd/iretq'yi yapıyorsa,
    // bu handler sadece işini yapıp normal geri dönebilir.
}
