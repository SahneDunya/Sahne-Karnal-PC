#![no_std] // Kernel alanı için standart kütüphaneye ihtiyaç yok
#![allow(dead_code)] // Geliştirme sırasında bazı fonksiyonlar doğrudan kullanılmayabilir
#![allow(unused_variables)] // Geliştirme sırasında argümanlar geçici olarak kullanılmayabilir
#![allow(unused_imports)] // Geliştirme sırasında importlar geçici olarak kullanılmayabilir

 Çekirdek içindeki temel tiplerimizi kullanacağız
 use crate::karnal64::{KError, KHandle}; // Karnal64 tipleri buradan import edilecek
 use crate::karnal64::kresource::{self, ResourceProvider, MODE_READ, MODE_WRITE}; // Kaynak yönetimi modülü
 use crate::karnal64::kmemory; // Bellek yönetimi modülü

// Yer tutucu Karnal64 tipleri ve modülleri (Gerçek projede import edilecek)
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i64)]
pub enum KError {
    PermissionDenied = -1,
    NotFound = -2,
    InvalidArgument = -3,
    Interrupted = -4,
    BadHandle = -9,
    Busy = -11,
    OutOfMemory = -12,
    BadAddress = -14,
    AlreadyExists = -17,
    NotSupported = -38,
    NoMessage = -61,
    InternalError = -255,
    // DTB'ye özel hata kodları eklenebilir
    DtbParseError = -1000,
    DtbInvalidStructure = -1001,
    DtbPropertyNotFound = -1002,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct KHandle(u64);

// Yer tutucu ResourceProvider trait (Gerçek projede import edilecek)
pub trait ResourceProvider {
    fn read(&self, buffer: &mut [u8], offset: u64) -> Result<usize, KError>;
    fn write(&self, buffer: &[u8], offset: u64) -> Result<usize, KError>;
    fn control(&self, request: u64, arg: u64) -> Result<i64, KError>;
    fn seek(&self, position: KseekFrom) -> Result<u64, KError>;
    fn get_status(&self) -> Result<KResourceStatus, KError>;
    // DTB ayrıştırıcısı bu trait'i implemente etmez, ama DTB'den
    // bulunan cihazlar için ResourceProvider implementasyonları
    // başka yerlerde yapılabilir ve DTB bilgisi bu implementasyonlarda kullanılır.
    // DTB ayrıştırıcısının rolü bilgiyi *sağlamaktır*.

    // DTB ayrıştırıcısı için helper traitler/yapılar gerekebilir
    // DTB okumak için bir "salt okunur" ResourceProvider olarak düşünülebilir,
    // veya sadece ayrıştırma fonksiyonları sunan bir modül olarak.
    // Genellikle ikinci yol izlenir: DTB parser sadece veriyi döner.
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum KseekFrom { Start(u64), Current(i64), End(i64) }
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct KResourceStatus {} // Yer tutucu


// Yer tutucu Karnal64 modülleri (Gerçek projede import edilecek)
mod kresource {
    use super::*;
    // DTB'den bulunan cihazları kaydetmek için kullanılacak fonksiyonlar burada yer alır
    pub const MODE_READ: u32 = 1 << 0;
    pub const MODE_WRITE: u32 = 1 << 1;
     fn register_provider(id: &str, provider: Box<dyn ResourceProvider>) -> Result<KHandle, KError> { /* ... */ Ok(KHandle(0)) }
}

mod kmemory {
     use super::*;
     // DTB'den okunan bellek bölgelerini sisteme kaydetmek için kullanılacak
      fn register_memory_region(start: usize, size: usize, flags: u32) -> Result<(), KError> { /* ... */ Ok(()) }
}


// DTB Yapıları ve Sabitleri (FDT - Flattened Device Tree spesifikasyonuna göre)
const FDT_MAGIC: u32 = 0xd00dfeed; // Big-endian
const FDT_VERSION: u32 = 17; // Genellikle kullanılan versiyon

// FDT structure block etiketleri (big-endian)
const FDT_BEGIN_NODE: u32 = 0x00000001;
const FDT_END_NODE: u32 = 0x00000002;
const FDT_PROP: u32 = 0x00000003;
const FDT_NOP: u32 = 0x00000004;
const FDT_END: u32 = 0x00000009;

// DTB Header yapısı
#[repr(C)] // C uyumlu bellek düzeni
struct FdtHeader {
    magic: u32, // Sihirli sayı (big-endian)
    totalsize: u32, // DTB bloğunun toplam boyutu (big-endian)
    dt_off: u32, // Structure block ofseti (big-endian)
    dt_strings_off: u32, // Strings block ofseti (big-endian)
    mem_rsvmap_off: u32, // Reserved memory map ofseti (big-endian)
    version: u32, // Versiyon (big-endian)
    last_comp_version: u32, // Son uyumlu versiyon (big-endian)
    boot_cpuid_phys: u32, // Boot CPU fiziksel ID (big-endian)
    size_dt_strings: u32, // Strings block boyutu (big-endian)
    size_dt_struct: u32, // Structure block boyutu (big-endian)
}

// Ayrıştırılmış DTB'den çıkarılacak önemli bilgiler için yapılar
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub start_address: usize,
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: Option<&'static str>, // Cihaz düğümü adı (genellikle sadece son bileşen)
    pub full_path: Option<core::string::String>, // Cihaz düğümünün tam yolu (heap gerekebilir)
    pub compatible: Option<&'static str>, // 'compatible' özelliği değeri (ilk değer)
    pub address_ranges: core::vec::Vec<usize>, // 'reg' özelliği değerleri (adres ve boyut çiftleri)
    pub interrupts: core::vec::Vec<u32>, // 'interrupts' özelliği değerleri
    // Diğer önemli özellikler buraya eklenebilir (status, #address-cells, #size-cells vb.)
}

#[derive(Debug, Default)] // Default trait'i init için kolaylık sağlar
pub struct ParsedDtb {
    pub memory_regions: core::vec::Vec<MemoryRegion>,
    pub devices: core::vec::Vec<DeviceInfo>,
    // Diğer çekirdek için önemli bilgiler (bootargs, stdout-path vb.) buraya eklenebilir
    pub bootargs: Option<&'static str>,
    pub stdout_path: Option<&'static str>,
}


// --- DTB Ayrıştırma Mantığı ---

// Yardımcı fonksiyon: Big-endian u32'yi yerel endian'a çevir (SPARC genellikle big-endian'dır,
// bu durumda doğrudan okumak işe yarar, ancak taşınabilirlik için çeviri iyidir)
#[inline]
fn u32_from_be(val: u32) -> u32 {
    #[cfg(target_endian = "big")]
    { val }
    #[cfg(target_endian = "little")]
    { val.swap_bytes() }
}

// Yardımcı fonksiyon: Big-endian u64'ü yerel endian'a çevir (bazı DTB özellikleri u64 olabilir)
#[inline]
fn u64_from_be(val: u64) -> u64 {
    #[cfg(target_endian = "big")]
    { val }
    #[cfg(target_endian = "little")]
    { val.swap_bytes() }
}


/// Belirtilen bellek adresindeki DTB bloğunu ayrıştırır.
/// Bulunan önemli donanım bilgilerini içeren ParsedDtb yapısını döndürür.
///
/// Güvenlik Notu: `dtb_address`'ın çekirdek alanında geçerli ve okunabilir
/// bir bellek bölgesini işaret ettiği varsayılır. Bu fonksiyonun çağrılmadan
/// önce bootloader tarafından sağlanan adresin doğrulanması önemlidir.
pub fn parse_dtb(dtb_address: usize) -> Result<ParsedDtb, KError> {
    let base_ptr = dtb_address as *const u8;

    // DTB Header'ı oku
    // UNSAFE: Doğrudan ham pointer'dan veri okuma
    let header = unsafe {
        if base_ptr.is_null() {
            return Err(KError::InvalidArgument);
        }
        // Header boyutunu ve toplam boyutu kontrol etmek için ilk birkaç baytı okuyup
        // magic number'ı doğrulamak gerekir. Tam header'ı okumadan boyut kontrolü zor.
        // Basitlik adına, header'ın var olduğunu ve en az FdtHeader boyutu kadar
        // bellek alanının geçerli olduğunu varsayalım şimdilik.
        &*(base_ptr as *const FdtHeader)
    };

    // Magic number kontrolü
    if u32_from_be(header.magic) != FDT_MAGIC {
        //println!("DTB Magic number mismatch: Expected {:x}, got {:x}", FDT_MAGIC, u32_from_be(header.magic));
        return Err(KError::DtbInvalidStructure);
    }

    // Versiyon kontrolü (basitlik adına sadece beklenen versiyonu kabul edelim)
    if u32_from_be(header.version) < FDT_VERSION {
         //println!("DTB Versiyonu çok eski: Expected at least {}, got {}", FDT_VERSION, u32_from_be(header.version));
         // Veya daha eski versiyonları destekleyen mantık eklenebilir
        return Err(KError::DtbInvalidStructure);
    }


    let total_size = u32_from_be(header.totalsize) as usize;
    let struct_offset = u32_from_be(header.dt_off) as usize;
    let strings_offset = u32_from_be(header.dt_strings_off) as usize;
    let struct_size = u32_from_be(header.size_dt_struct) as usize;
    let strings_size = u32_from_be(header.size_dt_strings) as usize;

    // DTB blobu için çekirdek içi slice'lar oluştur
    // UNSAFE: Tamamen DTB bloğu için geçerli bir bellek bölgesine işaret ettiği varsayılır
    let full_dtb_slice = unsafe {
        core::slice::from_raw_parts(base_ptr, total_size)
    };

    // Structure ve Strings block slice'ları
    let struct_slice = full_dtb_slice.get(struct_offset..struct_offset + struct_size)
        .ok_or(KError::DtbInvalidStructure)?;
    let strings_slice = full_dtb_slice.get(strings_offset..strings_offset + strings_size)
        .ok_or(KError::DtbInvalidStructure)?;


    // --- DTB Yapısını Ayrıştırma ---
    let mut parsed_data = ParsedDtb::default();
    let mut struct_ptr = struct_slice.as_ptr();
    let struct_end_ptr = unsafe { struct_ptr.add(struct_size) };

    let mut current_node_path = core::string::String::new(); // Heap allocation here

    // UNSAFE: Structure block'u pointer aritmetiği ile gezme
    unsafe {
        while struct_ptr < struct_end_ptr {
            let token = u32_from_be(*(struct_ptr as *const u32));
            struct_ptr = struct_ptr.add(4); // Token boyutunu atla

            match token {
                FDT_BEGIN_NODE => {
                    // Düğüm adını oku (null ile sonlanan string)
                    let name_start = struct_ptr;
                    let mut name_end = name_start;
                    while *name_end != 0 {
                        name_end = name_end.add(1);
                    }
                    let name = core::str::from_utf8(core::slice::from_raw_parts(name_start, name_end.offset_from(name_start) as usize))
                        .map_err(|_| KError::DtbParseError)?;

                    struct_ptr = name_end.add(1); // Null baytını atla

                    // 4 bayt hizalaması yap (padding)
                    struct_ptr = struct_ptr.add((4 - (struct_ptr as usize % 4)) % 4);

                    // Düğüm yolunu güncelle
                    if !current_node_path.is_empty() {
                        current_node_path.push('/');
                    }
                    // Root düğüm için özel durum, adı boş stringdir.
                    if !name.is_empty() {
                         current_node_path.push_str(name);
                    } else {
                         // Root düğüm '/' olarak temsil edilir.
                         if current_node_path.is_empty() {
                             current_node_path.push('/');
                         }
                     }
                    println!("BEGIN_NODE: {}", current_node_path);

                    // Yeni cihaz bilgisi yapısı oluştur (şimdilik sadece yolu kaydet)
                    let device_info = DeviceInfo {
                        name: if name.is_empty() { None } else { Some(unsafe { core::str::from_utf8_unchecked(core::slice::from_raw_parts(name_start, name_end.offset_from(name_start) as usize)) }) }, // 'static lifetime varsayımı
                        full_path: Some(current_node_path.clone()),
                        compatible: None,
                        address_ranges: core::vec::Vec::new(), // Heap allocation
                        interrupts: core::vec::Vec::new(), // Heap allocation
                    };
                    parsed_data.devices.push(device_info); // Heap allocation
                }
                FDT_END_NODE => {
                    println!("END_NODE");
                    // Düğüm yolundan son bileşeni çıkar
                    if let Some(last_slash_idx) = current_node_path.rfind('/') {
                        current_node_path.truncate(last_slash_idx);
                    } else {
                         // Root düğümden çıkış yapılıyor
                        current_node_path.clear();
                    }
                }
                FDT_PROP => {
                    let prop_len = u32_from_be(*(struct_ptr as *const u32)) as usize;
                    struct_ptr = struct_ptr.add(4); // Uzunluk baytlarını atla
                    let prop_name_offset = u32_from_be(*(struct_ptr as *const u32)) as usize;
                    struct_ptr = struct_ptr.add(4); // İsim ofseti baytlarını atla

                    let prop_value_ptr = struct_ptr;
                    let prop_value_slice = core::slice::from_raw_parts(prop_value_ptr, prop_len);

                    // İsim stringini Strings block'tan al
                    let prop_name = get_string_from_strings_block(strings_slice, prop_name_offset)
                        .ok_or(KError::DtbInvalidStructure)?;

                    println!("PROP: {} ({} bytes)", prop_name, prop_len);

                    // Son eklenen cihaz bilgisi yapısını güncelle (şu anki düğüm)
                    if let Some(device_info) = parsed_data.devices.last_mut() {
                        match prop_name {
                            "compatible" => {
                                if prop_len > 0 {
                                    // İlk compatible stringini al (null ile sonlanan string)
                                    if let Ok(s) = core::str::from_utf8(prop_value_slice.split(|b| *b == 0).next().unwrap_or(&[])) {
                                        device_info.compatible = Some(unsafe { core::str::from_utf8_unchecked(prop_value_slice.split(|b| *b == 0).next().unwrap_or(&[])) }); // 'static lifetime varsayımı
                                    }
                                }
                            }
                            "reg" => {
                                // 'reg' özelliği genellikle (address, size) çiftlerinden oluşur.
                                // Boyut genellikle düğümün #address-cells ve #size-cells özelliklerine bağlıdır.
                                // Basitlik adına burada sadece raw u32/u64 değerlerini topluyoruz.
                                // Gerçek bir parserda parent düğümlerin #address-cells/#size-cells
                                // özellikleri takip edilmeli ve adres çevirileri yapılmalıdır.
                                let mut val_ptr = prop_value_ptr;
                                while val_ptr < prop_value_ptr.add(prop_len) {
                                    // Basitçe her 4 bayt veya 8 baytı bir değer olarak alalım (SPARC için genellikle 4 bayt adres/boyut olabilir)
                                    if prop_len - (val_ptr.offset_from(prop_value_ptr) as usize) >= 4 {
                                         let val = u32_from_be(*(val_ptr as *const u32));
                                         device_info.address_ranges.push(val as usize);
                                         val_ptr = val_ptr.add(4);
                                    } else {
                                         // Yetersiz veri kaldı, hata veya döngüden çıkış?
                                         break;
                                    }
                                }
                                // Memory node'u için özel işleme
                                if current_node_path == "/memory" || current_node_path == "/memory@0" { // Örnek yollar
                                     // 'reg' özelliği memory@... düğümlerinde bellek bölgelerini tanımlar
                                     // Her çift (adres, boyut) bir MemoryRegion'dır
                                     if device_info.address_ranges.len() >= 2 {
                                         // Basitlik adına ilk çifti ana bellek kabul edelim
                                         // Gerçek DTB'lerde birden çok reg özelliği veya çift olabilir
                                         for chunk in device_info.address_ranges.chunks_exact(2) {
                                             parsed_data.memory_regions.push(MemoryRegion {
                                                 start_address: chunk[0],
                                                 size: chunk[1],
                                             });
                                         }
                                     }
                                }
                            }
                            "interrupts" => {
                                // 'interrupts' özelliği genellikle interrupt numaraları listesidir.
                                // Karmaşık interrupt spesifikatörleri (controller, cell count, value list)
                                // için #interrupt-cells özelliği takip edilmelidir.
                                // Basitlik adına sadece raw u32 değerlerini alalım.
                                let mut val_ptr = prop_value_ptr;
                                while val_ptr < prop_value_ptr.add(prop_len) {
                                    if prop_len - (val_ptr.offset_from(prop_value_ptr) as usize) >= 4 {
                                         let val = u32_from_be(*(val_ptr as *const u32));
                                         device_info.interrupts.push(val);
                                         val_ptr = val_ptr.add(4);
                                    } else {
                                         break;
                                    }
                                }
                            }
                             "bootargs" => {
                                 // `/chosen` düğümü altında olabilir
                                 if current_node_path.ends_with("/chosen") {
                                     if let Ok(s) = core::str::from_utf8(prop_value_slice.split(|b| *b == 0).next().unwrap_or(&[])) {
                                          parsed_data.bootargs = Some(unsafe { core::str::from_utf8_unchecked(prop_value_slice.split(|b| *b == 0).next().unwrap_or(&[])) }); // 'static lifetime varsayımı
                                     }
                                 }
                             }
                            "stdout-path" => {
                                // `/chosen` düğümü altında olabilir
                                if current_node_path.ends_with("/chosen") {
                                     if let Ok(s) = core::str::from_utf8(prop_value_slice.split(|b| *b == 0).next().unwrap_or(&[])) {
                                          parsed_data.stdout_path = Some(unsafe { core::str::from_utf8_unchecked(prop_value_slice.split(|b| *b == 0).next().unwrap_or(&[])) }); // 'static lifetime varsayımı
                                     }
                                }
                            }
                            // Diğer önemli özellikler buraya eklenebilir ve işlenebilir
                            _ => { /* Bilinmeyen özelliği yoksay */ }
                        }
                    }


                    struct_ptr = struct_ptr.add(prop_len); // Değer baytlarını atla

                    // 4 bayt hizalaması yap (padding)
                    struct_ptr = struct_ptr.add((4 - (struct_ptr as usize % 4)) % 4);
                }
                FDT_NOP => {
                    // NO-OP, bir şey yapmaya gerek yok, sadece geç
                }
                FDT_END => {
                    // DTB bloğunun sonu
                    println!("FDT_END");
                    break;
                }
                _ => {
                    // Bilinmeyen/geçersiz token
                    println!("Bilinmeyen DTB token: {:x}", token);
                    return Err(KError::DtbInvalidStructure);
                }
            }
        }
    } // unsafe bloğu sonu

    // Ayrıştırma başarılı olduysa, toplanan veriyi döndür
    Ok(parsed_data)
}

/// Strings block'tan belirtilen ofsetteki null ile sonlanan stringi alır.
fn get_string_from_strings_block(strings_slice: &[u8], offset: usize) -> Option<&'static str> {
    if offset >= strings_slice.len() {
        return None; // Ofset bounds dışında
    }

    let str_start_ptr = unsafe { strings_slice.as_ptr().add(offset) };
    let mut str_end_ptr = str_start_ptr;
    let slice_end_ptr = unsafe { strings_slice.as_ptr().add(strings_slice.len()) };

    // String sonunu (null baytını) bul
    // UNSAFE: strings_slice bounds içinde arama yapıldığı varsayılır
    unsafe {
        while str_end_ptr < slice_end_ptr && *str_end_ptr != 0 {
            str_end_ptr = str_end_ptr.add(1);
        }

        if str_end_ptr == slice_end_ptr && *str_end_ptr.sub(1) != 0 {
             // String null ile sonlanmıyor ve slice'ın sonuna ulaşıldı
             return None; // Hata durumu
        }

        let str_slice = core::slice::from_raw_parts(str_start_ptr, str_end_ptr.offset_from(str_start_ptr) as usize);
        core::str::from_utf8(str_slice).ok().map(|s| unsafe { core::str::from_utf8_unchecked(str_slice) }) // 'static lifetime varsayımı
    }
}

// --- Entegrasyon İçin Örnek Kullanım (Başka Bir Kernel Modülünde) ---
// Örneğin, çekirdek boot/init sürecini yöneten bir modülde:

use crate::arch::sparc::srcdtb_sparc::{parse_dtb, ParsedDtb, DeviceInfo, MemoryRegion};
use crate::karnal64::{KError, KHandle};
use crate::karnal64::kresource::{self, ResourceProvider, MODE_READ, MODE_WRITE};
use crate::karnal64::kmemory;

// Bootloader tarafından sağlanan DTB adresi
static mut BOOT_DTB_ADDRESS: usize = 0; // Boot kodunda ayarlanacak

pub fn sparc_kernel_boot_init() -> Result<(), KError> {
    // DTB adresini bootloader'dan aldığımızı varsayalım
    let dtb_address = unsafe { BOOT_DTB_ADDRESS };
    if dtb_address == 0 {
        println!("Hata: DTB adresi bootloader tarafından ayarlanmadı.");
        return Err(KError::InternalError); // Veya uygun bir hata
    }

    // DTB'yi ayrıştır
    let parsed_dtb = parse_dtb(dtb_address)?;
    println!("DTB başarıyla ayrıştırıldı: {:?}", parsed_dtb);

    // Bellek bölgelerini kaydet
    for region in parsed_dtb.memory_regions {
        println!("Bellek Bölgesi bulundu: Başlangıç: {:#x}, Boyut: {:#x}", region.start_address, region.size);
         kmemory::register_memory_region(region.start_address, region.size, kmemory::FLAGS_RAM)?; // Gerçek bir memory manager fonksiyonu
    }

    // Cihazları kaydet
    for device in parsed_dtb.devices {
        println!("Cihaz bulundu: Yolu: {:?}, Compatible: {:?}, Reg: {:?}, Ints: {:?}",
                 device.full_path, device.compatible, device.address_ranges, device.interrupts);

        // Örneğin, compatible string'ine göre cihaz türünü belirleyip ResourceProvider'ını kaydet
        if let Some(compatible) = device.compatible {
            match compatible {
                "uart-16550" | "ns16550" => {
                    // UART cihazı bulundu
                    if let Some(uart_reg_addr) = device.address_ranges.first() {
                        // UART provider'ını oluştur ve kaydet
                         let uart_provider = Box::new(Uart16550Provider::new(*uart_reg_addr, device.interrupts.first().copied())); // Uart16550Provider başka bir yerde implemente edilmeli
                         if let Some(path) = device.full_path {
                             kresource::register_provider(&format!("karnal://{}", path), uart_provider)?;
                        } else {
                             println!("UART cihazının yolu yok, kaydedilemedi.");
                          }
                    }
                }
                "disk" | "block-device" => {
                    // Disk cihazı bulundu (reg property disk bilgisi içerebilir)
                    // TODO: Disk provider'ını oluştur ve kaydet
                }
                // Diğer cihaz türleri...
                _ => {
                    println!("Desteklenmeyen veya bilinmeyen cihaz: {}", compatible);
                }
            }
        }
    }

    // Boot argümanlarını ve stdout yolunu işle
    if let Some(bootargs) = parsed_dtb.bootargs {
        println!("Boot Argümanları: {}", bootargs);
        // TODO: Boot argümanlarını ayrıştır ve kullan
    }
    if let Some(stdout_path) = parsed_dtb.stdout_path {
         println!("Stdout Yolu: {}", stdout_path);
         // TODO: Bu yolu kullanarak uygun konsol cihazını bul/kaydet/varsayılan yap
     }


    Ok(())
}

// Uart16550Provider gibi cihaz implementasyonları başka dosyalarda yer almalıdır
 struct Uart16550Provider { /* ... */ }
 impl ResourceProvider for Uart16550Provider { /* ... */ }
