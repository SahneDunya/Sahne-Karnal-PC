#![no_std]
use crate::{memory, SahneError};
use core::ptr::NonNull;
use core::slice;
use core::mem::ManuallyDrop; // Drop'u manuel olarak kontrol etmek için

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
/// ```
/// if let Some(mut mem) = AllocatedMemory::new(1024) {
///     mem.initialize(0); // Belleği 0 ile başlat
///     println!("Bellek boyutu: {}", mem.size());
///     println!("İlk byte: {}", mem.as_slice()[0]);
/// }
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
    /// Başarılı olursa `Some(AllocatedMemory)`, başarısız olursa `None`.
    ///
    /// # Örnekler
    ///
    /// ```
    /// let mem = AllocatedMemory::new(1024);
    /// assert!(mem.is_some());
    ///
    /// let mem_zero_size = AllocatedMemory::new(0);
    /// assert!(mem_zero_size.is_none());
    /// ```
    pub fn new(size: usize) -> Option<Self> {
        if size == 0 {
            return None;
        }

        match unsafe { memory::allocate(size) } {
            Ok(ptr) => {
                // NonNull::new_unchecked çünkü memory::allocate başarılı olursa null dönmez
                Some(Self {
                    ptr: unsafe { NonNull::new_unchecked(ptr) },
                    size,
                    initialized: false,
                })
            }
            Err(_) => None, // Bellek ayırma başarısız oldu
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
    /// let mut mem = AllocatedMemory::new(10).unwrap();
    /// // mem.initialize(0); // Yorum satırı kaldırılırsa panic olmaz
    /// let slice = mem.as_slice(); // Panic verir çünkü bellek henüz başlatılmadı
    /// ```
    ///
    /// ```
    /// let mut mem = AllocatedMemory::new(10).unwrap();
    /// mem.initialize(0);
    /// let slice = mem.as_slice(); // Panic vermez
    /// assert_eq!(slice.len(), 10);
    /// ```
    pub fn as_slice(&self) -> &[u8] {
        if !self.initialized {
            panic!("Bellek henüz başlatılmadı! `as_slice` çağrılmadan önce `initialize` fonksiyonlarından birini kullanarak belleği başlatmanız gerekmektedir."); // Geliştirilmiş hata mesajı
        }
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
    /// ```
    /// let mut mem = AllocatedMemory::new(10).unwrap();
    /// let mut_slice = mem.as_mut_slice();
    /// mut_slice[0] = 42; // Belleğin ilk byte'ını değiştir
    /// ```
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
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
    /// ```
    /// let mut mem = AllocatedMemory::new(10).unwrap();
    /// mem.initialize(0); // Belleği 0 ile başlat
    /// assert_eq!(mem.as_slice()[0], 0);
    /// assert_eq!(mem.as_slice()[9], 0);
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
    /// ```
    /// let mut mem = AllocatedMemory::new(5).unwrap();
    /// mem.initialize_with(|i| (i * 2) as u8); // Belleği indeksin iki katı ile başlat
    /// assert_eq!(mem.as_slice(), &[0, 2, 4, 6, 8]);
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
    /// ```
    /// let mem = AllocatedMemory::new(2048).unwrap();
    /// assert_eq!(mem.size(), 2048);
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
            match unsafe { memory::free(ptr, size) } {
                Ok(_) => (), // Bellek başarıyla serbest bırakıldı
                Err(e) => {
                    // Kernel ortamında panic yerine daha uygun bir loglama mekanizması kullanılabilir.
                    // Şimdilik sadece bir uyarı veriyoruz.
                    eprintln!("UYARI: Bellek serbest bırakılırken hata oluştu: {:?}", e);
                }
            }
        }
    }
}