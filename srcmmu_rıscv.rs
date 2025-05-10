#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz

// Karnal64 çekirdek tiplerini ve potansiyel KError'u kullanacağız
// (karnal64.rs dosyası içinde tanımlanmış olmaları gerekir)
// use crate::karnal64::{KError, KHandle}; // Eğer karnal64 modülü crate kökünde ise
use super::karnal64::{KError, KHandle}; // Eğer karnal64 modülü super modülde ise (yaygın kernel yapısı)

// --- RISC-V MMU Sabitleri ---

/// Sayfa boyutu (4KB)
const PAGE_SIZE: usize = 4096;
/// Sayfa boyutu bit kaydırma değeri
const PAGE_SHIFT: usize = 12;

/// Sanal adresin sayfa ofseti (VA[11:0])
const VA_OFFSET_BITS: usize = 12;
/// Sanal adresin 1. seviye dizini (VA[21:12])
const VA_VPN0_BITS: usize = 10;
/// Sanal adresin 2. seviye dizini (VA[30:22])
const VA_VPN1_BITS: usize = 10;
/// Sanal adresin 3. seviye dizini (VA[38:31]) - Sv39 için
const VA_VPN2_BITS: usize = 9;

/// Sayfa tablosu girişlerinin bir tablodaki sayısı (PAGE_SIZE / 8 byte/PTE)
const ENTRIES_PER_PAGE_TABLE: usize = PAGE_SIZE / core::mem::size_of::<PageTableEntry>(); // 512

/// Sayfa Tablosu Girişi (PTE) bayrakları
#[repr(u64)]
#[allow(dead_code)] // Bazı bayraklar şimdilik kullanılmayabilir
enum PteFlags {
    /// Geçerli (Valid)
    V = 1 << 0,
    /// Okunabilir (Readable)
    R = 1 << 1,
    /// Yazılabilir (Writable)
    W = 1 << 2,
    /// Çalıştırılabilir (Executable)
    X = 1 << 3,
    /// Kullanıcı modu erişimi (User)
    U = 1 << 4,
    /// Global sayfa (Global)
    G = 1 << 5,
    /// Erişilmiş (Accessed)
    A = 1 << 6,
    /// Kirli (Dirty)
    D = 1 << 7,
}

impl PteFlags {
    /// Bayrakları birleştirme
    fn from_bits(bits: u64) -> Self {
        unsafe { core::mem::transmute(bits) }
    }

    /// Bayrakları u64'e dönüştürme
    fn bits(&self) -> u64 {
        *self as u64
    }
}


// --- Sayfa Tablosu Yapıları ---

/// Sayfa Tablosu Girişi (Page Table Entry - PTE)
/// Sv39 modunda 64 bit (8 byte)
#[repr(C)] // C uyumlu bellek yerleşimi
#[derive(Copy, Clone)]
struct PageTableEntry(u64);

impl PageTableEntry {
    /// Boş (geçersiz) PTE oluşturur.
    pub fn empty() -> Self {
        PageTableEntry(0)
    }

    /// PTE'nin geçerli olup olmadığını kontrol eder.
    pub fn is_valid(&self) -> bool {
        (self.0 & PteFlags::V.bits()) != 0
    }

    /// PTE'nin yaprak (leaf - yani fiziksel sayfaya işaret eden) olup olmadığını kontrol eder.
    /// V bit'i set edilmiş ve R, W, X bitlerinden en az biri set edilmişse yapraktır.
    pub fn is_leaf(&self) -> bool {
        self.is_valid() && (self.0 & (PteFlags::R.bits() | PteFlags::W.bits() | PteFlags::X.bits())) != 0
    }

    /// PTE'nin bir sonraki seviye sayfaya işaret edip etmediğini kontrol eder (yaprak değilse).
    pub fn is_table(&self) -> bool {
        self.is_valid() && !self.is_leaf()
    }

    /// PTE'deki fiziksel sayfa numarası (PPN) alanını alır.
    /// Sv39'da PPN[2:0] PTE[19:10], PPN[1:0] PTE[28:19], PPN[0] PTE[53:28]
    /// Bu fonksiyon, yaprak olmayan PTE'ler için bir sonraki sayfa tablosunun fiziksel adresini döndürür.
    /// Yaprak PTE'ler için haritalanan fiziksel sayfanın adresini döndürür (yalnızca sayfa hizalı kısmı).
    pub fn ppn(&self) -> u64 {
        // Sv39 formatında PPN alanını maskele ve sağa kaydır.
         PTE[53:10] -> PPN[43:0]
        (self.0 >> 10) & 0xFF_FFFF_FFFF // 44 bitlik PPN
    }

    /// PTE'deki bayrakları alır.
    pub fn flags(&self) -> PteFlags {
        PteFlags::from_bits(self.0 & 0xFF) // İlk 8 bit bayraklar
    }

    /// Fiziksel sayfa numarasını (PPN) ve bayrakları kullanarak bir PTE oluşturur.
    /// `ppn`: Fiziksel sayfa numarasının değeri (sayfa hizalı fiziksel adres / PAGE_SIZE).
    /// `flags`: PTE bayrakları (V, R, W, X, U vb.).
    pub fn new(ppn: u64, flags: u64) -> Self {
        PageTableEntry((ppn << 10) | flags)
    }

    // TODO: A ve D bitlerini set etmek/temizlemek için metotlar.
}

/// Sayfa Tablosu
/// Sayfa tablosu girişlerinden (PTE) oluşan bir dizidir.
#[repr(C, align(4096))] // Sayfa hizalı olmalı
struct PageTable {
    entries: [PageTableEntry; ENTRIES_PER_PAGE_TABLE],
}

impl PageTable {
    /// Boş (tüm girişleri geçersiz) bir sayfa tablosu oluşturur.
    pub fn new() -> Self {
        PageTable {
            entries: [PageTableEntry::empty(); ENTRIES_PER_PAGE_TABLE],
        }
    }

    /// Verilen dizindeki PTE'ye erişim.
    pub fn entry(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }
}


// --- Fiziksel Sayfa Çerçeve Yöneticisi (Varsayımsal) ---
// Gerçekte başka bir modülde implemente edilmesi gereken arayüz.
// Basitlik adına burada sadece fonksiyon imzaları yer alıyor.

/// Fiziksel bir sayfa çerçevesi (4KB) tahsis eder.
/// Başarı durumunda tahsis edilen çerçevenin fiziksel adresini (sayfa hizalı),
/// hata durumunda KError döndürür.
fn allocate_physical_frame() -> Result<u64, KError> {
    // TODO: Fiziksel bellek havuzundan boş bir sayfa çerçevesi bul ve işaretle
    // Yer tutucu: Başarı durumunda 0x81000000 gibi dummy bir adres döndürsün
    // (gerçek kernelde buradan bellek altyapısı devreye girer)
    Err(KError::OutOfMemory) // Placeholder: Henüz implemente edilmedi
}

/// Fiziksel bir sayfa çerçevesini serbest bırakır.
/// `paddr`: Serbest bırakılacak sayfa çerçevesinin fiziksel adresi (sayfa hizalı).
fn free_physical_frame(paddr: u64) {
    // TODO: Fiziksel bellek havuzunda ilgili çerçeveyi boş olarak işaretle
    // Yer tutucu: Hiçbir şey yapmıyor
}


// --- RISC-V MMU Yöneticisi ---

/// RISC-V Bellek Yönetim Birimi Yöneticisi
/// Sayfa tablolarını ve adres alanlarını yönetir.
pub struct RiscvMemoryManager {
    // Kernel sayfa tablosunun fiziksel adresi (SATP kaydına yüklenecek değer)
    // Genellikle bir statik değişken veya singletion içinde tutulur.
    kernel_page_table_paddr: u64,
    // TODO: Aktif görevlerin sayfa tablolarına (adres alanlarına) referanslar.
    // Örneğin, bir KTaskId'den kök sayfa tablosu fiziksel adresine harita.
    // `no_std` uyumlu bir Map yapısı gerektirir.
}

impl RiscvMemoryManager {
    /// MMU Yöneticisini başlatır.
    /// Çekirdek sayfa tablosunu oluşturur ve gerekli başlangıç haritalamalarını yapar.
    /// `kernel_start_paddr`: Çekirdeğin fiziksel başlangıç adresi
    /// `kernel_end_paddr`: Çekirdeğin fiziksel bitiş adresi
    /// `physical_memory_end`: Kullanılabilir fiziksel belleğin son adresi
    pub fn init(kernel_start_paddr: u64, kernel_end_paddr: u64, physical_memory_end: u64) -> Result<Self, KError> {
        // 1. Çekirdek için yeni bir kök sayfa tablosu tahsis et
        let kernel_root_pt_paddr = allocate_physical_frame()?;
        let kernel_root_pt = unsafe {
            // Tahsis edilen fiziksel adresi sanal bellekte bir PageTable yapısı olarak kullan.
            // Burada kimlik haritalama (identity mapping) veya çekirdeğin başlangıçta
            // statik olarak haritalanmış olması varsayılıyor.
            // DİKKAT: Burası kritik bir varsayım, çekirdek kendi sayfa tablosunu
            // oluştururken nasıl belleğe erişecek? Genellikle bootloader
            // başlangıçta çekirdeğin ihtiyacı olan basic haritalamayı yapar.
            &mut *(kernel_root_pt_paddr as *mut PageTable)
        };
        // Sayfa tablosunu sıfırla (tüm girişler geçersiz)
        unsafe { core::ptr::write_bytes(kernel_root_pt_paddr as *mut u8, 0, PAGE_SIZE) };


        // 2. Çekirdek kod/veri/yığın alanını kimlik haritala (identity map) veya uygun sanal adrese haritala.
        // Basitlik için kimlik haritalama (sanal adres == fiziksel adres) varsayalım.
        // Gerçekte çekirdek sanal adresi genellikle 0x80000000 gibi yüksek adreslerden başlar.
        let kernel_mapping_start_vaddr = kernel_start_paddr; // Identity mapping varsayımı
        let kernel_mapping_end_vaddr = kernel_end_paddr; // Identity mapping varsayımı
        let kernel_mapping_size = kernel_end_paddr - kernel_start_paddr;

        // Çekirdek segmentleri için sayfa tablosu girdilerini oluştur
        let mut current_paddr = kernel_start_paddr;
        let mut current_vaddr = kernel_mapping_start_vaddr;

        // Çekirdek metin (kod) bölümü için RX haritalama (varsayımsal)
        let text_segment_end = kernel_start_paddr + 0x100000; // Örnek: ilk 1MB kod olsun
        while current_vaddr < text_segment_end {
             Self::map_page_in_table(
                kernel_root_pt_paddr, // Kök sayfa tablosunun fiziksel adresi
                current_vaddr,
                current_paddr,
                PteFlags::V.bits() | PteFlags::R.bits() | PteFlags::X.bits(), // Valid, Read, Execute
                true // Kernel sayfası
            )?;
            current_vaddr += PAGE_SIZE as u64;
            current_paddr += PAGE_SIZE as u64;
        }

        // Çekirdek veri bölümü için RW haritalama (varsayımsal)
         let data_segment_end = kernel_end_paddr; // Kalan kısım veri/yığın olsun
         while current_vaddr < data_segment_end {
             Self::map_page_in_table(
                kernel_root_pt_paddr, // Kök sayfa tablosunun fiziksel adresi
                current_vaddr,
                current_paddr,
                PteFlags::V.bits() | PteFlags::R.bits() | PteFlags::W.bits(), // Valid, Read, Write
                true // Kernel sayfası
            )?;
            current_vaddr += PAGE_SIZE as u64;
            current_paddr += PAGE_SIZE as u64;
        }

        // TODO: Diğer gerekli başlangıç haritalamaları (örneğin, belleğin geri kalanı için, I/O alanları).
        // Şimdilik sadece çekirdek alanını haritaladık.

        // 3. Kök sayfa tablosu fiziksel adresini sakla
        let manager = RiscvMemoryManager {
            kernel_page_table_paddr: kernel_root_pt_paddr,
            // TODO: Address space map'i başlat
        };

        // TODO: SATP kaydını çekirdek sayfa tablosuna işaret edecek şekilde yükle
        // Bu işlem genellikle düşük seviyeli Rust veya assembly ile yapılır.
        // Burada sadece çağrıyı simüle edelim:
         unsafe { set_satp_register(manager.kernel_page_table_paddr, satp_mode_sv39) };
        // sfence.vma instruction da gerekebilir.

        Ok(manager)
    }

    /// Verilen bir sanal adresi (vaddr) fiziksel adrese (paddr) çevirir.
    /// Hata durumunda (eşleşme yoksa) KError döner.
    /// `root_page_table_paddr`: Kullanılacak kök sayfa tablosunun fiziksel adresi.
    /// `vaddr`: Çevrilecek sanal adres.
    pub fn translate_address(root_page_table_paddr: u64, vaddr: u64) -> Result<u64, KError> {
        let mut current_paddr = root_page_table_paddr;
        let mut current_vaddr = vaddr;

        for level in (0..3).rev() { // Sv39'da 3 seviye: 2, 1, 0
            let page_table = unsafe { &*(current_paddr as *const PageTable) };
            let vpn_index = match level {
                2 => ((current_vaddr >> (PAGE_SHIFT + VA_VPN0_BITS + VA_VPN1_BITS)) & ((1 << VA_VPN2_BITS) - 1)) as usize,
                1 => ((current_vaddr >> (PAGE_SHIFT + VA_VPN0_BITS)) & ((1 << VA_VPN1_BITS) - 1)) as usize,
                0 => ((current_vaddr >> PAGE_SHIFT) & ((1 << VA_VPN0_BITS) - 1)) as usize,
                _ => unreachable!(), // Sv39'da bu seviyelere inilmez
            };

            if vpn_index >= ENTRIES_PER_PAGE_TABLE {
                 // Bu durumda sanal adres yapısı hatalı veya desteklenmiyor.
                 // Ancak normal bir MMU translate'de bu durum olmamalı, index VA'dan geldiği için geçerli olmalı.
                 // Yine de bir hata mekanizması bulundurmak iyi olabilir.
                 return Err(KError::InvalidArgument);
            }

            let pte = page_table.entries[vpn_index];

            if !pte.is_valid() {
                // PTE geçerli değil, adres eşlenmemiş
                return Err(KError::NotFound); // Veya KError::BadAddress
            }

            if pte.is_leaf() {
                // Yaprak PTE bulundu, fiziksel adresi hesapla
                // Yaprak PTE'lerde PPN, fiziksel sayfanın (veya büyük sayfanın) başlangıcını gösterir.
                // Sanal adresin ofset kısmı eklenmelidir.
                let page_offset = current_vaddr & ((1 << PAGE_SHIFT) - 1); // vaddr & 0xFFF
                let paddr = (pte.ppn() << PAGE_SHIFT) | page_offset;
                // TODO: Büyük sayfa (superpage) haritalamaları için kontrol ve hesaplama ekle (Sv39'da 2MB, 1GB)
                return Ok(paddr);
            } else {
                // Tablo PTE bulundu, bir sonraki seviye sayfa tablosuna in
                current_paddr = pte.ppn() << PAGE_SHIFT; // Bir sonraki sayfa tablosunun fiziksel adresi
            }
        }

        // Eğer 0. seviyeye kadar inildi ve hala yaprak bulunamadıysa (ki is_table kontrolü bunu engeller),
        // veya beklenmedik bir durum oluştuysa.
        Err(KError::InternalError) // Bu noktaya gelinmemeli normalde
    }


    /// Sayfa tablosunda belirli bir sanal adrese fiziksel bir sayfayı haritalar.
    /// Gerekirse ara seviye sayfa tablolarını otomatik olarak oluşturur.
    /// `root_page_table_paddr`: Güncellenecek kök sayfa tablosunun fiziksel adresi.
    /// `vaddr`: Haritalanacak sanal adres (sayfa hizalı).
    /// `paddr`: Haritalanacak fiziksel adres (sayfa hizalı).
    /// `flags`: PTE için set edilecek bayraklar (V, R, W, X, U vb.).
    /// `is_kernel`: Eğer çekirdek adres alanına haritalama yapılıyorsa true. Kullanıcı içinse false (U bayrağı eklenir).
    pub fn map_page_in_table(
        root_page_table_paddr: u64,
        vaddr: u64,
        paddr: u64,
        mut flags: u64,
        is_kernel: bool,
    ) -> Result<(), KError> {
        let mut current_pt_paddr = root_page_table_paddr;
        let mut current_vaddr = vaddr;

        // Kullanıcı alanı haritalaması için U bayrağını ekle
        if !is_kernel {
            flags |= PteFlags::U.bits();
        } else {
             // Çekirdek haritalaması için U bayrağının olmadığından emin ol
             flags &= !PteFlags::U.bits();
        }

        for level in (0..3).rev() { // Sv39'da 3 seviye: 2, 1, 0
            let page_table = unsafe { &mut *(current_pt_paddr as *mut PageTable) };
            let vpn_index = match level {
                2 => ((current_vaddr >> (PAGE_SHIFT + VA_VPN0_BITS + VA_VPN1_BITS)) & ((1 << VA_VPN2_BITS) - 1)) as usize,
                1 => ((current_vaddr >> (PAGE_SHIFT + VA_VPN0_BITS)) & ((1 << VA_VPN1_BITS) - 1)) as usize,
                0 => ((current_vaddr >> PAGE_SHIFT) & ((1 << VA_VPN0_BITS) - 1)) as usize,
                _ => unreachable!(),
            };

            let pte = page_table.entry(vpn_index);

            if level == 0 {
                // Son seviyeye geldik, fiziksel sayfayı haritala (yaprak PTE)
                if pte.is_valid() {
                    // Zaten bir eşleşme var! Bu bir hata veya mevcut eşleşme kaldırılmalı.
                    // Basitlik için hata döndürelim.
                    return Err(KError::AlreadyExists); // veya KError::InvalidArgument
                }
                *pte = PageTableEntry::new(paddr >> PAGE_SHIFT, flags | PteFlags::V.bits());
                // TODO: sfence.vma ile TLB'yi temizle (bu vaddr için veya global)
                return Ok(())
            } else {
                // Ara seviye, bir sonraki sayfa tablosuna in
                if !pte.is_valid() {
                    // Bir sonraki seviye sayfa tablosu yok, oluştur
                    let next_level_pt_paddr = allocate_physical_frame()?;
                    // Yeni sayfa tablosunu sıfırla
                     unsafe { core::ptr::write_bytes(next_level_pt_paddr as *mut u8, 0, PAGE_SIZE) };
                    // Ara PTE'yi bir sonraki sayfa tablosuna işaret edecek şekilde ayarla
                    *pte = PageTableEntry::new(next_level_pt_paddr >> PAGE_SHIFT, PteFlags::V.bits()); // Sadece Valid bayrağı
                    current_pt_paddr = next_level_pt_paddr;
                     // TODO: sfence.vma (bu vaddr için veya global)
                } else if pte.is_leaf() {
                    // Beklenmedik şekilde ara seviyede yaprak PTE var. Bu sanal adresin bir kısmı zaten
                    // büyük bir sayfaya (superpage) haritalanmış. Bu durumda çakışma var.
                     return Err(KError::AlreadyExists); // veya KError::InvalidArgument
                } else {
                    // Bir sonraki seviye sayfa tablosu zaten var, adresini al ve devam et
                    current_pt_paddr = pte.ppn() << PAGE_SHIFT;
                }
            }
        }
         // Bu noktaya gelinmemeli normalde
        Err(KError::InternalError)
    }

     /// Sayfa tablosundan belirli bir sanal adrese ait eşleşmeyi kaldırır.
     /// Gerekirse boşalan ara seviye sayfa tablolarını serbest bırakır (basit implementasyonda serbest bırakmayabiliriz).
    /// `root_page_table_paddr`: Güncellenecek kök sayfa tablosunun fiziksel adresi.
    /// `vaddr`: Eşleşmesi kaldırılacak sanal adres (sayfa hizalı).
    pub fn unmap_page_in_table(root_page_table_paddr: u64, vaddr: u64) -> Result<(), KError> {
        let mut current_pt_paddr = root_page_table_paddr;
        let mut current_vaddr = vaddr;

         // TODO: Bu implementasyon sadece yaprak PTE'yi geçersiz yapar, ara seviye tabloları temizlemez.
         // Tam bir unmap implementasyonu, ağacı yukarı doğru takip edip boşalan tabloları serbest bırakmalıdır.

        for level in (0..3).rev() { // Sv39'da 3 seviye: 2, 1, 0
            let page_table = unsafe { &mut *(current_pt_paddr as *mut PageTable) };
            let vpn_index = match level {
                 2 => ((current_vaddr >> (PAGE_SHIFT + VA_VPN0_BITS + VA_VPN1_BITS)) & ((1 << VA_VPN2_BITS) - 1)) as usize,
                1 => ((current_vaddr >> (PAGE_SHIFT + VA_VPN0_BITS)) & ((1 << VA_VPN1_BITS) - 1)) as usize,
                0 => ((current_vaddr >> PAGE_SHIFT) & ((1 << VA_VPN0_BITS) - 1)) as usize,
                _ => unreachable!(),
            };

            let pte = page_table.entry(vpn_index);

            if !pte.is_valid() {
                // Zaten eşleşme yok
                return Err(KError::NotFound);
            }

            if level == 0 {
                // Son seviyeye geldik, yaprak PTE'yi geçersiz yap
                 if !pte.is_leaf() {
                      // Beklenmedik şekilde tablo PTE var
                     return Err(KError::InternalError);
                 }
                 let freed_paddr = pte.ppn() << PAGE_SHIFT; // Serbest bırakılacak fiziksel sayfa
                *pte = PageTableEntry::empty(); // PTE'yi geçersiz yap
                free_physical_frame(freed_paddr); // Fiziksel sayfayı serbest bırak
                // TODO: sfence.vma ile TLB'yi temizle (bu vaddr için veya global)
                return Ok(());
            } else {
                // Ara seviye
                 if pte.is_leaf() {
                     // Beklenmedik şekilde ara seviyede yaprak PTE var (superpage?)
                     return Err(KError::InternalError);
                 }
                // Bir sonraki sayfa tablosunun adresini al ve in
                current_pt_paddr = pte.ppn() << PAGE_SHIFT;
            }
        }
        // Bu noktaya gelinmemeli normalde
         Err(KError::InternalError)
    }


    // --- Karnal64 Memory API Karşılıkları ---
    // Bu fonksiyonlar, karnal64.rs'deki public `memory_*` fonksiyonları tarafından çağrılmak üzere tasarlanmıştır.
    // Buradaki implementasyonlar, yukarıda tanımlanan MMU operasyonlarını kullanır.
    // Tek bir RiscvMemoryManager instance'ı olduğu varsayılır (singleton gibi).

    // Not: Gerçek implementasyonda, bu fonksiyonlar hangi görevin (task) sanal alanında işlem
    // yapacaklarını bilmelidir. Bu bilgi genellikle görev kontrol bloğundan (TCB) alınır
    // ve TCB, aktif görevin kök sayfa tablosunun fiziksel adresini (SATP değeri) saklar.
    // Basitlik adına, şimdilik sadece çekirdek sayfa tablosunu veya varsayılan aktif görevinkini kullandığımızı varsayalım.

    /// Kullanıcı alanı için bellek tahsis eder.
    /// `size`: Tahsis edilecek bellek boyutu (byte olarak).
    /// Başarı durumunda kullanıcı alanında geçerli bir sanal adrese işaret eden pointer,
    /// hata durumunda KError döner.
    /// Güvenlik: Döndürülen pointer, çağrı yapan görev için geçerli ve erişilebilir olmalıdır.
    pub fn allocate_user_memory(&mut self, size: usize) -> Result<*mut u8, KError> {
        if size == 0 { return Ok(core::ptr::null_mut()); }
        // Boyutu sayfa boyutuna yuvarla
        let pages_needed = (size + PAGE_SIZE - 1) / PAGE_SIZE;

        // TODO: Görev kontrol bloğundan (TCB) veya görev adres alanı yöneticisinden
        // kullanıcı alanı için boş bir sanal adres aralığı bul.
        let user_vaddr_start = 0x40000000; // Örnek: Kullanıcı alanı 1GB'dan başlasın
        // TODO: Bu aralığın müsait olduğunu doğrula.
        let mut current_vaddr = user_vaddr_start;

        let current_task_root_pt_paddr = self.kernel_page_table_paddr; // TODO: Gerçek görevinki olmalı!

        // Gerekli sayıda fiziksel sayfa tahsis et ve sanal adreslere haritala
        for _ in 0..pages_needed {
            let paddr = allocate_physical_frame()?; // Fiziksel sayfa tahsis et
            // Kullanıcı için haritala (Okuma, Yazma, Kullanıcı izni)
            Self::map_page_in_table(
                current_task_root_pt_paddr,
                current_vaddr,
                paddr,
                PteFlags::R.bits() | PteFlags::W.bits(), // RW izinleri (X hariç)
                false // Kullanıcı sayfası
            )?;
            current_vaddr += PAGE_SIZE as u64;
        }

        // Tahsis edilen aralığın başlangıç sanal adresini döndür (pointer olarak)
        Ok(user_vaddr_start as *mut u8)
    }

    /// Kullanıcı alanı belleğini serbest bırakır.
    /// `ptr`: Serbest bırakılacak bellek bloğunun başlangıç pointer'ı (kullanıcı alanı sanal adresi).
    /// `size`: Serbest bırakılacak bellek bloğunun boyutu (byte olarak).
    /// Başarı veya hata (geçersiz pointer gibi) döner.
    /// Güvenlik: Verilen pointer'ın ve boyutun çağıran görev tarafından tahsis edilmiş geçerli bir bloğu temsil ettiğini doğrula.
    pub fn free_user_memory(&mut self, ptr: *mut u8, size: usize) -> Result<(), KError> {
         if ptr.is_null() || size == 0 { return Ok(()); }
         let vaddr_start = ptr as u64;
         // Boyutu sayfa boyutuna yuvarla
        let pages_needed = (size + PAGE_SIZE - 1) / PAGE_SIZE;

         // TODO: Verilen pointer'ın ve boyutun gerçekten daha önce bu görev için tahsis edilmiş bir blok olduğunu doğrula.
         // Bu, görev adres alanı yöneticisinin takip etmesi gereken bir bilgidir.
         // Basitlik için sadece unmap işlemi yapalım.

        let current_task_root_pt_paddr = self.kernel_page_table_paddr; // TODO: Gerçek görevinki olmalı!
        let mut current_vaddr = vaddr_start;

        for _ in 0..pages_needed {
             Self::unmap_page_in_table(current_task_root_pt_paddr, current_vaddr)?;
            current_vaddr += PAGE_SIZE as u64;
        }

        Ok(())
    }

    /// Paylaşımlı bellek alanı oluşturur.
    /// `size`: Oluşturulacak paylaşımlı bellek boyutu (byte olarak).
    /// Başarı durumunda paylaşımlı bellek alanını temsil eden bir Karnal64 handle'ı,
    /// hata durumunda KError döner.
    /// Paylaşımlı bellek, farklı görevler tarafından kendi adres alanlarına haritalanabilir.
    pub fn create_shared_memory(&mut self, size: usize) -> Result<KHandle, KError> {
        if size == 0 { return Err(KError::InvalidArgument); }
         let pages_needed = (size + PAGE_SIZE - 1) / PAGE_SIZE;

         // Paylaşımlı bellek için fiziksel sayfalar tahsis et
         let mut physical_frames = Vec::new(); // Veya `no_std` uyumlu bir liste/dizi
         for _ in 0..pages_needed {
             let paddr = allocate_physical_frame()?;
             physical_frames.push(paddr);
         }

         // TODO: Paylaşımlı bellek alanını çekirdek içinde bir nesne olarak kaydet.
         // Bu nesne, tahsis edilen fiziksel sayfa listesini ve boyutu saklar.
         // Kaynak yöneticisi (kresource) veya ayrı bir paylaşımlı bellek yöneticisi bunu yapabilir.
         // Bu nesne için bir KHandle oluşturulur ve döndürülür.

         // Yer tutucu: Dummy handle döndürelim ve fiziksel frame'leri hemen serbest bırakalım (yapılmamalı!)
         for paddr in physical_frames {
             free_physical_frame(paddr); // Gerçekte, handle serbest bırakılınca serbest bırakılmalı
         }
         Ok(KHandle(42)) // Dummy handle
    }


    /// Mevcut görevin adres alanına paylaşımlı bellek alanını haritalar.
    /// `shm_handle_value`: Paylaşımlı bellek alanının handle değeri.
    /// `offset`: Paylaşımlı bellek alanının başlangıcından haritalanacak ofset (byte olarak).
    /// `size`: Haritalanacak bölümün boyutu (byte olarak).
    /// Başarı durumunda görevin adres alanında geçerli bir sanal adrese işaret eden pointer,
    /// hata durumunda KError döner.
    /// Güvenlik: Handle'ın geçerli bir paylaşımlı bellek nesnesine ait olduğunu ve çağıran görevin haritalama izni olduğunu doğrula.
    pub fn map_shared_memory(&mut self, shm_handle_value: u64, offset: usize, size: usize) -> Result<*mut u8, KError> {
        if size == 0 { return Ok(core::ptr::null_mut()); }
         if offset % PAGE_SIZE != 0 || size % PAGE_SIZE != 0 {
             // Ofset ve boyut sayfa hizalı olmalı
             return Err(KError::InvalidArgument);
         }

         // TODO: shm_handle_value'yu kullanarak paylaşımlı bellek nesnesini bul.
         // Bu nesneden fiziksel sayfa listesini ve toplam boyutu al.
         // Offset ve size'ın geçerli aralıkta olduğunu doğrula.

         // Yer tutucu: Dummy paylaşımlı bellek nesnesi varsayalım
         let dummy_shm_physical_frames = vec![0x82000000, 0x82001000]; // Örnek: 2 sayfalık SHM
         let total_shm_size = dummy_shm_physical_frames.len() * PAGE_SIZE;

         if offset + size > total_shm_size {
             return Err(KError::InvalidArgument); // Aralık SHM boyutunu aşıyor
         }

         // TODO: Görev kontrol bloğundan (TCB) veya görev adres alanı yöneticisinden
         // kullanıcı alanı için boş bir sanal adres aralığı bul (haritalama için).
         let user_vaddr_start = 0x50000000; // Örnek: Paylaşımlı bellek 1.25GB'dan başlasın
         // TODO: Bu aralığın müsait olduğunu doğrula.
         let mut current_vaddr = user_vaddr_start;

         let current_task_root_pt_paddr = self.kernel_page_table_paddr; // TODO: Gerçek görevinki olmalı!
         let start_page_index = offset / PAGE_SIZE;
         let pages_to_map = size / PAGE_SIZE;

         // Paylaşımlı belleğin ilgili fiziksel sayfalarını görevin sanal adres alanına haritala
        for i in 0..pages_to_map {
             let paddr_to_map = dummy_shm_physical_frames[start_page_index + i];
             // Kullanıcı için haritala (Okuma, Yazma, Kullanıcı izni - X olmamalı)
             Self::map_page_in_table(
                 current_task_root_pt_paddr,
                 current_vaddr,
                 paddr_to_map,
                 PteFlags::R.bits() | PteFlags::W.bits(), // RW izinleri
                 false // Kullanıcı sayfası
             )?;
             current_vaddr += PAGE_SIZE as u64;
        }

         // Haritalanan aralığın başlangıç sanal adresini döndür (pointer olarak)
        Ok(user_vaddr_start as *mut u8)
    }

     /// Paylaşımlı bellek alanının görevin adres alanındaki eşleşmesini kaldırır.
    /// `ptr`: Haritalanmış paylaşımlı bellek bloğunun başlangıç pointer'ı (kullanıcı alanı sanal adresi).
    /// `size`: Haritalanmış bloğun boyutu (byte olarak).
    /// Başarı veya hata (geçersiz pointer gibi) döner.
    /// Güvenlik: Verilen pointer'ın ve boyutun çağıran görev tarafından haritalanmış geçerli bir paylaşımlı bellek bloğunu temsil ettiğini doğrula.
     pub fn unmap_shared_memory(&mut self, ptr: *mut u8, size: usize) -> Result<(), KError> {
         if ptr.is_null() || size == 0 { return Ok(()); }
         let vaddr_start = ptr as u64;
         // Boyutu sayfa boyutuna yuvarla
        let pages_to_unmap = (size + PAGE_SIZE - 1) / PAGE_SIZE;

         // TODO: Verilen pointer'ın ve boyutun gerçekten daha önce bu görev için haritalanmış bir SHM bloğu olduğunu doğrula.
         // Bu, görev adres alanı yöneticisinin takip etmesi gereken bir bilgidir.
         // Basitlik için sadece unmap işlemi yapalım.

        let current_task_root_pt_paddr = self.kernel_page_table_paddr; // TODO: Gerçek görevinki olmalı!
        let mut current_vaddr = vaddr_start;

        for _ in 0..pages_to_unmap {
             Self::unmap_page_in_table(current_task_root_pt_paddr, current_vaddr)?;
            current_vaddr += PAGE_SIZE as u64;
        }

         // NOT: unmap işlemi fiziksel sayfaları serbest BIRAKMAZ, sadece sanal eşleşmeyi kaldırır.
         // Paylaşımlı bellek handle'ı serbest bırakıldığında (resource_release ile) fiziksel sayfalar serbest bırakılmalıdır.

        Ok(())
     }


    // TODO: Diğer MMU/Bellek Yönetimi ile ilgili fonksiyonlar:
    // - Adres alanları arasında geçiş (switch_to_address_space) - SATP kaydı güncelleme
    // - Sanal adres aralığı ayırma/takip etme (VM Area management)
    // - Copy-on-Write implementasyonu
    // - Page fault işleme
    // - Büyük sayfa (superpage) desteği
}

// TODO: Physical Frame Allocator implementasyonu (başka bir dosyada/modülde olmalı)

// TODO: SATP kaydını ayarlayan düşük seviyeli assembly fonksiyonu (yer tutucu)

extern "C" {
    fn set_satp_register(satp_value: u64);
}


// TODO: SFENCE.VMA instruction'ı çağıran fonksiyon (yer tutucu)

pub fn flush_tlb(vaddr: Option<u64>) {
    if let Some(addr) = vaddr {
        // SFENCE.VMA vaddr
        unsafe {
            asm!("sfence.vma {}", in(reg) addr);
        }
    } else {
        // SFENCE.VMA (global)
         unsafe {
            asm!("sfence.vma zero");
        }
    }
}
