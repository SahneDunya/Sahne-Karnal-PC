#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz

// Sahne64 API'sine erişim için gerekli modülleri içeri aktar
// Bu, 'sahne64' crate'ine bağımlılık anlamına gelir.
use sahne64::{
    resource,  // Kaynak yönetimi
    SahneError, // Hata türü
    Handle,    // Kaynak tanıtıcısı
};
use core::slice; // initrd belleğine slice olarak erişmek için (örnek)
use core::ptr; // Pointer işlemleri için

// Çıktı makrolarını kullanabilmek için (Sahne64 tarafından sağlanan)
// #[cfg] kullanımı önceki dosyalara paralel.

/// Initrd işlemleri sırasında oluşabilecek hataları tanımlar.
#[derive(Debug)]
pub enum InitrdError {
    /// Sahne64 API çağrısı sırasında hata oluştu.
    SahneApiError(SahneError),
    /// Geçersiz initrd adresi veya boyutu.
    InvalidAddressOrSize,
    /// Initrd'yi kaydetmekten sorumlu kaynak yöneticisi edinilemedi.
    ResourceManagerAcquisitionFailed(SahneError),
    // Diğer initrd formatı parsing hataları vb. eklenebilir.
}

// From<SahneError> implementasyonu, Sahne64 hatalarını InitrdError'a çevirir.
impl From<SahneError> for InitrdError {
    fn from(error: SahneError) -> Self {
        InitrdError::SahneApiError(error)
    }
}


/// Başlangıç RAM diskini (initrd) Sahne64 sistemine kaydeder.
///
/// Bu fonksiyon, bellekte yüklü olan initrd bloğunun adresini ve boyutunu alır
/// ve Sahne64 Kaynak Yöneticisi'ne bu initrd'yi kullanarak
/// `sahne://initrd/` gibi bir isim altında kaynaklar sunmasını bildirir.
///
/// # Parametreler
/// * `address`: Initrd'nin bellekteki başlangıç adresi.
/// * `size`: Initrd bloğunun boyutu (bayt cinsinden).
///
/// # Geri Dönüş Değeri
/// Başarılı olursa `Ok(())`, hata oluşursa `Err(InitrdError)`.
///
/// # Güvenlik
/// `address` ve `size` parametrelerinin geçerli bir bellek bölgesini işaret ettiği
/// çağırıcı tarafından garanti edilmelidir.
pub fn init(address: usize, size: usize) -> Result<(), InitrdError> { // <-- Return Result
    // Temel geçerlilik kontrolü
    if address == 0 || size == 0 {
        #[cfg(feature = "std")] std::eprintln!("Initrd::init: Geçersiz adres (0) veya boyut (0) sağlandı.");
        #[cfg(not(feature = "std"))] eprintln!("Initrd::init: Geçersiz adres (0) veya boyut (0) sağlandı.");
        return Err(InitrdError::InvalidAddressOrSize);
    }
    // Adres ve boyutun geçerli bir bellek bölgesine işaret ettiğini daha detaylı kontrol etmek
    // platforma ve bootloader'a bağlıdır. Sahne64 çekirdeği bu kontrolü yapmalıdır.

    #[cfg(feature = "std")] std::println!("Initrd::init: Initrd bellekte 0x{:X} adresinde, {} bayt boyutta.", address, size);
    #[cfg(not(feature = "std"))] println!("Initrd::init: Initrd bellekte 0x{:X} adresinde, {} bayt boyutta.", address, size);


    // ** Önemli: Initrd'yi Sahne64 Kaynak Yöneticisi'ne kaydetmek için API kullanımı **
    // Sahne64 API'sinin initrd'yi kaydetmek için özel bir syscall veya resource::control isteği
    // sağladığını varsayıyoruz. Bu, API'nin kullanıcı alanından çekirdeğe Initrd bilgisini aktarma yoludur.

    // Initrd'yi kaydetmekten sorumlu özel bir kaynak (veya genel kaynak yöneticisi) edinelim.
    // Varsayımsal resource ID: "sahne://resource/initrd_manager" veya "sahne://resource/"
    let initrd_manager_resource_id = "sahne://resource/initrd_manager"; // Örnek ID

    #[cfg(feature = "std")] std::println!("Initrd yöneticisi kaynağı '{}' ediniliyor...", initrd_manager_resource_id);
    #[cfg(not(feature = "std"))] println!("Initrd yöneticisi kaynağı '{}' ediniliyor...", initrd_manager_resource_id);


    // Kaynağı edin
    match resource::acquire(initrd_manager_resource_id, resource::MODE_WRITE) { // Yazma izni isteyelim
        Ok(manager_handle) => {
            #[cfg(feature = "std")] std::println!("Initrd yöneticisi kaynağı edinildi. Handle: {:?}", manager_handle);
            #[cfg(not(feature = "std"))] println!("Initrd yö yöneticisi kaynağı edinildi. Handle: {:?}", manager_handle);


            // Varsayımsal resource::control isteği: INITRD_REGISTER
            // Bu isteğin adres ve boyutu 64-bit argüman olarak aldığını varsayalım.
            const INITRD_REGISTER_REQUEST: u64 = 1; // Örnek kontrol isteği kodu

            #[cfg(feature = "std")] std::println!("Initrd kaydı için control isteği gönderiliyor...");
            #[cfg(not(feature = "std"))] println!("Initrd kaydı için control isteği gönderiliyor...");


            // control çağrısı Result<i64, SahneError> döner. Başarılı dönüş değeri yoruma bağlıdır.
            match resource::control(manager_handle, INITRD_REGISTER_REQUEST, address as u64) { // Argüman 1: adres
                 Ok(raw_result) => {
                     // Başarılı dönüş değerini (raw_result) yorumla. Genellikle 0 başarılıdır.
                     if raw_result >= 0 { // Negatif değerler resource::control tarafından hata olarak döndürülür
                         #[cfg(feature = "std")] std::println!("Initrd kaydı başarılı. Kontrol sonucu: {}", raw_result);
                         #[cfg(not(feature = "std"))] println!("Initrd kaydı başarılı. Kontrol sonucu: {}", raw_result);

                         // Not: resource::control fonksiyonu sadece 3 argüman alır (handle, request, arg).
                         // Initrd kaydı için hem adres hem boyut gerekiyorsa, API'nin
                         // ya adres+boyutu paketleyen bir struct'ın pointer'ını 3. argüman olarak alması
                         // ya da resource::control API'sinin daha fazla argümanı desteklemesi gerekir.
                         // Mevcut API tanımına göre (sadece 1 arg), bu basit bir simülasyondur
                         // veya API'nin genişletilmesi gereklidir.
                         // Eğer API genişletilemezse, Shared Memory + IPC mesajlaşması ile bilgi göndermek gerekebilir.

                         // Basitlik adına, boyut bilgisinin başka bir kontrol isteğiyle veya
                         // adres argümanının bir parçası olarak (örn. struct pointer) gittiğini varsayalım.
                         // Mevcut API ile tam uyum için, belki adres ve boyut ayrı control çağrılarıyla gider?
                         // Veya adres argümanı, bir SharedMemory handle'ı + offset'i temsil eder?
                         // En basit varsayım: resource::control(handle, INITRD_REGISTER_REQUEST, (adres << 32 | boyut)) gibi bir paketleme (eğer boyut küçükse).
                         // Veya adres argümanı olarak struct pointer: resource::control(handle, INITRD_REGISTER_REQUEST, initrd_info_struct_ptr as u64)
                         // Mevcut API'ye en uygun olan, tek bir u64 argümanı kullanmaktır. İkinci argüman olarak boyutu gönderebilsek ideal olurdu.
                         // API'yi temel alarak, adres ve boyutun tek argümanla veya birden çok control çağrısıyla gittiğini varsayalım.
                         // Şimdilik tek control çağrısı ve adres argümanını kullanalım, boyutun başka yolla iletildiğini varsayarak.

                         Ok(()) // Başarıyı bildir
                     } else {
                         // resource::control'ün negatif değer döndürmesi zaten hata olmalıydı.
                         // Buraya düşmek, API veya çekirdekte beklenmedik bir durum olduğunu gösterebilir.
                         #[cfg(feature = "std")] std::eprintln!("Initrd kaydı başarısız: control çağrısı negatif değer döndürdü {}", raw_result);
                         #[cfg(not(feature = "std"))] eprintln!("Initrd kaydı başarısız: control çağrısı negatif değer döndürdü {}", raw_result);
                         // Hata zaten Err(SahneError) olarak map edilmiş olmalıydı.
                         // Eğer bu Err'den gelen bir dönüş değilse, UnknownSystemCall veya benzeri bir hata dönebiliriz.
                         Err(InitrdError::SahneApiError(SahneError::UnknownSystemCall)) // Simülasyon hatası
                     }
                 }
                 Err(e) => {
                     // resource::control çağrısı sırasında Sahne64 API hatası oluştu
                     #[cfg(feature = "std")] std::eprintln!("Initrd kaydı için control çağrısı hatası: {:?}", e);
                     #[cfg(not(feature = "std"))] eprintln!("Initrd kaydı için control çağrısı hatası: {:?}", e);
                     Err(InitrdError::from(e)) // SahneError'ı InitrdError'a çevir
                 }
            }

            // Not: manager_handle artık kullanılmayacaksa burada serbest bırakılabilir.
            // Eğer manager kalıcı ise veya başka işlemler için gerekliyse tutulur.
             resource::release(manager_handle); // Eğer handle geçici ise
        }
        Err(e) => {
            // Initrd yöneticisi kaynağı edinilemedi hatası
            #[cfg(feature = "std")] std::eprintln!("Initrd yöneticisi kaynağı '{}' edinilemedi: {:?}", initrd_manager_resource_id, e);
            #[cfg(not(feature = "std"))] eprintln!("Initrd yö yöneticisi kaynağı '{}' edinilemedi: {:?}", initrd_manager_resource_id, e);
            Err(InitrdError::ResourceManagerAcquisitionFailed(e)) // Özel hata varyantı kullan
        }
    }
}

// --- Örnek Kullanım (Platform başlangıç kodundan çağrılır) ---
// Bu fonksiyon normal bir main değildir, platformun _start fonksiyonu tarafından çağrılır.
// Test amacıyla bir #[cfg(feature = "std")] main eklenebilir, ancak gerçek kullanım _start'tadır.

#[cfg(feature = "std")]
fn main() {
     // Bu örnek std ortamında çalışacak şekilde yapılandırılmıştır.
     // no_std ortamında Sahne64'ün çıktı makroları veya özel bir çıktı mekanizması gerektirir.

     #[cfg(feature = "std")] std::println!("Initrd Modülü Test Örneği (std)");
     #[cfg(not(feature = "std"))] println!("Initrd Modülü Test Örneği (no_std)");


     // Varsayımsal initrd belleği
     let mut initrd_memory: [u8; 4096] = [0; 4096]; // 4KB initrd simülasyonu
     // Normalde bootloader bu belleği yükler.
      initrd içeriğini simüle etmek için ilk birkaç byte'a "cpio" signature yazalım
      initrd_memory[0..4].copy_from_slice(b"cpio"); // Slice kullanmak için core::slice import et
     unsafe { ptr::copy_nonoverlapping(b"cpio".as_ptr(), initrd_memory.as_mut_ptr(), 4); }


     let initrd_address = initrd_memory.as_ptr() as usize;
     let initrd_size = initrd_memory.len();

     // Initrd'yi başlat/kaydet
     match init(initrd_address, initrd_size) {
         Ok(()) => {
             #[cfg(feature = "std")] std::println!("Initrd başarıyla kaydedildi!");
             #[cfg(not(feature = "std"))] println!("Initrd başarıyla kaydedildi!");

             // Artık Sahne64 Kaynak API'sini kullanarak initrd içindeki dosyalara erişilebilir olmalı:
              match resource::acquire("sahne://initrd/bin/init", resource::MODE_READ) { ... }
         }
         Err(e) => {
              #[cfg(feature = "std")] std::eprintln!("Initrd kaydı başarısız: {:?}", e);
              #[cfg(not(feature = "std"))] eprintln!("Initrd kaydı başarısız: {:?}", e);
         }
     }
}
