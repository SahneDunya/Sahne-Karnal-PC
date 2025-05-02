#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz
#[allow(dead_code)] // Henüz kullanılmayan kodlar için uyarı vermesin - Let's keep for now

// Use the Sahne64 crate's modules/types
// Assuming this file is part of a separate crate that depends on 'sahne64'
use sahne64::{SahneError, arch, syscall, resource, Handle}; // <-- Changed import, added resource and Handle
use core::ptr::{read_volatile, write_volatile}; // Keep for simulating direct memory access if needed, though less likely for GDDR

// Need access to the custom print/eprint macros from Sahne64's stdio_impl in no_std
// Assuming these are made available.

/// GDDR Bellek Tipleri
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GddrType {
    DDR3, // Seems like a typo in original code, should be GDDR3? Let's fix based on usage below.
    GDDR3,
    GDDR4,
    GDDR5,
    GDDR6,
    GDDR7,
    Unknown,
}

impl GddrType {
    /// Bir tamsayı değerinden GDDR tipini çıkarır.
    ///
    /// Yaygın GDDR standartlarına göre tip belirlemesi yapar.
    pub fn from_u32(value: u32) -> Self {
        match value {
            3 => GddrType::GDDR3, // <-- Corrected variant name
            4 => GddrType::GDDR4,
            5 => GddrType::GDDR5,
            6 => GddrType::GDDR6,
            7 => GddrType::GDDR7,
            _ => GddrType::Unknown,
        }
    }

    /// GDDR tipini bir dize olarak döndürür.
    pub fn as_str(&self) -> &'static str {
        match self {
            GddrType::DDR3 => "DDR3 (Hata?)", // Keep original typo variant for safety? Or remove. Let's remove.
            GddrType::GDDR3 => "GDDR3",
            GddrType::GDDR4 => "GDDR4",
            GddrType::GDDR5 => "GDDR5",
            GddrType::GDDR6 => "GDDR6",
            GddrType::GDDR7 => "GDDR7",
            GddrType::Unknown => "Bilinmeyen GDDR Tipi",
        }
    }
}

/// GDDR Bellek Yöneticisi işlemleri sırasında oluşabilecek hataları tanımlar.
#[derive(Debug)]
pub enum GddrError { // Renamed from DDRError to GddrError for clarity
    InitializationError(String),
    ReadError(String), // Likely refers to read *commands*, not direct memory reads in this context
    WriteError(String), // Likely refers to write *commands*
    UnsupportedGddrType(GddrType), // Renamed variant
    CommandQueueError(String), // Added for command queue specific errors
    InvalidHandle(SahneError), // When resource::acquire fails
    /// GPU kaynağı veya bellek kontrolcüsü henüz başlatılmadı/edinilemedi.
    NotInitialized, // Or perhaps ResourceNotAvailable?
    MemoryOperationError(SahneError), // Re-used from previous file approach
    // Note: AddressOutOfRange might not be relevant for GDDR allocated chunks managed by GPU driver
    // Instead, operations on allocated chunks might return InvalidChunkHandle or similar.
    // For now, keep errors related to failed Sahne64 calls.
}

impl From<SahneError> for GddrError { // Renamed From impl
    fn from(error: SahneError) -> Self {
        match error {
            SahneError::InvalidHandle => GddrError::InvalidHandle(error), // Map invalid handle specifically
            e => GddrError::MemoryOperationError(e), // Wrap other SahneErrors
        }
    }
}


/// GDDR Bellek Yöneticisi Yapısı. GPU'nun GDDR belleğini yönetmek için kullanılır.
/// Bu yapı, GPU sürücüsü ile Sahne64'ün kaynak kontrol mekanizması (`resource::control`)
/// aracılığıyla etkileşim kurar.
///
/// **Dikkat:** Bu örnek kod, GDDR bellek kontrolcüsünün basitleştirilmiş bir modelidir.
/// Gerçek bir GDDR kontrolcüsü çok daha karmaşık donanım etkileşimleri ve zamanlama
/// gereksinimleri içerir. Bu kod sadece kavramsal bir örnek olarak sunulmuştur ve
/// gerçek donanım üzerinde çalışması amaçlanmamıştır.
pub struct GddrMemoryManager {
    gpu_resource_handle: Handle, // <-- Değişiklik 4: GPU sürücüsü kaynağının Handle'ı
    gddr_type: GddrType,
    total_memory_size: usize, // GPU tarafından bildirilen toplam bellek boyutu (bayt cinsinden)
    allocated_memory: usize, // Bu yönetici aracılığıyla ayrılmış bellek miktarı (bayt cinsinden, takip amaçlı)
    is_initialized: bool, // Bu yöneticinin init edildiğini gösterir
    // Not: Gerçek uygulamada, ayrılan bellek bloklarının Handle'ları veya tanıtıcıları tutulmalıdır.
     allocated_blocks: Vec<GddrMemoryBlockHandle>, // Örnek: gerçek implementasyonda gerekli
}

// Özel IOCTL/Kontrol istek kodları (GPU sürücüsü ile anlaşılmış olmalı)
// Bu değerler Sahne64'ün genel IOCTL numaralarından ayrıdır.
const GDDR_REQUEST_GET_INFO: u64 = 0x1000; // GPU/GDDR bilgisi al
const GDDR_REQUEST_ALLOCATE_MEM: u64 = 0x1001; // GDDR bellek ayır
const GDDR_REQUEST_FREE_MEM: u64 = 0x1002; // GDDR bellek serbest bırak
const GDDR_REQUEST_COMMAND_QUEUE: u64 = 0x1003; // Komut kuyruğuna ekle

impl GddrMemoryManager {
    /// Yeni bir GDDR Bellek Yöneticisi oluşturur ve GPU kaynağını edinir.
    ///
    /// Bu fonksiyon, belirtilen kaynak ID'sine sahip GPU sürücüsünü edinir
    /// ve temel GDDR bilgilerini (tip, toplam boyut) sorgular.
    ///
    /// # Parametreler
    ///
    /// * `gpu_resource_id`: GPU sürücüsünü temsil eden Sahne64 kaynak ID'si (örn. "sahne://gpu/main").
    ///
    /// # Dönüş
    ///
    /// Başarılıysa `Ok(GddrMemoryManager)`, hata oluşursa `Err(GddrError)`.
    pub fn new(gpu_resource_id: resource::ResourceId) -> Result<Self, GddrError> {
        // Değişiklik 4: GPU kaynağını resource::acquire ile edin
        let gpu_resource_handle = resource::acquire(gpu_resource_id, resource::MODE_READ | resource::MODE_WRITE) // Okuma/Yazma izni iste
             .map_err(GddrError::from)?; // SahneError'ı GddrError'a çevir

        // GPU'dan GDDR bilgilerini sorgula (Simüle edilmiş IOCTL/Control isteği)
        // GDDR_REQUEST_GET_INFO isteği ile bilgi alınacağını varsayalım.
        // Bu istekten dönecek 64-bit değerin bellek tipi ve boyutu bilgisini
        // içerdiğini varsayalım. Örnek olarak, üst 32-bit bellek boyutu (MB),
        // alt 32-bit GDDR tipi numarası olabilir.
        let info_result = resource::control(gpu_resource_handle, GDDR_REQUEST_GET_INFO, 0) // Argüman olarak 0 gönderelim
             .map_err(GddrError::from)?; // SahneError'ı GddrError'a çevir

        // Başarılı syscall'dan dönen değeri yorumla
        let info_value = info_result as u64; // control Result<i64, _> döndürür
        if info_value < 0 {
            // control zaten negatif değerleri Err(SahneError) olarak döndürmeliydi.
            // Buraya düşmek normalde control implementasyonunda bir sorun olduğunu gösterir.
            // Yine de bir kontrol ekleyelim.
             #[cfg(feature = "std")]
            std::eprintln!("UYARI: GDDR Bilgi kontrol çağrısı negatif değer döndürdü: {}", info_value);
             #[cfg(not(feature = "std"))]
             eprintln!("UYARI: GDDR Bilgi kontrol çağrısı negatif değer döndürdü: {}", info_value);

             // Hata dönecekse, SahneError olarak control tarafından gelmeliydi.
             // Eğer buraya düşerse, belki de InvalidOperation gibi bir hata dönebiliriz.
             return Err(GddrError::MemoryOperationError(SahneError::InvalidOperation));
        }


        // Bilgi değerini yorumla (varsayımsal format: üst 32-bit size_mb, alt 32-bit type_id)
        let memory_size_mb = (info_value >> 32) as u32;
        let gddr_type_id = info_value as u32;

        let total_memory_size = (memory_size_mb as usize) * 1024 * 1024; // MB'tan bayta
        let gddr_type = GddrType::from_u32(gddr_type_id);

        if gddr_type == GddrType::Unknown {
             // Eğer GPU bilinmeyen bir tip bildiriyorsa
             #[cfg(feature = "std")]
            std::eprintln!("UYARI: GPU tarafından bildirilen bilinmeyen GDDR Tipi: {}", gddr_type_id);
             #[cfg(not(feature = "std"))]
             eprintln!("UYARI: GPU tarafından bildirilen bilinmeyen GDDR Tipi: {}", gddr_type_id);
             // Belki de bu bir hata kabul edilmeli? Duruma bağlı. Şimdilik devam edelim.
              return Err(GddrError::UnsupportedGddrType(GddrType::from_u32(gddr_type_id)));
        }


        Ok(GddrMemoryManager {
            gpu_resource_handle, // Edinilen handle'ı sakla
            gddr_type,
            total_memory_size,
            allocated_memory: 0, // Başlangıçta hiçbir bellek ayrılmamış
            is_initialized: true, // Handle edinildi ve bilgi alındı, başlatıldı sayılabilir
        })
    }

    /// Bellek Yöneticisinin GDDR tipini döndürür.
    pub fn gddr_tipini_al(&self) -> GddrType {
        self.gddr_type
    }

    /// Bellek Yöneticisinin toplam bellek boyutunu döndürür (bayt cinsinden).
    pub fn bellek_boyutunu_al(&self) -> usize {
        self.total_memory_size
    }

    /// Şu anda bu yönetici aracılığıyla ayrılmış bellek miktarını döndürür (bayt cinsinden).
    pub fn ayrilmis_bellek_boyutunu_al(&self) -> usize {
        self.allocated_memory
    }


    /// Belirtilen boyutta GDDR bellek ayırır.
    ///
    /// Bu, GPU sürücüsüne `resource::control` sistem çağrısını kullanarak bir istek gönderir.
    ///
    /// # Parametreler
    ///
    /// * `size`: Ayrılacak bellek boyutu (bayt cinsinden).
    ///
    /// # Dönüş
    ///
    /// Başarılıysa `Ok(())`, bellek yetersizse veya başka bir hata oluşursa `Err(GddrError)`.
    /// Başarı durumunda, gerçek ayrılmış bellek bloğunu temsil eden bir tanıtıcı (Handle?)
    /// dönmesi gerekebilir, ancak bu basitleştirilmiş örnekte sadece ayrılan boyutu takip ediyoruz.
    ///
    /// # Örnek
    ///
    /// ```ignore // Bu örneklerin derlenebilmesi için Sahne64 crate'ine bağımlılık ve uygun bir çalışma ortamı gerekir.
     use your_crate_name::GddrMemoryManager; // GddrMemoryManager'ın bulunduğu crate'ten import
     use sahne64::{resource, SahneError}; // Sahne64'ten gerekli importlar
    ///
     fn example_gddr_alloc(manager: &mut GddrMemoryManager) -> Result<(), GddrError> {
     match manager.bellek_ayir(512) {
         Ok(_) => { /* println!("512 bayt GDDR bellek başarıyla ayrıldı."); */ Ok(()) },
         Err(hata) => { /* eprintln!("GDDR Bellek ayırma hatası: {:?}", hata); */ Err(hata) },
     }
     }
    /// ```
    pub fn bellek_ayir(&mut self, boyut: usize) -> Result<(), GddrError> {
        // Basit takip: Toplam boyutu aşmıyor mu?
        if self.allocated_memory.checked_add(boyut).unwrap_or(usize::MAX) > self.total_memory_size { // Overflow check
            return Err(GddrError::MemoryOperationError(SahneError::OutOfMemory)); // Kendi kontrolümüzde yetersiz bellek
        }

        if !self.is_initialized {
             return Err(GddrError::NotInitialized);
        }

        // ** ÖNEMLİ: Gerçek GDDR bellek ayırma işlemi GPU sürücüsü tarafından yapılmalıdır. **
        // Biz resource::control kullanarak GPU sürücüsüne bir istek gönderiyoruz.

        // Değişiklik 5: resource::control kullanarak GDDR bellek ayırma isteği gönder
        // GDDR_REQUEST_ALLOCATE_MEM isteği ile ayırılacak boyutu gönderiyoruz.
        // resource::control Result<i64, SahneError> döner. Başarı durumunda dönen i64 değeri
        // (örn. 0 veya ayrılan bloğun tanıtıcısı) yoruma tabidir.
        let alloc_result = resource::control(
            self.gpu_resource_handle, // GPU sürücüsü handle'ı
            GDDR_REQUEST_ALLOCATE_MEM, // Ayırma isteği kodu
            boyut as u64 // Argüman: Ayrılacak boyut
        ).map_err(GddrError::from)?; // SahneError'ı GddrError'a çevir

        // control'den başarıyla dönen i64 sonucunu kontrol et
        if alloc_result < 0 {
            // Normalde resource::control negatif sonuçları zaten Err olarak döndürür.
            // Bu kontrol fazladan güvenlik içindir.
             #[cfg(feature = "std")]
            std::eprintln!("UYARI: GDDR Bellek Ayırma kontrol çağrısı negatif değer döndürdü: {}", alloc_result);
             #[cfg(not(feature = "std"))]
             eprintln!("UYARI: GDDR Bellek Ayırma kontrol çağrısı negatif değer döndürdü: {}", alloc_result);
            // Hata dönüşü zaten map_err tarafından yapıldıysa buraya düşmemeli.
            // Eğer düştüyse, mantıksal bir hata veya bilinmeyen bir dönüş değeri var demektir.
             return Err(GddrError::MemoryOperationError(SahneError::UnknownSystemCall)); // Veya daha spesifik bir hata
        }

        // IOCTL başarılı oldu (GPU sürücüsü belleği ayrılmış olabilir).
        // Burada ayrılmış bellek miktarını kendi takibimizde güncelliyoruz.
        // Gerçek implementasyonda, resource::control'dan dönen değer bir bellek bloğu tanıtıcısı olabilir.
        self.allocated_memory += boyut;
        Ok(())
    }

    /// Daha önce ayrılmış GDDR belleğini serbest bırakır.
    ///
    /// Bu da GPU sürücüsüne `resource::control` sistem çağrısını kullanarak bir istek gönderir.
    /// Serbest bırakılacak bloğun tanıtıcısı (handle) parametre olarak alınmalıdır (Bu örnekte boyut kullanılıyor).
    ///
    /// # Parametreler
    ///
    /// * `size`: Serbest bırakılacak bellek boyutu (bayt cinsinden). Bu örnekte basitleştirme için boyut kullanılıyor.
    ///           Gerçek uygulamada, ayrılan bellek bloğunu temsil eden bir Handle veya tanıtıcı kullanılmalıdır.
    ///
    /// # Dönüş
    ///
    /// Başarılıysa `Ok(())`, serbest bırakılacak boyut yanlışsa veya başka bir hata oluşursa `Err(GddrError)`.
    ///
    /// # Örnek
    ///
    /// ```ignore // Bu örneklerin derlenebilmesi için Sahne64 crate'ine bağımlılık ve uygun bir çalışma ortamı gerekir.
     use your_crate_name::GddrMemoryManager; // GddrMemoryManager'ın bulunduğu crate'ten import
     use sahne64::{resource, SahneError}; // Sahne64'ten gerekli importlar
    ///
     fn example_gddr_free(manager: &mut GddrMemoryManager) -> Result<(), GddrError> {
     match manager.bellek_serbest_birak(512) {
         Ok(_) => { /* println!("512 bayt GDDR bellek başarıyla serbest bırakıldı."); */ Ok(()) },
         Err(hata) => { /* eprintln!("GDDR Bellek serbest bırakma hatası: {:?}", hata); */ Err(hata) },
     }
     }
    /// ```
    pub fn bellek_serbest_birak(&mut self, boyut: usize) -> Result<(), GddrError> {
         // Basit takip: Serbest bırakılacak boyut ayrılmış boyuttan fazla olmamalı
        if boyut > self.allocated_memory {
             return Err(GddrError::MemoryOperationError(SahneError::InvalidParameter)); // Hatalı serbest bırakma boyutu
        }

        if !self.is_initialized {
             return Err(GddrError::NotInitialized);
        }

        // ** ÖNEMLİ: Gerçek GDDR bellek serbest bırakma işlemi GPU sürücüsü tarafından yapılmalıdır. **
        // Biz resource::control kullanarak GPU sürücüsüne bir istek gönderiyoruz.

        // Değişiklik 5: resource::control kullanarak GDDR bellek serbest bırakma isteği gönder
        // GDDR_REQUEST_FREE_MEM isteği ile serbest bırakılacak boyutu (veya gerçekte bloğun tanıtıcısını) gönderiyoruz.
        let free_result = resource::control(
            self.gpu_resource_handle, // GPU sürücüsü handle'ı
            GDDR_REQUEST_FREE_MEM, // Serbest bırakma isteği kodu
            boyut as u64 // Argüman: Serbest bırakılacak boyut (örnekte, gerçekte Handle olabilir)
        ).map_err(GddrError::from)?; // SahneError'ı GddrError'a çevir

        // control'den başarıyla dönen i64 sonucunu kontrol et
        if free_result < 0 {
            // Normalde resource::control negatif sonuçları zaten Err olarak döndürür.
             #[cfg(feature = "std")]
            std::eprintln!("UYARI: GDDR Bellek Serbest Bırakma kontrol çağrısı negatif değer döndürdü: {}", free_result);
             #[cfg(not(feature = "std"))]
             eprintln!("UYARI: GDDR Bellek Serbest Bırakma kontrol çağrısı negatif değer döndürdü: {}", free_result);
            return Err(GddrError::MemoryOperationError(SahneError::UnknownSystemCall)); // Veya daha spesifik bir hata
        }

        // IOCTL başarılı oldu (GPU sürücüsü belleği serbest bırakmış olabilir).
        // Burada ayrılmış bellek miktarını kendi takibimizde güncelliyoruz.
        self.allocated_memory -= boyut;
        Ok(())
    }

    // Not: Drop implementasyonunda deinit gibi bir fonksiyon çağırmak GDDR için de anlamlı olabilir,
    // ancak bu durumda manager'ın Drop olması tüm ayrılmış GDDR belleğini serbest bırakmalı.
    // Mevcut yapıdaki bellek takibi (ayrilmis_bellek) basitleştirilmiş, gerçekte her blok takip edilmeli.
    // Drop eklersek, edinilen GPU Handle'ını da resource::release ile serbest bırakmalıdır.
    // Şimdilik Drop eklemiyorum, yöneticiyi explicit olarak kullanıp bitince scope dışına çıkmasını bekleyebiliriz.
    // Eğer Drop eklersek:
    
    fn deinit(&mut self) -> Result<(), GddrError> {
        // Tüm ayrılmış blokları serbest bırak (gerçek implementasyonda)
        // GPU Handle'ını serbest bırak
        if self.gpu_resource_handle.is_valid() {
             match resource::release(self.gpu_resource_handle) {
                 Ok(_) => {
                     // println!("GPU Kaynak Handle'ı serbest bırakıldı.");
                     self.gpu_resource_handle = Handle::invalid(); // Handle'ı geçersiz yap
                     Ok(())
                 },
                 Err(e) => {
                     // eprintln!("GPU Kaynak Handle'ı serbest bırakılırken hata: {:?}", e);
                     Err(GddrError::MemoryOperationError(e)) // Hata döndür
                 }
             }
        } else {
            Ok(()) // Zaten serbest bırakılmış
        }
    }
    
     
     impl Drop for GddrMemoryManager {
         fn drop(&mut self) {
             // Drop içinde hata yoksayılır, sadece loglama yapılır
             if let Err(e) = self.deinit() {
                  #[cfg(feature = "std")] std::eprintln!("UYARI: GDDR Bellek Yöneticisi deinit hatası: {:?}", e);
                  #[cfg(not(feature = "std"))] eprintln!("UYARI: GDDR Bellek Yöneticisi deinit hatası: {:?}", e);
             }
         }
     }
     


    // --- GDDR Standardına Özgü İşlemler (Örnek olarak) ---

    /// GDDR komut kuyruğuna bir komut ekler (Örnek İşlem).
    ///
    /// Bu, GPU sürücüsüne `resource::control` sistem çağrısını kullanarak bir komut gönderir.
    /// Komut verisinin nasıl geçirileceği (argüman veya ayrı bir shared memory) implementasyona bağlıdır.
    /// Bu örnekte, sadece komut tipini geçiriyoruz.
    ///
    /// # Parametreler
    ///
    /// * `command`: Eklenecek GDDR komutu.
    ///
    /// # Dönüş
    ///
    /// Başarılıysa `Ok(())`, hata oluşursa `Err(GddrError)`.
    ///
    /// # Örnek
    ///
    /// ```ignore // Bu örneklerin derlenebilmesi için Sahne64 crate'ine bağımlılık ve uygun bir çalışma ortamı gerekir.
     use your_crate_name::{GddrMemoryManager, GddrKomut}; // Gerekli importlar
     use sahne64::{resource, SahneError};
    ///
     fn example_gddr_command(manager: &mut GddrMemoryManager) -> Result<(), GddrError> {
     let read_cmd = GddrKomut::Oku(0x2000, 64); // Adres 0x2000'den 64 byte oku
     match manager.komut_kuyruguna_ekle(read_cmd) {
         Ok(_) => { println!("Okuma komutu kuyruğa eklendi."); */ Ok(()) },
         Err(hata) => { eprintln!("Komut kuyruğuna ekleme hatası: {:?}", hata); Err(hata) },
     }
     }
    /// ```
    pub fn komut_kuyruguna_ekle(&mut self, komut: GddrKomut) -> Result<(), GddrError> {
        if !self.is_initialized {
             return Err(GddrError::NotInitialized);
        }

        // ** ÖNEMLİ: Gerçek GDDR komut kuyruğu yönetimi GPU sürücüsü tarafından yapılmalıdır. **
        // resource::control kullanarak GPU sürücüsüne bir komut isteği gönderiyoruz.

        // Komutu ve argümanlarını uygun bir formata dönüştürmek gerekir.
        // Bu örnekte sadece komut tipini ve belki ilk argümanı gönderiyoruz.
        let (command_type_code, arg1) = match &komut {
             GddrKomut::Oku(addr, size) => (1, *addr as u64),  // Örnek değerler
             GddrKomut::Yaz(addr, data) => (2, *addr as u64), // Örnek değerler, veri ayrı gönderilmeli
             // ... Diğer komutlar ...
        };

        // Değişiklik 5: resource::control kullanarak komut isteği gönder
        let cmd_result = resource::control(
            self.gpu_resource_handle, // GPU sürücüsü handle'ı
            GDDR_REQUEST_COMMAND_QUEUE, // Komut kuyruğu isteği kodu
            command_type_code // Argüman: Komut tipi kodu (basit örnek)
            // Not: Gerçekte, komut verisi ve diğer argümanlar için daha karmaşık bir mekanizma gerekir.
            // Örneğin, argümanları içeren bir struct'ın pointer'ını ve boyutunu göndermek.
            // resource::control tek bir u64 argüman alabildiği için bu örnek basitleştirilmiştir.
            // Belki IOCTL isteğine ek argümanlar için Sahne64 API'si genişletilmeli
            // veya SharedMemory + Message/Signal kombinasyonu kullanılmalı.
        ).map_err(GddrError::from)?; // SahneError'ı GddrError'a çevir


        // control'den başarıyla dönen i64 sonucunu kontrol et
        if cmd_result < 0 {
            // Normalde resource::control negatif sonuçları zaten Err olarak döndürür.
             #[cfg(feature = "std")]
            std::eprintln!("UYARI: GDDR Komut Kuyruğu kontrol çağrısı negatif değer döndürdü: {}", cmd_result);
             #[cfg(not(feature = "std"))]
             eprintln!("UYARI: GDDR Komut Kuyruğu kontrol çağrısı negatif değer döndürdü: {}", cmd_result);
            return Err(GddrError::MemoryOperationError(SahneError::UnknownSystemCall)); // Veya daha spesifik bir hata
        }

        // Komut kuyruğuna başarıyla eklendi varsayalım
        Ok(())
    }

    // ... Diğer GDDR standardına özgü işlemler eklenebilir ...
}

/// Örnek GDDR Komut yapısı (Standartlara göre farklılık gösterir).
/// Not: Yaz komutundaki Vec<u8>, no_std ortamında dikkatli kullanılmalıdır, heap tahsisi gerektirir.
/// Gerçek driver etkileşiminde verinin SharedMemory gibi bir yerden referansla geçirilmesi daha olasıdır.
#[derive(Debug)]
pub enum GddrKomut {
    Oku(usize, usize), // Adres, Boyut - GPU belleğindeki adres ve okunacak boyut
    Yaz(usize, core::vec::Vec<u8>), // Adres, Veri - GPU belleğindeki adres ve yazılacak veri (Vec<u8> std veya alloc gerektirir)
    // ... Diğer GDDR komutları eklenebilir ...
}

#[cfg(test)]
mod tests {
    use super::*;
    // Testler için Sahne64 mock'ları veya test ortamı gereklidir.
    // Şu anki test sadece GddrMemoryManager struct'ının oluşturulabildiğini test eder.
    // Yeni Sahne64 API'si Handle edinmeyi gerektirdiğinden, bu testin çalışması için
    // resource::acquire'ı mocklamak veya geçerli bir handle döndürecek bir test ortamı kurmak gerekir.
    // Bu nedenle test şimdilik ignore ediliyor veya uygun bir test altyapısı kurulmalı.

    #[test]
    #[ignore = "Requires Sahne64 test environment with resource::acquire mock"]
    fn gddr_memory_manager_olusturma() {
         // resource::acquire mock'u veya gerçek bir test handle'ı sağlamanız gerekir.
         // Örneğin:
          let mock_gpu_handle = Handle(123); // Varsayımsal mock handle
          let mock_gpu_info_return_value: i64 = (1024u64 << 32 | 6u32 as u64) as i64; // 1GB GDDR6

         // Burada bir mock framework'ü veya test environment setup'ı olmalı
         // ki resource::acquire çağrısı panic yapmasın veya hata dönmesin.

        // Şu anki haliyle bu test muhtemelen resource::acquire içinde panic yapacaktır.
        // Geçici olarak ignore edildi.
         let yonetici = GddrMemoryManager::new("sahne://gpu/test").unwrap(); // acquire başarılı olmalı
         assert_eq!(yonetici.gddr_tipini_al(), GddrType::Gddr6); // Varsayılan test ortamına göre
         assert_eq!(yonetici.bellek_boyutunu_al(), 1024 * 1024 * 1024); // Varsayılan test ortamına göre
         assert_eq!(yonetici.ayrilmis_bellek_boyutunu_al(), 0);
    }

     // Yeni testler Sahne64 test altyapısı ile yazılmalıdır.
}

// Değişiklik 6: Örnek kullanım (main fonksiyonu)
// Bu kısım std ortamında çalışacak şekilde yapılandırılmıştır ve Sahne64'ün çıktı makrolarını kullanır.
#[cfg(feature = "std")]
fn main() {
    // Bu örnek std ortamında çalışacak şekilde yapılandırılmıştır.
    // no_std ortamında Sahne64'ün çıktı makroları veya özel bir çıktı mekanizması gerektirir.

    #[cfg(feature = "std")]
    std::println!("Sahne64 GDDR Bellek Yöneticisi Örneği (std)");
    #[cfg(not(feature = "std"))]
    println!("Sahne64 GDDR Bellek Yöneticisi Örneği (no_std)");

    // GPU kaynağı ID'si (örnek)
    let gpu_resource_id = "sahne://gpu/main";

    // GDDR Bellek Yöneticisi oluştur (GPU kaynağını edinir ve bilgi alır)
    match GddrMemoryManager::new(gpu_resource_id) {
        Ok(mut gddr_manager) => {
             #[cfg(feature = "std")]
            std::println!("GDDR Bellek Yöneticisi başarıyla oluşturuldu.");
             #[cfg(not(feature = "std"))]
            println!("GDDR Bellek Yöneticisi başarıyla oluşturuldu.");

             #[cfg(feature = "std")]
            std::println!("GPU Tipi: {:?}", gddr_manager.gddr_tipini_al());
             #[cfg(not(feature = "std"))]
            println!("GPU Tipi: {:?}", gddr_manager.gddr_tipini_al());

             #[cfg(feature = "std")]
            std::println!("Toplam GDDR Boyutu: {} bayt", gddr_manager.bellek_boyutunu_al());
             #[cfg(not(feature = "std"))]
            println!("Toplam GDDR Boyutu: {} bayt", gddr_manager.bellek_boyutunu_al());


            // --- Bellek Ayırma/Serbest Bırakma Örneği ---
             #[cfg(feature = "std")]
            std::println!("\n--- Bellek Ayırma/Serbest Bırakma Örneği ---");
             #[cfg(not(feature = "std"))]
            println!("\n--- Bellek Ayırma/Serbest Bırakma Örneği ---");

            let alloc_size = 1024 * 1024; // 1MB ayır
            match gddr_manager.bellek_ayir(alloc_size) {
                Ok(_) => {
                     #[cfg(feature = "std")]
                    std::println!("{} bayt GDDR bellek başarıyla ayrıldı.", alloc_size);
                     #[cfg(not(feature = "std"))]
                    println!("{} bayt GDDR bellek başarıyla ayrıldı.", alloc_size);

                     #[cfg(feature = "std")]
                    std::println!("Ayrılmış GDDR bellek: {} bayt", gddr_manager.ayrilmis_bellek_boyutunu_al());
                     #[cfg(not(feature = "std"))]
                    println!("Ayrılmış GDDR bellek: {} bayt", gddr_manager.ayrilmis_bellek_boyutunu_al());


                    // Belleği serbest bırak
                    match gddr_manager.bellek_serbest_birak(alloc_size) {
                        Ok(_) => {
                             #[cfg(feature = "std")]
                            std::println!("{} bayt GDDR bellek başarıyla serbest bırakıldı.", alloc_size);
                             #[cfg(not(feature = "std"))]
                             println!("{} bayt GDDR bellek başarıyla serbest bırakıldı.", alloc_size);

                             #[cfg(feature = "std")]
                            std::println!("Ayrılmış GDDR bellek: {} bayt", gddr_manager.ayrilmis_bellek_boyutunu_al());
                             #[cfg(not(feature = "std"))]
                             println!("Ayrılmış GDDR bellek: {} bayt", gddr_manager.ayrilmis_bellek_boyutunu_al());
                        }
                        Err(e) => {
                             #[cfg(feature = "std")]
                            std::eprintln!("GDDR bellek serbest bırakma hatası: {:?}", e);
                             #[cfg(not(feature = "std"))]
                            eprintln!("GDDR bellek serbest bırakma hatası: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                     #[cfg(feature = "std")]
                    std::eprintln!("GDDR bellek ayırma hatası: {:?}", e);
                     #[cfg(not(feature = "std"))]
                    eprintln!("GDDR bellek ayırma hatası: {:?}", e);
                }
            }

             // --- Komut Kuyruğu Örneği ---
            #[cfg(feature = "std")]
            std::println!("\n--- Komut Kuyruğu Örneği ---");
            #[cfg(not(feature = "std"))]
            println!("\n--- Komut Kuyruğu Örneği ---");

            // Not: GddrKomut::Yaz Vec<u8> içeriyor, std veya alloc gerektirir.
            // no_std ortamında alloc feature açık değilse veya başka bir allocator yoksa bu çalışmaz.
            // Basitlik için sadece Ok komutu deneyelim.
            use core::vec::Vec; // Vec kullanılıyorsa import edilmeli
            let read_cmd = GddrKomut::Oku(0x2000, 64); // Adres 0x2000'den 64 byte oku (örnek)
            match gddr_manager.komut_kuyruguna_ekle(read_cmd) {
                 Ok(_) => {
                     #[cfg(feature = "std")]
                     std::println!("Okuma komutu kuyruğa eklendi (simüle).");
                     #[cfg(not(feature = "std"))]
                     println!("Okuma komutu kuyruğa eklendi (simüle).");
                 }
                 Err(e) => {
                     #[cfg(feature = "std")]
                     std::eprintln!("Komut kuyruğuna ekleme hatası: {:?}", e);
                     #[cfg(not(feature = "std"))]
                     eprintln!("Komut kuyruğuna ekleme hatası: {:?}", e);
                 }
            }


            // GddrMemoryManager scope dışına çıktığında, Drop implementasyonu (eğer eklenmişse)
            // edinilen GPU Handle'ını serbest bırakacaktır.
        }
        Err(e) => {
             #[cfg(feature = "std")]
            std::eprintln!("GDDR Bellek Yöneticisi oluşturma hatası: {:?}", e);
             #[cfg(not(feature = "std"))]
            eprintln!("GDDR Bellek Yöneticisi oluşturma hatası: {:?}", e);
        }
    }
}
