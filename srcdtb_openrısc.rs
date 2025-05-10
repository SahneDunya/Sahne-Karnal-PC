#![no_std] // Bu dosya da çekirdek alanında çalışıyor

use crate::karnal64::{ // Karnal64 modüllerine erişim varsayılıyor
    kresource, // Kaynak yönetimi modülü
    KError,    // Hata tipi
    KHandle,   // Handle tipi
    // İhtiyaç duyulursa KTaskId, KThreadId vb.
};

// DTB (Device Tree Blob) verisini tutacak statik tampon.
// Gerçek bir sistemde boyut tahmini veya dinamik bellek yönetimi gerekebilir.
// Şimdilik sabit ve makul büyüklükte bir tampon varsayalım.
const MAX_DTB_SIZE: usize = 64 * 1024; // 64 KB
static mut DTB_BUFFER: [u8; MAX_DTB_SIZE] = [0; MAX_DTB_SIZE];
static mut DTB_SIZE: usize = 0; // Gerçek okunan DTB boyutu

/// DTB kaynağını Karnal64 üzerinden yükler ve statik tampona kaydeder.
/// Başarı durumunda DTB verisinin statik slice'ını, hata durumunda KError döner.
pub fn load_dtb() -> Result<&'static [u8], KError> {
    // DTB kaynağının adı/path'i. Karnal64'ün kaynak kayıt sisteminde
    // bu ismin bir ResourceProvider'a eşlendiği varsayılır.
    let dtb_resource_name = "karnal://boot/dtb"; // Örnek path

    // 1. DTB kaynağını Karnal64 üzerinden edin (acquire).
    // Okuma izni talep ediyoruz.
    let dtb_handle = kresource::resource_acquire(
        dtb_resource_name.as_ptr(),
        dtb_resource_name.len(),
        kresource::MODE_READ, // Okuma modu
    )?; // KError::NotFound, KError::PermissionDenied, KError::InvalidArgument dönebilir

    // Başarı durumunda bir KHandle elde ettik.
    crate::println!("DTB kaynağı başarıyla edinildi. Handle: {:?}", dtb_handle);

    // 2. Kaynaktan DTB verisini oku.
    // Okuma işlemi için çekirdek alanındaki statik tamponumuzu kullanacağız.
    let bytes_read = unsafe {
        kresource::resource_read(
            dtb_handle.0, // Ham handle değeri
            DTB_BUFFER.as_mut_ptr(), // Tampon pointer'ı (writable)
            DTB_BUFFER.len(), // Tampon boyutu
        )? // KError::BadHandle, KError::BadAddress, KError::Interrupted vb. dönebilir
    };

    // Okunan byte sayısını güncelleyelim.
    unsafe {
        DTB_SIZE = bytes_read;
    }

    crate::println!("DTB verisi başarıyla okundu. {} byte.", bytes_read);

    // 3. İşimiz bitince kaynağı serbest bırak (release).
    // Bu, Handle Yöneticisindeki kaydı temizler.
    kresource::resource_release(dtb_handle.0)?; // KError::BadHandle vb. dönebilir

    crate::println!("DTB kaynağı serbest bırakıldı.");

    // 4. Okunan veriyi içeren slice'ı döndür.
    // Statik tampondan okunan byte sayısı kadar bir slice oluşturulur.
    let dtb_data_slice = unsafe {
        // Güvenlik: DTB_BUFFER statik mutable, DTB_SIZE okunan byte sayısıdır.
        // Slice oluşturma güvenlidir, çünkü okunan boyutu kullanıyoruz.
        core::slice::from_raw_parts(DTB_BUFFER.as_ptr(), DTB_SIZE)
    };

    Ok(dtb_data_slice)
}

/// DTB verisine erişim sağlayan fonksiyon (eğer daha önce yüklendiyse).
/// load_dtb() çağrıldıktan sonra kullanılır.
pub fn get_loaded_dtb() -> Option<&'static [u8]> {
    unsafe {
        if DTB_SIZE > 0 {
            Some(core::slice::from_raw_parts(DTB_BUFFER.as_ptr(), DTB_SIZE))
        } else {
            None // DTB henüz yüklenmemiş veya hata oluşmuş
        }
    }
}

// TODO: DTB verisini parse edecek ve donanım bilgilerini çıkaracak fonksiyonlar eklenebilir.
// Örneğin: find_node_by_compatible, get_property vb.
// Bu parsing mantığı OF-fdt (Open Firmware Flattened Device Tree) spesifikasyonuna göre yapılır.
// Şu an sadece veriyi yükleme ve erişim kısmı Karnal64 ile etkileşimi gösteriyor.

// Çekirdek içinden kullanılacak placeholder print! makrosu (debug amaçlı)
// Gerçek çekirdekte konsol sürücüsü üzerinden implemente edilmelidir.
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ({
        // Çekirdek içi konsol yazma mantığı buraya gelir
        // Şimdilik boş bırakalım veya debug output'a yönlendirelim
         #[cfg(feature = "debug_console")]
         crate::debug_console::print(format_args!($($arg)*));
    });
}

// Placeholder debug_console modülü (eğer println! kullanılacaksa)
#[cfg(feature = "debug_console")]
mod debug_console {
     use core::fmt::Write;
     // Dummy writer veya gerçek donanım (UART) yazıcısı
     struct DummyWriter;

     impl Write for DummyWriter {
         fn write_str(&mut self, s: &str) -> core::fmt::Result {
             // Buraya gerçek UART veya konsol yazma kodu gelir
              print!("{}", s); // Bu olmaz, std gerektirir. Donanıma yazmalı.
             Ok(())
         }
     }

     pub fn print(args: core::fmt::Arguments) {
         let mut writer = DummyWriter;
         writer.write_fmt(args).ok();
     }
}
