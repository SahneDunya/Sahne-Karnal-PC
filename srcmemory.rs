#![no_std] // Bu modül çekirdek alanında çalışır, standart kütüphaneye ihtiyacı yoktur.

// Karnal64 API'sından gerekli tipleri ve trait'leri içe aktar
// Gerçek bir projede, bu 'crate::karnal64' yolunun projenizin modül yapısına
// uygun olması gerekir.
use crate::karnal64::{KError, KHandle, KTaskId, /* İhtiyaç olursa diğer trait/tipler */};

// --- Yer Tutucu Düşük Seviye Bellek Yöneticileri ---
// Bunlar, gerçek donanım (MMU) ve fiziksel bellek ayırma mantığını içerecek
// ancak burada sadece kavramsal işlevselliği gösteren yer tutuculardır.

mod pma {
    // Hipotetik Fiziksel Bellek Ayırıcı (Physical Memory Allocator)
    // Fiziksel bellek çerçevelerini (page frame) tahsis etmekten ve serbest bırakmaktan sorumludur.
    // Basitlik adına, sadece artan fiziksel adresler döndüren bir simülasyon.

    /// Bir fiziksel bellek çerçevesi (page frame, genellikle 4KB) tahsis eder.
    /// Başarı durumunda çerçevenin fiziksel adresini, yetersiz bellek durumunda None döner.
    pub fn alloc_frame() -> Option<usize> {
        // TODO: Gerçek Fiziksel Bellek Ayırıcı mantığı (free list, bitmap vb.)
        // Basit Simülasyon:
        static mut NEXT_FRAME_ADDR: usize = 0x10000000; // Fiziksel belleğin başlangıcı varsayımı
        const FRAME_SIZE: usize = 4096; // 4KB
        const MAX_FRAMES: usize = 1024; // Simülasyon sınırı (4MB fiziksel bellek)

        unsafe {
            if NEXT_FRAME_ADDR < 0x10000000 + FRAME_SIZE * MAX_FRAMES {
                let frame_addr = NEXT_FRAME_ADDR;
                NEXT_FRAME_ADDR += FRAME_SIZE;
                Some(frame_addr)
            } else {
                None // Yetersiz fiziksel bellek
            }
        }
    }

    /// Tahsis edilmiş bir fiziksel bellek çerçevesini serbest bırakır.
    /// _frame_addr: Serbest bırakılacak çerçevenin fiziksel adresi.
    pub fn free_frame(_frame_addr: usize) {
        // TODO: Gerçek Fiziksel Bellek Ayırıcı mantığı (çerçeveyi free list'e ekle vb.)
        // Simülasyon: Şimdilik bir şey yapmıyor.
    }
}

mod vmm {
    use super::*;
    // Hipotetik Sanal Bellek Yöneticisi (Virtual Memory Manager)
    // Sayfa tablolarını, sanal-fiziksel adres eşleşmelerini ve bellek koruma ayarlarını yönetir.
    // Göreve (task) özel adres alanlarını ele almalıdır.

    /// Sanal bir adresi (vaddr) fiziksel bir adrese (paddr) mevcut görevin sayfa tablosunda eşler.
    /// vaddr: Eşlenecek sanal adres (sayfa hizalı olmalı).
    /// paddr: Eşlenecek fiziksel adres (sayfa hizalı olmalı).
    /// flags: Eşleme bayrakları (okuma, yazma, çalıştırma, kullanıcı erişimi vb.).
    /// Bu fonksiyon, MMU donanımı ile etkileşime girmelidir.
    pub fn map_page(vaddr: usize, paddr: usize, flags: u64) -> Result<(), KError> {
        // TODO: Mevcut görevin sayfa tablosunu bul.
        // TODO: Sayfa tablosuna vaddr -> paddr eşlemesini flags ile ekle.
        // TODO: Çakışma kontrolü, hizalama kontrolü, izin kontrolü yap.
        // TODO: MMU donanımına yaz (örneğin, page table entry - PTE güncelle).
        // Placeholder Simülasyon:
         println!("VMM: Mapping vaddr {:x} to paddr {:x} with flags {:b}", vaddr, paddr, flags); // Çekirdek içi print! gerektirir
        Ok(()) // Başarı simülasyonu
    }

    /// Mevcut görevin sayfa tablosundan sanal bir adresi (vaddr) eşleşmesini kaldırır.
    /// vaddr: Eşleşmesi kaldırılacak sanal adres (sayfa hizalı olmalı).
    /// Bu fonksiyon, MMU donanımı ile etkileşime girmelidir.
    pub fn unmap_page(vaddr: usize) -> Result<(), KError> {
        // TODO: Mevcut görevin sayfa tablosunu bul.
        // TODO: Sayfa tablosundan vaddr eşleşmesini kaldır.
        // TODO: TLB (Translation Lookaside Buffer) girdisini geçersiz kıl (TLB shootdown).
        // Placeholder Simülasyon:
         println!("VMM: Unmapping vaddr {:x}", vaddr); // Çekirdek içi print! gerektirir
        Ok(()) // Başarı simülasyonu
    }

    /// Mevcut görevin adres alanında belirtilen boyutta (size) boş bir sanal adres aralığı bulur.
    /// size: İstenen aralığın boyutu (sayfa hizalı olmalı).
    /// Bulunan başlangıç sanal adresini veya bulunamazsa None döner.
    pub fn find_free_virtual_address_range(size: usize) -> Option<usize> {
        // TODO: Mevcut görevin sanal adres alanı haritasını (Virtual Memory Area - VMA listesi gibi) tara.
        // TODO: Belirtilen boyutta bitişik ve boş bir aralık bul.
        // TODO: Bulunan aralığın başlangıç adresini döndür.
        // Basit Simülasyon: Kullanıcı alanı sanal adresleri için artan bir adres döndür.
        static mut NEXT_USER_VADDR: usize = 0x40000000; // Kullanıcı alanı başlangıcı varsayımı
        const PAGE_SIZE: usize = 4096;

        if size == 0 { return Some(0); } // Sıfır boyut için 0 döndürmek geçerli olabilir, veya InvalidArg.

        // Boyutu sayfa hizalı yapalım (çağıranın hizalı gönderdiği varsayılabilir ama kontrol iyi olur)
         let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        // Bu fonksiyon genellikle page-aligned size alır.

        unsafe {
             // Çok basit 'ilk uygun' yerleştirme simülasyonu
             let found_vaddr = NEXT_USER_VADDR;
             NEXT_USER_VADDR += size; // Bir sonraki tahsis için adresi ilerlet
             // Gerçekte, bu vaddr'ın boş olup olmadığını kontrol etmek gerekir!
             Some(found_vaddr)
        }
        // TODO: Gerçekte, bir boş VMA listesi tutulmalı ve bu liste aranmalıdır.
    }

    // Bellek koruma bayrakları (örnek)
    pub const PAGE_READ: u64 = 1 << 0;
    pub const PAGE_WRITE: u64 = 1 << 1;
    pub const PAGE_EXECUTE: u64 = 1 << 2;
    pub const PAGE_USER: u64 = 1 << 3; // Kullanıcı kipinden erişilebilir
    // TODO: Kernel, Read-Only, No-Execute vb. bayraklar
}


// --- Dahili Bellek Yönetimi Veri Yapıları ---
// Kullanıcı alanına tahsis edilen bellek bölgelerini ve paylaşımlı bellek nesnelerini izlemek için
// basit, 'no_std' uyumlu (statik diziler kullanan) kavramsal yapılar.

const MAX_USER_ALLOCATIONS_PER_TASK: usize = 64; // Bir görev için maksimum eş zamanlı tahsis sayısı varsayımı
// Not: Gerçek bir çekirdekte, bu yapı göreve özel (per-task) olmalı ve dinamik listeler/ağaçlar kullanılmalıdır.
// Basitlik adına, global statik dizi ve tek görev varsayımı yapıyoruz.

struct UserAllocRegion {
    vaddr: usize, // Sanal adres
    size: usize,  // Boyut
    is_used: bool, // Kullanımda mı?
    // TODO: Bu tahsise karşılık gelen fiziksel çerçevelerin listesi/haritası tutulmalı
    // veya VMM'den vaddr -> paddr lookup yapılabilmelidir.
}

// Global statik dizi (tek görev için tahsisleri izlemek üzere basit yer tutucu)
static mut USER_ALLOCATED_REGIONS: [UserAllocRegion; MAX_USER_ALLOCATIONS_PER_TASK] = [
    UserAllocRegion { vaddr: 0, size: 0, is_used: false }; MAX_USER_ALLOCATIONS_PER_TASK
];


const MAX_SHARED_MEM_OBJECTS: usize = 32; // Maksimum paylaşımlı bellek nesnesi sayısı varsayımı

struct SharedMemObject {
    id: u64, // Paylaşımlı bellek nesnesi için benzersiz kimlik (KHandle değeriyle eşleşebilir)
    size: usize, // Toplam boyutu (sayfa hizalı)
    physical_frames: [usize; 64], // Nesneye ait fiziksel çerçevelerin listesi (sabit boyut, basitleştirme)
    frame_count: usize, // Kaç fiziksel çerçeve kullanılıyor
    ref_count: usize, // Kaç KHandle veya kaç görev bu nesneye referans veriyor/haritalamış?
    is_used: bool, // Bu slot kullanımda mı?
    // TODO: Bu nesneyi hangi görevlerin hangi sanal adreslere haritaladığını izleyen bir yapı (map listesi)
}

// Global statik dizi (paylaşımlı bellek nesnelerini izlemek için basit yer tutucu)
static mut SHARED_MEM_OBJECTS: [SharedMemObject; MAX_SHARED_MEM_OBJECTS] = [
    SharedMemObject { id: 0, size: 0, physical_frames: [0; 64], frame_count: 0, ref_count: 0, is_used: false }; MAX_SHARED_MEM_OBJECTS
];
static mut NEXT_SHARED_MEM_ID: u64 = 1; // Yeni nesnelere benzersiz ID atamak için

/// Dahili yardımcı: Paylaşımlı bellek nesnesini ID'sine göre bulur.
fn get_shared_mem_object_mut(id: u64) -> Option<&'static mut SharedMemObject> {
    unsafe {
        SHARED_MEM_OBJECTS.iter_mut().find(|obj| obj.is_used && obj.id == id)
    }
}

/// Dahili yardımcı: KHandle değerini kullanarak paylaşımlı bellek nesnesini bulur.
/// Basitlik adına, KHandle değerinin doğrudan nesne ID'si olduğunu varsayıyoruz.
/// Gerçekte, KHandle, çekirdeğin genel handle tablosunda bu nesneye işaret eden bir girişe karşılık gelir.
fn get_shared_mem_object_by_handle_value_mut(handle_value: u64) -> Option<&'static mut SharedMemObject> {
     get_shared_mem_object_mut(handle_value)
}


// --- Karnal64 kmemory Modülü Implementasyonu ---
// Bu modül, karnal64.rs'te tanımlanan public API fonksiyonlarının gerçek mantığını içerir.
// handle_syscall fonksiyonu bu fonksiyonları çağırır.

pub mod kmemory {
    use super::*; // Üst kapsamdaki öğelere (KError, KHandle, pma, vmm, statik diziler) erişim sağlar

    /// Bellek yöneticisini başlatır. Çekirdek başlatılırken Karnal64 init() tarafından çağrılır.
    pub fn init_manager() {
        // TODO: PMA, VMM ve dahili izleme yapılarını gerçekte başlat.
        // Statik diziler zaten derleme zamanında başlatılır, ancak ek mantık gerekebilir.
        unsafe {
             // Konsol çıktısı varsa kullanılabilir
              println!("Karnal64: Bellek Yöneticisi Başlatıldı (src/memory.rs implementasyonu)");
        }
    }

    /// Kullanıcı alanı için bellek tahsis eder. handle_syscall tarafından çağrılır (SYSCALL_MEMORY_ALLOCATE).
    /// size: İstenen tahsis boyutu (byte). Sayfa hizalı olması beklenir veya hizalanır.
    /// Başarı durumunda kullanıcı alanındaki tahsis edilmiş bellek bloğunun sanal adresini döner.
    pub fn allocate_user_memory(size: usize) -> Result<*mut u8, KError> {
        if size == 0 {
            return Ok(core::ptr::null_mut()); // Sıfır byte tahsisi geçerli, null dönebilir
        }
        // İstenen boyutu sayfa hizalı yapalım
        const PAGE_SIZE: usize = 4096;
        let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        let num_pages = aligned_size / PAGE_SIZE;

        // Görevin adres alanında boş bir sanal adres aralığı bul
        let vaddr = match vmm::find_free_virtual_address_range(aligned_size) {
            Some(addr) => addr,
            None => return Err(KError::OutOfMemory), // Uygun sanal adres aralığı yok
        };

        // Fiziksel çerçeveleri tahsis et ve sanal adrese eşle
        let mut allocated_physical_frames: [usize; 64]; // Yeterince büyük dizi veya Vector (alloc varsa)
        // Basitlik adına, statik dizimizin boyutunu kontrol edelim
        if num_pages > 64 { // VEYA user_allocated_regions içindeki fiziksel çerçeve listesi boyutu
             // Bu implementasyon bu kadar büyük bir tahsisi desteklemiyor
             // TODO: Daha dinamik fiziksel frame yönetimi
             return Err(KError::OutOfMemory);
        }
        allocated_physical_frames = [0; 64];


        for i in 0..num_pages {
            let paddr = match pma::alloc_frame() {
                Some(addr) => addr,
                None => {
                    // Fiziksel bellek tahsisi başarısız oldu. Şimdiye kadar tahsis edilenleri temizle.
                    // TODO: i'ye kadar tahsis edilen fiziksel çerçeveleri serbest bırak
                    // TODO: i'ye kadar yapılan sanal eşlemeleri kaldır
                    return Err(KError::OutOfMemory);
                }
            };
            allocated_physical_frames[i] = paddr;

            // Mevcut görevin sayfa tablosunda sanal adresi fiziksel adrese eşle
            let current_vaddr_page = vaddr + i * PAGE_SIZE;
            let flags = vmm::PAGE_READ | vmm::PAGE_WRITE | vmm::PAGE_USER; // Kullanıcı R/W izinleri
            if let Err(e) = vmm::map_page(current_vaddr_page, paddr, flags) {
                 // Eşleme başarısız oldu. Şimdiye kadar tahsis edilen fiziksel ve sanal belleği temizle.
                 // TODO: i'ye kadar tahsis edilen fiziksel çerçeveleri serbest bırak
                 // TODO: i'ye kadar yapılan sanal eşlemeleri kaldır
                 return Err(e); // VMM'den gelen hatayı döndür
            }
        }

        // Tahsisi dahili izleme yapımıza kaydet
        let mut found_slot = false;
        unsafe {
            for region in &mut USER_ALLOCATED_REGIONS {
                if !region.is_used {
                    region.vaddr = vaddr;
                    region.size = aligned_size;
                    region.is_used = true;
                    // TODO: Burada bu bölgeyle ilişkili fiziksel çerçeveleri de kaydetmek gerekir
                    // free_user_memory fonksiyonu için.
                    found_slot = true;
                    break;
                }
            }
        }

        if !found_slot {
            // İzleme dizisi dolu. Bu basit implementasyonun bir sınırlaması.
            // TODO: Tahsis edilen tüm bellekleri (fiziksel ve sanal eşlemeler) serbest bırak
            return Err(KError::OutOfMemory); // İzleme slotları dolu
        }

        Ok(vaddr as *mut u8) // Kullanıcı alanındaki başlangıç sanal adresini döndür
    }

    /// Kullanıcı alanı için tahsis edilmiş belleği serbest bırakır. handle_syscall tarafından çağrılır (SYSCALL_MEMORY_RELEASE).
    /// ptr: Serbest bırakılacak bellek bloğunun başlangıç sanal adresi.
    /// size: Serbest bırakılacak bellek bloğunun boyutu.
    pub fn free_user_memory(ptr: *mut u8, size: usize) -> Result<(), KError> {
        if ptr.is_null() || size == 0 {
            return Ok(()); // Null pointer veya sıfır boyut serbest bırakmak bir şey yapmaz
        }
        let vaddr = ptr as usize;
         const PAGE_SIZE: usize = 4096;

         // Boyutun sayfa hizalı olduğunu varsayalım veya kontrol edelim/hizalayalım
         let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
         let num_pages = aligned_size / PAGE_SIZE;


        // Dahili izleme yapımızda bu bölgeyi bul
        let mut found_region_index = None;
        unsafe {
            for i in 0..MAX_USER_ALLOCATIONS_PER_TASK {
                let region = &mut USER_ALLOCATED_REGIONS[i];
                if region.is_used && region.vaddr == vaddr && region.size == aligned_size {
                    found_region_index = Some(i);
                    break;
                }
            }
        }

        let region_index = match found_region_index {
            Some(index) => index,
            None => return Err(KError::InvalidArgument), // Geçerli bir tahsis edilmiş bölge değil
        };

        // Sanal eşlemeleri kaldır ve fiziksel çerçeveleri serbest bırak
        for i in 0..num_pages {
            let current_vaddr_page = vaddr + i * PAGE_SIZE;

            // TODO: vaddr -> paddr lookup yaparak fiziksel adresi bul (VMM veya tahsis kaydı)
            // Placeholder: Simülasyon unmap
             if let Err(e) = vmm::unmap_page(current_vaddr_page) {
                 // Eşlemeyi kaldıramama durumu. Ne yapmalıyız? (Logla, devam et?)
                  println!("Warning: Failed to unmap page at vaddr {:x} during free_user_memory: {:?}", current_vaddr_page, e); // Çekirdek içi print!
                 // Devam edelim, diğer sayfaları temizlemeye çalışalım.
             }

             // TODO: Fiziksel çerçeveyi serbest bırak (pma::free_frame(paddr))
        }

        // İzleme yapımızdaki slotu boşalt
        unsafe {
            let region = &mut USER_ALLOCATED_REGIONS[region_index];
            region.is_used = false;
            region.vaddr = 0; // Bilgileri temizle
            region.size = 0;
            // TODO: İlişkili fiziksel çerçeve listesini de temizle
        }


        Ok(())
    }

    /// Paylaşımlı bir bellek bölgesi oluşturur. handle_syscall tarafından çağrılır (SYSCALL_SHARED_MEM_CREATE).
    /// size: İstenen paylaşımlı bellek boyutu (byte). Sayfa hizalı yapılır.
    /// Başarı durumunda paylaşımlı bellek nesnesi için bir KHandle döner.
    pub fn shared_mem_create(size: usize) -> Result<KHandle, KError> {
         if size == 0 {
             return Err(KError::InvalidArgument); // Sıfır boyutlu paylaşımlı bellek oluşturulamaz
         }
         const PAGE_SIZE: usize = 4096;
         let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
         let num_pages = aligned_size / PAGE_SIZE;

         // Basit SharedMemObject yapımızın frame listesi boyutunu kontrol edelim
         if num_pages > 64 {
             return Err(KError::InvalidArgument); // Bu kadar büyük paylaşımlı bellek desteklenmiyor
         }

         // Paylaşımlı bellek nesnesi için boş bir slot bul
         let mut object_index = None;
         unsafe {
             for i in 0..MAX_SHARED_MEM_OBJECTS {
                 if !SHARED_MEM_OBJECTS[i].is_used {
                     object_index = Some(i);
                     break;
                 }
             }
         }

         let object_index = match object_index {
             Some(index) => index,
             None => return Err(KError::OutOfMemory), // Paylaşımlı bellek nesne slotları dolu
         };

         // Paylaşımlı bellek için fiziksel çerçeveleri tahsis et
         let mut physical_addrs: [usize; 64] = [0; 64];
         for i in 0..num_pages {
             let paddr = match pma::alloc_frame() {
                 Some(addr) => addr,
                 None => {
                     // Fiziksel bellek tahsisi başarısız. Şimdiye kadar tahsis edilenleri temizle.
                     // TODO: i'ye kadar tahsis edilen fiziksel çerçeveleri serbest bırak
                     return Err(KError::OutOfMemory);
                 }
             };
             physical_addrs[i] = paddr;
         }

         // Paylaşımlı bellek nesnesini başlat
         let object_id = unsafe {
              let id = NEXT_SHARED_MEM_ID;
              NEXT_SHARED_MEM_ID += 1; // Sonraki ID'yi ayarla
              id
         };

         unsafe {
             let obj = &mut SHARED_MEM_OBJECTS[object_index];
             obj.id = object_id;
             obj.size = aligned_size;
             // Tahsis edilen fiziksel çerçeveleri kaydet
             obj.physical_frames[0..num_pages].copy_from_slice(&physical_addrs[0..num_pages]);
             obj.frame_count = num_pages;
             obj.ref_count = 0; // Başlangıçta kimse haritalamadı veya handle almadı
             obj.is_used = true;
         }

         // Bu nesne için bir KHandle oluştur.
         // Gerçek bir çekirdekte, kresource yöneticisi bu nesneye işaret eden bir handle verir.
         // Basitlik adına, nesne ID'sini KHandle değeri olarak kullanıyoruz.
         let shared_mem_handle = KHandle(object_id);

         // TODO: Bu paylaşımlı bellek nesnesini kresource yöneticisine kaydet.
         // Kaynak yöneticisi, kullanıcıdan gelen handle değerini çözerek bu nesneye ulaşabilmelidir.
         // Bu, SharedMemObject için bir ResourceProvider implementasyonu veya kresource'un
         // farklı çekirdek nesne türlerini (bellek, kilit vb.) tanımasıyla yapılabilir.
          kresource::register_kmemory_resource(object_id, &SHARED_MEM_OBJECTS[object_index]);

         Ok(shared_mem_handle)
    }

    /// Paylaşımlı bir bellek bölgesini mevcut görevin sanal adres alanına haritalar (map).
    /// handle_syscall tarafından çağrılır (SYSCALL_SHARED_MEM_MAP).
    /// k_handle_value: Paylaşımlı bellek nesnesinin raw handle değeri.
    /// offset: Paylaşımlı bellek içindeki haritalamaya başlanacak ofset (byte, sayfa hizalı).
    /// size: Haritalanacak bölgenin boyutu (byte, sayfa hizalı).
    /// Başarı durumunda kullanıcı alanındaki haritalanmış bölgenin başlangıç sanal adresini döner.
    pub fn shared_mem_map(k_handle_value: u64, offset: usize, size: usize) -> Result<*mut u8, KError> {
         if size == 0 {
             return Ok(core::ptr::null_mut()); // Sıfır boyut haritalama geçerli
         }
         const PAGE_SIZE: usize = 4096;
         if offset % PAGE_SIZE != 0 || size % PAGE_SIZE != 0 {
             return Err(KError::InvalidArgument); // Ofset veya boyut sayfa hizalı değil
         }

         // Handle değerini kullanarak paylaşımlı bellek nesnesini bul
         let obj = match get_shared_mem_object_by_handle_value_mut(k_handle_value) {
             Some(obj) => obj,
             None => return Err(KError::BadHandle), // Geçersiz paylaşımlı bellek handle'ı
         };

         // Ofset ve boyutun nesne sınırları içinde olduğunu kontrol et
         if offset + size > obj.size {
             return Err(KError::InvalidArgument); // Haritalama isteği sınırlar dışında
         }

         let start_page_index_in_obj = offset / PAGE_SIZE;
         let num_pages_to_map = size / PAGE_SIZE;

         // Mevcut görevin adres alanında haritalama için boş bir sanal adres aralığı bul
         let vaddr = match vmm::find_free_virtual_address_range(size) {
             Some(addr) => addr,
             None => return Err(KError::OutOfMemory), // Uygun sanal adres aralığı yok
         };

         // Paylaşımlı belleğe ait ilgili fiziksel çerçeveleri görevin sanal adres alanına eşle
         for i in 0..num_pages_to_map {
             let physical_frame_index_in_obj_list = start_page_index_in_obj + i;
             // Bounds check (obj.physical_frames ve obj.frame_count)
             if physical_frame_index_in_obj_list >= obj.frame_count {
                 // Bu durum, ofset/size kontrolü doğruysa olmamalı. Dahili hata.
                 // TODO: Kısmi haritalamaları temizle
                 return Err(KError::InternalError);
             }
             let paddr_to_map = obj.physical_frames[physical_frame_index_in_obj_list];

             let current_vaddr_page = vaddr + i * PAGE_SIZE;

             let flags = vmm::PAGE_READ | vmm::PAGE_WRITE | vmm::PAGE_USER; // Paylaşımlı bellek genellikle R/W kullanıcı erişimli
             if let Err(e) = vmm::map_page(current_vaddr_page, paddr_to_map, flags) {
                 // Eşleme başarısız. Şimdiye kadar yapılan eşlemeleri temizle.
                 // TODO: i'ye kadar yapılan sanal eşlemeleri kaldır
                 return Err(e); // VMM'den gelen hatayı döndür
             }
         }

         // Paylaşımlı bellek nesnesinin referans sayısını artır (bu haritalamayı izlemek için)
         obj.ref_count += 1;

         // TODO: Gerçek bir çekirdekte, bu görevin hangi sanal adres aralığının hangi paylaşımlı bellek nesnesine
         // haritalandığını izleyen bir yapı tutulmalıdır (göreve özel VMA listesi içinde).
         // Bu, unmap ve free işlemlerinde gereklidir.

         Ok(vaddr as *mut u8) // Kullanıcı alanındaki haritalanmış bölgenin başlangıç sanal adresini döndür
    }

    /// Paylaşımlı bir bellek bölgesini mevcut görevin sanal adres alanından kaldırır (unmap).
    /// handle_syscall tarafından çağrılır (SYSCALL_SHARED_MEM_UNMAP).
    /// ptr: Haritalanmış bölgenin kullanıcı alanındaki başlangıç sanal adresi.
    /// size: Haritalanmış bölgenin boyutu.
    pub fn shared_mem_unmap(ptr: *mut u8, size: usize) -> Result<(), KError> {
         if ptr.is_null() || size == 0 {
             return Ok(()); // Null pointer veya sıfır boyut unmap bir şey yapmaz
         }
         let vaddr = ptr as usize;
         const PAGE_SIZE: usize = 4096;

         // Boyutun sayfa hizalı olduğunu varsayalım veya kontrol edelim/hizalayalım
         let aligned_size = (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
         let num_pages = aligned_size / PAGE_SIZE;

         // TODO: Hangi paylaşımlı bellek nesnesinin bu sanal adres aralığına haritalandığını bul.
         // Bu, görevin sanal adres alanı haritalarını (VMA listesi) sorgulayarak yapılır.
         // Basit izleme yapımız bu ters lookup'ı desteklemiyor.

         // Placeholder: Sanal eşlemeleri kaldır (fiziksel bellek serbest bırakılamaz çünkü hangi nesneye ait olduğunu bilmiyoruz)
         for i in 0..num_pages {
              let current_vaddr_page = vaddr + i * PAGE_SIZE;
              if let Err(e) = vmm::unmap_page(current_vaddr_page) {
                  // Eşlemeyi kaldıramama durumu. Logla, devam et.
                   println!("Warning: Failed to unmap page at vaddr {:x} during shared_mem_unmap: {:?}", current_vaddr_page, e); // Çekirdek içi print!
              }
         }

         // TODO: Eğer paylaşımlı bellek nesnesi bulunabilseydi, ref_count'unu azalt.
         // Eğer ref_count sıfıra inerse (hiçbir görev haritalamadı veya handle tutmuyor),
         // nesneye ait fiziksel çerçeveleri pma::free_frame kullanarak serbest bırak.

         Ok(())
    }

    // TODO: Karnal64 API'sındaki diğer bellek fonksiyonlarını ekle ve implemente et
    // (örneğin, memory_release aslında free_user_memory'yi çağırabilir,
    // shared_mem_release ise paylaşımlı bellek handle'ı serbest bırakıldığında çağrılacak dahili bir fonksiyon olabilir)

    // Bir paylaşımlı bellek handle'ı (KHandle) serbest bırakıldığında kresource tarafından çağrılacak olası dahili fonksiyon.
    // handle_value: Serbest bırakılan handle değeri.
    // Bu, doğrudan bir sistem çağrısı DEĞİLDİR.
    pub fn shared_mem_handle_released(handle_value: u64) {
        // TODO: Handle değeriyle ilişkili paylaşımlı bellek nesnesini bul.
        // Paylaşımlı bellek nesnesinin referans sayısını azalt (veya handle referans sayısını).
        // Eğer ref_count (veya handle count) sıfıra düşerse, fiziksel çerçeveleri serbest bırak.
        // Bu, kresource'un handle türüne göre ilgili modüle bildirim yapması gerektiğini gösterir.

        if let Some(obj) = get_shared_mem_object_by_handle_value_mut(handle_value) {
            // Basitlik: Hem haritalama hem de handle referanslarını aynı ref_count'ta tuttuğumuzu varsayalım.
            // Gerçekte bu muhtemelen ayrı takip edilmelidir.
             obj.ref_count -= 1; // Sadece handle serbest bırakılması ref_count'u azaltmayabilir!

            // Doğru yaklaşım: Paylaşımlı bellek nesnesi ResourceProvider trait'ini implemente etmeli ve
            // kresource, ResourceProvider::release metodunu çağırmalıdır.
            // shared_mem_unmap ise ayrı bir mekanizmadır.
              println!("Kmemory: Shared memory handle {} serbest bırakıldı. Fiziksel temizlik mantığı eksik.", handle_value); // Çekirdek içi print!
             // TODO: Fiziksel belleği serbest bırakma mantığı (obj.ref_count 0 olduğunda)
        } else {
              println!("Kmemory: Serbest bırakılan handle {} bilinen bir paylaşımlı bellek nesnesine karşılık gelmiyor.", handle_value); // Çekirdek içi print!
        }
    }


    // TODO: Gelişmiş bellek yönetimi özellikleri: mmap benzeri eşlemeler, bellek koruma ayarlarını değiştirme vb.
}
