#![no_std]
use core::arch::asm;
use core::ptr;
use crate::exception;

// x86'da kesme numaraları (IRQ)
// **DİKKAT**: USB_IRQ değeri DONANIMA GÖRE DEĞİŞİR!
// Bu sadece bir ÖRNEK değerdir. Kendi donanımınızın
// teknik dökümanlarından doğru IRQ numarasını öğrenmelisiniz.
const USB_IRQ: u8 = 11; // ÖRNEK USB IRQ numarası. DONANIMA ÖZGÜ DEĞER!

// Kesme tanımlama tablosu (IDT) için yapı
#[repr(C, packed)]
struct IDTEntry {
    offset_low: u16,
    selector: u16,
    zero: u8,
    type_attributes: u8,
    offset_high: u16,
}

// IDT'nin kendisi. Sabit boyutlu bir dizi olarak tanımlanır.
// x86 mimarisinde 256 kesme vektörü vardır.
static mut IDT: [IDTEntry; 256] = [IDTEntry {
    offset_low: 0,
    selector: 0,
    zero: 0,
    type_attributes: 0,
    offset_high: 0,
}; 256];

// Harici sembol olarak IDT'nin başlangıç adresi (linker tarafından sağlanır)
// Bu satıra artık gerek yok çünkü IDT statik değişkeni tanımlandı.
// extern "C" {
//     static IDT: u8; // Kesme tanımlama tablosu (IDT) başlangıç adresi
// }

pub fn init() {
    unsafe {
        // IDT'yi ayarla

        // USB kesme işleyicisi fonksiyonunun adresini al
        let usb_interrupt_handler_addr = usb_interrupt_handler as usize;

        // IDT'nin USB_IRQ numaralı girişini yapılandır
        IDT[USB_IRQ as usize].offset_low = (usb_interrupt_handler_addr as u16) & 0xFFFF;
        IDT[USB_IRQ as usize].selector = 0x08; // Kod segment seçicisi (genellikle 0x08)
        IDT[USB_IRQ as usize].zero = 0;
        IDT[USB_IRQ as usize].type_attributes = 0x8E; // Varlık biti, DPL=0, Kesme kapısı (32-bit)
        IDT[USB_IRQ as usize].offset_high = (usb_interrupt_handler_addr >> 16) as u16;


        // IDT işaretçi yapısı (lidt komutu için gerekli)
        #[repr(C, packed)]
        struct IDTPointer {
            limit: u16,       // IDT'nin boyutu - 1
            base: u32,        // IDT'nin başlangıç adresi
        }

        let idt_pointer = IDTPointer {
            limit: (core::mem::size_of::<[IDTEntry; 256]>() - 1) as u16,
            base: &IDT as *const _ as u32,
        };


        // IDT'yi yükle
        asm!(
            "lidt [{0}]", // IDT'yi yükle
            in(reg) &idt_pointer
        );

        // **KRİTİK**: USB kesmesini etkinleştirme kısmı DONANIMA ÖZGÜDÜR!
        // Aşağıdaki örnek kod BLOKE EDİLMİŞTİR ve ÇALIŞMAYABİLİR.
        // Bu kısım, hedef donanımınızın interrupt controller'ına (PIC, APIC, vb.)
        // ve USB kontrolcüsünün interrupt enable mekanizmasına göre
        // TAMAMEN DEĞİŞKENLİK GÖSTERİR.

        // **ÖRNEK 1: PIC (Programmable Interrupt Controller) ile USB kesmesini etkinleştirme (ÇOK BASİT VE GENELDE YANLIŞ)**
        // Eğer sisteminizde sadece PIC varsa ve USB kontrolcüsü doğrudan PIC'e bağlıysa (NADİR DURUM):
        // outb(0x21, inb(0x21) & !(1 << USB_IRQ)); // PIC mask kaydını değiştirerek USB IRQ'sunu aç.
        // **UYARI**: Bu kod modern sistemlerde ÇALIŞMAYABİLİR. APIC yaygın olarak kullanılır.

        // **ÖRNEK 2: APIC (Advanced Programmable Interrupt Controller) ile USB kesmesini etkinleştirme (ÇOK DAHA KARMAŞIK VE DONANIMA ÖZGÜ)**
        // APIC kullanılıyorsa, memory-mapped APIC register'larını doğru şekilde yapılandırmak gerekir.
        // Bu işlem chipset'e ve anakart tasarımına göre DEĞİŞİR.
        // APIC Local Vector Table (LVT) veya Interrupt Destination Registers (IDR) gibi kayıtlar
        // doğru değerlerle yapılandırılmalıdır.
        // BU KISIM İÇİN DONANIM DÖKÜMANTASYONUNA BAŞVURMAK ZORUNLUDUR.
        // Örnek APIC yapılandırması (TAMAMEN HAYAL ÜRÜNÜ VE ÇALIŞMAYACAKTIR):
        // volatile_store!(APIC_BASE + APIC_LVT_OFFSET + USB_IRQ * 0x10, APIC_LVT_ENABLE | USB_IRQ_VECTOR);
        // ... diğer APIC kayıtları yapılandırılabilir ...
        // **UYARI**: APIC yapılandırması çok karmaşıktır ve bu örnek KESİNLİKLE ÇALIŞMAYACAKTIR.

        // **ÖRNEK 3: MSI (Message Signaled Interrupts) ile USB kesmesini etkinleştirme (MODERN SİSTEMLERDE YAYGIN)**
        // Modern USB kontrolcüleri genellikle MSI kullanır. MSI yapılandırması,
        // kontrolcünün kendi register'ları üzerinden yapılır, interrupt controller (PIC/APIC) ile DOĞRUDAN İLGİLİ DEĞİLDİR.
        // USB kontrolcüsünün memory-mapped register'larına yazarak MSI etkinleştirilir ve
        // hangi bellek adresine ve veri ile hangi kesme vektörünün gönderileceği ayarlanır.
        // BU KISIM USB KONTROLCÜSÜNÜN DÖKÜMANTASYONUNA BAŞVURMAYI GEREKTİRİR.
        // Örnek MSI yapılandırması (TAMAMEN HAYAL ÜRÜNÜ VE ÇALIŞMAYACAKTIR):
        // volatile_store!(USB_CTRL_BASE + USB_MSI_ENABLE_REG, MSI_ENABLE_BIT | MSI_ADDRESS | MSI_DATA);
        // ... diğer MSI kayıtları yapılandırılabilir ...
        // **UYARI**: MSI yapılandırması da donanıma özgüdür ve bu örnek KESİNLİKLE ÇALIŞMAYACAKTIR.


    }
    exception::init(); // İstisna işleyicisini başlat
}

// Örnek USB kesme işleyicisi (DONANIMA ÖZGÜ UYGULAMA GEREKLİ)
#[no_mangle]
pub extern "C" fn usb_interrupt_handler() {
    // **KRİTİK**: Bu fonksiyonun içeriği DONANIMA ÖZGÜDÜR!
    // Gerçek USB sürücü kodu burada yer alır.
    // Bu fonksiyon, USB kontrolcüsünden veri okuma, veri gönderme,
    // USB durumunu kontrol etme, USB paketlerini işleme vb.
    // işlemleri gerçekleştirmelidir.

    // **ÖRNEK USB İŞLEMLERİ (TAMAMEN HAYAL ÜRÜNÜ VE ÇALIŞMAYACAKTIR):**
    // let status = volatile_load!(USB_STATUS_REGISTER); // USB durum kaydını oku
    // if status & USB_DATA_RECEIVED_BIT != 0 {
    //     // Veri alındı, işle
    //     let data = volatile_load!(USB_DATA_REGISTER); // Veri kaydını oku
    //     // ... veriyi işle ...
    // }
    // volatile_store!(USB_COMMAND_REGISTER, USB_SEND_ACK_COMMAND); // Onay gönder

    // **ÇOK ÖNEMLİ**: Kesme bayrağını temizleyin ve/veya EOI (End Of Interrupt) gönderin.
    // **BU KISIM DA DONANIMA ÖZGÜDÜR!** Aksi takdirde kesme sürekli tetiklenir (kesme fırtınası).

    unsafe {
        // **ÖRNEK 1: USB durum kaydından kesme bayrağını temizleme (DONANIMA ÖZGÜ)**
        // volatile_store!(USB_STATUS_REGISTER, status & !USB_INTERRUPT_FLAG_BIT); // Bayrağı temizle
        // **UYARI**: Bu sadece bir örnek. Doğru yöntem donanım dökümanında belirtilir.


        // **ÖRNEK 2: PIC ile kesme onaylama (EOI) gönderme (SADECE PIC KULLANILIYORSA)**
        // Eğer sistem PIC kullanıyorsa, kesmenin işlendiğini PIC'e bildirmek için EOI gönderilmelidir.
        // outb(0x20, 0x20); // PIC'e EOI gönder (ana PIC için)
        // Eğer cascade yapıda PIC varsa (8259A), slave PIC için de EOI gönderilmesi gerekebilir:
        // outb(0xA0, 0x20); // PIC'e EOI gönder (ikincil/slave PIC için)
        // **UYARI**: EOI gönderme işlemi sadece PIC tabanlı sistemler için geçerlidir. APIC veya MSI kullanılıyorsa GEREKLİ DEĞİLDİR veya YANLIŞTIR.

        // **ÖRNEK 3: APIC veya MSI ile kesme onaylama (DONANIMA ÖZGÜ)**
        // APIC veya MSI kullanılıyorsa, kesme onaylama mekanizması farklıdır ve genellikle
        // memory-mapped APIC register'larına veya USB kontrolcüsünün kendi register'larına yazmayı içerir.
        // BU KISIM İÇİN DONANIM DÖKÜMANTASYONUNA BAŞVURMAK ZORUNLUDUR.
        // Örnek APIC EOI (TAMAMEN HAYAL ÜRÜNÜ VE ÇALIŞMAYACAKTIR):
        // volatile_store!(APIC_BASE + APIC_EOI_REGISTER, 0); // APIC EOI kaydına yaz
        // Örnek MSI EOI (TAMAMEN HAYAL ÜRÜNÜ VE ÇALIŞMAYACAKTIR):
        // volatile_store!(USB_CTRL_BASE + USB_MSI_EOI_REG, EOI_VALUE); // USB kontrolcü MSI EOI kaydına yaz
        // **UYARI**: APIC/MSI EOI yöntemleri donanıma göre değişir ve bu örnekler KESİNLİKLE ÇALIŞMAYACAKTIR.
    }
}