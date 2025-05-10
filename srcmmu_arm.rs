#![no_std]
#![allow(unused)] // Geliştirme aşamasında kullanılmayan kodlara izin ver

// 'alloc' crate'ini dahil ediyoruz. no_std ortamında Vec gibi yapılar için gereklidir.
// Çekirdeğin bir global ayırıcıya sahip olması gerekir (örneğin, bootloader tarafından ayarlanır).
extern crate alloc;

use core::ptr;
use core::slice;
use core::result::Result;
// ARM sistem registerlarına erişim için assembly kullanacağız veya bir crate'e bağımlı olacağız.
// Şimdilik konsepti göstermek için yorum satırı olarak bırakıyoruz.
use core::arch::asm;
use alloc::vec::Vec;
use alloc::boxed::Box; // SharedMemObject gibi yapıları heap'te tutmak için
use core::sync::atomic::{AtomicU64, Ordering}; // Basit sayaçlar veya handle üretimi için

// Karnal64 core tiplerini burada yeniden tanımlıyoruz veya erişilebilir varsayıyoruz.
// Gerçek projede 'use crate::karnal64::...' şeklinde dahil edilmelidir.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i64)] // Hata kodları kullanıcı alanına negatif i64 olarak dönecek.
pub enum KError {
    PermissionDenied = -1,
    NotFound = -2,
    InvalidArgument = -3,
    Interrupted = -4,
    BadHandle = -9,
    Busy = -11,
    OutOfMemory = -12,
    BadAddress = -14, // Geçersiz bellek adresi (kullanıcı pointer'ı veya sanal adres)
    AlreadyExists = -17, // Kaynak zaten mevcut (örn: sanal adres zaten haritalı)
    NotSupported = -38,
    NoMessage = -61,
    InternalError = -255, // Beklenmedik dahili çekirdek durumu
    // MMU'ya özel hata kodları
    PageTableFull = -100, // Sayfa tablosunda yer yok (daha üst seviye tablo gerekiyorsa)
    AddressNotPageAligned = -101, // Adres sayfa sınırına hizalanmamış
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct KTaskId(u64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct KThreadId(u64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
// KHandle'ın kresource modülünde tanımlı olduğunu varsayıyoruz ama burada kullanmak için tanımlıyoruz.
pub struct KHandle(u64);


// --- Fiziksel Çerçeve Ayırıcı (Physical Frame Allocator - PFA) ---
// Bellek yöneticisinin fiziksel belleği tahsis etmek için kullandığı katman.
// Bu modül sadece bir yer tutucudur. Gerçek bir çekirdekte çok daha karmaşıktır.
mod pfa {
    use super::*;
    use alloc::vec::Vec;

    // Çok basit bir 'bump' ayırıcı (fiziksel belleği sırayla dağıtır, serbest bırakmayı desteklemez)
    // Gerçek bir PFA boş çerçevelerin listesini tutar ve eşzamanlı erişimi yönetir.
    static mut HEAP_START: usize = 0;
    static mut HEAP_END: usize = 0;
    static mut NEXT_FREE: usize = 0;

    // Bootloader veya erken boot kodu tarafından çağrılmalıdır.
    pub fn init(heap_start: usize, heap_size: usize) {
        unsafe {
            HEAP_START = heap_start;
            HEAP_END = heap_start + heap_size;
            NEXT_FREE = heap_start;
        }
         println!("PFA initialized: {:x} to {:x}", heap_start, HEAP_END); // Çekirdek içi print! gerekli
    }

    // 4KB'lik fiziksel bir çerçeve (sayfa) tahsis eder.
    pub fn allocate_frame() -> Option<*mut u8> {
        unsafe {
            let frame_addr = NEXT_FREE;
            if frame_addr + super::PAGE_SIZE <= HEAP_END {
                 NEXT_FREE += super::PAGE_SIZE;
                 // println!("PFA: Allocated frame {:x}", frame_addr);
                 Some(frame_addr as *mut u8)
            } else {
                 println!("PFA: Out of memory");
                None // Fiziksel bellek tükendi
            }
        }
    }

    // Fiziksel bir çerçeveyi serbest bırakır.
    // Bu 'bump' ayırıcıda bu fonksiyon aslında hiçbir şey yapmaz.
    pub fn free_frame(frame_addr: *mut u8) {
         println!("PFA: Frame {:p} conceptually freed", frame_addr);
        // Gerçek bir PFA'da frame_addr boş listeye eklenir.
    }
}

// --- ARM MMU Mimarisine Özel Tanımlar ---

// Sayfa Boyutu (4KB)
const PAGE_SIZE: usize = 0x1000;
const PAGE_SIZE_BITS: usize = 12; // log2(4096)

// Sayfa tablosu seviyesindeki girdi sayısı (4KB sayfa boyutu, 64-bit girdi)
// 4KB / 8 byte/girdi = 512 girdi
const PAGE_TABLE_ENTRIES: usize = PAGE_SIZE / 8;
const PAGE_TABLE_ENTRIES_BITS: usize = 9; // log2(512)

// Sanal Adres (VA) yerleşimi varsayımları (ARMv8-A, 39-bit VAs, TTBCR_EL1.T1SZ = 25)
// Bu yapılandırma, TTBR0_EL1 (genellikle kullanıcı alanı) için 256GB'lık bir VA alanı sağlar.
 bit [63:39]: Göz ardı edilir (TTBR0_EL1 için 0)
 bit [38:30]: Level 1 indeksi (9 bit)
 bit [29:21]: Level 2 indeksi (9 bit)
 bit [20:12]: Level 3 indeksi (9 bit)
 bit [11:0]: Sayfa içi ofset (12 bit)

const L1_INDEX_SHIFT: usize = PAGE_SIZE_BITS + PAGE_TABLE_ENTRIES_BITS * 2; // 12 + 9*2 = 30
const L2_INDEX_SHIFT: usize = PAGE_SIZE_BITS + PAGE_TABLE_ENTRIES_BITS;     // 12 + 9 = 21
const L3_INDEX_SHIFT: usize = PAGE_SIZE_BITS;                              // 12

const INDEX_MASK: usize = PAGE_TABLE_ENTRIES - 1; // 511 (0b111111111)

// Bir sayfa tablosu seviyesini temsil eden yapı (sadece 64-bit girdilerin dizisi)
#[repr(align(4096))] // Sayfa tabloları 4KB'ye hizalanmalıdır (ARMv8-A kısıtlaması)
#[derive(Copy, Clone)] // Kopyalanabilir olması init gibi durumlar için pratik
pub struct PageTable {
    entries: [u64; PAGE_TABLE_ENTRIES],
}

impl PageTable {
    // Yeni, içi sıfırlarla dolu bir sayfa tablosu oluşturur.
    pub const fn new() -> Self {
        PageTable { entries: [0; PAGE_TABLE_ENTRIES] }
    }

    // Belirtilen indeksteki girdiye mutable referans döner.
    #[inline]
    pub fn entry_mut(&mut self, index: usize) -> Option<&mut u64> {
        self.entries.get_mut(index)
    }

     // Belirtilen indeksteki girdiye immutable referans döner.
    #[inline]
    pub fn entry(&self, index: usize) -> Option<&u64> {
        self.entries.get(index)
    }

     // Sayfa tablosunun boş olup olmadığını (tüm girdilerin sıfır olup olmadığını) kontrol eder.
     pub fn is_empty(&self) -> bool {
         self.entries.iter().all(|&e| e == 0)
     }
}

// Sayfa Tablosu Girdisi (PTE) Bayrakları (demo için basitleştirildi)
#[allow(unused)]
mod pte_flags {
    // Alt Öznitelikler (blok/sayfa tanımlayıcıları için geçerli)
    pub const VALID: u64      = 1 << 0; // Girdi geçerli mi?
    pub const TABLE: u64      = 1 << 1; // L1/L2 için: sonraki seviye tabloya işaret eder.
    pub const PAGE: u64       = 1 << 1; // L3 için: bir sayfaya işaret eder (VALID ile birlikte kullanılır).

    // Aşama 1 Öznitelikleri (blok ve sayfa tanımlayıcıları için ortak)
    // MAIR_EL1 (Memory Attribute Indirection Register) içindeki indeksi belirtir.
    // Bu değerler, MAIR_EL1'deki ATTR[n] alanlarına programlanan değerlere karşılık gelir.
    pub const ATTR_INDEX_0_NOCACHE: u64 = 0 << 2; // nGnRnE (Device) veya Normal Non-cacheable
    pub const ATTR_INDEX_1_CACHED: u64  = 1 << 2; // Normal Memory, Write-back, Write-Allocate (Cacheable)

    pub const NON_SECURE: u64 = 1 << 5; // Güvenli olmayan bellek (eğer EL3 güvenlik yönetiyorsa)
    // Erişim İzinleri (AP[2:1])
    pub const AP_EL1_RW_EL0_NO: u64  = 0 << 6; // EL1 Okuma/Yazma, EL0 Erişim Yok
    pub const AP_EL1_RW_EL0_RW: u64  = 1 << 6; // EL1 Okuma/Yazma, EL0 Okuma/Yazma
    pub const AP_EL1_RO_EL0_NO: u64  = 2 << 6; // EL1 Salt Okunur, EL0 Erişim Yok
    pub const AP_EL1_RO_EL0_RO: u64  = 3 << 6; // EL1 Salt Okunur, EL0 Salt Okunur

    // Paylaşılabilirlik (SH[1:0])
    pub const SH_NON: u64     = 0 << 8; // Paylaşılamaz
    pub const SH_OUTER: u64  = 2 << 8; // Dış Paylaşılabilir (eşevreli bellek için)
    pub const SH_INNER: u64   = 3 << 8; // İç Paylaşılabilir (eşevreli bellek için)

    pub const AF: u64         = 1 << 10; // Erişim Bayrağı (Donanım tarafından ilk erişimde ayarlanır)
    pub const NSTABLE: u64    = 1 << 11; // Güvenli olmayan tablo (tablo tanımlayıcıları için)

    // Yürütme İzinleri (PXN, UXN)
    pub const PXN: u64        = 1 << 53; // Privileged Execute Never (EL1'de asla yürütme)
    pub const UXN: u64        = 1 << 54; // Unprivileged Execute Never (EL0'da asla yürütme)

    // Kullanıcı veri sayfaları için yaygın bayrakların kombinasyonu
    pub const USER_DATA_FLAGS: u64 = VALID | PAGE | ATTR_INDEX_1_CACHED | AP_EL1_RW_EL0_RW | SH_INNER | AF | UXN;
     // Kullanıcı yürütülebilir kod sayfaları için yaygın bayrakların kombinasyonu
    pub const USER_CODE_FLAGS: u64 = VALID | PAGE | ATTR_INDEX_1_CACHED | AP_EL1_RW_EL0_RO | SH_INNER | AF | PXN; // EL0 Salt Okunur, EL1 Okuma/Yazma, PXN
     // Cihaz belleği için yaygın bayrakların kombinasyonu
    pub const DEVICE_FLAGS: u64 = VALID | PAGE | ATTR_INDEX_0_NOCACHE | AP_EL1_RW_EL0_NO | SH_OUTER | AF | PXN | UXN; // EL1 R/W, EL0 No, önbelleklenemez, yürütülemez
}


// --- Global MMU Durumu ve Yardımcı Yapılar ---

// Kullanıcı alanı için Sanal Adres (VA) Ayırıcı (her görev için ayrı olmalıdır gerçek çekirdekte)
// Bu sadece bir yer tutucudur, gerçek bir ayırıcı boş aralıkları takip eder.
struct VmspaceAllocator {
    next_vaddr: usize, // Bir sonraki tahsisin başlayabileceği yer (basit bump için)
    end_vaddr: usize, // VA alanının sonu (hariç)
     start_vaddr_range: usize, // VA aralığının başlangıcı
}

impl VmspaceAllocator {
    // Belirli bir VA aralığı için yeni bir ayırıcı oluşturur.
    const fn new_with_range(start: usize, end: usize) -> Self {
        VmspaceAllocator {
            next_vaddr: start,
            end_vaddr: end,
            start_vaddr_range: start,
        }
    }

     // Kullanıcı VA alanının başlangıcı ve sonu (örnek değerler)
     const USER_VA_START: usize = 0x1000_0000; // Kullanıcı alanı 256MB'den başlar
     const USER_VA_END: usize   = 0x8000_0000; // Kullanıcı alanı 2GB'de biter (örnek)


    // Belirtilen sayfaları (`num_pages`) haritalamak için bitişik bir sanal adres bloğu bulur ve tahsis eder.
    // Bu, çok basitleştirilmiş bir doğrusal aramadır ve parçalanmayı iyi yönetmez.
    // Gerçek bir ayırıcı boş liste veya benzeri teknikler kullanır.
     // Sayfa tablosu (`l1_table_ptr`) kullanılır çünkü VA'nın zaten haritalı olup olmadığını kontrol etmemiz gerekir.
    fn allocate_pages(&mut self, num_pages: usize, l1_table_ptr: *mut PageTable) -> Result<usize, KError> {
        if num_pages == 0 { return Ok(0); }
        let size = num_pages * PAGE_SIZE;

        // Basit bir 'bump' tahsis denemesi (son tahsisin bitişinden başla)
        let mut current_attempt = self.next_vaddr;
        if current_attempt + size > self.end_vaddr {
             // Aralığın sonuna geldiysek, başa dön (parçalanmış alanları kontrol etmek için)
             current_attempt = self.start_vaddr_range;
        }

        // Tam VA aralığında arama yap
        while current_attempt + size <= self.end_vaddr {
             // Bu aralığın haritalı olup olmadığını kontrol et (çok verimsiz - sayfa tablosunu yürür)
             let mut conflict = false;
             for i in 0..num_pages {
                 let vaddr = current_attempt + i * PAGE_SIZE;
                 // `lookup_page` kullanarak adresin haritalı olup olmadığını kontrol et.
                 // Hata yoksa, adres haritalıdır (çünkü `lookup_page` sadece geçerli L3 girdilerini döndürür).
                 if lookup_page(l1_table_ptr, vaddr).is_ok() {
                     conflict = true;
                     // Çakışma varsa, aramayı çakışan sayfanın bir sonraki sayfasından devam ettir.
                     current_attempt = vaddr + PAGE_SIZE;
                     break; // İç döngüyü kır, arama döngüsünü devam ettir.
                 }
             }

             if !conflict {
                 // Bitişik ve boş bir blok bulundu
                 self.next_vaddr = current_attempt + size; // Bir sonraki tahsis noktasını güncelle
                 return Ok(current_attempt); // Başlangıç sanal adresini döndür
             }

             // Eğer çakışma olduysa, current_attempt zaten bir sonraki arama noktasına güncellendi.
             // İç döngü kırılmadıysa (yani çakışma yoksa), döngü zaten ilerliyor demektir.
             if !conflict {
                  current_attempt += PAGE_SIZE; // Çakışma yoksa sayfalar halinde ilerle
             }
        }


        Err(KError::OutOfMemory) // Yeterince büyük bitişik boş blok bulunamadı
    }

}


// Gerçek bir çekirdekte, her Görev Kontrol Bloğunun (TCB) kendi kullanıcı sayfa tablosu işaretçisi ve VA ayırıcısı vardır.
// Bu basitleştirilmiş örnek için, tek bir global kullanıcı sayfa tablosu ve ayırıcı kullanacağız.
// BU ÇOKLU GÖREV ORTAMLARI İÇİN UYGUN DEĞİLDİR ve Senkronizasyon Gerekir!
static mut SINGLE_USER_PAGE_TABLE: Option<*mut PageTable> = None;
static mut SINGLE_USER_VA_ALLOCATOR: VmspaceAllocator = VmspaceAllocator::new_with_range(VmspaceAllocator::USER_VA_START, VmspaceAllocator::USER_VA_END);

// ktask modülünün bir kısmıymış gibi davranan yer tutucu.
mod ktask {
    use super::*;
    // Mevcut görevin kullanıcı sayfa tablosu işaretçisini döndüren yer tutucu fonksiyon.
    // Gerçek bir çekirdekte bu, mevcut TCB'den okunur.
    pub fn get_current_user_page_table() -> Option<*mut PageTable> {
        unsafe {
            if SINGLE_USER_PAGE_TABLE.is_none() {
                // Eğer sayfa tablosu henüz tahsis edilmemişse, şimdi et.
                 if let Some(frame) = pfa::allocate_frame() {
                     let pt_ptr = frame as *mut PageTable;
                     // volatile yazma kullanarak derleyicinin yazmayı optimize etmediğinden emin ol.
                     ptr::write_volatile(pt_ptr, PageTable::new()); // Sıfırlarla başlat
                     SINGLE_USER_PAGE_TABLE = Some(pt_ptr);
                      println!("Allocated initial user page table at {:p}", pt_ptr);
                 } else {
                      println!("Failed to allocate initial user page table!");
                     return None; // Sayfa tablosu olmadan devam edilemez
                 }
            }
            SINGLE_USER_PAGE_TABLE
        }
    }

     // Mevcut görevin VA ayırıcısını döndüren yer tutucu fonksiyon.
     pub fn get_current_user_va_allocator() -> Option<&'static mut VmspaceAllocator> {
         unsafe {
              // Gerçek çekirdekte bu, TCB'den allocator referansını alır.
             Some(&mut SINGLE_USER_VA_ALLOCATOR)
         }
     }

    // Mevcut görev ID'sini döndüren yer tutucu.
    pub fn get_current_task_id() -> KTaskId {
        // Çok basit bir dummy ID döndür.
        KTaskId(unsafe { SINGLE_USER_PAGE_TABLE.is_some() as u64 }) // Sayfa tablosu varsa ID 1, yoksa 0
    }

     // Başka bir görevin sayfa tablosuna geçiş yapan yer tutucu.
     // Zamanlayıcı bağlam değiştirirken çağırır.
     pub fn switch_to_task_page_table(l1_table_phys_addr: u64) {
         unsafe {
             // ARM sistem registerına (TTBR0_EL1) yeni L1 sayfa tablosunun fiziksel adresini yaz.
             // Bu, CPU'nun bundan sonra bu sayfa tablosunu kullanmasını sağlar.
              println!("Switching TTBR0_EL1 to {:x}", l1_table_phys_addr);
              asm!("msr ttbr0_el1, {}", in(reg) l1_table_phys_addr);
              asm!("isb"); // Talimat Senkronizasyon Bariyeri
         }
     }
}


// --- Sayfa Tablosu Yönetimi için Yardımcı Fonksiyonlar ---

// Belirtilen sanal adres için sayfa tablosunu yürür ve L3 girdisine mutable bir işaretçi döner.
// `create_if_missing`: true ise, eksik olan ara sayfa tablolarını (L1, L2) tahsis eder ve oluşturur.
// false ise, eksik bir girdi bulursa KError::BadAddress döner.
fn walk_page_table_mut(l1_table_ptr: *mut PageTable, virt_addr: usize, create_if_missing: bool) -> Result<*mut u64, KError> {
     // Adres sayfa sınırına hizalı mı?
     if virt_addr % PAGE_SIZE != 0 { return Err(KError::AddressNotPageAligned); }

     // VA'dan sayfa tablosu indekslerini çıkar
     let l1_index = (virt_addr >> L1_INDEX_SHIFT) & INDEX_MASK;
     let l2_index = (virt_addr >> L2_INDEX_SHIFT) & INDEX_MASK;
     let l3_index = (virt_addr >> L3_INDEX_SHIFT) & INDEX_MASK;

     // İndeksler geçerli aralıkta mı? (PAGE_TABLE_ENTRIES = 512)
     if l1_index >= PAGE_TABLE_ENTRIES || l2_index >= PAGE_TABLE_ENTRIES || l3_index >= PAGE_TABLE_ENTRIES {
         return Err(KError::BadAddress); // VA desteklenen aralığın dışında
     }

     let l1_table = unsafe { &mut *l1_table_ptr };
     let l1_entry_ptr = l1_table.entry_mut(l1_index).ok_or(KError::InternalError)?; // Geçerli indeksle başarısız olmamalı

     // L1 girdisini işle
     let mut l1_entry = unsafe { ptr::read_volatile(l1_entry_ptr) };
     let mut l2_table_ptr: *mut PageTable;

     if (l1_entry & pte_flags::VALID) == 0 {
         if !create_if_missing { return Err(KError::BadAddress); } // Oluşturma izni yoksa hata dön
         // L1 girdisi geçerli değil, L2 tablosu oluşturmamız gerekiyor
         let l2_table_frame = pfa::allocate_frame().ok_or(KError::OutOfMemory)?; // Fiziksel çerçeve tahsis et
         l2_table_ptr = l2_table_frame as *mut PageTable;
         unsafe { ptr::write_volatile(l2_table_ptr, PageTable::new()); } // L2 tablosunu sıfırlarla başlat

         // L1 girdisini yeni L2 tablosuna işaret edecek şekilde güncelle
         l1_entry = (l2_table_frame as u64) | pte_flags::TABLE | pte_flags::VALID | pte_flags::NON_SECURE | pte_flags::NSTABLE;
         unsafe { ptr::write_volatile(l1_entry_ptr, l1_entry); }
          // PTE yazdıktan sonra hafıza bariyeri gerekli
          unsafe { asm!("dsb ishst", options(nostack, preserves_flags)); } // Data Sync Barrier, Inner Shareable, Store
     } else if (l1_entry & pte_flags::TABLE) == 0 {
          // L1 girdisi geçerli ama bir tabloya işaret etmiyor - çakışma veya geçersiz durum
          return Err(KError::BadAddress); // Ya da AlreadyExists eğer create_if_missing true ise ve çakışma varsa
     } else {
          // L1 girdisi geçerli ve bir tabloya işaret ediyor, L2 tablo işaretçisini al
          let l2_table_phys_addr = l1_entry & !((PAGE_SIZE as u64) - 1); // Bayrakları maskele, fiziksel adresi al
          l2_table_ptr = l2_table_phys_addr as *mut PageTable;
     }

     let l2_table = unsafe { &mut *l2_table_ptr };
     let l2_entry_ptr = l2_table.entry_mut(l2_index).ok_or(KError::InternalError)?;

     // L2 girdisini işle
     let mut l2_entry = unsafe { ptr::read_volatile(l2_entry_ptr) };
     let mut l3_table_ptr: *mut PageTable;

     if (l2_entry & pte_flags::VALID) == 0 {
          if !create_if_missing { return Err(KError::BadAddress); }
         // L2 girdisi geçerli değil, L3 tablosu oluşturmamız gerekiyor
         let l3_table_frame = pfa::allocate_frame().ok_or(KError::OutOfMemory)?;
         l3_table_ptr = l3_table_frame as *mut PageTable;
         unsafe { ptr::write_volatile(l3_table_ptr, PageTable::new()); } // L3 tablosunu sıfırlarla başlat

         // L2 girdisini yeni L3 tablosuna işaret edecek şekilde güncelle
         l2_entry = (l3_table_frame as u64) | pte_flags::TABLE | pte_flags::VALID | pte_flags::NON_SECURE | pte_flags::NSTABLE;
         unsafe { ptr::write_volatile(l2_entry_ptr, l2_entry); }
          unsafe { asm!("dsb ishst", options(nostack, preserves_flags)); } // Data Sync Barrier
     } else if (l2_entry & pte_flags::TABLE) == 0 {
          // L2 girdisi geçerli ama bir tabloya işaret etmiyor
          return Err(KError::BadAddress); // Çakışma
     } else {
          // L2 girdisi geçerli ve bir tabloya işaret ediyor, L3 tablo işaretçisini al
          let l3_table_phys_addr = l2_entry & !((PAGE_SIZE as u64) - 1); // Bayrakları maskele
          l3_table_ptr = l3_table_phys_addr as *mut PageTable;
     }

     let l3_table = unsafe { &mut *l3_table_ptr };
     let l3_entry_ptr = l3_table.entry_mut(l3_index).ok_or(KError::InternalError)?;

     Ok(l3_entry_ptr) // L3 girdisine işaretçi dön
}

// Belirtilen sanal adres için sayfa tablosunu yürür ve L3 girdi değerini döner (salt okunur).
// Haritalı değilse KError::BadAddress döner.
fn lookup_page(l1_table_ptr: *mut PageTable, virt_addr: usize) -> Result<u64, KError> {
    if virt_addr % PAGE_SIZE != 0 { return Err(KError::AddressNotPageAligned); }

    let l1_index = (virt_addr >> L1_INDEX_SHIFT) & INDEX_MASK;
    let l2_index = (virt_addr >> L2_INDEX_SHIFT) & INDEX_MASK;
    let l3_index = (virt_addr >> L3_INDEX_SHIFT) & INDEX_MASK;

     if l1_index >= PAGE_TABLE_ENTRIES || l2_index >= PAGE_TABLE_ENTRIES || l3_index >= PAGE_TABLE_ENTRIES {
         return Err(KError::BadAddress); // VA desteklenen aralığın dışında
     }

    let l1_table = unsafe { &*l1_table_ptr };
    let l1_entry = l1_table.entry(l1_index).copied().ok_or(KError::BadAddress)?; // L1 girdisini oku

    // L1 girdisi geçerli ve bir tabloya mı işaret ediyor?
    if (l1_entry & pte_flags::VALID) == 0 || (l1_entry & pte_flags::TABLE) == 0 {
        return Err(KError::BadAddress); // L1 girdisi geçersiz veya tablo değil
    }

    let l2_table_phys_addr = l1_entry & !((PAGE_SIZE as u64) - 1);
    let l2_table_ptr = l2_table_phys_addr as *mut PageTable;
    let l2_table = unsafe { &*l2_table_ptr };
    let l2_entry = l2_table.entry(l2_index).copied().ok_or(KError::BadAddress)?; // L2 girdisini oku

     // L2 girdisi geçerli ve bir tabloya mı işaret ediyor?
     if (l2_entry & pte_flags::VALID) == 0 || (l2_entry & pte_flags::TABLE) == 0 {
        return Err(KError::BadAddress); // L2 girdisi geçersiz veya tablo değil
    }

    let l3_table_phys_addr = l2_entry & !((PAGE_SIZE as u64) - 1);
    let l3_table_ptr = l3_table_phys_addr as *mut PageTable;
    let l3_table = unsafe { &*l3_table_ptr };
    let l3_entry = l3_table.entry(l3_index).copied().ok_or(KError::BadAddress)?; // L3 girdisini oku

    // L3 girdisi geçerli ve bir sayfaya mı işaret ediyor?
    if (l3_entry & pte_flags::VALID) == 0 || (l3_entry & pte_flags::PAGE) == 0 {
        return Err(KError::BadAddress); // L3 girdisi geçersiz veya sayfa değil (yani haritalı değil)
    }

    Ok(l3_entry) // L3 sayfa girdi değerini dön
}


// Belirtilen fiziksel çerçeveyi (`phys_frame_addr`) sanal adrese (`virt_addr`) haritalar.
// Belirtilen L1 sayfa tablosunda (`l1_table_ptr`) işlemi yapar.
// İzinler `flags` (pte_flags kombinasyonu) ile belirtilir.
fn map_page(l1_table_ptr: *mut PageTable, virt_addr: usize, phys_frame_addr: *mut u8, flags: u64) -> Result<(), KError> {
    // Adresler sayfa sınırına hizalı mı?
    if virt_addr % PAGE_SIZE != 0 || (phys_frame_addr as usize) % PAGE_SIZE != 0 {
        return Err(KError::AddressNotPageAligned);
    }

    // Sayfa tablosunu yürü, eksik ara tabloları oluştur. L3 girdisine işaretçi al.
    let l3_entry_ptr = walk_page_table_mut(l1_table_ptr, virt_addr, true)?;

    // L3 girdisi zaten geçerli mi diye kontrol et (çakışma)
    let current_l3_entry = unsafe { ptr::read_volatile(l3_entry_ptr) };
    if (current_l3_entry & pte_flags::VALID) != 0 {
        // Bu sanal adres zaten haritalı! Bu, VA ayırıcısında bir hata veya yarış durumu olabilir.
        // Gerçek bir VA ayırıcısı, çakışma olmayacağını garanti etmelidir.
         println!("Map conflict: VA {:x} already mapped", virt_addr);
        return Err(KError::AlreadyExists);
    }

    // Yeni L3 sayfa girdisini oluştur (fiziksel adres + bayraklar)
    let new_l3_entry = (phys_frame_addr as u64) | flags;
    unsafe { ptr::write_volatile(l3_entry_ptr, new_l3_entry); }

    // PTE yazımının MMU ve diğer çekirdekler tarafından görünür olduğundan emin ol
    unsafe { asm!("dsb ishst", options(nostack, preserves_flags)); } // Data Sync Barrier, Inner Shareable, Store
    // Mevcut çekirdekte bu sanal adresin TLB (Translation Lookaside Buffer) girdisini geçersiz kıl.
    // Geçersiz kılma sonrası MMU'nun yeni PTE'yi okumasını zorlamak için bariyerler gerekli.
    unsafe { asm!("tlbi vaa, {}", in(reg) virt_addr, options(nostack, preserves_flags)); } // Geçerli adrese göre TLB geçersiz kıl
    unsafe { asm!("dsb ish", options(nostack, preserves_flags)); } // Data Sync Barrier, Inner Shareable
    unsafe { asm!("isb", options(nostack, preserves_flags)); } // Instruction Synchronization Barrier

    Ok(())
}


// Belirtilen sanal adresteki haritalamayı kaldırır (`virt_addr`).
// Belirtilen L1 sayfa tablosunda (`l1_table_ptr`) işlemi yapar.
// Fiziksel çerçeveyi serbest BIRAKMAZ. Serbest bırakılan fiziksel adresini döner.
fn unmap_page(l1_table_ptr: *mut PageTable, virt_addr: usize) -> Result<*mut u8, KError> {
    // Adres sayfa sınırına hizalı mı?
    if virt_addr % PAGE_SIZE != 0 { return Err(KError::AddressNotPageAligned); }

    // Sayfa tablosunu yürü, L3 girdisini bul (eksikse oluşturma)
    let l3_entry_ptr = walk_page_table_mut(l1_table_ptr, virt_addr, false)?;

    // L3 girdisini oku
    let mut l3_entry = unsafe { ptr::read_volatile(l3_entry_ptr) };

    // L3 girdisi geçerli ve bir sayfaya mı işaret ediyor?
    if (l3_entry & pte_flags::VALID) == 0 || (l3_entry & pte_flags::PAGE) == 0 {
        // L3 girdisi geçersiz veya sayfa değil - bu sanal adreste haritalama yok.
        // println!("Unmap failed: VA {:x} not mapped", virt_addr);
        return Err(KError::BadAddress); // Haritalama mevcut değil
    }

    // Girdiyi temizlemeden önce fiziksel adresi al
    let phys_frame_addr = (l3_entry & !((PAGE_SIZE as u64) - 1)) as *mut u8;

    // L3 girdisini temizleyerek haritalamayı kaldır (INVALID olarak işaretle)
    unsafe { ptr::write_volatile(l3_entry_ptr, 0); }

    // Yazımın görünür olduğundan emin ol ve TLB'yi geçersiz kıl
    unsafe { asm!("dsb ishst", options(nostack, preserves_flags)); } // Data Sync Barrier
    unsafe { asm!("tlbi vaa, {}", in(reg) virt_addr, options(nostack, preserves_flags)); } // TLB geçersiz kıl
    unsafe { asm!("dsb ish", options(nostack, preserves_flags)); } // Data Sync Barrier
    unsafe { asm!("isb", options(nostack, preserves_flags)); } // Instruction Synchronization Barrier


    // TODO: Eğer bu silme sonucu üst seviye sayfa tabloları boşalırsa, onları serbest bırakabiliriz.
    // Bu karmaşık bir mantık gerektirir (referans sayma veya geri yürüme). Basitlik için atlandı.
    // Boş tabloların hafızada kalması basit implementasyonlarda kabul edilebilir bir 'leak'tir.

    Ok(phys_frame_addr) // Serbest bırakılan fiziksel çerçevenin adresini dön
}


// --- Karnal64 kmemory API Implementasyonları (srcmmu_arm.rs içinde) ---

// Bu `init_manager` fonksiyonu `karnal64::init()` tarafından çağrılacaktır.
// Çekirdek boot sürecinde MMU donanımını ve bellek yöneticisinin yapılarını başlatır.
// physical_memory_start ve size: PFA'nın kullanabileceği fiziksel bellek aralığı.
pub fn init_manager(physical_memory_start: usize, physical_memory_size: usize) {
    // Fiziksel çerçeve ayırıcıyı başlat (yer tutucu)
    pfa::init(physical_memory_start, physical_memory_size);

    // TODO: ARM MMU donanımını başlat (TTBR0_EL1, TTBR1_EL1, TCR_EL1, MAIR_EL1, vb.)
    // Bu, sistem registerlarına yazmayı gerektirir. Mimariye özgü assembly veya crate gerekli.

    // Örnek kavramsal register yazmaları:
    unsafe {
        // MAIR_EL1'i ayarla (bellek öznitelikleri için, önbelleklenebilirlik gibi)
        // Bu örnek için basit öznitelikler tanımlayalım:
        // ATTR0: nGnRnE (Device - cihaz belleği için, önbellekleme yok, reorder yok)
        // ATTR1: Normal Bellek, Inner/Outer Write-back, Write-Allocate (Önbelleklenebilir)
         MAIR_EL1 = (0x00 << (0*8)) | (0xFF << (1*8)); // Örnek kodlama
         asm!("msr mair_el1, {}", in(reg) MAIR_EL1, options(nostack, preserves_flags));

        // TCR_EL1'i ayarla (adres alanı boyutları, sayfa tablosu yürüme ayarları vb.)
        // TTBR0/TTBR1'de 39-bit VAlar için: T0SZ=25, T1SZ=25 (64-25=39 bit)
        // TTBR0_EL1 bit [38:0] kullanır (kullanıcı alanı), TTBR1_EL1 bit [63:25] kullanır (çekirdek alanı).
        // Sadece kullanıcı alanını (TTBR0_EL1) ayarladığımızı varsayalım.
        // TCR_EL1 ayarları çekirdeğin VA tasarımına büyük ölçüde bağlıdır.
        // TTBR0_EL1 (kullanıcı alanı) için örnek ayarlar:
        // T0SZ = 25 (39-bit VA)
        // EPD0 = 0 (TTBR0_EL1 çevirisini etkinleştir)
        // IRGN0 = 1 (Inner cacheable, Write-back)
        // ORGN0 = 1 (Outer cacheable, Write-back)
        // SH0 = 3 (Inner Shareable)
        // TG0 = 0 (4KB Granule - 4KB sayfa boyutu)
        // Tam bir TCR_EL1 değeri diğer alanları (T1SZ, EPD1, IPS, AS, TBI, vb.) ayarlamayı gerektirir.
        // Basitlik için demo VA aralığı için temel ayarların yeterli olduğunu varsayalım.
         TCR_EL1 = (25 << 0) | (1 << 8) | (1 << 10) | (3 << 12) | (0 << 14); // Kavramsal değer
         asm!("msr tcr_el1, {}", in(reg) TCR_EL1, options(nostack, preserves_flags));


        // TTBR0_EL1'i ilk kullanıcı alanı sayfa tablosu için ayarla.
        // İlk kullanıcı sayfa tablosunu şimdi tahsis et.
        let initial_user_pt_ptr = ktask::get_current_user_page_table().expect("Failed to allocate initial user page table during MMU init");
        // Sayfa tablosunun fiziksel adresi (çekirdek alanında identity mapping varsayımıyla)
        // Gerçek bir çekirdekte, çekirdek VA'sının fiziksel PA'ya çevrilmesi gerekir.
        let initial_user_pt_phys_addr = initial_user_pt_ptr as u64;

        // Kullanıcı L1 tablosunun fiziksel adresini TTBR0_EL1'e yaz.
         asm!("msr ttbr0_el1, {}", in(reg) initial_user_pt_phys_addr, options(nostack, preserves_flags));
         println!("Initialized TTBR0_EL1 with user page table at {:x}", initial_user_pt_phys_addr);


        // MMU'yu etkinleştir (SCTLR_EL1.M bitini set et)
         let mut sctlr_el1: u64;
         asm!("mrs {}, sctlr_el1", out(reg) sctlr_el1);
         sctlr_el1 |= 1; // M bitini set et
         asm!("msr sctlr_el1, {}", in(reg) sctlr_el1, options(nostack, preserves_flags));

        // Talimat Senkronizasyon Bariyeri (MMU etkinleşmeden sonraki talimatların çeviri kullanmasını sağlar)
         asm!("isb", options(nostack, preserves_flags));
    }

     println!("MMU (ARM): Manager Initialized (Placeholder)");
}


// `kmemory::allocate_user_memory` fonksiyonunu implemente eder.
// Kullanıcı alanı sanal bellekte `size` byte yer tahsis eder ve karşılık gelen fiziksel çerçeveleri ayırır.
// Tahsis edilen sanal adresin başlangıcını döner.
pub fn allocate_user_memory(size: usize) -> Result<*mut u8, KError> {
    if size == 0 { return Ok(ptr::null_mut()); }

    let num_pages = (size + PAGE_SIZE - 1) / PAGE_SIZE; // Kaç sayfa gerektiğini hesapla
    let user_l1_table_ptr = ktask::get_current_user_page_table().ok_or(KError::InternalError)?; // Mevcut görevin sayfa tablosunu al
    let user_va_allocator = ktask::get_current_user_va_allocator().ok_or(KError::InternalError)?; // Mevcut görevin VA ayırıcısını al

    // 1. `num_pages` kadar bitişik boş sanal adres alanı bul.
    let start_vaddr = user_va_allocator.allocate_pages(num_pages, user_l1_table_ptr)?;
     println!("Allocated VA range: {:x} - {:x}", start_vaddr, start_vaddr + size);

    let mut allocated_phys_frames: Vec<*mut u8> = Vec::new(); // Ayrılan fiziksel çerçeveleri tutacak vektör

    // 2. Sanal adres aralığı için fiziksel çerçeveleri tahsis et.
    for _ in 0..num_pages {
        let phys_frame = pfa::allocate_frame().ok_or_else(|| {
            // Eğer tahsis ortasında başarısız olursa, daha önce tahsis edilen çerçeveleri temizle.
            for frame in allocated_phys_frames.drain(..) { // drain() kullanarak vektörü tüketirken yinele
                pfa::free_frame(frame);
            }
            KError::OutOfMemory // Bellek yetersiz hatası dön
        })?;
        allocated_phys_frames.push(phys_frame); // Başarılı olursa listeye ekle
    }
     println!("Allocated {} physical frames", allocated_phys_frames.len());


    // 3. Tahsis edilen fiziksel çerçeveleri bulunan sanal adres aralığına haritala.
    for (i, phys_frame) in allocated_phys_frames.into_iter().enumerate() {
        let virt_addr = start_vaddr + i * PAGE_SIZE; // Haritalanacak sanal adres
        // Kullanıcı R/W izinleri, Önbelleklenebilir Normal Bellek öznitelikleri kullan.
        let flags = pte_flags::USER_DATA_FLAGS;

        // Sayfayı haritala.
        if map_page(user_l1_table_ptr, virt_addr, phys_frame, flags).is_err() {
            // Haritalama başarısız oldu - Bu, VA bulma doğruysa ve haritalama mantığı sağlamsa olmamalı.
            // Ciddi temizlik gerektirir: daha önce haritalanmış sayfaları kaldır, tahsis edilen tüm fiziksel çerçeveleri serbest bırak.
            // Bu hata işleme karmaşıktır. Basitlik için sadece hata dönüyoruz.
             // Gerçek bir çekirdekte, hata durumunda daha önce başarıyla haritalanmış sayfaları kaldırmaya çalışılır.
             let mut cleanup_vaddr = start_vaddr;
             for j in 0..i { // Hatadan önce başarıyla haritalanmış sayfaları kaldır
                 if unmap_page(user_l1_table_ptr, cleanup_vaddr).is_ok() {
                      // Temizlik sırasında başarıyla kaldırıldı
                 } else {
                     // Temizlik sırasında hata! Bu kötü. Logla.
                       println!("Cleanup error: Failed to unmap VA {:x} during allocate_user_memory error", cleanup_vaddr);
                 }
                 cleanup_vaddr += PAGE_SIZE;
             }
              // Kalan tahsis edilmiş fiziksel çerçeveleri serbest bırak.
              for frame in allocated_phys_frames.drain(..) { // Vektör artık boş olabilir, ama drain güvenlidir.
                  pfa::free_frame(frame);
              }
               println!("allocate_user_memory failed at VA {:x}", virt_addr);
             return Err(KError::InternalError); // Ya da daha spesifik bir hata
        }
         println!("Mapped VA {:x} to PA {:p}", virt_addr, phys_frame);
    }

    Ok(start_vaddr as *mut u8) // Tahsis edilen sanal adresin başlangıcını döndür
}


// `kmemory::free_user_memory` fonksiyonunu implemente eder.
// Kullanıcı alanı sanal bellek aralığındaki haritalamayı kaldırır ve karşılık gelen fiziksel çerçeveleri serbest bırakır.
// Belirtilen işaretçinin geçerli, tahsis edilmiş bir kullanıcı alanı işaretçisi olması gerekir.
pub fn free_user_memory(ptr: *mut u8, size: usize) -> Result<(), KError> {
    if ptr.is_null() || size == 0 { return Ok(()); }

    let start_vaddr = ptr as usize;
    // Başlangıç adresi sayfa sınırına hizalı mı?
    if start_vaddr % PAGE_SIZE != 0 { return Err(KError::AddressNotPageAligned); }

    let num_pages = (size + PAGE_SIZE - 1) / PAGE_SIZE; // Kaç sayfa serbest bırakılacak
    let user_l1_table_ptr = ktask::get_current_user_page_table().ok_or(KError::InternalError)?; // Mevcut görevin sayfa tablosunu al

    // TODO: Sanal adres aralığındaki haritalamaları kaldır ve fiziksel çerçeveleri serbest bırak.
    for i in 0..num_pages {
        let virt_addr = start_vaddr + i * PAGE_SIZE; // Haritalaması kaldırılacak sanal adres
        // Sayfanın haritalamasını kaldır ve ilişkili fiziksel adresi al.
        match unmap_page(user_l1_table_ptr, virt_addr) {
            Ok(phys_frame_addr) => {
                 // Başarıyla haritalama kaldırıldı, şimdi fiziksel çerçeveyi serbest bırak.
                 pfa::free_frame(phys_frame_addr);
                  println!("Unmapped VA {:x}, freed PA {:p}", virt_addr, phys_frame_addr);
            }
            Err(KError::BadAddress) => {
                // Haritalı olmayan veya beklendiği gibi haritalı olmayan belleği serbest bırakmaya çalışılıyor.
                // Bu, çift serbest bırakma veya geçersiz işaretçi kullanımı olabilir.
                // Gerçek bir çekirdekte bu muhtemelen bir panic veya ciddi bir hata olmalıdır.
                 // Bu örnek için, ilk haritalı olmayan sayfada hata dönüyoruz.
                 // Daha sağlam bir yaklaşım, aralıktaki tüm olası sayfaları kaldırmaya çalışıp
                 // sonra herhangi biri bulunamazsa raporlamak olurdu.
                   println!("free_user_memory error: VA {:x} not mapped", virt_addr);
                 return Err(KError::BadAddress);
            }
            Err(e) => {
                 // Diğer haritalama kaldırma hatası
                   println!("free_user_memory error: Failed to unmap VA {:x}: {:?}", virt_addr, e);
                 return Err(e);
            }
        }
    }

    // TODO: Eğer bu silme sonucu üst seviye sayfa tabloları boşalırsa, onları serbest bırakabiliriz. (Karmaşık, atlandı)

    Ok(()) // Başarı
}

// --- Paylaşımlı Bellek Yönetimi ---
// Görevler arasında bellek alanlarını paylaşmak için.

// Paylaşımlı Bellek Yöneticisi Yer Tutucu
// Gerçek bir yöneticide eşzamanlı erişim kontrolü (örn. Mutex) ve daha sofistike yapı gerekir.
mod shared_mem_manager {
     use super::*;
     use alloc::vec::Vec;
     use alloc::boxed::Box; // SharedMemObject'i heap'te tutmak için
     use core::sync::atomic::{AtomicU64, Ordering}; // Handle üretimi için

     // Bir paylaşımlı bellek nesnesinin detaylarını tutan yapı
     struct SharedMemObject {
         frames: Vec<*mut u8>, // Bu nesneye ait fiziksel çerçeveler
         // Birden fazla görev haritalayabilirse, ne zaman çerçeveleri serbest bırakacağımızı bilmek için referans sayacı eklenmeli.
          ref_count: AtomicUsize,
     }

     // Paylaşımlı bellek nesnelerini tutmak için static bir Option dizisi kullanıyoruz.
     // Gerçek bir çekirdekte bu, dinamik bir yapı veya daha sofistike bir yönetici olurdu.
     const MAX_SHARED_MEM_OBJECTS: usize = 32; // Maksimum paylaşımlı bellek nesnesi sayısı
     static mut SHARED_MEM_OBJECTS: [Option<Box<SharedMemObject>>; MAX_SHARED_MEM_OBJECTS] = {
         // Diziyi None ile başlat
         let mut objects: [Option<Box<SharedMemObject>>; MAX_SHARED_MEM_OBJECTS] = [None; MAX_SHARED_MEM_OBJECTS];
         // Dizi elemanlarını manuel olarak initialize etmemiz gerekebilir const fn içinde Box::new kullanamadığımız için.
         // Şimdilik bu şekilde bırakalım, ilk kullanımdan önce dolaylı olarak None olacak varsayımıyla.
         objects
     };
     static NEXT_HANDLE_VALUE: AtomicU64 = AtomicU64::new(1000); // Handle değerlerini çakışmaması için yüksekten başlatalım.

     // Yeni bir paylaşımlı bellek nesnesini kaydeder ve bir KHandle döner.
     pub fn register_object(frames: Vec<*mut u8>) -> Result<KHandle, KError> {
         let new_object = Box::new(SharedMemObject { frames });

         // Boş bir slot bul (basitleştirilmiş, gerçek çekirdekte kilitleme gerekli)
         unsafe {
             for i in 0..MAX_SHARED_MEM_OBJECTS {
                 if SHARED_MEM_OBJECTS[i].is_none() {
                      SHARED_MEM_OBJECTS[i] = Some(new_object); // Nesneyi slot'a yerleştir
                      // Slot indeksi ve benzersiz bir değer içeren bir handle oluştur.
                      // Gerçek handle güvenlik için rastgele bir bileşen de içerebilir.
                      let handle_value = NEXT_HANDLE_VALUE.fetch_add(1, Ordering::SeqCst); // Benzersiz değeri al ve artır
                      let k_handle = KHandle(((i as u64) << 32) | (handle_value & 0xFFFFFFFF)); // İndeks ve değeri birleştir
                       println!("Registered shared memory object in slot {} with handle {:x}", i, k_handle.0);
                      return Ok(k_handle); // Handle'ı dön
                 }
             }
         }
         // Boş slot yok
         Err(KError::OutOfMemory) // Ya da paylaşımlı bellek nesnelerine özel bir hata
     }

     // Bir KHandle'dan ilişkili fiziksel çerçevelerin listesini al.
     // Kullanıcının handle'a erişim izni olduğu varsayılır (kresource tarafından kontrol edilebilir).
     pub fn get_object_frames(handle: &KHandle) -> Result<&'static Vec<*mut u8>, KError> {
          let index = (handle.0 >> 32) as usize; // Handle'dan slot indeksini çıkar
          let uniqueifier = (handle.0 & 0xFFFFFFFF); // Handle'dan benzersiz değeri çıkar

         unsafe {
              if index < MAX_SHARED_MEM_OBJECTS {
                   if let Some(obj) = SHARED_MEM_OBJECTS[index].as_ref() {
                       // Gerçek sistemde, handle'ın benzersiz değerini kontrol ederek handle'ın geçerli olduğundan emin ol.
                       // Bu demo için, şimdilik sadece indekse güveniyoruz.
                        println!("Looked up shared memory object in slot {} from handle {:x}", index, handle.0);
                        Ok(&obj.frames) // Çerçevelerin referansını dön
                   } else {
                       // Slot boş, handle geçersiz.
                        println!("Shared memory handle {:x} -> Slot {} is empty", handle.0, index);
                       Err(KError::BadHandle)
                   }
              } else {
                   // İndeks aralık dışında, handle geçersiz.
                     println!("Shared memory handle {:x} -> Index {} out of bounds", handle.0, index);
                   Err(KError::BadHandle)
              }
         }
     }

     // Paylaşımlı bellek nesnesini serbest bırakır (ref sayısını azaltır, 0 olursa çerçeveleri serbest bırakır).
     // Bu, KHandle kullanıcı tarafından serbest bırakıldığında çağrılır.
     pub fn release_object(handle: &KHandle) -> Result<(), KError> {
         let index = (handle.0 >> 32) as usize; // İndeksi çıkar
         let uniqueifier = (handle.0 & 0xFFFFFFFF); // Benzersiz değeri çıkar

          unsafe {
               if index < MAX_SHARED_MEM_OBJECTS {
                    // Option'dan Box'ı al (move) - bu, slot'u None yapar.
                    if let Some(obj_box) = SHARED_MEM_OBJECTS[index].take() {
                        // Gerçek sistemde uniqueifier'ı kontrol et ve referans sayısını azalt.
                        // Referans sayısı 0 olursa, çerçeveleri serbest bırak.
                        // Bu demo için, serbest bırakıldığında çerçeveleri doğrudan serbest bırakıyoruz (basitlik).
                         println!("Releasing shared memory object in slot {} from handle {:x}", index, handle.0);
                         let mut obj = Box::into_inner(obj_box); // Box içindeki SharedMemObject'i al
                        for frame in obj.frames.drain(..) { // Çerçeveleri serbest bırak
                            pfa::free_frame(frame);
                        }
                        // 'obj' burada scope dışına çıkar ve düşer (drop olur), SharedMemObject yapısı serbest kalır.
                         Ok(())
                    } else {
                        // Slot zaten boştu (çift serbest bırakma?)
                         println!("Shared memory handle {:x} -> Slot {} was already empty on release", handle.0, index);
                        Err(KError::BadHandle)
                    }
               } else {
                    // İndeks aralık dışında
                      println!("Shared memory handle {:x} -> Index {} out of bounds on release", handle.0, index);
                    Err(KError::BadHandle)
               }
          }
     }
}


// `kmemory::shared_mem_create` fonksiyonunu implemente eder.
// `size` boyutunda bir paylaşımlı bellek bölgesi oluşturur. Fiziksel çerçeveleri tahsis eder ancak henüz haritalamaz.
// Bu paylaşımlı bellek nesnesini temsil eden bir KHandle döner.
pub fn shared_mem_create(size: usize) -> Result<KHandle, KError> {
     if size == 0 { return Err(KError::InvalidArgument); }

     let num_pages = (size + PAGE_SIZE - 1) / PAGE_SIZE; // Kaç sayfa gerekli
     let mut allocated_phys_frames: Vec<*mut u8> = Vec::new(); // Ayrılan fiziksel çerçeveler

     // Paylaşımlı bellek bölgesi için fiziksel çerçeveleri tahsis et
     for _ in 0..num_pages {
         let phys_frame = pfa::allocate_frame().ok_or_else(|| {
             // Tahsis sırasında hata olursa, daha önce tahsis edilenleri temizle
             for frame in allocated_phys_frames.drain(..) {
                 pfa::free_frame(frame);
             }
             KError::OutOfMemory
         })?;
         allocated_phys_frames.push(phys_frame); // Listeye ekle
     }

     // Ayrılan çerçeveleri paylaşımlı bellek yöneticisine kaydet ve handle al.
     shared_mem_manager::register_object(allocated_phys_frames)
}


// `kmemory::shared_mem_map` fonksiyonunu implemente eder.
// Bir paylaşımlı bellek bölgesini (`k_handle_value` ile tanımlanan), mevcut görevin sanal adres alanına haritalar.
// Haritalama, paylaşımlı bellek nesnesindeki `offset` adresinden başlayıp `size` byte sürer.
// Haritalamanın başladığı kullanıcı alanı sanal adresini döner.
pub fn shared_mem_map(k_handle_value: u64, offset: usize, size: usize) -> Result<*mut u8, KError> {
     if size == 0 { return Ok(ptr::null_mut()); }
     // Offset sayfa sınırına hizalı olmalı
     if offset % PAGE_SIZE != 0 { return Err(KError::AddressNotPageAligned); }

     let handle = KHandle(k_handle_value); // Handle değerinden KHandle oluştur

     // Yöneticiden paylaşımlı bellek nesnesine ait fiziksel çerçeveleri al.
     let shared_mem_frames = shared_mem_manager::get_object_frames(&handle)?;

     // Offset ve size'ı paylaşımlı bellek nesnesinin boyutuyla doğrula.
     let object_size_pages = shared_mem_frames.len();
     let offset_pages = offset / PAGE_SIZE; // Ofsetin sayfa cinsinden karşılığı
     let num_pages_to_map = (size + PAGE_SIZE - 1) / PAGE_SIZE; // Kaç sayfa haritalanacak

     // Haritalama aralığı nesnenin sınırları içinde mi?
     if offset_pages >= object_size_pages || offset_pages + num_pages_to_map > object_size_pages {
          println!("shared_mem_map: Invalid offset ({}) or size ({}) for object with {} pages", offset, size, object_size_pages);
         return Err(KError::InvalidArgument); // Haritalama aralığı nesne sınırları dışında
     }

     let user_l1_table_ptr = ktask::get_current_user_page_table().ok_or(KError::InternalError)?; // Mevcut görevin sayfa tablosunu al
     let user_va_allocator = ktask::get_current_user_va_allocator().ok_or(KError::InternalError)?; // Mevcut görevin VA ayırıcısını al

     // 1. Haritalanacak `num_pages_to_map` kadar sayfa için bitişik boş sanal adres alanı bul.
     let map_start_vaddr = user_va_allocator.allocate_pages(num_pages_to_map, user_l1_table_ptr)?;
      println!("shared_mem_map: Found VA space at {:x} for {} pages", map_start_vaddr, num_pages_to_map);

     // 2. Paylaşımlı bellek nesnesindeki ilgili fiziksel çerçeveleri bulunan VA aralığına haritala.
     // Paylaşımlı bellek genellikle kullanıcılar için R/W'dir.
     let flags = pte_flags::USER_DATA_FLAGS; // Ya da USER_CODE_FLAGS yürütülebilir kütüphaneyse.
     // Kullanıcı haritalama sırasında izinleri belirtebilir mi? Şimdilik varsayılan R/W.


     for i in 0..num_pages_to_map {
         let virt_addr = map_start_vaddr + i * PAGE_SIZE; // Haritalanacak sanal adres
         let phys_frame = shared_mem_frames[offset_pages + i]; // Nesneden belirli fiziksel çerçeveyi al

          println!("shared_mem_map: Mapping VA {:x} to PA {:p} with flags {:x}", virt_addr, phys_frame, flags);

         // Sayfayı haritala.
         if map_page(user_l1_table_ptr, virt_addr, phys_frame, flags).is_err() {
              // Haritalama başarısız - temizlik gerekli (bu aralıkta daha önce haritalanmış sayfaları kaldır).
              // Bu karmaşıktır. Basitlik için sadece hata dönüyoruz.
              // Hata oluşmadan önce başarıyla haritalanmış sayfaları kaldırmaya çalış.
             let mut cleanup_vaddr = map_start_vaddr;
              for j in 0..i {
                  if unmap_page(user_l1_table_ptr, cleanup_vaddr).is_ok() {
                       // Temizlik sırasında başarıyla haritalama kaldırıldı
                  } else {
                      // Temizlik sırasında hata! Logla.
                        println!("Cleanup error: Failed to unmap VA {:x} during shared_mem_map error", cleanup_vaddr);
                  }
                  cleanup_vaddr += PAGE_SIZE;
              }
              // Paylaşımlı bellek nesnesi (çerçeveler) burada serbest BIRAKILMAZ, yöneticinin mülkiyetindedir.
              return Err(KError::InternalError); // Ya da daha spesifik bir hata
         }
     }

     Ok(map_start_vaddr as *mut u8) // Haritalanan sanal adresin başlangıcını dön
}


// `kmemory::shared_mem_unmap` fonksiyonunu implemente eder.
// Bir paylaşımlı bellek bölgesini, mevcut görevin sanal adres alanından kaldırır.
// Fiziksel çerçeveleri serbest BIRAKMAZ (çerçeveler paylaşımlı bellek nesnesine aittir).
pub fn shared_mem_unmap(ptr: *mut u8, size: usize) -> Result<(), KError> {
    if ptr.is_null() || size == 0 { return Ok(()); }

    let start_vaddr = ptr as usize;
    // Başlangıç adresi sayfa sınırına hizalı mı?
    if start_vaddr % PAGE_SIZE != 0 { return Err(KError::AddressNotPageAligned); } // Sayfa hizalı olmalı

    let num_pages = (size + PAGE_SIZE - 1) / PAGE_SIZE; // Kaç sayfa kaldırılacak
    let user_l1_table_ptr = ktask::get_current_user_page_table().ok_or(KError::InternalError)?; // Mevcut görevin sayfa tablosunu al

    // TODO: Sanal adres aralığındaki haritalamaları kaldır.
    for i in 0..num_pages {
        let virt_addr = start_vaddr + i * PAGE_SIZE; // Haritalaması kaldırılacak sanal adres
        // Sayfanın haritalamasını kaldır. Dönen fiziksel adresi kullanmıyoruz çünkü çerçeveyi burada serbest bırakmıyoruz.
        match unmap_page(user_l1_table_ptr, virt_addr) {
            Ok(_) => {
                 // Başarıyla haritalama kaldırıldı.
                  println!("shared_mem_unmap: Unmapped VA {:x}", virt_addr);
            }
             Err(KError::BadAddress) => {
                // Bu sanal adreste haritalı olmayan belleği kaldırmaya çalışılıyor.
                // Bu, free_user_memory'deki duruma göre daha az kritiktir, belki sadece uyar veya OK dön.
                // Sağlamlık için, beklenen bir sayfayı kaldıramazsa ilk hatada hata dönelim.
                 // Daha sağlam bir yaklaşım, aralıktaki tüm olası sayfaları kaldırmaya çalışıp
                 // sonra herhangi biri bulunamazsa raporlamak olurdu.
                   println!("shared_mem_unmap: VA {:x} not mapped or not found", virt_addr);
                 return Err(KError::BadAddress);
             }
            Err(e) => {
                 // Diğer haritalama kaldırma hatası
                   println!("shared_mem_unmap: Error unmapping VA {:x}: {:?}", virt_addr, e);
                 return Err(e);
            }
        }
    }

    // TODO: Eğer bu silme sonucu üst seviye sayfa tabloları boşalırsa, onları serbest bırakabiliriz. (Karmaşık, atlandı)

    Ok(()) // Başarı
}


// TODO: Çekirdek belleği haritalama fonksiyonları ekle (TTBR1_EL1 kullanarak)
 pub fn map_kernel_memory(...) -> Result<(), KError> { ... }
 pub fn unmap_kernel_memory(...) -> Result<(), KError> { ... }

// TODO: Bellek durumu sorgulama fonksiyonları ekle (is_mapped, get_phys_addr gibi)
 pub fn lookup_vaddr_phys(virt_addr: usize) -> Result<*mut u8, KError> { ... } // VA'dan PA bulma
