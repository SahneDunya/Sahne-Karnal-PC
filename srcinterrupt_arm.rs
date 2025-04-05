#![no_std]

use core::arch::asm;
use core::ptr;
use volatile::Volatile;

// **AÇIKLAMA:** Donanım Özelleştirmesi Gereken Değerler **
// Aşağıdaki sabitler, hedef ARM çipinizin ve USB kontrolcünüzün
// teknik özelliklerine göre AYARLANMALIDIR.
// Veri sayfasına (datasheet) başvurmak esastır!

// Kesme Vektör Tablosu (IVT) Adresi
const IVT_ADDRESS: usize = 0x20000000; // Örnek adres, çipinize göre değişir!

// USB Kesme ile ilgili Sabitler
const USB_IRQ_NUMBER: usize = 12;         // Örnek USB kesme numarası, çipinize göre!
const USB_BASE_ADDRESS: usize = 0x40000000;    // Örnek USB temel adresi, çipinize göre!

// USB Kayıt Ofsetleri (USB kontrolcü referans kılavuzundan alınmalıdır)
const USB_DATA_REGISTER_OFFSET: usize = 0x00;        // USB Veri Kaydı Ofseti
const USB_STATUS_REGISTER_OFFSET: usize = 0x04;      // USB Durum Kaydı Ofseti
const USB_INTERRUPT_ENABLE_REGISTER_OFFSET: usize = 0x08; // USB Kesme Etkinleştirme Kaydı Ofseti
const USB_INTERRUPT_FLAG_REGISTER_OFFSET: usize = 0x0C;   // USB Kesme Bayrağı Kaydı Ofseti

// ** Kesme Vektör Tablosu (IVT) Yapılandırması **

// Kesme işleyici fonksiyonları için tip tanımı
type InterruptHandler = extern "C" fn();

// Kesme Vektör Tablosu (IVT) - 256 girişlik statik dizi
#[no_mangle]
static mut IVT: [InterruptHandler; 256] = [default_interrupt_handler; 256];

// Varsayılan kesme işleyicisi (boş işlem yapar)
extern "C" fn default_interrupt_handler() {
    // TODO: Beklenmedik bir kesme oluştuğunda yapılacak işlemler (örn. hata kaydı)
    // Şimdilik boş işleyici olarak bırakıyoruz.
}

// ** USB Kesme İşleme Fonksiyonları **

// USB kesme işleyicisi
#[no_mangle]
pub extern "C" fn usb_interrupt_handler() {
    // ** DİKKAT: Bu bölüm DONANIMA ÖZGÜDÜR ve ÇOK DETAYLI UYGULAMA GEREKTİRİR! **
    // USB kontrolcünüzün referans kılavuzuna bakarak aşağıdaki işlemleri GERÇEKLEŞTİRİN:
    // 1. Kesme nedenini belirle (USB durum kaydını okuyarak)
    // 2. Gerekli USB işlemlerini yap (veri okuma, gönderme, kontrol vb.)
    // 3. Kesme bayrağını TEMİZLE (aksi takdirde kesme sürekli tekrar eder!)

    // --- ÖRNEK USB VERİ OKUMA (Donanıma göre ADAPTASYON GEREKLİ) ---
    unsafe {
        let usb_data_register_address = USB_BASE_ADDRESS + USB_DATA_REGISTER_OFFSET;
        let usb_data = Volatile::new(usb_data_register_address as *mut u32).read();
        // TODO: usb_data ile ilgili işlemler (veri işleme, tampona yazma vb.)

        // --- KESME BAYRAĞINI TEMİZLEME (Donanıma göre MUTLAKA UYGULANMALI) ---
        let usb_interrupt_flag_register_address = USB_BASE_ADDRESS + USB_INTERRUPT_FLAG_REGISTER_OFFSET;
        let mut usb_interrupt_flag = Volatile::new(usb_interrupt_flag_register_address as *mut u32);
        usb_interrupt_flag.write(1 << USB_IRQ_NUMBER); // Örnek: USB kesme bayrağını temizle
         // ** ÖNEMLİ: Kesme bayrağını temizleme yöntemi çipe özgüdür.
         // Bazen farklı bitleri yazmak veya okumak gerekebilir.
         // Mutlaka çipinizin referans kılavuzuna bakın! **
    }

    // TODO: Diğer USB kesme işleme görevleri (durum kontrolü, hata işleme vb.)
}

// ** Başlangıç Fonksiyonu **
pub fn init() {
    unsafe {
        // 1. Kesme Vektör Tablosu (IVT) Kurulumu
        let ivt_ptr = &mut IVT as *mut _ as usize;
        ptr::write_volatile(IVT_ADDRESS as *mut usize, ivt_ptr);
        // ** AÇIKLAMA: IVT adresini sisteme bildirme işlemi DONANIMA ÖZGÜDÜR! **
        // Bazı ARM çiplerde SCB (System Control Block) veya benzeri bir birime
        // IVT adresini yazmak gerekebilir. Bu örnekte doğrudan bellek adresine yazıyoruz.
        // Çipinizin referans kılavuzunu kontrol edin!


        // 2. USB Kesme İşleyicisini IVT'ye Yerleştirme
        let usb_interrupt_handler_address = usb_interrupt_handler as *const _ as usize;
        IVT[USB_IRQ_NUMBER] = core::mem::transmute(usb_interrupt_handler_address);

        // 3. USB Kesmesini Etkinleştirme (DONANIMA ÖZGÜ)
        let usb_interrupt_enable_register_address = USB_BASE_ADDRESS + USB_INTERRUPT_ENABLE_REGISTER_OFFSET;
        let mut usb_interrupt_enable = Volatile::new(usb_interrupt_enable_register_address as *mut u32);
        usb_interrupt_enable.write(1 << USB_IRQ_NUMBER); // Örnek: USB kesmesini etkinleştir
        // ** DİKKAT: Kesme etkinleştirme yöntemi ve bit maskesi DONANIMA ÖZGÜDÜR! **
        // Referans kılavuzundan doğru kayıt adresini ve bit maskesini kontrol edin.


        // 4. Genel Kesmeleri Etkinleştirme (ARM CPSR register'ı ile)
        asm!("cpsie i"); // IRQ (genel kesmeler) etkinleştirme
        // ** UYARI: Genel kesmeleri etkinleştirmeden önce IVT ve kesme işleyicilerin
        // doğru yapılandırıldığından EMİN OLUN! **
    }
}