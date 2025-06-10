#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz

// Sahne64 API'sine erişim için gerekli modülleri içeri aktar
// Bu, 'sahne64' crate'ine bağımlılık anlamına gelir.
use sahne64::{
    resource,  // Kaynak yönetimi (cihazlar, dosyalar vb.)
    task,      // Görev (process) yönetimi
    kernel,    // Çekirdek bilgisi ve genel işlemler
    memory,    // Bellek yönetimi (isteğe bağlı olarak kullanılabilir)
    SahneError, // Hata türü
    Handle,    // Kaynak tanıtıcısı
};

// Konsol modülünü içeri aktar (varsayılan çıktı için)
// Konsol modülü Sahne64 kaynak API'sini kullanır.
use crate::console; // Konsol implementasyonumuz

// Çıktı makrolarını kullanabilmek için (Sahne64 tarafından sağlanan)
// Bu, ya crate root'ta #[macro_use] extern crate sahne64; ile yapılır
// ya da Sahne64 crate'i makroları public olarak dışa aktarırsa buradan import edilir.
// Bu örnekte, #[cfg] ile std/no_std çıktısını ayarlayarak makroların
// uygun ortamda kullanılabilir olduğunu varsayıyoruz.

// Sistem başlangıç fonksiyonu
// Bu fonksiyon, linker script tarafından çağrılan ilk koddur.
// Genellikle C ABI uyumlu ve isim karıştırmayı (mangling) önlemek için
// #[no_mangle] extern "C" olarak işaretlenir.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Dikkat: Bu fonksiyon std::prelude ve global allocator gibi şeylere
    // güvenmemelidir, çünkü bunlar henüz başlatılmamış olabilir.

    // 1. Konsolu Başlat
    // console::init() fonksiyonu, USB seri gibi platforma özgü bir
    // konsol kaynağını Sahne64 resource API'sini kullanarak edinir ve ayarlar.
    console::init();

    // Konsol kullanıma hazır olduktan sonra çıktı verebiliriz
    // Çıktı makroları için cfg ayarı
    #[cfg(feature = "std")] std::println!("Sahne64 Platform Başlangıcı: Konsol Başlatıldı.");
    #[cfg(not(feature = "std"))] println!("Sahne64 Platform Başlangıcı: Konsol Başlatıldı.");


    // 2. Temel Çekirdek Bilgilerini Al (Örnek)
    // Çekirdek API'sini kullanarak sistem hakkında bilgi alalım.
    #[cfg(feature = "std")] std::println!("Çekirdek Bilgileri Sorgulanıyor...");
    #[cfg(not(feature = "std"))] println!("Çekirdek Bilgileri Sorgulanıyor...");

    match kernel::get_info(kernel::KERNEL_INFO_VERSION_MAJOR) {
        Ok(major_ver) => {
            #[cfg(feature = "std")] std::println!(" Çekirdek Ana Versiyon: {}", major_ver);
            #[cfg(not(feature = "std"))] println!(" Çekirdek Ana Versiyon: {}", major_ver);
        }
        Err(e) => {
             #[cfg(feature = "std")] std::eprintln!(" Çekirdek ana versiyonu alınamadı: {:?}", e);
             #[cfg(not(feature = "std"))] eprintln!(" Çekirdek ana versiyonu alınamadı: {:?}", e);
        }
    }

    match kernel::get_info(kernel::KERNEL_INFO_ARCHITECTURE) {
        Ok(arch_code) => {
             // Mimari kodunu okunabilir bir stringe çevirmek gerekebilir.
             #[cfg(feature = "std")] std::println!(" Çekirdek Mimarisi Kodu: {}", arch_code);
             #[cfg(not(feature = "std"))] println!(" Çekirdek Mimarisi Kodu: {}", arch_code);
        }
        Err(e) => {
             #[cfg(feature = "std")] std::eprintln!(" Çekirdek mimari kodu alınamadı: {:?}", e);
             #[cfg(not(feature = "std"))] eprintln!(" Çekirdek mimari kodu alınamadı: {:?}", e);
        }
    }

    // 3. İlk Görevi (Task) Başlat
    // Sistemin çalışmaya başlayacağı ilk kullanıcı alanı görevini başlatalım.
    // Bunun için çalıştırılabilir kod kaynağının Handle'ına ihtiyacımız var.
    // Bu Handle, genellikle bir dosya sistemi kaynağından (resource) edinilir.
    let init_task_resource_id = "sahne://bin/init"; // Varsayımsal ilk görev ikili dosyası ID'si

    #[cfg(feature = "std")] std::println!("İlk görev '{}' başlatılıyor...", init_task_resource_id);
    #[cfg(not(feature = "std"))] println!("İlk görev '{}' başlatılıyor...", init_task_resource_id);


    // Çalıştırılabilir kod kaynağını edin
    // Bu, dosya sistemi sürücüsü gibi bir resource sağlayıcı tarafından karşılanacaktır.
    match resource::acquire(init_task_resource_id, resource::MODE_READ) { // Okuma izni yeterli olabilir
        Ok(code_handle) => {
            // Görev başlatma argümanları (isteğe bağlı)
            let args: &[u8] = b"--init --verbose";

            // Görevi başlat (spawn)
            match task::spawn(code_handle, args) {
                Ok(task_id) => {
                    #[cfg(feature = "std")] std::println!(" İlk görev başlatıldı, Task ID: {:?}", task_id);
                    #[cfg(not(feature = "std"))] println!(" İlk görev başlatıldı, Task ID: {:?}", task_id);

                    // Not: resource::acquire ile edinilen code_handle, spawn edildikten sonra
                    // bu görev tarafından serbest bırakılmalı veya devredilmelidir.
                    // Basitlik adına burada serbest bırakmıyoruz, ancak gerçekte yönetilmelidir.
                     resource::release(code_handle); // Normalde spawn başarılıysa kaynak bırakılır
                }
                Err(e) => {
                    // Görev başlatma hatası genellikle kritik bir durumdur.
                    #[cfg(feature = "std")] std::eprintln!(" İlk görev başlatılamadı: {:?}", e);
                    #[cfg(not(feature = "std"))] eprintln!(" İlk görev başlatılamadı: {:?}", e);
                    // Kritk hata durumunda sistemin durması gerekebilir
                    halt_system();
                }
            }
             // Elde edilen code_handle artık bu platform kodu tarafından tutulmamalıdır
             // veya göreve devredilmiş olmalıdır. Eğer devredilmediyse burada bırakılır.
              resource::release(code_handle); // Eğer handle göreve devredilmediyse burada bırakılır
        }
        Err(e) => {
            // Kaynak (çalıştırılabilir dosya) edinilemedi hatası kritik olabilir.
             #[cfg(feature = "std")] std::eprintln!(" İlk görev ikili dosyası '{}' edinilemedi: {:?}", init_task_resource_id, e);
             #[cfg(not(feature = "std"))] eprintln!(" İlk görev ikili dosyası '{}' edinilemedi: {:?}", init_task_resource_id, e);
            // Kritk hata durumunda sistemin durması gerekebilir
            halt_system();
        }
    }

    // 4. Kontrolü Devret veya Bekle
    // Platform başlatma tamamlandı. Artık kontrolü başlatılan görevlere
    // veya çekirdek zamanlayıcıya devretmeliyiz. Genellikle _start fonksiyonu
    // asla geri dönmez. Sonsuz bir döngüye girmek veya bir bekleme durumuna geçmek yaygındır.
    // Alternatif olarak, Sahne64'ün bir task bitişini bekleme mekanizması olabilir.
    // En basit yol sonsuz döngüdür.
    #[cfg(feature = "std")] std::println!("Platform başlatma tamamlandı. Sistem çalışıyor...");
    #[cfg(not(feature = "std"))] println!("Platform başlatma tamamlandı. Sistem çalışıyor...");


    // _start fonksiyonu asla geri dönmez
    loop {
        // Çok basit sistemlerde burada işlemci boş döngüde bekler.
        // Gerçek OS'lerde zamanlayıcı çalışıyor ve görevler arasında geçiş yapıyordur.
        // Eğer Sahne64 çekirdeği zamanlayıcıyı başlatıyorsa, burası muhtemelen
        // çekirdek idle döngüsüne girecektir.
        core::hint::spin_loop(); // İşlemciyi meşgul etmeden bekleme ipucu
    }

    // NOT: task::exit(0); gibi bir çağrı _start içinde yapılmaz,
    // bu başlatılan görevler tarafından yapılır.
}

// Kritik hata durumunda sistemi durduran fonksiyon
// Gerçek donanımda bu, işlemciyi durdurmak veya resetlemek gibi bir işlem içerebilir.
fn halt_system() -> ! {
     #[cfg(feature = "std")] std::eprintln!("KRİTİK HATA! Sistem durduruluyor.");
     #[cfg(not(feature = "std"))] eprintln!("KRİTİK HATA! Sistem durduruluyor.");
    // Konsola hata mesajı yazdıktan sonra sonsuz döngü
    loop {
        core::hint::spin_loop();
    }
}
