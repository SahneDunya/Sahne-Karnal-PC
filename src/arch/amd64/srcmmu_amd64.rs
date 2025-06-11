#![no_std] // Standart kütüphaneye ihtiyacımız yok, çekirdek alanında çalışıyoruz

// Geliştirme sırasında kullanılmayan kod veya argümanlar için izinler
#![allow(dead_code)]
#![allow(unused_variables)]

use core::ptr;
use core::fmt;
// İhtiyaç duyulursa x86_64 spesifik intrinsikler için:
 use core::arch::x86_64;


// --- MMU Spesifik Hata Türü ---
// MMU işlemlerinde ortaya çıkabilecek hatalar için
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i64)] // İhtiyaç duyulursa KError'a dönüştürmek için veya doğrudan sistem çağrısına dönebilmek için
pub enum MmuError {
    /// Fiziksel bellek tahsisi yapılamadı (yeni sayfa tablosu için)
    OutOfMemory = -201,
    /// Adres zaten haritalanmış durumda
    AlreadyMapped = -202,
    /// Adres haritalanmamış durumda
    NotMapped = -203,
    /// Geçersiz argüman (örn: sayfa hizalı olmayan adres)
    InvalidArgument = -204,
    /// Sayfa tablosu yapısı bozuk veya tutarsız
    PageTableCorrupted = -205,
    /// Sayfa tablosu yürüyüşü sırasında beklenen bir tablo düzeyi bulunamadı
    MissingPageTable = -206,
    /// İşlem desteklenmiyor (örn: desteklenmeyen sayfa boyutu)
    NotSupported = -207,
    /// Dahili MMU hatası
    InternalError = -299,
}

// MMU işlemleri için kolaylık tipleri
pub type PhysAddr = u64; // Fiziksel Adres
pub type VirtAddr = u64; // Sanal Adres

// --- Sabitler ---
// Sayfa Boyutları
pub const PAGE_SIZE_4K: u64 = 4096; // 4 Kilobyte
pub const PAGE_SIZE_2M: u64 = 2 * 1024 * 1024; // 2 Megabyte
pub const PAGE_SIZE_1G: u64 = 1024 * 1024 * 1024; // 1 Gigabyte

// Sayfa Tablosu Giriş Bayrakları (x86_64)
// Daha yaygın kullanılan bayraklar tanımlanmıştır.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct PageFlags(u64);

impl PageFlags {
    // Ortak Bayraklar
    pub const PRESENT: Self = Self(1 << 0);         // P: Bellekte mevcut
    pub const WRITABLE: Self = Self(1 << 1);        // R/W: Yazılabilir
    pub const USER_ACCESSIBLE: Self = Self(1 << 2); // U/S: Kullanıcı tarafından erişilebilir (0=süpervizör)
    pub const WRITE_THROUGH: Self = Self(1 << 3);   // PWT: Write-Through önbellekleme
    pub const CACHE_DISABLED: Self = Self(1 << 4);  // PCD: Önbellekleme Devre Dışı
    pub const ACCESSED: Self = Self(1 << 5);        // A: Erişilmiş
    pub const DIRTY: Self = Self(1 << 6);           // D: Değiştirilmiş (Yazılmış)

    // Orta ve Son düzey sayfa tabloları için (2MB/1GB veya 4KB sayfa)
    pub const HUGE_PAGE: Self = Self(1 << 7);       // PS: Sayfa Boyutu (1=büyük sayfa, 0=4KB alt tabloya işaret)

    // Son düzey (PTE) için
    pub const GLOBAL: Self = Self(1 << 8);          // G: Global sayfa (CR3 değişse bile TLB'den atılmaz)
    // Bayrakların 52-62. bitleri rezervdir.
    pub const NO_EXECUTE: Self = Self(1 << 63);     // NX: Çalıştırma Engeli (EFER.NXE gerektirir)

    /// Belirtilen ham bayrak değeri ile yeni bir PageFlags oluşturur.
    pub const fn new(flags: u64) -> Self {
        Self(flags)
    }

    /// Belirli bir bayrağın ayarlı olup olmadığını kontrol eder.
    #[inline]
    pub fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) == flag.0
    }

    /// Mevcut bayraklara başka bayrakları ekler.
    #[inline]
    pub fn with(&self, flag: Self) -> Self {
        Self(self.0 | flag.0)
    }

    /// Mevcut bayraklardan başka bayrakları çıkarır.
    #[inline]
    pub fn without(&self, flag: Self) -> Self {
        Self(self.0 & !flag.0)
    }

    /// Ham bayrak değerini döndürür.
    #[inline]
    pub fn raw(&self) -> u64 {
        self.0
    }
}

// Fiziksel adresin sayfa tablosu girişindeki konumu için maskeler
// x86_64 mimarisinde fiziksel adres, girişin alt 12 bitindeki bayraklar ve
// 52. bitten sonraki bitlerdeki (PCID, vs.) bayraklar dışındaki alanda saklanır.
// 4KB sayfalar için adres 12. bitten başlar, 2MB için 21. bitten, 1GB için 30. bitten.
const PHYSICAL_ADDRESS_MASK_4K: u64 = 0x000F_FFFF_FFFF_F000; // Bits 12-51
const PHYSICAL_ADDRESS_MASK_2M: u64 = 0x000F_FFFF_FFE0_0000; // Bits 21-51
const PHYSICAL_ADDRESS_MASK_1G: u64 = 0x000F_FFFF_C000_0000; // Bits 30-51


/// Sayfa Tablosu Girişi (Page Table Entry - PTE)
/// PML4E, PDPTE, PDE veya PTE olabilir. Hepsi 64 bittir.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    /// Belirtilen fiziksel adres ve bayraklarla yeni bir sayfa tablosu girişi oluşturur.
    /// `phys_addr`, ilgili sayfa boyutuna (4K, 2M, 1G) göre hizalanmış olmalıdır.
    pub fn new(phys_addr: PhysAddr, flags: PageFlags) -> Self {
        // Güvenlik notu: Bu fonksiyon, phys_addr'nin uygun maske ile birleştirilmesi gerektiğini bilir.
        // Ancak bu, bayrakların da dahil edildiği ham bir birleştirmedir.
        // physical_address() metodu maskeyi doğru uygulamalıdır.
        Self(phys_addr | flags.raw())
    }

    /// Yeni, boş (mevcut olmayan) bir giriş oluşturur.
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Girişte kodlanmış fiziksel adresi (bayraklardan arındırılmış) döndürür.
    /// `page_size`: Bu girişin işaret ettiği sayfanın beklenen boyutu (4K, 2M, 1G).
    /// Bu bilgi, adresin hangi maske ile çıkarılacağını belirlemek için kullanılır.
    pub fn physical_address(&self, page_size: u64) -> PhysAddr {
        match page_size {
            PAGE_SIZE_4K => self.0 & PHYSICAL_ADDRESS_MASK_4K,
            PAGE_SIZE_2M => {
                 // 2MB sayfa ise PS biti set olmalı.
                 if self.flags().contains(PageFlags::HUGE_PAGE) {
                    self.0 & PHYSICAL_ADDRESS_MASK_2M
                 } else {
                     0 // Veya hata? Geçersiz kullanım olduğunu varsayalım.
                 }
            },
             PAGE_SIZE_1G => {
                 // 1GB sayfa ise PS biti set olmalı.
                 if self.flags().contains(PageFlags::HUGE_PAGE) {
                    self.0 & PHYSICAL_ADDRESS_MASK_1G
                 } else {
                     0 // Veya hata? Geçersiz kullanım olduğunu varsayalım.
                 }
            },
            _ => 0, // Desteklenmeyen sayfa boyutu
        }
    }

    /// Girişte kodlanmış bayrakları döndürür.
    #[inline]
    pub fn flags(&self) -> PageFlags {
        PageFlags(self.0)
    }

    /// Girişin mevcut (Present) bayrağının set olup olmadığını kontrol eder.
    #[inline]
    pub fn is_present(&self) -> bool {
        self.flags().contains(PageFlags::PRESENT)
    }

    /// Girişin büyük sayfa (Huge Page - 2MB veya 1GB) bayrağının set olup olmadığını kontrol eder.
    #[inline]
    pub fn is_huge_page(&self) -> bool {
        self.flags().contains(PageFlags::HUGE_PAGE)
    }

    /// Girişin ham 64-bit değerini ayarlar.
    #[inline]
    pub fn set(&mut self, value: u64) {
        self.0 = value;
    }

     /// Girişin fiziksel adresini ve bayraklarını ayarlar.
    #[inline]
    pub fn set_entry(&mut self, phys_addr: PhysAddr, flags: PageFlags) {
        self.0 = phys_addr | flags.raw();
    }


    /// Girişin Present bayrağını temizleyerek geçersiz hale getirir.
    /// Diğer bayraklar korunabilir veya temizlenebilir. Basitçe 0'a eşitlemek yaygındır.
    #[inline]
    pub fn clear(&mut self) {
        self.0 = 0;
    }

    /// Girişin ham u64 değerini döndürür.
    #[inline]
    pub fn raw(&self) -> u64 {
        self.0
    }
}

// PageTableEntry için Debug trait implementasyonu, ham değeri kolayca görebilmek için.
impl fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PageTableEntry")
         .field(&format_args!("0x{:x}", self.0))
         .finish()
    }
}

// --- Fiziksel Frame Ayırıcı Trait'i ---
// MMU kodunun yeni sayfa tablosu sayfaları için fiziksel bellek talep etmesi gerekir.
// Bu trait, çekirdeğin fiziksel bellek yöneticisinin sağlaması gereken arayüzü tanımlar.
pub trait FrameAllocator {
    /// Tek bir fiziksel frame (genellikle 4KB) tahsis eder ve fiziksel adresini döndürür.
    /// Uygun frame yoksa `None` döner.
    fn allocate_frame(&mut self) -> Option<PhysAddr>;

    /// Daha önce tahsis edilmiş bir fiziksel frame'i iade eder.
    /// # Güvenlik (Safety)
    /// Çağıran, `frame_addr`'nin geçerli bir fiziksel adres olduğundan ve
    /// daha önce bu ayırıcıdan alındığından ve artık kullanımda olmadığından emin olmalıdır.
    unsafe fn deallocate_frame(&mut self, frame_addr: PhysAddr);
}

// --- Yer Tutucu Frame Ayırıcı Implementasyonu ---
// Gerçek bir çekirdekte, bu, bootloader'dan alınan bellek haritasına göre implemente edilmelidir.
struct PlaceholderFrameAllocator;

impl FrameAllocator for PlaceholderFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysAddr> {
        // Burası gerçek bir tahsis mantığı içermelidir.
        // Örneğin:
         static mut NEXT_FREE_FRAME: PhysAddr = 0x1000000; // Örnek başlangıç adresi
         static mut REMAINING_FRAMES: usize = 100; // Örnek kalan frame sayısı
         unsafe {
             if REMAINING_FRAMES > 0 {
                 let addr = NEXT_FREE_FRAME;
                 NEXT_FREE_FRAME += PAGE_SIZE_4K;
                 REMAINING_FRAMES -= 1;
                  println!("PlaceholderFrameAllocator: Allocated frame 0x{:x}", addr); // Çekirdek içi print!
                 Some(addr)
             } else {
                  println!("PlaceholderFrameAllocator: Out of frames"); // Çekirdek içi print!
                 None
             }
         }
        // Şimdilik her zaman None döndürerek bellek yokmuş gibi simüle edelim.
        None
    }

    unsafe fn deallocate_frame(&mut self, frame_addr: PhysAddr) {
         // Burası gerçek bir iade mantığı içermelidir.
          println!("PlaceholderFrameAllocator: Deallocated frame 0x{:x}", frame_addr); // Çekirdek içi print!
    }
}


// --- x86_64 MMU Yöneticisi ---
/// x86_64 sayfa tablolarını yönetmek için fonksiyonlar sağlar.
/// 64-bit long mode ve 4-level paging varsayar.
/// Çekirdeğin fiziksel belleğe doğrudan erişimi olduğunu varsayar (identity mapping gibi).
pub struct X86MmuManager {
    // MMU yöneticisi, fiziksel frame ayırıcısına bir referans veya sahip olabilir.
    // Genellikle kernel başlangıcında statik bir ayırıcı kurulur ve buraya geçirilir.
    // allocator: &'static mut dyn FrameAllocator, // Örnek alan
}

impl X86MmuManager {
    /// Yeni bir MMU yöneticisi instance'ı oluşturur.
    /// Kernel başlangıcında çağrılmalıdır. Fiziksel frame ayırıcısı burada veya
    /// init_manager'da ayarlanmalıdır.
    pub fn new(/* allocator: &'static mut dyn FrameAllocator */) -> Self {
        X86MmuManager {
            // allocator: allocator
        }
    }

    /// Sayfa tablosu hiyerarşisinde belirtilen sanal adrese karşılık gelen
    /// Sayfa Tablosu Girişini (PTE) bulmak için yürüyüş yapar.
    /// Bulunan PTE'ye mutable bir referans ve bulunduğu sayfa tablosu sayfasının fiziksel adresini döndürür.
    /// Sayfa haritalanmamışsa veya sayfa tabloları bozuksa `MmuError` döner.
    ///
    /// # Argümanlar
    /// * `page_table_root_phys`: Üst düzey PML4 sayfa tablosunun fiziksel adresi.
    /// * `virt_addr`: Çevrilecek sanal adres.
    /// * `allocator`: Yeni sayfa tablosu sayfaları oluşturmak için fiziksel frame ayırıcısı.
    /// * `create_intermediate`: Eksik ara düzey tabloları oluşturmalı mıyız?
    ///
    /// # Güvenlik (Safety)
    /// Çağıran, `page_table_root_phys`'nin geçerli, aktif PML4 sayfa tablosunun
    /// fiziksel adresi olduğundan emin olmalıdır. Sayfa tablosu girişlerine erişim,
    /// ham pointer dereference işlemi içerir ve doğası gereği güvensizdir.
    /// Çekirdeğin fiziksel belleğe doğrudan erişimi olduğunu varsayar (identity map vb.).
    unsafe fn walk_page_table_mut<'a>(
        &self, // Eğer allocator alan olarak tutuluyorsa &self yerine &mut self olabilir
        page_table_root_phys: PhysAddr,
        virt_addr: VirtAddr,
        allocator: &mut dyn FrameAllocator,
        create_intermediate: bool,
    ) -> Result<(&'a mut PageTableEntry, PhysAddr), MmuError> {

        // Sanal adresin farklı düzeylerdeki indekslerini çıkar
        let pml4_index = (virt_addr >> 39) & 0x1FF; // Bitler 39-47
        let pdpt_index = (virt_addr >> 30) & 0x1FF; // Bitler 30-38
        let pd_index = (virt_addr >> 21) & 0x1FF;   // Bitler 21-29
        let pt_index = (virt_addr >> 12) & 0x1FF;   // Bitler 12-20

        // PML4 tablosu ile başla
        let mut current_phys_table_addr = page_table_root_phys;

        // Sayfa tablosu düzeyleri ve ilgili indeksler
        let levels = [
            (pml4_index, "PML4", PAGE_SIZE_1G), // PML4 -> PDPT (işaret ettiği düzey) -> Page Size if huge
            (pdpt_index, "PDPT", PAGE_SIZE_2M), // PDPT -> PD (işaret ettiği düzey) -> Page Size if huge
            (pd_index, "PD", PAGE_SIZE_4K),     // PD -> PT (işaret ettiği düzey) -> Page Size if huge
            (pt_index, "PT", PAGE_SIZE_4K),     // PT -> 4KB Sayfa (işaret ettiği düzey) -> Page Size
        ];

        for (i, &(index, level_name, huge_page_size_at_level)) in levels.iter().enumerate() {
            // Mevcut tablodaki girişin fiziksel adresini hesapla
            let entry_phys_addr = current_phys_table_addr + (index * core::mem::size_of::<PageTableEntry>() as u64);

            // Girişe mutable bir referans al.
            // DİKKAT: Burası kritik güvenlik noktasıdır. physical_address'in
            // kernel'ın erişebileceği bir adrese işaret ettiğini varsayıyoruz.
            // identity mapping veya yüksek yarıda (higher-half) bir haritalama olmalı.
            let entry_ptr = entry_phys_addr as *mut PageTableEntry; // !!! Identity map veya eşdeğeri varsayılıyor !!!
            let entry = &mut *entry_ptr;

            if i < 3 { // Son düzey (PT) değilse
                if entry.is_present() {
                    // Giriş mevcut. Büyük sayfayı (2MB/1GB) kontrol et.
                    if entry.is_huge_page() {
                         // Eğer ara bir düzeyde (PML4, PDPT, PD) büyük sayfa girişi bulduysak,
                         // talep edilen adres bu büyük sayfanın bir parçasıdır.
                         // 4KB PTE'ye kadar yürüyemeyiz. Fonksiyonun amacına göre hata dönebiliriz.
                         // Bu fonksiyon 4K PTE'yi bulmak veya ara tabloları oluşturmak için.
                        // return Err(MmuError::AlreadyMapped); // Veya başka bir hata kodu
                         // Eğer translate_address gibi sadece çeviri yapıyorsak, burada işlemi tamamlayıp adresi hesaplarız.
                         // Map/Unmap için burası bir hata durumudur çünkü 4K sayfa bekliyoruz.
                         if create_intermediate { // Sadece Map/Unmap durumunda bu hatayı ver
                              return Err(MmuError::AlreadyMapped);
                         } else { // Sadece çeviri (translate) durumunda, büyük sayfa adresini hesaplayıp dön.
                             let huge_page_phys_base = entry.physical_address(huge_page_size_at_level);
                             let offset_within_huge_page = virt_addr & (huge_page_size_at_level - 1);
                             // Bu senaryo translate_address fonksiyonuna daha uygun.
                             // walk_page_table_mut genelde 4K PTE'ye ulaşmak için kullanılır.
                             // Bu yüzden burada hata dönmek map/unmap için daha doğru.
                              return Err(MmuError::NotSupported); // Büyük sayfa varken 4K haritalama/kaldırma yapılamaz
                         }
                    }
                    // Sonraki düzeyin sayfa tablosuna geç. Adres mevcut girişte saklıdır.
                    // Ara tablolar her zaman 4K hizalıdır.
                    current_phys_table_addr = entry.physical_address(PAGE_SIZE_4K);

                } else {
                    // Ara tablo mevcut değil
                    if create_intermediate {
                        // Sonraki düzey için yeni bir fiziksel frame (sayfa tablosu sayfası) tahsis et.
                        let new_table_phys_addr = allocator.allocate_frame()
                            .ok_or(MmuError::OutOfMemory)?;

                        // Yeni sayfa tablosu sayfasını sıfırla (ÇOK ÖNEMLİ!).
                        // Aksi takdirde rastgele değerler güvenlik açığı oluşturabilir.
                        // Fiziksel adresi kernel sanal adresine çevirmek gerekir, identity map varsayalım.
                        ptr::write_bytes(new_table_phys_addr as *mut u8, 0, PAGE_SIZE_4K as usize); // !!! Identity map varsayılıyor !!!

                        // Yeni tabloya işaret eden bir giriş oluştur. Present ve Writable bayrakları set olmalı.
                        // Kullanıcı alanı sayfa tabloları için User Accessible da set olmalı.
                        let new_entry = PageTableEntry::new(
                            new_table_phys_addr,
                            PageFlags::PRESENT.with(PageFlags::WRITABLE).with(PageFlags::USER_ACCESSIBLE)
                        );
                        entry.set(new_entry.raw());
                         println!("MMU: Created new {} table at 0x{:x}", level_name, new_table_phys_addr); // Çekirdek içi print!

                        // Yeni oluşturulan tabloya geç
                        current_phys_table_addr = new_table_phys_addr;

                    } else {
                        // Tablo eksik ve oluşturmaya izin yok (örn: translate işlemi)
                        return Err(MmuError::MissingPageTable);
                    }
                }
            } else { // Bu en son düzey (PT)
                 // Hedef PTE'yi bulduk. Mutable referansını ve bulunduğu sayfa tablosunun fiziksel adresini döndür.
                 return Ok((entry, current_phys_table_addr));
            }
        }

         // Teorik olarak bu noktaya gelinmemeli (4 düzey yürüdükten sonra).
         Err(MmuError::InternalError) // Daha spesifik bir hata
    }

    /// Sanal adresi belirtilen fiziksel adrese ve bayraklara göre haritalar (mapping).
    /// Eksik ara düzey sayfa tabloları varsa oluşturur.
    /// `virt_addr` ve `phys_addr` 4KB sayfa hizalı olmalıdır.
    ///
    /// # Argümanlar
    /// * `page_table_root_phys`: Üst düzey PML4 tablosunun fiziksel adresi (genellikle aktif görevinki).
    /// * `virt_addr`: Haritalanacak sanal adres (4KB hizalı).
    /// * `phys_addr`: `virt_addr`'in haritalanacağı fiziksel adres (4KB hizalı).
    /// * `flags`: Haritalama için kullanılacak sayfa bayrakları (Present otomatik eklenir).
    /// * `allocator`: Yeni sayfa tablosu sayfalarını tahsis etmek için fiziksel frame ayırıcısı.
    ///
    /// # Güvenlik (Safety)
    /// Çağıran, `page_table_root_phys`'nin geçerli, aktif PML4 tablosunun fiziksel adresi olduğundan emin olmalıdır.
    /// Bu fonksiyon sayfa tablolarını değiştirir ve sistemin bellek haritasını etkiler.
    pub unsafe fn map_page(
        &self,
        page_table_root_phys: PhysAddr,
        virt_addr: VirtAddr,
        phys_addr: PhysAddr,
        flags: PageFlags,
        allocator: &mut dyn FrameAllocator,
    ) -> Result<(), MmuError> {
        // Temel adres hizalama kontrolleri
        if virt_addr % PAGE_SIZE_4K != 0 || phys_addr % PAGE_SIZE_4K != 0 {
             println!("MMU: Map error - Invalid alignment V:0x{:x}, P:0x{:x}", virt_addr, phys_addr); // Çekirdek içi print!
            return Err(MmuError::InvalidArgument);
        }

        // Sayfa tablosu yürüyüşünü yap, gerekirse ara tabloları oluştur, 4K PTE'yi bul.
        let (pte, _pt_phys_addr) = self.walk_page_table_mut(
            page_table_root_phys,
            virt_addr,
            allocator,
            true, // Ara tabloları oluştur
        )?;

        // Hedef PTE'nin zaten mevcut olup olmadığını kontrol et (zaten haritalı mı?)
        if pte.is_present() {
             println!("MMU: Map error - Already mapped V:0x{:x}", virt_addr); // Çekirdek içi print!
            return Err(MmuError::AlreadyMapped);
        }

        // Yeni PTE değerini oluştur (Present bayrağını otomatik ekle)
        let new_pte = PageTableEntry::new(phys_addr, flags.with(PageFlags::PRESENT));

        // Yeni PTE'yi sayfa tablosuna yaz
        pte.set(new_pte.raw());
         println!("MMU: Mapped V:0x{:x} to P:0x{:x} with flags 0x{:x}", virt_addr, phys_addr, flags.raw()); // Çekirdek içi print!


        // TODO: Mevcut core'da bu sanal adres için TLB'yi invalidate et (invlpg talimatı veya eşdeğeri)
        // Bu, CPU'nun önbelleğe alınmış çeviri bilgisini atmasını sağlar.
         core::arch::x86_64::instructions::tlb::flush(virt_addr as usize); // `x86_64` crate veya intrinsik gerektirir

        Ok(())
    }

    /// Belirtilen sanal adresin haritalamasını kaldırır (unmapping).
    /// Eğer sayfa tablosu sayfası boş kalırsa (opsiyonel olarak) frame'i iade eder.
    /// `virt_addr` 4KB sayfa hizalı olmalıdır.
    ///
    /// # Argümanlar
    /// * `page_table_root_phys`: Üst düzey PML4 tablosunun fiziksel adresi.
    /// * `virt_addr`: Haritalaması kaldırılacak sanal adres (4KB hizalı).
    /// * `allocator`: Sayfa tablosu sayfalarını iade etmek için fiziksel frame ayırıcısı.
    ///
    /// # Güvenlik (Safety)
    /// Çağıran, `page_table_root_phys`'nin geçerli, aktif PML4 tablosunun fiziksel adresi olduğundan emin olmalıdır.
    /// Bu fonksiyon sayfa tablolarını değiştirir. Halen kullanımda olan belleğin haritalamasını kaldırmak çökmeye neden olur.
    pub unsafe fn unmap_page(
        &self, // Eğer allocator alan olarak tutuluyorsa &mut self olabilir
        page_table_root_phys: PhysAddr,
        virt_addr: VirtAddr,
        allocator: &mut dyn FrameAllocator,
    ) -> Result<(), MmuError> {
         // Temel adres hizalama kontrolü
        if virt_addr % PAGE_SIZE_4K != 0 {
             println!("MMU: Unmap error - Invalid alignment V:0x{:x}", virt_addr); // Çekirdek içi print!
            return Err(MmuError::InvalidArgument);
        }

        // Sayfa tablosu yürüyüşünü yap, ara tablo oluşturma, 4K PTE'yi bul.
        let (pte, pt_phys_addr) = self.walk_page_table_mut(
            page_table_root_phys,
            virt_addr,
            allocator, // Deallocation için hala gerekebilir
            false, // Ara tabloları OLUŞTURMA
        )?;

        // Sayfanın gerçekten haritalı olup olmadığını kontrol et
        if !pte.is_present() {
             println!("MMU: Unmap error - Not mapped V:0x{:x}", virt_addr); // Çekirdek içi print!
            return Err(MmuError::NotMapped);
        }

        // PTE'yi temizle (Present bayrağını kaldır)
        pte.clear();
          println!("MMU: Unmapped V:0x{:x}", virt_addr); // Çekirdek içi print!

        // TODO: Mevcut core'da bu sanal adres için TLB'yi invalidate et.
          core::arch::x86_64::instructions::tlb::flush(virt_addr as usize); // `x86_64` crate veya intrinsik gerektirir

        // TODO: Sayfa tablosu sayfalarının iadesi (opsiyonel ve karmaşık)
        // Eğer bir sayfa tablosu sayfası (PT, PD, PDPT) haritalaması kaldırılan son
        // girdiye sahiptiyse ve tamamen boşaldıysa, bu fiziksel frame iade edilebilir.
        // Bu, yürüyüşü geri takip etmeyi ve her düzeyde tablonun boş olup olmadığını
        // kontrol etmeyi gerektirir. Basitlik için şimdilik bu adımı atlayabiliriz.
        // Implement edilirse, `allocator.deallocate_frame(tablo_phys_addr)` çağrılmalıdır.

        Ok(())
    }

     /// Sanal adresi karşılık gelen fiziksel adrese çevirir (translate).
     /// Haritalıysa `Ok(PhysAddr)` döner, haritalı değilse veya hata oluşursa `Err(MmuError)` döner.
     /// Büyük sayfa haritalamalarını da doğru şekilde ele almalıdır.
     ///
     /// # Güvenlik (Safety)
     /// Çağıran, `page_table_root_phys`'nin geçerli, aktif PML4 tablosunun fiziksel adresi olduğundan emin olmalıdır.
     /// Sayfa tablosu girişlerini okumak ham pointer dereference işlemi içerir ve güvensizdir.
     /// Çekirdeğin fiziksel belleğe doğrudan erişimi olduğunu varsayar.
    pub unsafe fn translate_address(
        &self,
        page_table_root_phys: PhysAddr,
        virt_addr: VirtAddr,
    ) -> Result<PhysAddr, MmuError> {
        // Bu, `walk_page_table_mut`'a benzer ama sadece okuma yapar ve büyük sayfaları hedefler.

        let pml4_index = (virt_addr >> 39) & 0x1FF;
        let pdpt_index = (virt_addr >> 30) & 0x1FF;
        let pd_index = (virt_addr >> 21) & 0x1FF;
        let pt_index = (virt_addr >> 12) & 0x1FF;

        let mut current_phys_table_addr = page_table_root_phys;

        // İndeksler ve o düzeyde karşılaşılabilecek büyük sayfa boyutları
        let levels = [
            (pml4_index, PAGE_SIZE_1G), // PML4 -> PDPT, ama PML4E 1GB sayfayı işaret edebilir
            (pdpt_index, PAGE_SIZE_2M), // PDPT -> PD, ama PDPTE 2MB sayfayı işaret edebilir
            (pd_index, PAGE_SIZE_4K),     // PD -> PT, ama PDE 2MB sayfayı işaret edebilir (HUGE_PAGE)
            (pt_index, PAGE_SIZE_4K),     // PT -> 4KB Sayfa
        ];


        for (i, &(index, possible_huge_page_size)) in levels.iter().enumerate() {

            let entry_phys_addr = current_phys_table_addr + (index * core::mem::size_of::<PageTableEntry>() as u64);
            let entry_ptr = entry_phys_addr as *const PageTableEntry; // Okuma için const pointer kullan
            let entry = &*entry_ptr;

            if !entry.is_present() {
                 println!("MMU: Translate error - Not present at level {}", i); // Çekirdek içi print!
                return Err(MmuError::NotMapped); // Sayfa veya ara tablo mevcut değil
            }

            if i < 3 { // Son düzey (PT) değilse
                if entry.is_huge_page() {
                    // Ara düzeyde büyük sayfa bulduk (2MB veya 1GB). Adres bu büyük sayfanın içinde.
                    let huge_page_phys_base = entry.physical_address(possible_huge_page_size);
                    let offset_within_huge_page = virt_addr & (possible_huge_page_size - 1);
                     // println!("MMU: Translated V:0x{:x} to P:0x{:x} (Huge Page)", virt_addr, huge_page_phys_base + offset_within_huge_page); // Çekirdek içi print!
                    return Ok(huge_page_phys_base + offset_within_huge_page);
                } else {
                    // Sonraki düzeye geç. Adres bir sonraki tablonun fiziksel adresi.
                     current_phys_table_addr = entry.physical_address(PAGE_SIZE_4K); // Ara tablo adresleri 4K hizalıdır
                }
            } else { // En son düzey (PT)
                // 4KB PTE'yi bulduk. Son fiziksel adresi hesapla.
                let page_phys_base = entry.physical_address(PAGE_SIZE_4K);
                let offset_within_page = virt_addr & (PAGE_SIZE_4K - 1);
                  println!("MMU: Translated V:0x{:x} to P:0x{:x} (4K Page)", virt_addr, page_phys_base + offset_within_page); // Çekirdek içi print!
                return Ok(page_phys_base + offset_within_page);
            }
        }

        // Teorik olarak buraya gelinmemeli
         Err(MmuError::InternalError)
    }

    // --- Ek MMU Fonksiyonları (TODO) ---
    // İhtiyaç duyuldukça eklenecek fonksiyonlar:

    /// Yeni, boş bir sayfa tablosu hiyerarşisi oluşturur (yeni bir PML4 tablosu tahsis eder).
    /// Görev (Task) oluşturulurken kendi adres alanını tanımlamak için kullanılır.
    /// # Güvenlik (Safety)
    /// Çağıran, dönen fiziksel adresin geçerli ve güvenli olduğundan emin olmalıdır.
    pub unsafe fn create_new_page_table(&self, allocator: &mut dyn FrameAllocator) -> Result<PhysAddr, MmuError> {
        // Yeni bir PML4 fiziksel frame tahsis et
        let pml4_phys_addr = allocator.allocate_frame().ok_or(MmuError::OutOfMemory)?;

        // PML4 tablosu sayfasını sıfırla
        ptr::write_bytes(pml4_phys_addr as *mut u8, 0, PAGE_SIZE_4K as usize); // Identity map varsayılıyor

        // TODO: Kernel'ın kendi adres alanını (yüksek yarısı) bu yeni PML4 tablosuna kopyala veya haritala.
        // Kernel, her görevde aynı yüksek yarıya (çekirdek belleği) erişebilmelidir.
        // Bu, kernel başlangıcında kurulan 'reference' kernel page table'dan
        // uygun PML4E girdisini (genellikle index 511 veya üstü) yeni tabloya kopyalamayı gerektirir.
         let kernel_pml4e = get_kernel_pml4e(); // Placeholder
         let new_pml4_table_ptr = pml4_phys_addr as *mut PageTableEntry;
         (*new_pml4_table_ptr.add(511)).set(kernel_pml4e.raw());

        Ok(pml4_phys_addr)
    }

    /// Belirtilen sanal adres aralığını haritalar. Sayfa bazında `map_page` çağrısı yapar.
    /// # Güvenlik (Safety)
    /// Belirtilen aralığın ve bayrakların geçerli ve güvenli olduğundan emin olunmalıdır.
    pub unsafe fn map_range(
        &self,
        page_table_root_phys: PhysAddr,
        virt_start: VirtAddr,
        phys_start: PhysAddr,
        size: usize,
        flags: PageFlags,
        allocator: &mut dyn FrameAllocator,
    ) -> Result<(), MmuError> {
        let mut current_virt = virt_start;
        let mut current_phys = phys_start;
        let size_pages = (size + PAGE_SIZE_4K as usize - 1) / PAGE_SIZE_4K as usize;

        for _i in 0..size_pages {
            self.map_page(
                page_table_root_phys,
                current_virt,
                current_phys,
                flags,
                allocator,
            )?; // Hata durumunda işlemi durdur

            current_virt += PAGE_SIZE_4K;
            current_phys += PAGE_SIZE_4K;
        }
        Ok(())
    }

     /// Belirtilen sanal adres aralığının haritalamasını kaldırır. Sayfa bazında `unmap_page` çağrısı yapar.
      /// # Güvenlik (Safety)
     /// Belirtilen aralığın geçerli ve güvenli olduğundan emin olunmalıdır.
     pub unsafe fn unmap_range(
        &self,
        page_table_root_phys: PhysAddr,
        virt_start: VirtAddr,
        size: usize,
        allocator: &mut dyn FrameAllocator,
    ) -> Result<(), MmuError> {
        let mut current_virt = virt_start;
        let size_pages = (size + PAGE_SIZE_4K as usize - 1) / PAGE_SIZE_4K as usize;

        for _i in 0..size_pages {
            self.unmap_page(
                page_table_root_phys,
                current_virt,
                allocator,
            )?; // Hata durumunda işlemi durdur

            current_virt += PAGE_SIZE_4K;
        }
        Ok(())
     }


    /// Aktif sayfa tablosunu değiştirir (CR3 register'ına yazar).
    /// # Güvenlik (Safety)
    /// Bu, tüm adres alanı haritalamasını değiştirir. Çağıran, yeni PML4'ün geçerli ve
    /// güvenli bir sayfa tablosuna işaret ettiğinden emin olmalıdır. Çekirdek kodunun
    /// yeni haritada erişilebilir olması KRİTİKTİR!
    pub unsafe fn switch_page_table(&self, pml4_phys_addr: PhysAddr) {
        // CR3 register'ına yeni PML4'ün fiziksel adresini yaz.
        // Bu işlem mimariye özgüdür. `x86_64` crate veya intrinsik kullanın.
        // `x86_64` crate kullanılıyorsa:
         use x86_64::PhysAddr;
         use x86_64::registers::control::{Cr3, Cr3Flags};
         Cr3::write(PhysAddr::new(pml4_phys_addr), Cr3Flags::empty());

        // Basitlik için doğrudan bellek yazma (CR3 adresi bilinmiyorsa bu sadece bir yer tutucudur!)
        // Gerçek adresi bulmak için x86_64 mimarisi dokümantasyonuna veya crate'lere bakılmalıdır.
         ptr::write_volatile(0xFFFFFFFF_80000000 as *mut u64, pml4_phys_addr); // Örnek/yer tutucu adres, DOĞRU ADRES DEĞİL!

        // Doğrudan assembly kullanmak gerekebilir. Veya `x86_64` crate'indeki safe olmayan fonk.
         asm!("mov %cr3, {}", in(reg) pml4_phys_addr, options(nostack, nomem)); // Örnek inline assembly


         println!("MMU: Switched to page table at 0x{:x}", pml4_phys_addr); // Çekirdek içi print!
    }

    // TODO: Sayfa tablosu hiyerarşisini yok etme (tüm sayfaları iade etme).
    // TODO: Sayfa tablosu hiyerarşisini kopyalama (fork için).
    // TODO: Büyük sayfalar (2MB/1GB) için haritalama/kaldırma fonksiyonları (şu anki map_page sadece 4K'yı hedefliyor).
    // TODO: TLB yönetimi fonksiyonları (global flush, specific address flush).
}


// --- Kavramsal Entegrasyon Notları (kmemory modülü içinde) ---

// karnal64.rs dosyası içindeki kmemory modülü şu şekilde srcmmu_x86'ı kullanabilir:

mod kmemory {
    use super::*; // karnal64.rs kapsamındaki tipleri kullan (KError, KHandle vb.)
    use crate::srcmmu_x86::{X86MmuManager, FrameAllocator, PhysAddr, VirtAddr, PageFlags, MmuError}; // srcmmu_x86.rs'dan import

    // Kernel'ın global fiziksel frame ayırıcısı (başlangıçta initialize edilmeli)
    static mut GLOBAL_FRAME_ALLOCATOR: Option<MyKernelFrameAllocatorImpl> = None; // Çekirdeğinizin gerçek ayırıcısı
    // Kernel'ın MMU yöneticisi instance'ı
    static mut X86_MMU_MANAGER: Option<X86MmuManager> = None;

    // Başlangıçta çağrılan fonksiyon
    pub fn init_manager(/* boot_info: &BootInfo */) { // Boot bilgisini alarak ayırıcıyı başlatabiliriz
        unsafe {
            // TODO: GLOBAL_FRAME_ALLOCATOR'ı boot_info'ya göre başlatın
             GLOBAL_FRAME_ALLOCATOR = Some(MyKernelFrameAllocatorImpl::new(boot_info.memory_map));

            // MMU yöneticisini başlatın
            X86_MMU_MANAGER = Some(X86MmuManager::new());

            // TODO: Kernel'ın kendi adres alanını ve ilk görev'in (örneğin bootstrapper) adres alanını kurun.
            // Bu, `create_new_page_table` ve `map_range` fonksiyonlarını kullanır.
             let kernel_pml4_phys = setup_initial_kernel_mapping(GLOBAL_FRAME_ALLOCATOR.as_mut().unwrap());
             X86_MMU_MANAGER.as_ref().unwrap().switch_page_table(kernel_pml4_phys); // Çekirdek sayfalarına geçiş
             // Kaydedin: struct Task { pml4_phys: PhysAddr, ... }

             // TODO: İlk kullanıcı alanı görevini (örn: init prosesi) başlatın
             // Yeni bir sayfa tablosu oluşturun, kodunu haritalayın, yığınını haritalayın vb.
              let init_task_pml4_phys = X86_MMU_MANAGER.as_ref().unwrap().create_new_page_table(GLOBAL_FRAME_ALLOCATOR.as_mut().unwrap()).expect("Failed to create init task page table");
             // Map init code, stack etc. into init_task_pml4_phys...

        }
         println!("Karnal64: Bellek Yöneticisi Başlatıldı (x86_64 MMU entegrasyonu bekleniyor)"); // Çekirdek içi print!
    }

    // Karnal64 API fonksiyonlarının implementasyonu (srcmmu_x86'ı kullanarak)

    // Örnek: Kullanıcı alanı bellek tahsisi (bir görev için sanal alan bulma ve fiziksel frame haritalama)
    pub fn memory_allocate(size: usize) -> Result<*mut u8, KError> {
        // TODO: Mevcut görevin adres alanında yeterli boyutta boş sanal adres aralığı bul.
        // TODO: İstenen `size` kadar fiziksel frame tahsis et (GLOBAL_FRAME_ALLOCATOR kullanarak).
        // TODO: Bulunan sanal aralığı tahsis edilen fiziksel frame'lere haritala (`X86_MMU_MANAGER.map_range` kullanarak).
        // TODO: Başarı durumunda tahsis edilen sanal adresin başlangıcını *kullanıcı alanına ait* pointer olarak döndür.

        // Yer tutucu: Her zaman hata dön
         Err(KError::NotSupported) // Implemente edilmediği için
    }

    // Örnek: Haritalanmış paylaşımlı belleği kaldırma (unmap)
    pub fn shared_mem_unmap(ptr: *mut u8, size: usize) -> Result<(), KError> {
        // TODO: `ptr`'nin geçerli bir kullanıcı alanı pointer'ı ve görev'in adres alanında olduğunu doğrula.
        // TODO: Mevcut görevin PML4 fiziksel adresini al.
        let mmu = unsafe { X86_MMU_MANAGER.as_ref().ok_or(KError::InternalError)? };
        let allocator = unsafe { GLOBAL_FRAME_ALLOCATOR.as_mut().ok_or(KError::InternalError)? };

        // Assuming we have the current task's PML4 physical address
         let current_task_pml4_phys = get_current_task_pml4_phys(); // TODO: Task yöneticisinden al

        unsafe {
              mmu.unmap_range(current_task_pml4_phys, ptr as VirtAddr, size, allocator)
                 .map_err(|mmu_err| map_mmu_error(mmu_err)) // MMU hatasını KError'a çevir
        }
         Err(KError::NotSupported) // Yer tutucu
    }


    // MMU Hatalarını Karnal64 Hatalarına Çeviren Yardımcı Fonksiyon
    fn map_mmu_error(mmu_err: MmuError) -> KError {
        match mmu_err {
            MmuError::OutOfMemory => KError::OutOfMemory,
            MmuError::InvalidArgument => KError::InvalidArgument,
            MmuError::AlreadyMapped => KError::AlreadyExists, // Zaten haritalı -> Zaten mevcut gibi düşünülebilir
            MmuError::NotMapped => KError::NotFound, // Haritalı değil -> Bulunamadı gibi düşünülebilir
            _ => KError::InternalError, // Diğer MMU hatalarını dahili hata olarak raporla
        }
    }

    // Diğer kmemory API fonksiyonlarını implemente edin...
    // memory_release, shared_mem_create, shared_mem_map vb.
    // Hepsi X86MmuManager'ı ve GLOBAL_FRAME_ALLOCATOR'ı kullanmalıdır.

}
