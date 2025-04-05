#![no_std]

use core::arch::asm;
use core::ptr::NonNull;
use fdt::{Fdt, FdtError};

/// ARM mimarisinde DTB adresini almak için kullanılan fonksiyon.
///
/// # ARM Mimarisine Özgü Notlar
///
/// ARM mimarisinde DTB adresi genellikle önyükleyici (bootloader) tarafından belirli bir yazmaca veya
/// bellekte belirli bir adrese yerleştirilir. Bu adresin konumu, kullanılan önyükleyiciye, platforma ve
/// ARM mimarisinin (32-bit ARMv7, 64-bit AArch64 vb.) türüne göre değişiklik gösterebilir.
///
/// **Örnek Yaklaşımlar (Platforma ve Önyükleyiciye Bağlı Değişir):**
///
/// 1.  **Yazmaçlardan Alma (Örnek):** Bazı önyükleyiciler DTB adresini belirli bir yazmaca (örneğin R1, R2 veya AArch64'te X1, X2) yerleştirebilir.
///     Bu örnek kodda, **varsayılan olarak X2 (AArch64) veya R2 (ARM)** yazmacından DTB adresinin alındığı varsayılmıştır.
///     **UYARI:** Bu sadece bir örnektir ve gerçek sistemlerde DTB adresi farklı yazmaçlarda veya yöntemlerle iletilebilir.
///
/// 2.  **Bellek Konumundan Alma (ATAG'ler, vb.):**  Eski ARM sistemlerinde veya bazı önyükleyicilerde, ATAG listesi gibi yapılar aracılığıyla
///     DTB adresi bellekte belirli bir konumda saklanabilir. ATAG'ler, bellekteki etiketlenmiş yapılar olup, sistem belleği ve donanımı
///     hakkında bilgi içerir. ATAG listesi içinde DTB adresini işaret eden bir etiket bulunabilir.
///
/// 3.  **UEFI Konfigürasyon Tabloları (Modern Sistemler):** UEFI (Unified Extensible Firmware Interface) kullanan modern ARM sistemlerinde,
///     DTB adresi UEFI konfigürasyon tabloları aracılığıyla elde edilebilir. Bu, daha standart bir yaklaşımdır ve işletim sisteminin
///     UEFI ortamından DTB'yi bulmasını sağlar.
///
/// **Bu Fonksiyonun Uygulanması:**
///
/// Bu `get_dtb_address` fonksiyonu, hedef ARM platformunuza ve kullandığınız önyükleyiciye göre uyarlanmalıdır.
/// Aşağıdaki örnek kod, **X2 (AArch64) veya R2 (ARM)** yazmacından DTB adresini almayı varsayar. Gerçek bir sistemde,
/// bu fonksiyonun içeriği farklılık gösterebilir. Örneğin, ATAG listesini taramak veya UEFI tablolarını okumak gerekebilir.
///
/// **Önemli:** Doğru DTB adresini alma yöntemi, donanım platformuna ve önyükleyiciye özgüdür. Üretici belgelerini ve önyükleyici
/// kaynak kodunu inceleyerek doğru yöntemi belirlemeniz gereklidir.
#[cfg(target_arch = "aarch64")]
pub fn get_dtb_address() -> usize {
    let dtb_address: usize;
    unsafe {
        // AArch64 mimarisinde DTB adresini X2 yazmacından al (varsayılan yaklaşım, platforma göre değişebilir).
        asm!("mov {}, x2", out(reg) dtb_address);
    }
    dtb_address
}

#[cfg(target_arch = "arm")]
pub fn get_dtb_address() -> usize {
    let dtb_address: usize;
    unsafe {
        // ARM mimarisinde DTB adresini R2 yazmacından al (varsayılan yaklaşım, platforma göre değişebilir).
        asm!("mov {}, r2", out(reg) dtb_address);
    }
    dtb_address
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "arm")))]
pub fn get_dtb_address() -> usize {
    // ARM mimarisi dışında bir mimari için DTB adresi alma yöntemi tanımlanmamış.
    // Bu durumda varsayılan bir adres döndürülüyor.
    // UYARI: Bu sadece bir örnektir ve gerçek sistemlerde doğru adres alma yöntemi platforma özgü olmalıdır!
    println!("UYARI: ARM dışı mimari için varsayılan DTB adresi (0x1000000) kullanılıyor. Doğru adres alma yöntemini uygulayın!");
    0x1000000 // Varsayılan adres (Örnek değer, gerçekte platforma göre değişir!)
}


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

    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    {
        // ARM mimarisinde DTB adresini al.
        dtb_address = get_dtb_address();
    }
    #[cfg(not(any(target_arch = "aarch64", target_arch = "arm")))]
    {
        // Diğer mimariler için varsayılan bir adres (UYARI: Bu sadece bir örnektir, gerçekte mimariye göre değişir!)
        dtb_address = 0x100000;
        println!("UYARI: ARM dışı mimari için varsayılan DTB adresi kullanılıyor: 0x{:X}. Doğru adresi ayarlayın!", dtb_address);
    }

    // DTB'yi yükle ve olası hataları işle.
    let dtb = load_dtb(dtb_address)?;

    // Uyumluluk bilgisini yazdır.
    print_compatible(&dtb);

    Ok(())
}