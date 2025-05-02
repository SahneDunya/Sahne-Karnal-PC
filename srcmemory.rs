#![no_std]
// Use the Sahne64 crate's modules/types
// Assuming this file is part of a separate crate that depends on 'sahne64'
// Sahne64 crate'ini projenize bağımlılık olarak eklemeniz gerekecek.
use sahne64::{memory, SahneError}; // <-- Değişiklik 1: Sahne64 crate'inden import
use core::ptr::NonNull;
use core::slice;
// core::mem::ManuallyDrop import'u bu dosyada kullanılmıyor, kaldırılabilir.
 use core::mem::ManuallyDrop; // Drop'u manuel olarak kontrol etmek için - Kullanılmıyor

// Not: Eğer bu component crate'i no_std ortamında `eprintln!` kullanacaksa,
// ya kendi basit çıktısını implement etmeli ya da Sahne64 crate'inin
// sağladığı stdio_impl gibi bir mekanizmaya erişimi olmalıdır.
// Aşağıdaki kod, her iki durum için de eprintln! kullanımını ayarlar,
// no_std durumu için Sahne64'ün custom macro'sunun scope'ta olduğunu varsayar.


/// # AllocatedMemory Struct
///
/// `AllocatedMemory` yapısı, dinamik olarak ayrılmış bir bellek bloğunu güvenli bir şekilde yönetmek için tasarlanmıştır.
/// Belleği ayırma, başlatma, dilimler halinde erişme ve serbest bırakma işlemlerini kapsüller.
///
/// ## Özellikler
///
/// - **Güvenli Bellek Yönetimi:** `Drop` özelliği ile bellek otomatik olarak serbest bırakılır, bellek sızıntılarını önler.
/// - **Başlatma Takibi:** `initialized` alanı ile belleğin başlatılıp başlatılmadığı takip edilir, yanlışlıkla başlatılmamış belleğe erişimi engeller.
/// - **Çoklu Başlatma Yöntemleri:** Belleği sabit bir değerle, bir fonksiyon ile başlatma seçenekleri sunar. (`MaybeUninit` düşük seviyede doğrudan desteklenmeyebilir)
/// - **Dilim Arayüzleri:** Belleğe güvenli bir şekilde `&[u8]` ve `&mut [u8]` dilimleri aracılığıyla erişim sağlar.
///
/// ## Örnekler
///
/// ```ignore // Bu örneklerin derlenebilmesi için Sahne64 crate'ine bağımlılık ve uygun bir çalışma ortamı gerekir.
/// // Bu örnekler muhtemelen testler veya dokümantasyon için `#![cfg(feature = "std")]` veya
/// // özel bir Sahne64 test ortamı gerektirecektir.
///
 use sahne64::{memory, SahneError}; // Kullanım örneğinde de doğru import
 use your_crate_name::AllocatedMemory; // AllocatedMemory'nin bulunduğu crate'ten import
///
/// // Eğer std kullanılmıyorsa, çıktı için özel makrolar gereklidir.
  #[cfg(not(feature = "std"))] use sahne64::println; // veya custom makro importu
///
 fn example_usage() -> Result<(), SahneError> {
///     // new Option döndürüyor, allocate ise Result. Match yapısı hala doğru.
     if let Some(mut mem) = AllocatedMemory::new(1024) {
       mem.initialize(0); // Belleği 0 ile başlat
///
///       // println! custom macro veya std feature gerektirir.
        #[cfg(feature = "std")]
        std::println!("Bellek boyutu: {}", mem.size());
        #[cfg(not(feature = "std"))]
        println!("Bellek boyutu: {}", mem.size());
///
        #[cfg(feature = "std")]
        std::println!("İlk byte: {}", mem.as_slice()[0]);
        #[cfg(not(feature = "std"))]
        println!("İlk byte: {}", mem.as_slice()[0]);
///
///         // Bellek Drop edildiğinde otomatik olarak serbest bırakılır.
     } // mem scope dışına çıktığında Drop çağrılır
     Ok(()) // Başarılı
 }
/// 
/// ```
pub struct AllocatedMemory {
    ptr: NonNull<u8>,
    size: usize,
    initialized: bool, // Belleğin başlatılıp başlatılmadığını takip etmek için
}

impl AllocatedMemory {
    /// Yeni bir `AllocatedMemory` örneği oluşturur.
    ///
    /// `size` boyutunda bir bellek bloğu ayırır. Eğer boyut 0 ise veya bellek ayırma başarısız olursa `None` döndürür.
    ///
    /// # Parametreler
    ///
    /// * `size`: Ayrılacak bellek boyutu (byte cinsinden).
    ///
    /// # Geri Dönüş Değeri
    ///
    /// Başarılı olursa `Some(AllocatedMemory)`, bellek ayırma Sahne64 API hatası döndürürse `None`.
    ///
    /// # Örnekler
    ///
    /// ```ignore // Örneklerin derlenebilirliğini sağlamak için
      use your_crate_name::AllocatedMemory;
      let mem = AllocatedMemory::new(1024);
      assert!(mem.is_some());
      let mem_zero_size = AllocatedMemory::new(0);
      assert!(mem_zero_size.is_none());
    /// ```
    pub fn new(size: usize) -> Option<Self> {
        if size == 0 {
            return None;
        }

        // Sahne64 memory::allocate fonksiyonu Result<*mut u8, SahneError> dönüyor
        match memory::allocate(size) {
            Ok(ptr) => {
                // memory::allocate başarılı olursa null olmayan bir pointer garanti eder,
                // bu nedenle NonNull::new_unchecked burada hala güvenlidir.
                Some(Self {
                    ptr: unsafe { NonNull::new_unchecked(ptr) },
                    size,
                    initialized: false,
                })
            }
            Err(_e) => {
                // Bellek ayırma Sahne64 API hatası döndürdü.
                // Burada hatayı loglayabilirsiniz eğer loglama altyapısı varsa.
                 #[cfg(not(feature = "std"))] eprintln!("Bellek ayırma hatası: {:?}", _e);
                None // Başarısızlık durumunda None döndür
            }
        }
    }

    /// Belleği başlatılmış bir byte dilimi olarak döndürür.
    ///
    /// Eğer bellek henüz başlatılmamışsa `panic!` hatası verir.
    ///
    /// # Panic
    ///
    /// Bellek henüz başlatılmamışsa `panic!` hatası verir. Bu, başlatılmamış belleğe erişimi engellemek için bir güvenlik mekanizmasıdır.
    ///
    /// # Geri Dönüş Değeri
    ///
    /// Başlatılmış belleğin `&[u8]` dilimi.
    ///
    /// # Örnekler
    ///
    /// ```should_panic
      use your_crate_name::AllocatedMemory;
      let mut mem = AllocatedMemory::new(10).unwrap();
       mem.initialize(0); // Yorum satırı kaldırılırsa panic olmaz
      let slice = mem.as_slice(); // Panic verir çünkü bellek henüz başlatılmadı
    /// ```
    ///
    /// ```ignore // Örneklerin derlenebilirliğini sağlamak için
      use your_crate_name::AllocatedMemory;
      let mut mem = AllocatedMemory::new(10).unwrap();
      mem.initialize(0);
      let slice = mem.as_slice(); // Panic vermez
      assert_eq!(slice.len(), 10);
    /// ```
    pub fn as_slice(&self) -> &[u8] {
        if !self.initialized {
            panic!("Bellek henüz başlatılmadı! `as_slice` çağrılmadan önce `initialize` fonksiyonlarından birini kullanarak belleği başlatmanız gerekmektedir."); // Geliştirilmiş hata mesajı
        }
        // Güvenli olmayan (unsafe) blok, pointer ve boyutun geçerli olduğunu varsayar,
        // bu varsayım `AllocatedMemory` yapısının doğru kullanımına dayanır.
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.size) }
    }

    /// Belleği mutable bir byte dilimi olarak döndürür.
    ///
    /// Bu fonksiyon, belleği doğrudan değiştirmek için kullanılabilir. Başlatma kontrolü yoktur, bu nedenle bu fonksiyonu kullanmadan önce belleği başlatmak kullanıcının sorumluluğundadır.
    ///
    /// # Geri Dönüş Değeri
    ///
    /// Belleğin `&mut [u8]` dilimi.
    ///
    /// # Örnekler
    ///
    /// ```ignore // Örneklerin derlenebilirliğini sağlamak için
      use your_crate_name::AllocatedMemory;
      let mut mem = AllocatedMemory::new(10).unwrap();
      let mut_slice = mem.as_mut_slice();
      mut_slice[0] = 42; // Belleğin ilk byte'ını değiştir
    /// ```
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // Güvenli olmayan (unsafe) blok, pointer ve boyutun geçerli olduğunu varsayar,
        // bu varsayım `AllocatedMemory` yapısının doğru kullanımına dayanır.
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size) }
    }

    /// Belleği belirtilen `value` ile başlatır.
    ///
    /// Tüm bellek bloğunu verilen `value` ile doldurur ve `initialized` bayrağını `true` olarak ayarlar.
    ///
    /// # Parametreler
    ///
    /// * `value`: Belleği başlatmak için kullanılacak byte değeri.
    ///
    /// # Örnekler
    ///
    /// ```ignore // Örneklerin derlenebilirliğini sağlamak için
      use your_crate_name::AllocatedMemory;
      let mut mem = AllocatedMemory::new(10).unwrap();
      mem.initialize(0); // Belleği 0 ile başlat
      assert_eq!(mem.as_slice()[0], 0);
      assert_eq!(mem.as_slice()[9], 0);
    /// ```
    pub fn initialize(&mut self, value: u8) {
        let slice = self.as_mut_slice();
        slice.fill(value);
        self.initialized = true;
    }

    /// Belleği bir fonksiyon kullanarak başlatır.
    ///
    /// Bellekteki her byte'ı, indeksine göre `f` fonksiyonunu çağırarak elde edilen değerle başlatır. `initialized` bayrağını `true` olarak ayarlar.
    ///
    /// # Parametreler
    ///
    /// * `f`: Her byte için değeri üretecek bir fonksiyon. Fonksiyon `usize` (byte indeksi) alır ve `u8` (byte değeri) döndürür.
    ///
    /// # Örnekler
    ///
    /// ```ignore // Örneklerin derlenebilirliğini sağlamak için
      use your_crate_name::AllocatedMemory;
      let mut mem = AllocatedMemory::new(5).unwrap();
      mem.initialize_with(|i| (i * 2) as u8); // Belleği indeksin iki katı ile başlat
      assert_eq!(mem.as_slice(), &[0, 2, 4, 6, 8]);
    /// ```
    pub fn initialize_with<F>(&mut self, f: F)
    where
        F: FnMut(usize) -> u8,
    {
        let slice = self.as_mut_slice();
        for (i, byte) in slice.iter_mut().enumerate() {
            *byte = f(i);
        }
        self.initialized = true;
    }

    /// Ayrılmış belleğin boyutunu döndürür.
    ///
    /// # Geri Dönüş Değeri
    ///
    /// Belleğin boyutu (byte cinsinden).
    ///
    /// # Örnekler
    ///
    /// ```ignore // Örneklerin derlenebilirliğini sağlamak için
      use your_crate_name::AllocatedMemory;
      let mem = AllocatedMemory::new(2048).unwrap();
      assert_eq!(mem.size(), 2048);
    /// ```
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for AllocatedMemory {
    /// `AllocatedMemory` örneği düşürüldüğünde belleği serbest bırakır.
    ///
    /// Bu, `AllocatedMemory` yapısının `Drop` trait'ini uygulayan fonksiyonudur. Bellek sızıntılarını önlemek için ayrılmış bellek bloğunu serbest bırakır.
    fn drop(&mut self) {
        // Drop çağrıldığında belleği serbest bırak
        let ptr = self.ptr.as_ptr();
        let size = self.size;
        if size > 0 {
            // <-- Değişiklik 2: memory::free yerine memory::release çağırıldı
            // memory::release yeni Sahne64 API'sine göre Result<(), SahneError> dönüyor
            match memory::release(ptr, size) {
                Ok(_) => (), // Bellek başarıyla serbest bırakıldı
                Err(e) => {
                    // Değişiklik 3: eprintln! kullanımı için std/no_std kontrolü
                    // Eğer no_std durumunda Sahne64'ün kendi eprintln!'ı kullanılacaksa
                    // ilgili macro'nun veya fonksiyonun scope'ta olması gerekir.
                    #[cfg(not(feature = "std"))]
                    {
                         // Eğer Sahne64 stdio_impl modülü public ise
                         // use sahne64::stdio_impl::eprintln; kullanabilirsiniz.
                         // Veya crate seviyesinde macro_use ile import etmelisiniz.
                         // Şimdilik macro'nun scope'ta olduğunu varsayalım.
                         eprintln!("UYARI: Bellek serbest bırakılırken hata oluştu: {:?}", e);
                    }
                     #[cfg(feature = "std")]
                     {
                         // std ortamında standart eprintln kullanabilirsiniz.
                         std::eprintln!("UYARI: Bellek serbest bırakılırken hata oluştu: {:?}", e);
                     }
                }
            }
        }
    }
}
