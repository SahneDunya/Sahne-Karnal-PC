#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz, çekirdek alanında çalışır

// Geliştirme sırasında kullanılmayan kod veya argümanlar için izinler (isteğe bağlı, hata ayıklama sırasında faydalı)
#![allow(dead_code)]
#![allow(unused_variables)]

// Çekirdek API'sından (karnal64.rs) ihtiyaç duyulan tipleri içe aktar
// Gerçek projede, bu muhtemelen 'use super::KError;' veya 'use crate::karnal64::KError;' gibi bir şey olurdu.
// Bu örnek için KError'ı burada yeniden tanımlayalım ki dosya kendi başına derlenebilsin (tamamen bağımsız olması adına).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i64)] // KError'ın i64 olarak temsil edilmesi, sistem çağrısı dönüş değerleriyle uyum için önemli
pub enum KError {
    OutOfMemory = -12,         // Yetersiz bellek
    InvalidArgument = -3,      // Geçersiz argüman (örn: yanlış hizalama, adres aralık dışında)
    InternalError = -255,      // Dahili hata (beklenmedik durum)
    NotSupported = -38,        // İşlem desteklenmiyor (örn: henüz implemente edilmemiş çoklu frame tahsisi)
    Busy = -11,                // Kaynak meşgul (kullanılmıyor olabilir ama ekledik)
}

// Bellek haritası bilgisi için temel bir yapı (bootloader'dan gelir)
// Gerçek çekirdekte bu daha detaylı olurdu.
#[derive(Debug, Copy, Clone)]
#[repr(C)] // C uyumluluğu için
pub struct MemoryRegion {
    pub base_addr: u64, // Bölgenin başlangıç fiziksel adresi
    pub size: u64,      // Bölgenin boyutu
    pub kind: u32,      // Bölgenin türü (0: Reserved, 1: Usable, vb.)
    pub attribute: u32, // Ek nitelikler (şu an kullanılmıyor)
}

// Basit bir spinlock (çekirdek içi senkronizasyon için)
// ksync modülünden gelmesi beklenen primitiflerden biri.
// Burada kendi temel implementasyonunu sağlıyoruz.
mod ksync_internal {
    use core::cell::UnsafeCell;
    use core::sync::atomic::{AtomicBool, Ordering};

    pub struct Spinlock {
        locked: AtomicBool,
        // Gerçek çekirdekte hata ayıklama için kilit sahibi görev/iş parçacığı ID'si eklenebilir.
    }

    unsafe impl Sync for Spinlock {} // Statik başlatmaya ve paylaşılan erişime izin verir

    impl Spinlock {
        #[inline(always)] // Çağrıldığı yere gömülmesini tercih et
        pub const fn new() -> Self {
            Spinlock {
                locked: AtomicBool::new(false),
            }
        }

        #[inline(always)]
        pub fn acquire(&self) {
            // Atomik takas (swap): Kilit açıksa (false), kilitler (true) ve eski değeri (false) döner.
            // Kilitliyse (true), true döndürür ve kilitli kalır.
            while self.locked.swap(true, Ordering::Acquire) {
                // Kilitliyse, serbest bırakılana kadar meşgul döngüde bekle.
                // Gerçek çekirdekte, CPU'nun boşa dönmesini veya başka iş parçacığına geçmesini sağlayacak
                // mimariye özel talimatlar (örn: x86'da PAUSE) veya zamanlayıcıyla entegre bekleme kullanılmalıdır.
                core::hint::spin_loop(); // Rust'ın meşgul bekleme ipucu
            }
        }

        #[inline(always)]
        pub fn release(&self) {
            // Atomik mağaza (store): Kilidi serbest bırakır (false).
            self.locked.store(false, Ordering::Release);
        }
    }

    // RAII stili kilit koruyucusu
    pub struct SpinlockGuard<'a> {
        lock: &'a Spinlock,
    }

    impl Spinlock {
        #[inline(always)]
        pub fn lock(&self) -> SpinlockGuard {
            self.acquire();
            SpinlockGuard { lock: self }
        }
    }

    impl<'a> Drop for SpinlockGuard<'a> {
        fn drop(&mut self) {
            self.lock.release();
        }
    }
}
use ksync_internal::Spinlock; // Dahili spinlock implementasyonumuzu kullan

// --- Fiziksel Çerçeve Ayırıcı Yapısı ---

// Fiziksel bellek çerçevesinin boyutu (genellikle sayfa boyutu ile aynıdır)
const FRAME_SIZE: usize = 4096; // 4KB

// Yönetilecek fiziksel bellek aralığı.
// Bunlar bootloader'dan gelmeli. Örnek olarak sabit değerler kullanıyoruz.
// Başlangıç adresi: 0x10000000
// Toplam boyut: 64MB (64 * 1024 * 1024 bytes)
// Bu durumda toplam çerçeve sayısı: 64MB / 4KB = 16384 çerçeve
const PHYS_MEMORY_START: usize = 0x10000000;
const PHYS_MEMORY_SIZE: usize = 64 * 1024 * 1024;
const TOTAL_FRAMES: usize = PHYS_MEMORY_SIZE / FRAME_SIZE;

// Çerçeve durumunu (boş/dolu) takip etmek için bitmap.
// Her bit bir çerçeveyi temsil eder. 0: boş, 1: dolu.
// Boyut = (Toplam Çerçeve Sayısı + 7) / 8 (byte cinsinden, yuvarlama)
const BITMAP_SIZE: usize = (TOTAL_FRAMES + 7) / 8;
static mut FRAME_BITMAP: [u8; BITMAP_SIZE] = [0; BITMAP_SIZE];

// Tahsis edilmiş çerçeve sayısını tutmak için atomik sayaç (stat/debug için)
static ALLOCATED_FRAME_COUNT: AtomicUsize = AtomicUsize::new(0);

// Bitmap'e eş zamanlı erişimi korumak için spinlock
static BITMAP_LOCK: Spinlock = Spinlock::new();

// Yönetilen bellek bölgesinin sınırları (init sırasında belirlenir)
static mut MANAGED_RANGE: Option<core::ops::Range<usize>> = None;


// --- Public API / kmemory Entegrasyon Fonksiyonları ---

/// Fiziksel Çerçeve Ayırıcısını başlatır.
/// Bootloader'dan gelen kullanılabilir bellek haritasını alır.
/// Karnal64'teki kmemory modülünün init fonksiyonu tarafından çağrılır.
///
/// Güvenlik Notu: Gerçek bir çekirdekte, bootloader'dan gelen bellek haritası
/// bilgisi dikkatlice doğrulanmalı ve çekirdeğin kendi kapladığı alanlar,
/// aygıtlar için ayrılmış alanlar bitmap'de dolu olarak işaretlenmelidir.
pub fn init(memory_map: &[MemoryRegion]) -> Result<(), KError> {
    let mut bitmap_guard = BITMAP_LOCK.lock();

    unsafe {
        // Bitmap'i sıfırla (tüm çerçeveleri başlangıçta boş işaretle)
        ptr::write_bytes(FRAME_BITMAP.as_mut_ptr(), 0, BITMAP_SIZE);
        ALLOCATED_FRAME_COUNT.store(0, Ordering::SeqCst);
        MANAGED_RANGE = None; // Başlangıçta yönetilen aralık yok
    }

    // Bellek haritasını işle, ilk kullanılabilir (Usable) bölgeyi bul
    // ve bu bölgeyi yönetecek şekilde bitmap'i ayarla.
    // Bu çok basit bir yaklaşım, gerçek bir ayırıcı tüm kullanılabilir
    // bölgeleri yönetebilir.
    let mut found_managed_region = false;
    for region in memory_map.iter() {
        // Kullanılabilir RAM bölgeleri (genellikle E820 veya benzeri bir standartla belirlenir)
        const MEMORY_REGION_USABLE: u32 = 1; // Bu değer bootloader/BIOS standardına göre değişebilir

        if region.kind == MEMORY_REGION_USABLE && region.size > 0 {
            let region_start = region.base_addr as usize;
            let region_end = (region.base_addr + region.size) as usize;

            // Yönetmek istediğimiz sabit aralığı bulduğumuz bölgeye göre ayarla
            // Veya doğrudan bootloader'ın sağladığı aralığı yönet.
            // Örnek olarak sabit aralığı kullanmaya devam edelim ve bootloader'ın
            // en azından bu aralığı kullanılabilir olarak işaretlediğini varsayalım.
            let managed_start = PHYS_MEMORY_START;
            let managed_end = PHYS_MEMORY_START + PHYS_MEMORY_SIZE;

            // Bootloader'ın sağladığı kullanılabilir bölgenin, bizim yönetmek
            // istediğimiz sabit aralığı tamamen veya kısmen içerdiğini kontrol et.
            if region_start <= managed_start && region_end >= managed_end {
                 unsafe {
                    MANAGED_RANGE = Some(managed_start..managed_end);
                 }
                 found_managed_region = true;
                 // Kernel kodunun, veri bölümlerinin ve başlangıç yığınının
                 // yönetilen alanda olduğunu varsayalım ve bu çerçeveleri DOLU işaretleyelim.
                 // Bu kısım bootloader'dan gelen bilgiye (kernel'in yüklendiği adresler)
                 // ve linker script'ine göre yapılmalıdır. Şimdilik atlıyoruz.
                 break; // İlk bulunan uygun bölgeyi kullanıyoruz
            }
            // TODO: Eğer managed_start < region_start veya managed_end > region_end ise
            // veya bootloader haritasında hiç uygun bölge yoksa hata döndürmeliyiz.
        }
    }

    if !found_managed_region {
        // Kullanılabilir bellek bölgesi bulunamadı veya yönetmek istediğimiz aralık geçerli değil.
          println!("Error: Could not find a suitable usable memory region to manage."); // Kernel print needed
        return Err(KError::InternalError); // Veya spesifik bir init hatası
    }


    // TODO: Çekirdek kod/veri alanlarını bitmap'de dolu olarak işaretle.
    // Bu, linker script'inden gelen sembollere (_kernel_start, _kernel_end vb.)
    // veya bootloader tarafından sağlanan kernel adresine dayanır.

     println!("DDR Ayırıcısı Başlatıldı. Yönetilen Alan: {:#x}-{:#x}",
              PHYS_MEMORY_START, PHYS_MEMORY_START + PHYS_MEMORY_SIZE); // Kernel print needed
     println!("Toplam {} çerçeve ({} KB), Bitmap boyutu {} byte",
              TOTAL_FRAMES, TOTAL_FRAMES * FRAME_SIZE / 1024, BITMAP_SIZE);

    Ok(())
}


/// Bir veya daha fazla fiziksel bellek çerçevesi tahsis eder.
/// Bu fonksiyon, kmemory modülü tarafından, kullanıcı alanı bellek tahsisi
/// veya paylaşılan bellek gibi işlemler için çağrılır.
///
/// `num_frames`: Tahsis edilecek çerçeve sayısı.
/// `align_frames`: (İsteğe bağlı) Tahsisin başlaması gereken çerçeve hizalaması.
///                 (örn: 1MB hizalaması için 256 çerçeve hizalaması istenir)
///
/// Başarı durumunda, tahsis edilen ilk çerçevenin fiziksel adresini döner.
/// Hata durumunda KError döner.
pub fn allocate_physical_frames(num_frames: usize, align_frames: usize) -> Result<NonNull<u8>, KError> {
    if num_frames == 0 {
        return Err(KError::InvalidArgument);
    }
    if align_frames == 0 || !align_frames.is_power_of_two() {
        // Hizalama 0 olamaz ve 2'nin kuvveti olmalıdır.
        return Err(KError::InvalidArgument);
    }

    let mut bitmap_guard = BITMAP_LOCK.lock(); // Kilidi al

    // Bitmap'de yeterli sayıda (num_frames) ardışık (contiguous) boş çerçeve arar.
    // Hizalama (align_frames) gereksinimi varsa, arama buna göre başlar.
    // Bu, basit bir 'ilk uyan' (first-fit) algoritmasıdır. Daha gelişmiş algoritmalar
    // (örn: 'en iyi uyan' - best-fit) parçalanmayı azaltabilir.

    let mut current_frame_index = 0;
    while current_frame_index < TOTAL_FRAMES {

        // Hizalama gereksinimini kontrol et ve uygula
        if align_frames > 1 {
            let current_frame_addr = PHYS_MEMORY_START + current_frame_index * FRAME_SIZE;
            let align_bytes = align_frames * FRAME_SIZE;

            // Eğer mevcut adres istenen hizalamada değilse, bir sonraki hizalı adrese atla.
            if current_frame_addr % align_bytes != 0 {
                let next_aligned_addr = (current_frame_addr / align_bytes + 1) * align_bytes;
                current_frame_index = (next_aligned_addr - PHYS_MEMORY_START) / FRAME_SIZE;
                // Atladıktan sonra toplam çerçeve sayısını aşmış olabiliriz
                if current_frame_index >= TOTAL_FRAMES {
                    break; // Döngüyü kır
                }
                continue; // Hizalamayı sağladık, şimdi bu indexten itibaren kontrol et
            }
        }


        // Mevcut indexten itibaren num_frames kadar ardışık çerçeve boş mu?
        let mut is_block_free = true;
        if current_frame_index + num_frames > TOTAL_FRAMES {
            // Yeterli sayıda çerçeve kalmadı
            is_block_free = false;
        } else {
            unsafe {
                for i in 0..num_frames {
                    let frame_index = current_frame_index + i;
                    let byte_index = frame_index / 8;
                    let bit_index_in_byte = frame_index % 8;
                    let bit_mask = 1 << bit_index_in_byte;

                    // Eğer herhangi bir çerçeve doluysa (bit 1 ise)
                    if (FRAME_BITMAP[byte_index] & bit_mask) != 0 {
                        is_block_free = false;
                        // Boş bloğu bulamadık, aramaya bir sonraki çerçevenin başından devam et
                        // (kontrol ettiğimiz bloğun başından değil!)
                        current_frame_index = frame_index + 1;
                        break; // İç döngüyü kır, dış döngü devam etsin
                    }
                }
            }
        }

        // Eğer blok boş bulunduysa
        if is_block_free {
            // Çerçeveleri dolu olarak işaretle (bitleri 1 yap)
            unsafe {
                for i in 0..num_frames {
                    let frame_index = current_frame_index + i;
                    let byte_index = frame_index / 8;
                    let bit_index_in_byte = frame_index % 8;
                    let bit_mask = 1 << bit_index_in_byte;
                    FRAME_BITMAP[byte_index] |= bit_mask;
                }
            }

            // Tahsis edilen sayıyı güncelle
            ALLOCATED_FRAME_COUNT.fetch_add(num_frames, Ordering::SeqCst);

            // Tahsis edilen bloğun başlangıç fiziksel adresini hesapla
            let phys_addr = PHYS_MEMORY_START + current_frame_index * FRAME_SIZE;

            // Kilidi serbest bırak ve adresi döndür
            return Ok(unsafe { NonNull::new_unchecked(phys_addr as *mut u8) });
        }

        // Eğer blok boş değilse ve iç döngü kırılmadıysa (yani tek çerçeveli blok aranıyorsa)
        // veya multi-frame blok kontrolünden sonra is_block_free false ise,
        // arama bir sonraki çerçevenin başından devam eder (current_frame_index zaten güncellendi).
        // Eğer iç döngü kırıldıysa (bir dolu çerçeve bulunduysa), current_frame_index
        // zaten dolu çerçevenin bir fazlasına ayarlanmıştır.
        // Eğer is_block_free false olduysa ama iç döngü kırılmadıysa (yalnızca TOTAL_FRAMES kontrolü),
        // current_frame_index'i bir artırıp devam etmemiz gerekir.
        if !is_block_free && current_frame_index + num_frames <= TOTAL_FRAMES {
             current_frame_index += 1;
        }
         // Eğer is_block_free false olduysa ve iç döngü Kırıldıysa, current_frame_index
         // zaten doğru şekilde ayarlanmış oluyor.

    } // Döngü sonu

    // Tüm bitmap arandı ama uygun boş blok bulunamadı
    Err(KError::OutOfMemory) // Kilidi döndürmeden önce Drop trait'i serbest bırakır
}

/// Daha önce tahsis edilmiş bir veya daha fazla fiziksel çerçeveyi serbest bırakır.
///
/// `frame_ptr`: Serbest bırakılacak ilk çerçevenin fiziksel adresi.
/// `num_frames`: Serbest bırakılacak ardışık çerçeve sayısı.
///
/// Başarı durumunda Ok(()), hata durumunda KError döner.
/// Güvenlik Notu: `frame_ptr`'nin gerçekten daha önce bu ayırıcıdan
/// tahsis edilmiş geçerli bir adres olduğu doğrulanmalıdır.
pub fn free_physical_frames(frame_ptr: NonNull<u8>, num_frames: usize) -> Result<(), KError> {
    let phys_addr = frame_ptr.as_ptr() as usize;

    // Temel doğrulama: Adresin yönetilen aralıkta ve çerçeve hizalı olup olmadığını kontrol et.
    if phys_addr < PHYS_MEMORY_START ||
       phys_addr >= PHYS_MEMORY_START + PHYS_MEMORY_SIZE ||
       (phys_addr - PHYS_MEMORY_START) % FRAME_SIZE != 0 ||
       num_frames == 0 || // Sıfır çerçeve serbest bırakmak geçerli değil
       (phys_addr - PHYS_MEMORY_START) / FRAME_SIZE + num_frames > TOTAL_FRAMES // Serbest bırakılan blok sınırları aşıyor
    {
         println!("Free_physical_frames: Invalid argument - addr {:#x}, num_frames {}", phys_addr, num_frames); // Kernel print needed
        return Err(KError::InvalidArgument);
    }

    // Çerçevenin başlangıç indeksini hesapla
    let start_frame_index = (phys_addr - PHYS_MEMORY_START) / FRAME_SIZE;

    let mut bitmap_guard = BITMAP_LOCK.lock(); // Kilidi al

    // Serbest bırakılacak çerçeveleri dolu olarak işaretliyse (bit 1) kontrol et
    // ve ardından boş olarak işaretle (bitleri 0 yap).
    unsafe {
        for i in 0..num_frames {
            let frame_index = start_frame_index + i;
            let byte_index = frame_index / 8;
            let bit_index_in_byte = frame_index % 8;
            let bit_mask = 1 << bit_index_in_byte;

            // Çerçevenin dolu olduğundan emin ol (bir hata kontrolü)
            if (FRAME_BITMAP[byte_index] & bit_mask) == 0 {
                 // println!("Free_physical_frames: Attempted to free a free or invalid frame at index {} (address {:#x})", frame_index, phys_addr + i * FRAME_SIZE); // Kernel print needed
                // Bir hata oluştu: serbest bırakılmaya çalışılan çerçeve zaten boş.
                // Bu, aynı çerçeve bloğunun iki kere serbest bırakıldığı anlamına gelebilir.
                // Bu durumda, o ana kadar serbest bırakılmış çerçeveleri geri "dolu" yapmamak gerekir,
                // ancak hata durumunu açıkça belirtmeliyiz.
                // Kilit serbest bırakılmadan çıkıyoruz.
                return Err(KError::InternalError); // Veya KError::InvalidArgument
            }

            // Çerçeveyi boş olarak işaretle (biti 0 yap)
            FRAME_BITMAP[byte_index] &= !bit_mask;
        }
    } // Kilit serbest bırakılır

    // Tahsis edilen sayıyı güncelle
    ALLOCATED_FRAME_COUNT.fetch_sub(num_frames, Ordering::SeqCst);

    Ok(())
}

/// Yönetilen toplam fiziksel çerçeve sayısını döndürür.
pub fn total_frames() -> usize {
    TOTAL_FRAMES
}

/// Şu anda tahsis edilmiş fiziksel çerçeve sayısını döndürür.
pub fn allocated_frames() -> usize {
    ALLOCATED_FRAME_COUNT.load(Ordering::SeqCst)
}

/// Fiziksel adresin yönetilen alanda olup olmadığını kontrol eder.
pub fn is_managed_physical_address(phys_addr: usize) -> bool {
    match unsafe { MANAGED_RANGE } {
        Some(ref range) => range.contains(&phys_addr),
        None => false, // Henüz başlatılmadıysa hiçbir adres yönetilmiyor
    }
}

// TODO: Diğer ihtiyaç duyulabilecek fonksiyonlar:
// - Fiziksel adresten çerçeve indeksini bulma
// - Çerçeve indeksinden fiziksel adresi bulma
// - Belirli bir çerçeve bloğunun durumunu sorgulama


// --- kmemory Modülü İçin Kullanılacak Yüksek Seviye Fonksiyonlar (İskelet) ---
// Bu fonksiyonlar, karnal64.rs'deki kmemory modülü tarafından çağrılacaktır.
// Bunlar, fiziksel ayırıcının üzerine kurulu daha yüksek seviye bellek yönetimi işlevleridir.
// Gerçek implementasyonları sanal bellek (sayfa tabloları) yönetimini içerecektir.

// Bu fonksiyon kmemory::init_manager() tarafından çağrılır.
// Buradaki init fonksiyonumuzu çağırır.
pub fn memory_manager_init(memory_map: &[MemoryRegion]) -> Result<(), KError> {
    init(memory_map)
}

// Bu fonksiyon handle_syscall'dan SYSCALL_MEMORY_ALLOCATE için çağrılır (kmemory üzerinden).
// Kullanıcı alanı için bellek tahsis eder (sanal bellek yönetimi burada başlar).
// Şu an sadece fiziksel çerçeve tahsis eder, sanal eşleme yapmaz.
pub fn allocate_user_memory(size: usize) -> Result<NonNull<u8>, KError> {
    // Kullanıcı alanı tahsisi genellikle sayfa boyutunda hizalanmalıdır.
    if size == 0 {
        return Err(KError::InvalidArgument);
    }
    let size_in_frames = (size + FRAME_SIZE - 1) / FRAME_SIZE;

    // TODO: Gerçek kullanıcı alanı tahsisi:
    // 1. allocate_physical_frames ile fiziksel çerçeve(ler) tahsis et.
    // 2. Mevcut görevin sanal adres alanında uygun bir sanal adres bul (örn: heap alanı).
    // 3. Sayfa tablosunu güncelleyerek sanal adresleri fiziksel çerçevelere eşle.
    // 4. Tahsis edilen sanal adresin başlangıcını döndür.

    // Basitlik adına, şimdilik sadece bir fiziksel çerçeve tahsis edelim ve onun adresini döndürelim.
    // Bu adres doğrudan kullanıcı alanında geçerli olmayabilir! Sanal eşleme GEREKLİ.
    let num_frames = 1; // Şimdilik sadece 1 çerçeve
    let align_frames = 1; // Hizalama yok (veya 1 çerçeve hizalama)
    match allocate_physical_frames(num_frames, align_frames) {
        Ok(phys_ptr) => {
            // Gerçek implementasyonda, burada phys_ptr'yi kullanıcı alanındaki bir sanal adrese eşlerdik.
            // Şimdilik fiziksel adresi (Non-null pointer olarak) döndürüyoruz, bu KESİNLİKLE YANLIŞTIR
            // gerçek bir çekirdekte sanal bellek olmadan yapılamaz.
              println!("allocate_user_memory: Allocated physical frame {:#p}, VIRTUAL mapping needed!", phys_ptr.as_ptr()); // Kernel print needed
            Ok(phys_ptr) // DİKKAT: Burası sanal adres DÖNDÜRMELİ, fiziksel değil!
        },
        Err(err) => Err(err),
    }
}

// Bu fonksiyon handle_syscall'dan SYSCALL_MEMORY_RELEASE için çağrılır (kmemory üzerinden).
// Kullanıcı alanı belleğini serbest bırakır (sanal bellek yönetimi burada devam eder).
// Şu an sadece fiziksel çerçeveyi serbest bırakır, sanal eşlemeyi kaldırmaz.
pub fn free_user_memory(ptr: NonNull<u8>, size: usize) -> Result<(), KError> {
     if size == 0 {
        return Err(KError::InvalidArgument);
     }
    let size_in_frames = (size + FRAME_SIZE - 1) / FRAME_SIZE;

    // TODO: Gerçek kullanıcı alanı serbest bırakma:
    // 1. Verilen sanal adresten (ptr) başlayarak eşlenmiş fiziksel çerçeveleri bul.
    // 2. Sayfa tablosundan sanal-fiziksel eşlemeyi kaldır.
    // 3. free_physical_frames ile fiziksel çerçeveleri serbest bırak.

    // Basitlik adına, verilen pointer'ın (yanlışlıkla fiziksel olduğunu varsaydığımız)
    // 1 çerçevelik bir tahsis olduğunu varsayalım ve o çerçeveyi serbest bırakalım.
    // Bu KESİNLİKLE YANLIŞTIR gerçek bir çekirdekte sanal bellek olmadan yapılamaz.
    let num_frames = 1; // Şimdilik sadece 1 çerçeve
     if size_in_frames > 1 {
          println!("free_user_memory: Only single frame deallocation supported in this basic example."); // Kernel print needed
         return Err(KError::NotSupported); // Çoklu çerçeve serbest bırakma desteklenmiyor
     }
     if (ptr.as_ptr() as usize - PHYS_MEMORY_START) % FRAME_SIZE != 0 {
           println!("free_user_memory: Pointer {:#p} is not frame-aligned for basic free.", ptr.as_ptr()); // Kernel print needed
          return Err(KError::InvalidArgument); // Çerçeve hizalı değilse basit serbest bırakma yapamayız
     }


    // DİKKAT: ptr burada sanal bir adres olmalı, fiziksel değil.
    // Aşağıdaki çağrı, sanki ptr fiziksel bir adresmiş gibi free_physical_frames'i çağırıyor.
    // Bu yanlıştır ve gerçek implementasyonda sanal adres -> fiziksel adres çevirisi GEREKLİDİR.
      println!("free_user_memory: Attempting to free physical frame at {:#p}", ptr.as_ptr()); // Kernel print needed
    free_physical_frames(ptr, num_frames) // DİKKAT: Bu çağrı doğrudan fiziksel adresi bekler, sanal değil!
}

// TODO: Paylaşılan bellek fonksiyonları (shared_mem_create, map, unmap) implementasyonu
// Bunlar da sanal bellek yönetimini ve farklı görevlerin aynı fiziksel çerçeveleri kendi
// sanal adres alanlarına eşlemesini gerektirir.
 pub fn shared_mem_create(size: usize) -> Result<KHandle, KError> { /* ... */ }
 pub fn shared_mem_map(k_handle_value: u64, offset: usize, size: usize) -> Result<*mut u8, KError> { /* ... */ }
 pub fn shared_mem_unmap(ptr: *mut u8, size: usize) -> Result<(), KError> { /* ... */ }
