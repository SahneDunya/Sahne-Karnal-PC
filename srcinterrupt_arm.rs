#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz

// Gerekli core modüllerini içeri aktar
use core::arch::asm; // Inline assembly için
use core::ptr;      // Pointer işlemleri için
use core::mem;      // transmute gibi bellek işlemleri için

// `volatile` crate'inden Volatile sarmalayıcıyı içeri aktar (cargo.toml'da tanımlı olmalı)
use volatile::Volatile;

// Konsol çıktı makrolarını kullanabilmek için (hata durumlarında loglama için)
// Bu, ya crate root'ta #[macro_use] extern crate sahne64; ile yapılır
// ya da Sahne64 crate'i makroları public olarak dışa aktarırsa buradan import edilir.
// Bu örnekte, #[cfg] ile std/no_std çıktısını ayarlayarak makroların
// uygun ortamda kullanılabilir olduğunu varsayıyoruz.
 use sahne64::eprintln; // Örnek import eğer macro publicse

// **AÇIKLAMA:** Donanım Özelleştirmesi Gereken Değerler **
// Aşağıdaki sabitler, hedef ARM çipinizin ve ilgili çevre birimlerinin (örn. GIC, USB kontrolcü)
// teknik özelliklerine göre AYARLANMALIDIR.
// Veri sayfasına (datasheet) başvurmak esastır!

// Kesme Vektör Tablosu (IVT) veya Vektör Tablosu Ofset Kaydı (VTOR) Adresi
// VTOR, ARM Cortex-M'de IVT'nin RAM veya Flash'taki başlangıç adresini tutar.
// Buradaki IVT_ADDRESS, VTOR kaydının adresini simüle ediyor olabilir.
const IVT_ADDRESS: usize = 0xE000ED08; // Tipik Cortex-M VTOR adresi (Örnek, çipinize göre değişir!)
// Eğer vektör tablosunu doğrudan RAM'e koyup adresini yazıyorsanız, IVT_ADDRESS yazacağınız donanım kaydının adresi olmalıdır.

// USB Kesme ile ilgili Sabitler
const USB_IRQ_NUMBER: usize = 12;         // Örnek USB kesme numarası (IRQ numarası), çipinize ve GIC'ye göre!
const USB_BASE_ADDRESS: usize = 0x40000000;    // Örnek USB temel adresi (Memory-mapped I/O), çipinize göre!

// USB Kayıt Ofsetleri (USB kontrolcü referans kılavuzundan alınmalıdır)
const USB_DATA_REGISTER_OFFSET: usize = 0x00;        // USB Veri Kaydı Ofseti
const USB_STATUS_REGISTER_OFFSET: usize = 0x04;      // USB Durum Kaydı Ofseti
const USB_INTERRUPT_ENABLE_REGISTER_OFFSET: usize = 0x08; // USB Kesme Etkinleştirme Kaydı Ofseti
const USB_INTERRUPT_FLAG_REGISTER_OFFSET: usize = 0x0C;   // USB Kesme Bayrağı Kaydı Ofseti
// Bu ofsetler çipe göre değişir. Bit pozisyonları da önemlidir!

// ** Kesme Vektör Tablosu (IVT) Yapılandırması **

// Kesme işleyici fonksiyonları için tip tanımı
// Kesme işleyicileri genellikle 'extern "C"' ve 'unsafe' olmalıdır.
// ARM Cortex-M'de işleyiciler argüman almaz ve geri dönmez.
type InterruptHandler = unsafe extern "C" fn();

// Kesme Vektör Tablosu (IVT) - 256 girişlik statik dizi (Örnek boyut, GIC'ye göre değişir)
// Bu IVT, RAM'de yer alacak ve adresi VTOR kaydına yazılacaktır.
#[no_mangle] // Linker script bu simgeye başvurabilir
static mut IVT: [InterruptHandler; 256] = [default_interrupt_handler; 256];

// Varsayılan kesme işleyicisi (beklenmedik kesmeler için)
// Güvenli olmayan (unsafe) extern "C" fn olarak tanımlanmalıdır.
unsafe extern "C" fn default_interrupt_handler() {
    // ** GÜVENLİK: Bu işleyici unsafe'dir çünkü kesme bağlamında çalışır **
    // ve muhtemelen donanım veya paylaşılan bellekle etkileşime girecektir.

    // TODO: Beklenmedik bir kesme oluştuğunda yapılacak işlemler (örn. hata kaydı, panik)
    // Bu, genellikle GIC (Generic Interrupt Controller) gibi bir birimin
    // ISR (Interrupt Service Register) kaydını okuyarak hangi kesmenin
    // beklendiği belirlenir. Beklenmeyen bir IRQ gelirse:

    // Hata mesajı loglama (Sahne64 konsol makrolarını kullanarak)
    // Kesme işleyicileri hassas bir bağlamda çalışır, loglama dikkatli yapılmalıdır.
    // Loglama fonksiyonu interrupt-safe olmalıdır.
    #[cfg(feature = "std")] std::eprintln!("UYARI: Beklenmeyen kesme (IRQ).");
    #[cfg(not(feature = "std"))] eprintln!("UYARI: Beklenmeyen kesme (IRQ)."); // Sahne64 macro varsayımı

    // Kesme nereden geldi? GIC ISR kaydı okunabilir.
     let irq_source = read_some_gic_register(); // Donanıma özel

    // Kritik bir durumsa sistem durdurulabilir veya panik yapılabilir.
    // Panik handler'ı kesme bağlamında çalışacak şekilde tasarlanmalıdır.
     panic!("Beklenmeyen kesme"); // Eğer panik güvenliyse
     loop {} // Sistem durdurulursa

    // Şimdilik boş işleyici olarak bırakıyoruz (hata ayıklama sırasında kesme kaynağını bulmak gerekebilir).
}

// ** USB Kesme İşleme Fonksiyonları **

// USB kesme işleyicisi
// Güvenli olmayan (unsafe) extern "C" fn olarak tanımlanmalıdır.
#[no_mangle] // Linker script veya bootloader tarafından çağrılabilir
pub unsafe extern "C" fn usb_interrupt_handler() {
    // ** DİKKAT: Bu bölüm DONANIMA ÖZGÜDÜR ve ÇOK DETAYLI UYGULAMA GEREKTİRİR! **
    // USB kontrolcünüzün referans kılavuzuna bakarak aşağıdaki işlemleri GERÇEKLEŞTİRİN:
    // Bu işleyici çekirdek içinde çalışır ve donanımla etkileşim kurar.
    // Kullanıcı alanındaki Sahne64 API'sini doğrudan çağırmaz.
    // Bunun yerine, gelen USB verisini çekirdek tamponlarına yazar
    // ve ardından ilgili Sahne64 Kaynak (Resource) veya Görev (Task)
    // için bekleyenleri uyandırabilir (örn. resource::read için bekleyen görevi uyandır).

    // 1. Kesme nedenini belirle (USB durum/kesme bayrağı kaydını okuyarak)
    let usb_interrupt_flag_register_address = USB_BASE_ADDRESS + USB_INTERRUPT_FLAG_REGISTER_OFFSET;
    let interrupt_status = Volatile::new(usb_interrupt_flag_register_address as *mut u32).read();

    // Örnek: Gelen veri kesmesi (Rx interrupt) veya TX tamamlandı kesmesi (Tx interrupt)
    const USB_RX_IRQ_BIT: u32 = 1 << 0; // Örnek bit pozisyonu
    const USB_TX_DONE_IRQ_BIT: u32 = 1 << 1; // Örnek bit pozisyonu

    if (interrupt_status & USB_RX_IRQ_BIT) != 0 {
        // Gelen veri kesmesi oluştu
        // Veriyi USB kontrolcüsünden oku (hardware-specific)
        // Örnek: Volatile read using the example address
         let usb_data_register_address = USB_BASE_ADDRESS + USB_DATA_REGISTER_OFFSET;
         let received_byte = Volatile::new(usb_data_register_address as *mut u8).read(); // Byte okuma örneği

        // TODO: Okunan veriyi çekirdekteki USB sürücüsü tamponuna yaz
        // Bu tampon, kullanıcı alanından resource::read ile erişilebilen veriyi tutar.
        // Bu, çekirdek içindeki başka bir modülle etkileşim demektir, doğrudan API çağrısı değil.

        // TODO: Resource'u bekleyen görevleri uyandır (örn. resource::read çağrısı yapan görev).
        // Bu, Sahne64 çekirdeği içinde bir senkronizasyon veya zamanlama mekanizması kullanılarak yapılır.
         task::wake_up(waiting_task_id); // API gibi görünüyor ama çekirdek fonksiyonu olmalı
    }

    if (interrupt_status & USB_TX_DONE_IRQ_BIT) != 0 {
         // TX tamamlandı kesmesi oluştu
         // TODO: Gönderilmeyi bekleyen bir sonraki veri bloğunu USB kontrolcüsüne yaz.
         // TODO: Resource'u bekleyen görevleri uyandır (örn. resource::write çağrısının tamamlanmasını bekleyen görev).
    }

    // TODO: Diğer kesme nedenleri (hata kesmeleri, bağlantı kesmeleri vb.)

    // --- KESME BAYRAĞINI TEMİZLEME (Donanıma göre MUTLAKA UYGULANMALI) ---
    // Bu ADIM ÇOK KRİTİKTİR! Bayrak temizlenmezse kesme tekrar tekrar tetiklenir (spinlock gibi olur).
    // Temizleme yöntemi çipe ve kesme kontrolcüsüne (örn. GIC) göre değişir.
    // Bazen kesme bayrağı kaydına 1 yazılır, bazen 0 yazılır, bazen de başka bir kayıt kullanılır.
    // Bu örnekteki yöntem sadece bir simülasyondur.
    Volatile::new(usb_interrupt_flag_register_address as *mut u32).write(interrupt_status); // Örnek: Okunan bayrağı geri yazarak temizle
    // ** Mutlaka çipinizin referans kılavuzuna bakın! **

    // TODO: GIC gibi harici bir kesme kontrolcüsü kullanılıyorsa, burada EOI (End of Interrupt) işlemi yapılmalıdır.
     write_gic_eoi_register(irq_number); // Donanıma özel GIC işlemi
}

// ** Başlatma Fonksiyonu **
/// ARM özelindeki kesme alt sistemini başlatır.
/// Kesme Vektör Tablosu'nu kurar, ilgili kesme işleyicilerini yerleştirir ve kesmeleri etkinleştirir.
/// Bu fonksiyon, sistem başlangıcında (kernel init sürecinde) çağrılmalıdır.
pub fn init() {
    // Bu fonksiyon kernel privilege seviyesinde çalışmalıdır.
    unsafe {
        // 1. Kesme Vektör Tablosu (IVT) Kurulumu
        // IVT dizisinin adresini donanımdaki VTOR kaydına yaz.
        let ivt_ptr = IVT.as_mut_ptr() as usize; // IVT dizisinin bellekteki adresi

        // IVT_ADDRESS (VTOR) adresine IVT dizisinin adresini volatile olarak yaz.
        // Bu, CPU'ya kesme vektörlerini nerede bulacağını söyler.
        ptr::write_volatile(IVT_ADDRESS as *mut usize, ivt_ptr);
        // ** AÇIKLAMA: IVT adresini sisteme bildirme işlemi DONANIMA ÖZGÜDÜR! **
        // Bazı ARM çiplerde SCB (System Control Block) veya benzeri bir birime
        // IVT adresini yazmak gerekebilir. VTOR Cortex-M'de standarttır.
        // Çipinizin referans kılavuzunu kontrol edin!


        // 2. USB Kesme İşleyicisini IVT'ye Yerleştirme
        // USB IRQ numarasına karşılık gelen IVT girişine usb_interrupt_handler fonksiyonunun adresini yaz.
        // Fonksiyon pointer'ını doğru tipe dönüştürmek için transmute kullanılır.
        if USB_IRQ_NUMBER < IVT.len() {
             let handler_fn_ptr = usb_interrupt_handler as InterruptHandler; // Fonksiyon pointer'ı
             IVT[USB_IRQ_NUMBER] = handler_fn_ptr; // IVT'ye yerleştir
             // core::mem::transmute(usb_interrupt_handler_address) yerine doğrudan pointer ataması daha temiz.
        } else {
             // USB_IRQ_NUMBER IVT boyutundan büyükse kritik hata
             #[cfg(feature = "std")] std::eprintln!("KRİTİK HATA: USB IRQ numarası ({}) IVT boyutundan ({}) büyük!", USB_IRQ_NUMBER, IVT.len());
             #[cfg(not(feature = "std"))] eprintln!("KRİTİK HATA: USB IRQ numarası ({}) IVT boyutundan ({}) büyük!", USB_IRQ_NUMBER, IVT.len());
             // Bu durumda sistem başlamamalı veya durdurulmalıdır.
             // halt_system(); // Tanımlıysa çağrılabilir
             loop { core::hint::spin_loop(); } // Veya sonsuz döngü
        }


        // 3. İlgili Çevre Birimi Kesmesini Etkinleştirme (DONANIMA ÖZGÜ)
        // USB kontrolcüsünün kesme etkinleştirme kaydını ayarla.
        let usb_interrupt_enable_register_address = USB_BASE_ADDRESS + USB_INTERRUPT_ENABLE_REGISTER_OFFSET;
        // Kesme etkinleştirme kaydına ilgili bit maskesini volatile olarak yaz.
        // Bu, USB kontrolcüsünün kesme üretmesine izin verir.
        // ** DİKKAT: Kesme etkinleştirme yöntemi ve bit maskesi DONANIMA ÖZGÜDÜR! **
        // Genellikle "set enable" veya "clear enable" kayıdı olur. Buradaki offset ve bit sadece örnek.
        Volatile::new(usb_interrupt_enable_register_address as *mut u32).write(1 << USB_IRQ_NUMBER); // Örnek: IRQ numarasının bitini set et


        // TODO: GIC gibi harici bir kesme kontrolcüsü kullanılıyorsa, GIC'de de bu IRQ etkinleştirilmelidir.
         enable_gic_irq(USB_IRQ_NUMBER); // Donanıma özel GIC işlemi


        // 4. Genel Kesmeleri Etkinleştirme (ARM CPSR register'ı veya özel yönergeler ile)
        // CPSR (Current Program Status Register) içindeki I (IRQ) bitini temizle.
        // Bu, işlemcinin IRQ istisnalarını almasına izin verir.
        asm!("cpsie i", options(nostack)); // cpsie i: Change Processor State, Enable Interrupts (IRQ)
                                            // options(nostack) kesme işleyicilerinde kullanılabilir, burada init'te
                                            // stack manipulation olmadığından güvenli sayılabilir.
        // ** UYARI: Genel kesmeleri etkinleştirmeden önce IVT ve TÜM kesme işleyicilerin
        // doğru yapılandırıldığından ve geçerli olduğundan EMİN OLUN! **
        // Aksi halde ilk kesmede çift hata (double fault) veya kilitlenme olabilir.
    }
    // init fonksiyonu başarıyla tamamlanırsa geri döner.
}
