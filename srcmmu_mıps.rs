#![no_std] // Çekirdek alanında çalışıyoruz, standart kütüphaneye ihtiyacımız yok.

// Geliştirme sırasında kullanılmayan kod veya argümanlar için izinler (geçici olabilir)
#![allow(dead_code)]
#![allow(unused_variables)]

use core::ptr; // Pointer işlemleri için

// Karnal64'ten alınabilecek temel tipler veya hatalar
// Bunları doğrudan burada tanımlayabilir veya karnal64::* altında import edebiliriz.
// Şimdilik gerekli olanları burada tanımlayalım veya türetelim.
// KError'ı karnal64.rs'ten alıyormuş gibi yapalım veya minimal bir versiyonunu tanımlayalım.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i64)]
pub enum MipsMmuError {
    InvalidArgument = -100, // MMU özelinde bir hata kodu
    OutOfMemory = -101,     // Sayfa tablosu için bellek yok
    BadAddress = -102,      // Geçersiz sanal/fiziksel adres
    PermissionDenied = -103, // Talep edilen izin desteklenmiyor
    AlreadyMapped = -104,   // Adres zaten haritalanmış
    NotMapped = -105,       // Adres haritalanmamış
    InternalError = -106,   // Dahili MMU hatası
}

// Basitlik adına, Karnal64'ün KError'ına dönüşümü varsayalım
 impl From<MipsMmuError> for KError { ... }

// Adres Tipleri
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtualAddress(pub usize);
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysicalAddress(pub usize);

// MIPS MMU/TLB İle İlgili Sabitler ve Yapılar

// MIPS genellikle 4KB sayfalar kullanır, ancak mimariye göre değişebilir.
pub const PAGE_SIZE: usize = 4096; // 4 KB
pub const PAGE_SIZE_LOG2: usize = 12; // log2(4096)

// Sayfa Ofsetini bulmak için maske
pub const PAGE_OFFSET_MASK: usize = PAGE_SIZE - 1;
// Sayfa Numarasını (PN) bulmak için maske (ofseti atar)
pub const PAGE_NUMBER_MASK: usize = !(PAGE_OFFSET_MASK);

// Sanal Adresi Sayfa Numarasına dönüştür
#[inline]
pub fn virtual_address_to_page_number(vaddr: VirtualAddress) -> usize {
    vaddr.0 >> PAGE_SIZE_LOG2
}

// Sayfa Numarasını Sanal Adres (sayfa başlangıcı) dönüştür
#[inline]
pub fn page_number_to_virtual_address(page_num: usize) -> VirtualAddress {
    VirtualAddress(page_num << PAGE_SIZE_LOG2)
}

// Fiziksel Adresi Sayfa Çerçevesi Numarasına (PFN) dönüştür
#[inline]
pub fn physical_address_to_page_frame_number(paddr: PhysicalAddress) -> usize {
    paddr.0 >> PAGE_SIZE_LOG2
}

// Sayfa Çerçevesi Numarasını Fiziksel Adres (sayfa başlangıcı) dönüştür
#[inline]
pub fn page_frame_number_to_physical_address(pfn: usize) -> PhysicalAddress {
    PhysicalAddress(pfn << PAGE_SIZE_LOG2)
}


// MIPS TLB Entry Yapısı (basitleştirilmiş)
// Gerçekte bu, MIPS CP0 kaydetlerindeki EntryHi, EntryLo0, EntryLo1 formatına uymalıdır.
// MIPS TLB'si iki ardışık sanal sayfayı tek bir girişte haritalayabilir (EntryLo0 ve EntryLo1).
// Burada tek bir sayfa için EntryLo benzeri bir yapı taslağı sunalım.

#[repr(C)] // C uyumlu bellek düzeni (genellikle donanım formatına yakınlık için)
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TlbEntryLo {
    // Fiziksel Sayfa Çerçevesi Numarası (PFN)
    // EntryLo'daki gerçek bit pozisyonlarına dikkat edilmelidir.
    pub pfn: usize,
    // Sayfa Özellikleri/İzinleri (Caching, Dirty/Writable, Global, Valid)
    // Bu bitlerin anlamı MIPS mimarisine özeldir (CP0).
    pub flags: u32,
    // Valid biti: Bu girdi geçerli mi?
    pub valid: bool,
    // Dirty biti: Sayfaya yazıldı mı? (Genellikle yazma izni olarak kullanılır)
    pub dirty: bool, // Genellikle Writable iznine denk gelir
    // Global biti: Tüm görevler tarafından erişilebilir mi?
    pub global: bool, // TLB aramalarında ASID'yi yoksayar
    // Diğer flagler (Caching tipi vb.) eklenebilir.
}

// Sayfa İzinleri Bayrakları (Karnal64 veya Sahne64 ile uyumlu olabilir)
// Bu flagler, TlbEntryLo'daki 'flags' alanına dönüştürülmelidir.
pub struct PageFlags(u32);

impl PageFlags {
    pub const READ: PageFlags = PageFlags(1 << 0);
    pub const WRITE: PageFlags = PageFlags(1 << 1); // Dirty bitiyle eşleşebilir
    pub const EXECUTE: PageFlags = PageFlags(1 << 2);
    pub const GLOBAL: PageFlags = PageFlags(1 << 3); // Global bitiyle eşleşebilir
    // Diğer flagler (User/Supervisor, Cacheable/Uncacheable vb.) eklenebilir.

    // Bu Karnal64 PageFlags'larını MIPS'in TlbEntryLo 'flags' bitlerine dönüştüren yardımcı fonksiyon
    #[inline]
    pub fn to_mips_tlb_flags(&self) -> u32 {
        let mut mips_flags = 0;
        // Örnek dönüşümler (MIPS CP0 dokümantasyonuna göre ayarlanmalı)
        if self.0 & Self::WRITE.0 != 0 {
            mips_flags |= 0x4; // Örnek MIPS Dirty/Writable biti
        }
        if self.0 & Self::GLOBAL.0 != 0 {
            mips_flags |= 0x8; // Örnek MIPS Global biti
        }
        // TODO: READ, EXECUTE ve diğer flagler için MIPS bitlerini ekle
        mips_flags
    }
}


// --- MIPS MMU Yönetim Fonksiyonları ---

// Kernel'in fiziksel bellek ayırıcısından (kmemory'den gelmeli) sayfa çerçevesi almak için yer tutucu trait.
// kmemory modülü bu trait'i implemente eden bir nesne sağlamalıdır.
pub trait PhysFrameAllocator {
    /// Boş bir fiziksel sayfa çerçevesi tahsis eder.
    fn allocate_frame(&self) -> Result<PhysicalAddress, MipsMmuError>;
    /// Daha önce tahsis edilmiş bir fiziksel sayfa çerçevesini serbest bırakır.
    fn free_frame(&self, paddr: PhysicalAddress) -> Result<(), MipsMmuError>;
}

// Sayfa tablosu yapısı (MIPS genellikle tersine sayfa tabloları veya segment/sayfa hiyerarşisi kullanabilir)
// Basitlik adına, sanal sayfa numaralarını TlbEntryLo benzeri bilgilere eşleyen
// kavramsal bir yapı kullanalım. Gerçek MIPS implementasyonunda bu çok daha karmaşıktır.
struct PageTable; // Yer tutucu

impl PageTable {
    // Yeni bir sayfa tablosu (veya adres alanı) oluştur
    fn new() -> Result<Self, MipsMmuError> {
        // TODO: Fiziksel bellekten sayfa tablosu yapıları için bellek ayır.
        // Bu, genellikle fiziksel bir frame'e haritalanmış bir veri yapısıdır.
        Ok(PageTable {}) // Yer tutucu
    }

    // Sanal adresi sayfa tablosunda ara ve TlbEntryLo bilgisini döndür
    fn lookup(&self, vaddr: VirtualAddress) -> Result<TlbEntryLo, MipsMmuError> {
        // TODO: vaddr'ı kullanarak sayfa tablosunda karşılığı olan PTE'yi (veya TLB girişi için bilgiyi) ara.
        // Bu, MIPS'in sayfa tablosu yapısına bağlıdır.
        // Eğer girdi bulunursa TlbEntryLo'ya dönüştürülür.
        // Eğer bulunamazsa MipsMmuError::NotMapped döner.
        Err(MipsMmuError::NotMapped) // Yer tutucu
    }

    // Sayfa tablosuna yeni bir haritalama ekle/güncelle
    fn map(&mut self, vaddr: VirtualAddress, paddr: PhysicalAddress, flags: PageFlags) -> Result<(), MipsMmuError> {
        // TODO: vaddr, paddr ve flags bilgilerini kullanarak sayfa tablosunda uygun girdiyi oluştur/güncelle.
        // Bu, MIPS'in sayfa tablosu formatına ve hiyerarşisine bağlıdır.
        // Sayfa ofsetleri 0 olmalıdır (sayfa başlangıcı).
        if vaddr.0 % PAGE_SIZE != 0 || paddr.0 % PAGE_SIZE != 0 {
            return Err(MipsMmuError::InvalidArgument);
        }

        // Örnek: Çok basit bir durumda, sadece sanal adresi fiziksel adrese eşleyen bir yapı tuttuğumuzu varsayalım (gerçekçi değil)
        // Gerçekte burada sayfa tablosu girişleri oluşturulup fiziksel belleğe yazılacaktır.

        // Başarı döner
        Ok(())
    }

    // Sayfa tablosundan bir haritalamayı kaldır
    fn unmap(&mut self, vaddr: VirtualAddress) -> Result<(), MipsMmuError> {
        // TODO: vaddr'ı kullanarak sayfa tablosundaki girdiyi geçersiz kıl/kaldır.
        if vaddr.0 % PAGE_SIZE != 0 {
            return Err(MipsMmuError::InvalidArgument);
        }
        // Başarı döner
        Ok(())
    }
}


// --- MIPS CP0 (Yardımcı İşlemci 0) Etkileşimi için Yer Tutucular ---
// MIPS'te MMU ve TLB, CP0 özel kayıtları aracılığıyla yönetilir.
// Rust'ta doğrudan inline assembly veya özel donanım erişim crate'leri gerekir.
// Burada kavramsal fonksiyonlar olarak gösterilmiştir.

#[inline]
fn read_cp0_entryhi() -> u64 {
    // TODO: MIPS `mfc0 $reg, $10` (EntryHi) talimatını kullanarak CP0 EntryHi kaydını oku.
    // Assembly kodu veya donanım erişim mekanizması gerektirir.
    0 // Yer tutucu değer
}

#[inline]
fn write_cp0_entryhi(value: u64) {
    // TODO: MIPS `mtc0 $reg, $10` (EntryHi) talimatını kullanarak CP0 EntryHi kaydına yaz.
    // Assembly kodu veya donanım erişim mekanizması gerektirir.
}

#[inline]
fn read_cp0_entrylo0() -> u64 { /* ... */ 0 }
#[inline]
fn write_cp0_entrylo0(value: u64) { /* ... */ }
#[inline]
fn read_cp0_entrylo1() -> u64 { /* ... */ 0 }
#[inline]
fn write_cp0_entrylo1(value: u64) { /* ... */ }
#[inline]
fn read_cp0_index() -> u64 { /* ... */ 0 }
#[inline]
fn write_cp0_index(value: u64) { /* ... */ }
#[inline]
fn read_cp0_random() -> u64 { /* ... */ 0 } // TLB yazarken kullanılır
#[inline]
fn write_cp0_pagemask(value: u64) { /* ... */ } // Farklı sayfa boyutları için

// TLB Yönetimi için CP0 talimatları (Assembly gerekir)
#[inline]
fn tlb_read() {
    // TODO: MIPS `tlbpr` (TLB Probe) veya `tlbr` (TLB Read) talimatı.
    // Genellikle Index registerı kullanılır.
}

#[inline]
fn tlb_write_indexed() {
    // TODO: MIPS `tlbwi` (TLB Write Indexed) talimatı. Index registerı kullanılır.
}

#[inline]
fn tlb_write_random() {
    // TODO: MIPS `tlbwr` (TLB Write Random) talimatı. Random registerı kullanılır.
}

#[inline]
fn tlb_probe() -> bool {
    // TODO: MIPS `tlbp` (TLB Probe) talimatı. EntryHi kullanılır.
    // TLB'de eşleşme bulunursa Index registerını günceller ve CP0 Index registerında özel bir bit ayarlar.
    // Bu bitin kontrol edilmesi gerekir.
    false // Yer tutucu
}

// --- MIPS MMU Modülü API'sı ---
// Bu fonksiyonlar, kmemory veya çekirdeğin diğer kısımları tarafından çağrılabilir.

/// MIPS MMU alt sistemini başlatır. Boot sürecinin erken aşamalarında çağrılır.
pub fn init() {
    // TODO: CP0 kayıtlarını (Status, Config vb.) MMU/TLB kullanımına uygun ayarlayın.
    // TODO: Kernel için başlangıç sayfa tablolarını veya haritalamalarını oluşturun (Identity mapping vb.).
    // TODO: MMU'yu etkinleştirin.

    // Örnek: Basit bir CP0 ayarı simülasyonu
     write_cp0_status(read_cp0_status() | STATUS_ERL | STATUS_VM); // Örnek: Hata seviyesi, Sanal Bellek açık
     write_cp0_pagemask(PAGE_SIZE_MASK_FOR_4KB); // Örnek: 4KB sayfa boyutu maskesi

    // Kernel'in fiziksel adres alanını sanal adrese haritalama (kimlik haritalama veya yüksek adreslere)
     let kernel_start_paddr = PhysicalAddress(...);
     let kernel_start_vaddr = VirtualAddress(...);
     map_range(kernel_start_vaddr, kernel_start_paddr, kernel_size, PageFlags::READ | PageFlags::WRITE | PageFlags::EXECUTE | PageFlags::GLOBAL).expect("Kernel mapping failed");

    println!("MIPS MMU Başlatıldı (Yer Tutucu)"); // Çekirdek içi print! gerektirir
}

/// Belirli bir görevin adres alanı için bir PageTable yapısı oluşturur.
/// Karnal64'teki görev yöneticisi (ktask) veya bellek yöneticisi (kmemory)
/// tarafından yeni bir görev başlatılırken çağrılabilir.
pub fn create_address_space() -> Result<PageTable, MipsMmuError> {
    PageTable::new()
}

/// Belirli bir sanal adresi, belirli bir fiziksel adrese, verilen izinlerle haritalar.
/// `page_table`: Haritalamanın yapılacağı görev/adres alanının sayfa tablosu.
/// `vaddr`: Haritalanacak sanal adres (sayfa başlangıcı olmalı).
/// `paddr`: Eşlenecek fiziksel adres (sayfa başlangıcı olmalı).
/// `flags`: Sayfa izinleri (Okuma, Yazma, Çalıştırma, Global vb.).
pub fn map_page(
    page_table: &mut PageTable, // Sayfa tablosu mutable olmalı çünkü değişecek
    vaddr: VirtualAddress,
    paddr: PhysicalAddress,
    flags: PageFlags,
) -> Result<(), MipsMmuError> {
    if vaddr.0 % PAGE_SIZE != 0 || paddr.0 % PAGE_SIZE != 0 {
        return Err(MipsMmuError::InvalidArgument);
    }

    // TODO: Sayfa tablosunda ilgili girişi oluştur/güncelle.
    page_table.map(vaddr, paddr, flags)?;

    // TODO: Eğer haritalama mevcut görev için yapılıyorsa, TLB'yi geçersiz kılmak gerekebilir.
     invalidate_tlb_entry(vaddr);

    Ok(())
}

/// Belirli bir sanal adresin haritalamasını kaldırır.
/// `page_table`: İşlemin yapılacağı sayfa tablosu.
/// `vaddr`: Haritalaması kaldırılacak sanal adres (sayfa başlangıcı olmalı).
pub fn unmap_page(
    page_table: &mut PageTable,
    vaddr: VirtualAddress,
) -> Result<(), MipsMmuError> {
    if vaddr.0 % PAGE_SIZE != 0 {
        return Err(MipsMmuError::InvalidArgument);
    }

    // TODO: Sayfa tablosundaki ilgili girdiyi geçersiz kıl.
    page_table.unmap(vaddr)?;

    // TODO: Eğer haritalama mevcut görev için yapılıyorsa, TLB'yi geçersiz kıl.
     invalidate_tlb_entry(vaddr);

    Ok(())
}

/// Sanal adresi fiziksel adrese çevirir (yalnızca çekirdek kullanımı için).
/// `page_table`: Çevirinin yapılacağı sayfa tablosu.
/// `vaddr`: Çevrilecek sanal adres.
pub fn translate_address(
    page_table: &PageTable,
    vaddr: VirtualAddress,
) -> Result<PhysicalAddress, MipsMmuError> {
    // TODO: Sayfa tablosunda vaddr için lookup yap.
    // TLB'yi kontrol etmek gerekebilir, ancak genellikle bu fonksiyon sayfa tablosunu kullanır.
    let entry_lo = page_table.lookup(vaddr)?;

    if !entry_lo.valid {
        return Err(MipsMmuError::NotMapped);
    }

    // Sayfa çerçevesi numarasını al ve ofseti ekle.
    let pfn = entry_lo.pfn;
    let offset = vaddr.0 & PAGE_OFFSET_MASK;
    Ok(PhysicalAddress(page_frame_number_to_physical_address(pfn).0 + offset))
}


/// Belirli bir sanal adres için TLB girdisini geçersiz kılar.
/// Sayfa tabloları güncellendiğinde veya adres alanı değiştirildiğinde çağrılmalıdır.
/// `vaddr`: Geçersiz kılınacak sanal adres (TLB araması için sayfa numarası ve ASID kullanılır).
pub fn invalidate_tlb_entry(vaddr: VirtualAddress) {
    // TODO: MIPS CP0 EntryHi kaydına sanal sayfa numarasını ve mevcut ASID'yi yaz.
    // TODO: MIPS `tlbp` (TLB Probe) talimatını çalıştır.
    // TODO: Eğer eşleşme bulunursa (CP0 Index registerını kontrol et),
    // TODO: CP0 EntryLo0/1 kayıtlarını geçersiz bir değerle (valid=false) yaz.
    // TODO: MIPS `tlbwi` veya `tlbwr` talimatını çalıştır.

    // Bu işlem genellikle `mtc0` ve TLB talimatları gerektiren kısa bir assembly parçasıdır.
    // Örnek akış:
     let page_num = virtual_address_to_page_number(vaddr);
     let current_asid = get_current_asid(); // Mevcut görev/adres alanı ID'si
     write_cp0_entryhi((page_num << PAGE_SIZE_LOG2) | current_asid);
     tlb_probe();
     if (check_cp0_index_probe_success()) {
         write_cp0_entrylo0(0); // Geçersiz değer
         write_cp0_entrylo1(0); // Geçersiz değer
         tlb_write_indexed();
     }
}

/// Tüm TLB girdilerini geçersiz kılar.
/// Adres alanı tamamen değiştirildiğinde (örneğin görev geçişinde) kullanılabilir.
pub fn invalidate_all_tlb() {
    // MIPS'te tüm TLB'yi temizlemenin çeşitli yolları olabilir.
    // Bir yöntem, TLB'deki her girişi tek tek geçersiz kılmaktır.
    // Başka bir yöntem, TLB Random registerını kullanarak yazmaktır (daha az deterministik).
    // Genellikle görev geçişlerinde tüm global olmayan girdileri temizlemek yeterlidir.
    // En basit yol, Global bitini kullanmayan girdileri temizlemektir.

    // TODO: TLB'deki tüm veya ilgili girdileri geçersiz kılmak için MIPS'e özel mekanizmayı kullan.
    // Örneğin, tüm girdiler üzerinde döngü yapıp `tlbwi` ile geçersiz yazmak.

    println!("Tüm MIPS TLB geçersiz kılındı (Yer Tutucu)");
}


// --- İstisna İşleyicileri İçin Yer Tutucular ---
// MIPS MMU hataları (TLB eksikliği, sayfa hatası) istisna olarak işlenir.
// İstisna vektör tablosunda bu fonksiyonlara yönlendirme olmalıdır.

/// TLB Yenileme (Refill) istisnası işleyicisi.
/// Bir sanal adrese erişilirken TLB'de geçerli bir eşleşme bulunamadığında oluşur.
/// Çekirdek bu adresi sayfa tablosunda arar ve bulunursa TLB'ye yeni bir girdi yazar.
#[no_mangle] // İstisna işleyicisi olarak dışarıdan çağrılabilmesi için
pub extern "C" fn handle_tlb_refill(exception_frame: *mut ()) { // İstisna çerçevesi argümanı MIPS'e özeldir
    // TODO: İstisna çerçevesinden hataya neden olan sanal adresi (BadVAddr CP0) ve görevi (EntryHi CP0, ASID) al.
    // TODO: Mevcut görevin sayfa tablosunu bul.
    // TODO: Sayfa tablosunda BadVAddr için haritalamayı ara.
     let vaddr = VirtualAddress(read_cp0_badvaddr() as usize);
     let current_page_table = get_current_task_page_table(); // Görev yöneticisinden/bellek yöneticisinden

     match current_page_table.lookup(vaddr) {
         Ok(tlb_entry_info) => {
    //         // Haritalama bulundu, TLB'ye yaz
              write_cp0_entryhi(...); // Sayfa numarası + ASID
              write_cp0_entrylo0(...); // entry_lo0 bilgisi
              write_cp0_entrylo1(...); // entry_lo1 bilgisi (ardışık sayfa varsa)
              tlb_write_indexed() veya tlb_write_random();
    //         // TODO: İstisna çerçevesini güncelleyip istisnadan dön (eret talimatı gibi).
         }
         Err(_) => {
    //         // Haritalama bulunamadı (gerçek sayfa hatası)
    //         // Sayfa hatası işleyicisine yönlendir.
             handle_page_fault(exception_frame); // Veya başka bir hata işleyicisi
         }
     }

    // Yer Tutucu: Basitçe hata döndür veya panik yap
    println!("MIPS TLB Refill İstisnası! Adres: {:p}", ptr::null::<u8>()); // BadVAddr'ı göster
    // TODO: Gerçek hata işleme veya panik
    loop {} // İşlem durdurulmalı
}

/// Genel Sayfa Hatası (Page Fault) istisnası işleyicisi.
/// Sayfa haritalanmamışsa (TLB Refill'den sonra) veya erişim izni yoksa oluşur.
#[no_mangle]
pub extern "C" fn handle_page_fault(exception_frame: *mut ()) {
    // TODO: İstisna çerçevesinden BadVAddr, EPC (hata anındaki PC), Cause CP0 (hatanın nedeni) al.
    // TODO: Hataya neden olan adresi ve erişim türünü (okuma/yazma/çalıştırma) belirle.
     let vaddr = VirtualAddress(read_cp0_badvaddr() as usize);
     let cause = read_cp0_cause(); // Erişim türünü anlamak için
     let pc = read_cp0_epc(); // Hatanın oluştuğu komut

    // TODO: Bu hatanın "onarılabilecek" bir hata mı (demand paging, copy-on-write)
    // yoksa gerçek bir erişim ihlali mi olduğunu belirle.
    // Örneğin, demand paging için sayfayı fiziksel belleğe yükle, haritala ve TLB'ye ekle.
    // Erişim ihlali ise, ilgili görevi sonlandır (SIGSEGV gibi sinyal gönder).

    // Yer Tutucu: Basitçe hata mesajı ve panik
    println!("MIPS Page Fault İstisnası! Adres: {:p}, PC: {:p}", ptr::null::<u8>(), ptr::null::<u8>()); // BadVAddr ve EPC'yi göster
    // TODO: Gerçek hata işleme veya görevi sonlandırma
    loop {} // İşlem durdurulmalı
}


// TODO: Diğer MIPS'e özgü MMU/TLB ile ilgili fonksiyonlar ve yapılar eklenebilir.
// Örneğin: ASID (Address Space ID) yönetimi, farklı sayfa boyutları desteği, cache yönetimi (MMU ile ilişkili olabilir).

// Yer Tutucu: ASID yönetimi
// MIPS'te her görev kendi ASID'sine sahip olabilir, TLB aramaları ASID'yi dikkate alır (Global bit ayarlanmamışsa).
// Görev geçişlerinde CP0 EntryHi'daki ASID güncellenmelidir.
 pub fn set_current_asid(asid: u8) { /* ... */ }
 pub fn get_current_asid() -> u8 { /* ... */ 0 }

// Yer Tutucu: Cache yönetimi fonksiyonları (MMU ile genellikle yakından ilişkilidir)
 pub fn flush_data_cache(vaddr: VirtualAddress, size: usize) { /* ... */ }
 pub fn invalidate_instruction_cache(vaddr: VirtualAddress, size: usize) { /* ... */ }
