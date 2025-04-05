#![no_std]

use core::arch::asm; // Elbrus için assembly gerekebilir, simdilik ekliyoruz.
use core::ptr::NonNull;
use fdt::{Fdt, FdtError};

// Elbrus mimarisi için DTB adresini alma fonksiyonu
#[cfg(target_arch = "elbrus")] // Lütfen 'elbrus' hedef mimarisinin doğru olup olmadığını kontrol edin.
/// Elbrus mimarisinde DTB adresini bir registerdan veya sabit bir adresten alır.
/// Bu örnekte, varsayılan bir bellek adresi kullanılmaktadır.
pub fn get_dtb_address() -> usize {
    let dtb_address: usize;

    // *** UYARI: Elbrus mimarisine özgü DTB adresini alma yöntemi burada uygulanmalıdır. ***
    // *** Aşağıdaki örnek adres sadece bir YER TUTUCUDUR ve gerçek sistemde farklılık gösterebilir. ***
    // *** Elbrus sisteminizin boot protokolüne ve DTB'nin nasıl konumlandırıldığına göre bu kısmı ayarlayın. ***

    // Örnek olarak, varsayılan bir bellek adresi kullanıyoruz.
    dtb_address = 0x200000; // Örnek adres, gerçek Elbrus sistemine göre değişmelidir!
    println!("UYARI: Elbrus mimarisi için varsayılan DTB adresi kullanılıyor: 0x{:X}. Doğru adresi ayarlayın!", dtb_address);


    // *** Eğer Elbrus mimarisinde DTB adresi bir yazmaçta (register) tutuluyorsa, ***
    // *** aşağıdaki gibi satır içi assembly kullanmanız gerekebilir (RISC-V örneğindeki gibi): ***
    /*
    unsafe {
        asm!(
            "// Elbrus'a özgü assembly komutu buraya", // Gerçek komutu Elbrus ISA'ya göre yazın!
            "// Örneğin, DTB adresini bir registerdan dtb_address'e taşıyacak komut",
            out(reg) dtb_address, // 'reg' uygun register sınıfını temsil etmeli.
            // in(...), options(...)  gerekirse eklenebilir.
        );
    }
    */


    dtb_address
}

// DTB'yi bellek adresinden yükleme fonksiyonu (Mimari bağımsız)
/// Verilen bellek adresinden bir Fdt (Device Tree Blob) yapısı yükler.
///
/// # Arguments
///
/// * `dtb_address` - DTB'nin bellek adresi.
///
/// # Returns
///
/// `Ok(Fdt)` eğer DTB başarıyla yüklendiyse, `Err(FdtError)` aksi takdirde.
///
/// # Errors
///
/// `FdtError::NullPtr` eğer verilen adres geçerli bir işaretçi değilse.
pub fn load_dtb(dtb_address: usize) -> Result<Fdt<'static>, FdtError> {
    // Verilen adresi ham bir işaretçiye dönüştür ve NonNull ile kontrol et.
    let ptr = NonNull::new(dtb_address as *const u8).ok_or(FdtError::NullPtr)?;
    // Güvenli olmayan blok: ham işaretçiden Fdt yapısı oluşturuluyor.
    unsafe { Fdt::from_ptr(ptr.as_ptr()) }
}

// Device Tree node özelliğini alma fonksiyonu (Mimari bağımsız)
/// Bir Device Tree node'unun belirli bir özelliğini alır.
///
/// # Arguments
///
/// * `dtb` - Fdt yapısı referansı.
/// * `node_path` - Node'un yolu (örneğin "/memory").
/// * `property_name` - Özellik adı (örneğin "reg").
///
/// # Returns
///
/// `Some(&[u8])` eğer özellik bulunduysa, `None` aksi takdirde.
pub fn get_property<'a>(dtb: &'a Fdt, node_path: &str, property_name: &str) -> Option<&'a [u8]> {
    dtb.find_node(node_path) // Node'u bul
        .and_then(|node| node.property(property_name)) // Node içinde özelliği bul
        .map(|property| property.value()) // Özellik değerini al
}

// Device Tree node string özelliğini alma fonksiyonu (Mimari bağımsız)
/// Bir Device Tree node'unun belirli bir string özelliğini alır.
///
/// # Arguments
///
/// * `dtb` - Fdt yapısı referansı.
/// * `node_path` - Node'un yolu.
/// * `property_name` - Özellik adı.
///
/// # Returns
///
/// `Some(&str)` eğer özellik bulundu ve UTF-8 string olarak çözümlenebildiyse, `None` aksi takdirde.
pub fn get_property_str(dtb: &Fdt, node_path: &str, property_name: &str) -> Option<&str> {
    get_property(dtb, node_path, property_name) // Özelliği byte dizisi olarak al
        .and_then(|value| core::str::from_utf8(value).ok()) // Byte dizisini UTF-8 string'e dönüştürmeyi dene
}

// Kök node "compatible" özelliğini yazdırma fonksiyonu (Mimari bağımsız)
/// Kök node'un "compatible" özelliğini okur ve yazdırır.
/// Bu genellikle cihaz uyumluluğunu belirten bir stringdir.
///
/// # Arguments
///
/// * `dtb` - Fdt yapısı referansı.
pub fn print_compatible(dtb: &Fdt) {
    if let Some(compatible) = get_property_str(dtb, "/", "compatible") {
        println!("Cihaz uyumluluğu: {}", compatible);
    } else {
        println!("Uyumluluk bilgisi bulunamadı."); // Uyumluluk özelliği bulunamazsa bilgi mesajı
    }
}

// Örnek init fonksiyonu (Mimariden DTB adresini almayı kullanır)
/// Örnek init fonksiyonu: DTB'yi yükler ve uyumluluk bilgisini yazdırır.
/// Bu fonksiyon, çekirdek veya bootloader gibi ortamlarda kullanılmak üzere tasarlanmıştır.
///
/// # Returns
///
/// `Ok(())` eğer init başarıyla tamamlandıysa, `Err(FdtError)` aksi takdirde.
///
/// # Errors
///
/// `FdtError` DTB yükleme sırasında bir hata oluşursa.
pub fn init() -> Result<(), FdtError>{
    let dtb_address: usize;

    #[cfg(target_arch = "elbrus")] // Elbrus mimarisi için DTB adresini al
    {
        dtb_address = get_dtb_address();
    }
    #[cfg(not(target_arch = "elbrus"))] // Diğer mimariler için varsayılan adres (UYARI!)
    {
        // Diğer mimariler için varsayılan bir adres (UYARI: Bu sadece bir örnektir, gerçekte mimariye göre değişir!)
        dtb_address = 0x100000;
        println!("UYARI: Elbrus dışı mimari için varsayılan DTB adresi kullanılıyor: 0x{:X}. Doğru adresi ayarlayın!", dtb_address);
    }

    // DTB'yi yükle ve olası hataları işle.
    let dtb = load_dtb(dtb_address)?;

    // Uyumluluk bilgisini yazdır.
    print_compatible(&dtb);

    Ok(())
}