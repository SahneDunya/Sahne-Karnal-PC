#![no_std]

// Geliştirme sırasında kullanılmayan kod veya argümanlar için izinler
#![allow(dead_code)]
#![allow(unused_variables)]

// Çekirdek türleri ve hata kodu için ana Karnal64 modülünden KError'ı import et.
// Karnal64 API'sını implement etmiyoruz, sadece onun tanımladığı hata tipini kullanıyoruz.
use crate::kernel_types::KError; // KError'ın tanımlandığı gerçek yolunuza göre ayarlayın

// Bellek ayırma (allocate) ihtiyacı olan fdt-rs ve diğer yapılar için `alloc` gereklidir.
extern crate alloc;

// fdt-rs crate'inden gerekli yapıları import et.
use fdt_rs::prelude::*; // Node ve Prop gibi traitler için
use fdt_rs::{
    error::FdtError,
    // Fdt yapısı, bir Cihaz Ağacı Blobu'nu temsil eder
    Fdt,
    // Düğüm (node) ve Özellik (property) yapıları
    Node,
    Prop,
};

// Çoklu iş parçacığı ortamında ayrıştırılmış DTB verisine güvenli erişim için bir spinlock kullan.
// `spin` crate'i `#![no_std]` için uygun bir Mutex sağlar.
use spin::Mutex;

// --- DTB'den Ayrıştırılan Verileri Tutacak Yapı ---
/// DTB'den çıkarılan temel sistem bilgilerini tutar.
struct ParsedDtbData<'a> {
    /// Ayrıştırılmış FDT nesnesinin kendisi.
    /// `'static` ömrü varsayımı, DTB belleğinin çekirdek ömrü boyunca geçerli kalacağı anlamına gelir.
    /// Güvenlik Notu: Bu, bootloader tarafından sağlanan adresin çekirdek tarafından eşlenmiş ve
    /// erişilebilir olduğundan emin olmayı gerektirir.
    fdt: Fdt<'a>,
    /// Bellek bölgelerinin listesi (adres, boyut).
    /// DTB'nin `/memory` düğümündeki `reg` özelliğinden çıkarılır.
    memory_regions: alloc::vec::Vec<(u64, u64)>,
    // TODO: İhtiyaç duyulan diğer temel bilgiler buraya eklenebilir:
    // - CPU bilgileri (sayısı, çekirdek ID'leri)
    // - Kesme denetleyicisi (interrupt controller) bilgileri
    // - Seri port/konsol cihaz yolu
    // - Boot argümanları (`/chosen` düğümünden)
}

// Ayrıştırılmış DTB verilerini tutan global (çekirdek içi) değişken.
// `Mutex` ile korunur, böylece farklı çekirdek iş parçacıkları/çekirdek kodları ona güvenli erişebilir.
static PARSED_DTB: Mutex<Option<ParsedDtbData<'static>>> = Mutex::new(None);

// --- Modülün Genel API'sı (Diğer Çekirdek Bileşenleri Tarafından Kullanılır) ---

/// DTB ayrıştırma modülünü başlatır ve DTB'yi ayrıştırarak bilgiyi global olarak saklar.
/// Bu fonksiyon çekirdek başlatma (boot) sürecinin başlarında, DTB'nin adresi bilindiğinde çağrılmalıdır.
///
/// `dtb_ptr`: Bootloader tarafından sağlanan DTB'nin bellekteki başlangıç adresi.
/// `dtb_size`: DTB bloğunun boyutu (byte olarak).
///
/// Başarı durumunda `Ok(())`, hata durumunda `KError` döner.
/// Güvenlik Notu: `dtb_ptr` adresinin geçerli ve okunabilir olduğundan emin olunmalıdır.
pub fn init(dtb_ptr: *const u8, dtb_size: usize) -> Result<(), KError> {
    // Güvenlik/Doğrulama: Gelen pointer ve boyutun geçerli olup olmadığını kontrol et.
    // Daha kapsamlı doğrulama (bellek haritasına göre geçerlilik kontrolü) gerekebilir.
    if dtb_ptr.is_null() || dtb_size == 0 {
        // Karnal64'ün hata tipini kullan.
        return Err(KError::InvalidArgument);
    }

    // Ham pointer'dan bir `fdt_rs::Fdt` nesnesi oluştur.
    // Bu `unsafe` bir işlemdir çünkü ham pointer'a güveniyoruz.
    // Fdt::from_ptr, pointer'ın işaret ettiği belleğin DTB formatında olduğunu varsayar.
    let fdt = unsafe {
        // fdt-rs'in from_ptr fonksiyonu usize alır, bu yüzden *const u8'den usize'a cast yapıyoruz.
        // 'static ömrü, DTB'nin çekirdek çalıştığı sürece bellekte kalacağını varsayar.
        Fdt::from_ptr(dtb_ptr as usize).map_err(|e| map_fdt_error(e))?
    };

    // DTB başlığını doğrulayarak temel bir format kontrolü yap.
    fdt.header().map_err(|e| map_fdt_error(e))?;

    // DTB'den ilgili bilgileri ayrıştır.
    let memory_regions = extract_memory_regions(&fdt)?;
    // TODO: Diğer extraction fonksiyonlarını çağır (CPU, kesmeler vb.)

    // Ayrıştırılan veriyi global Mutex korumalı değişkene kaydet.
    let mut parsed_data = PARSED_DTB.lock();

    // DTB zaten başlatılmışsa hata ver (tekrar başlatılmamalı).
    if parsed_data.is_some() {
        // Karnal64'ün AlreadyExists hatasını kullan.
        return Err(KError::AlreadyExists);
    }

    // Veriyi Mutex içerisine taşı.
    *parsed_data = Some(ParsedDtbData {
        fdt,
        memory_regions,
        // TODO: Ayrıştırılan diğer verileri ekle
    });

    // Başarı.
    Ok(())
}

/// Ayrıştırılmış DTB'den bellek bölgelerini döndürür.
/// Çekirdeğin bellek yöneticisi tarafından kullanılabilir.
///
/// Başarı durumunda bellek bölgelerinin listesini, hata durumunda `KError` döner.
pub fn get_memory_regions() -> Result<alloc::vec::Vec<(u64, u64)>, KError> {
    // Mutex'i kilitle ve ayrıştırılmış veriye eriş.
    let parsed_data = PARSED_DTB.lock();

    // Veri mevcut mu kontrol et. Modül init edilmemişse hata döner.
    match &*parsed_data {
        Some(data) => {
            // Bellek bölgelerinin bir kopyasını döndür.
            // Bu, çağıranın Mutex'i tutmasına gerek kalmamasını sağlar.
            Ok(data.memory_regions.clone())
        }
        None => {
            // DTB init edilmemişse hata döndür.
            Err(KError::InternalError) // Veya KError::NotFound gibi daha spesifik bir hata
        }
    }
}

/// Belirli bir "compatible" dizesine sahip bir cihaz düğümünü bulur.
/// Sürücüler veya cihaz yöneticisi tarafından kullanılabilir.
///
/// `compatible`: Aranacak "compatible" dizesi (örn. "ns16550a", "simple-framebuffer").
///
/// Başarı durumunda bulunan düğümü (varsa) veya `None`, hata durumunda `KError` döner.
pub fn find_compatible_device<'a>(compatible: &str) -> Result<Option<Node<'a, 'static>>, KError> {
    let parsed_data = PARSED_DTB.lock();
    match &*parsed_data {
        Some(data) => {
            let fdt = &data.fdt;
            // DTB'nin kök düğümünden başlayarak tüm düğümleri dolaş.
            for node in fdt.root().nodes() {
                // Düğümde "compatible" özelliği var mı kontrol et.
                if let Some(prop) = node.props().find(|p| p.name() == Ok("compatible")) {
                    // "compatible" özelliği birden fazla dize içerebilir (list of strings).
                    if let Ok(compat_list) = prop.value().as_str_list() {
                        // Aranan dize listede var mı kontrol et.
                        if compat_list.iter().any(|&s| s == compatible) {
                            // Bulundu, düğümü döndür.
                            return Ok(Some(node));
                        }
                    }
                }
            }
            // Hiçbir düğüm bulunamadı.
            Ok(None)
        }
        None => Err(KError::InternalError), // DTB init edilmemiş
    }
}

/// Belirli bir yola sahip bir düğümü bulur (örn. "/soc/serial@1000000").
///
/// `path`: Aranacak düğüm yolu.
///
/// Başarı durumunda bulunan düğümü (varsa) veya `None`, hata durumunda `KError` döner.
pub fn find_node_by_path<'a>(path: &str) -> Result<Option<Node<'a, 'static>>, KError> {
    let parsed_data = PARSED_DTB.lock();
    match &*parsed_data {
        Some(data) => {
            let fdt = &data.fdt;
            Ok(fdt.find_node(path))
        }
        None => Err(KError::InternalError), // DTB init edilmemiş
    }
}


// TODO: İhtiyaç duyuldukça başka erişim fonksiyonları eklenebilir:
 - get_cpu_count() -> Result<usize, KError>
 - get_interrupt_controller_info() -> Result<..., KError>
 - get_boot_args() -> Result<Option<&'static str>, KError>
 - get_property_u32(node: &Node, prop_name: &str) -> Result<Option<u32>, KError>
 - get_property_bytes(node: &Node, prop_name: &str) -> Result<Option<&'static [u8]>, KError>


// --- Yardımcı Fonksiyonlar ---

/// `fdt-rs` hatalarını Karnal64'ün `KError` tipine dönüştürür.
fn map_fdt_error(error: FdtError) -> KError {
    match error {
        FdtError::TooShort | FdtError::BadMagic | FdtError::BadVersion => KError::InvalidArgument, // Geçersiz format
        FdtError::BadAlign | FdtError::BadOffset | FdtError::BadLen | FdtError::BadAddress => KError::BadAddress, // Bellek veya yapısal hata
        FdtError::TooManyNops | FdtError::TooManyCells | FdtError::TooManyStrings => KError::OutOfMemory, // Kaynak limiti aşıldı
        FdtError::NotFound => KError::NotFound, // Öğre bulunamadı
        FdtError::BadState => KError::InternalError, // Crate'in iç durumuyla ilgili beklenmedik hata
        FdtError::BadArgs => KError::InvalidArgument, // Fonksiyona geçersiz argüman
        FdtError::Exists => KError::AlreadyExists, // Öğre zaten mevcut
        FdtError::ReadOnly => KError::PermissionDenied, // Yazma izni olmayan bir yere yazma
        FdtError::BadPhandle => KError::InvalidArgument, // Geçersiz phandle referansı
        // `fdt-rs`'in gelecekte ekleyebileceği diğer hata tipleri için bir yakalayıcı ekleyebilirsiniz.
        _ => KError::InternalError, // Bilinmeyen fdt hatası
    }
}

/// DTB'den bellek bölgelerini ayrıştıran özel yardımcı fonksiyon.
/// `reg` özelliği formatı (`#address-cells` ve `#size-cells`'e bağlı) PowerPC'ye özel olabilir,
/// ancak `fdt-rs` bu detayı soyutlar.
fn extract_memory_regions(fdt: &Fdt) -> Result<alloc::vec::Vec<(u64, u64)>, KError> {
    let mut regions = alloc::vec::Vec::new();

    // DTB'nin kök düğümünü al.
    let root = fdt.root();

    // `memory` düğümünü ara. Genellikle "/memory" yolundadır.
    // PowerPC DTB'lerinde farklı yerlerde veya farklı isimlerle olabilir,
    // ancak standart "/memory" en yaygın olanıdır.
    let memory_node = root
        .nodes()
        .find(|node| node.name() == Ok("memory")) // Doğrudan "memory" isimli çocuğu ara
        .ok_or(KError::NotFound)?; // memory düğümü bulunamadıysa hata ver

    // `memory` düğümündeki `reg` özelliğini ara.
    let reg_prop = memory_node
        .props()
        .find(|p| p.name() == Ok("reg"))
        .ok_or(KError::NotFound)?; // reg özelliği bulunamadıysa hata ver

    // `reg` özelliği, (adres, boyut) çiftlerinin bir listesidir.
    // `fdt-rs`'in `iter_reg_parts` metodunu kullanarak bu çiftleri güvenli bir şekilde ayrıştırabiliriz.
    // Bu metod, düğümün (#address-cells ve #size-cells) özelliklerini otomatik olarak dikkate alır.
    for (addr, size) in reg_prop
        .value()
        .iter_reg_parts(memory_node)
        .map_err(|e| map_fdt_error(e))? // Ayrıştırma hatasını KError'a dönüştür
    {
        regions.push((addr, size));
    }

    // Başarı, bellek bölgeleri listesini döndür.
    Ok(regions)
}
