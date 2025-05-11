#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz, kullanıcı alanında minimal ortamda çalışırız
#![no_main] // Rust'ın default main fonksiyonunu kullanmıyoruz, kendi giriş noktamızı tanımlayacağız

// Bu dosya kullanıcı alanında çalıştığı için, doğrudan kernel'deki
// `KError` enum'unu veya `KHandle` struct'ını kullanamayız.
// Bunun yerine, sistem çağrılarından dönen ham i64 değerleri ile çalışırız
// veya kullanıcı alanı için tanımlanmış eşdeğerlerini (Sahne64 gibi) kullanırız.
// Burada basitlik adına ham i64 dönüş değerlerini işleyeceğiz.

// --- Sistem Çağrısı Numaraları (Karnal64'ün handle_syscall ile eşleşmeli) ---
// Bu numaralar, Karnal64'ün handle_syscall fonksiyonundaki match bloğu ile
// senkronize olmalıdır. Bunlar Sahne64 gibi bir kullanıcı alanı kütüphanesi
// tarafından sağlanabilir, biz burada kendimiz tanımlıyoruz.
const SYSCALL_TASK_EXIT: u64 = 4;
const SYSCALL_RESOURCE_ACQUIRE: u64 = 5;
const SYSCALL_RESOURCE_READ: u64 = 6;
const SYSCALL_RESOURCE_WRITE: u64 = 7;
const SYSCALL_RESOURCE_RELEASE: u64 = 8;

// --- Kaynak Erişim Modları (Karnal64'ün MODE_* sabitleri ile eşleşmeli) ---
// Bunlar da kullanıcı alanı kütüphanesi tarafından sağlanabilir.
const MODE_READ: u32 = 1 << 0;
const MODE_WRITE: u32 = 1 << 1;

// --- Temel Sistem Çağrısı Sarmalayıcıları (Placeholder) ---
// Bu fonksiyonlar aslında alt seviye assembly/C kodu ile kernel'in
// sistem çağrısı giriş noktasına (handle_syscall) zıplayan kısımları temsil eder.
// Sahne64 gibi bir kütüphane bu sarmalayıcıları sağlayacaktır.
// Biz burada fonksiyon imzalarını tanımlayıp 'extern "C"' ile dışarıdan geldiğini
// belirtiyoruz. Gerçek implementasyon linkleme aşamasında sağlanmalıdır.

/// Gerçek sistem çağrısını yapan düşük seviye fonksiyon (örneğin assembly)
extern "C" {
    fn syscall(
        number: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
        arg4: u64,
        arg5: u64,
    ) -> i64;
}

// --- Kullanıcı Alanı API Fonksiyonları (Sistem Çağrılarını Kullanır) ---
// Bu fonksiyonlar, yukarıdaki düşük seviye 'syscall' fonksiyonunu kullanarak
// daha anlamlı bir kullanıcı alanı API'si sunar. Sahne64'ün bir parçası olabilirler.

/// Kaynak edinme sistemi çağrısı sarmalayıcısı.
/// Başarı durumunda pozitif handle değeri, hata durumunda negatif KError kodu döner.
fn resource_acquire(resource_id: &str, mode: u32) -> i64 {
    let ptr = resource_id.as_ptr() as u64;
    let len = resource_id.len() as u64;
    syscall(SYSCALL_RESOURCE_ACQUIRE, ptr, len, mode as u64, 0, 0)
}

/// Kaynağa yazma sistemi çağrısı sarmalayıcısı.
/// Başarı durumunda yazılan byte sayısı, hata durumunda negatif KError kodu döner.
fn resource_write(handle: u64, buffer: &[u8]) -> i64 {
    let ptr = buffer.as_ptr() as u64;
    let len = buffer.len() as u64;
    syscall(SYSCALL_RESOURCE_WRITE, handle, ptr, len, 0, 0)
}

/// Kaynak handle'ını serbest bırakma sistemi çağrısı sarmalayıcısı.
/// Başarı durumunda 0, hata durumunda negatif KError kodu döner.
fn resource_release(handle: u64) -> i64 {
    syscall(SYSCALL_RESOURCE_RELEASE, handle, 0, 0, 0, 0)
}

/// Görevden çıkış sistemi çağrısı sarmalayıcısı.
/// Bu fonksiyon genellikle geri dönmez.
#[inline(never)] // Optimize edilip yok edilmemesi için
fn task_exit(code: i32) -> ! {
    let _ = syscall(SYSCALL_TASK_EXIT, code as u64, 0, 0, 0, 0);
    loop {} // Sistem çağrısı geri dönmezse buraya gelinmez, ama emin olmak için sonsuz döngü
}


// --- Görev Giriş Noktası ---
// Bu fonksiyon, çekirdek görevi başlattığında çağrılacak fonksiyondur.
// #[no_mangle] özniteliği, isminin değiştirilmeden bırakılmasını sağlar ki
// çekirdek veya bootloader bu fonksiyonu bulup çağırabilsin.
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Konsol kaynağını edinme (acquire)
    // Varsayım: "karnal://device/console" çekirdek tarafından kaydedilmiş bir kaynaktır.
    let console_handle_result = resource_acquire("karnal://device/console", MODE_WRITE);

    let console_handle: u64;
    if console_handle_result < 0 {
        // Handle edinilemedi, hata oluştu.
        // Gerçek bir uygulamada hata kodunu işlemek gerekir.
        // Burada basitçe hata koduyla çıkıyoruz.
        task_exit(console_handle_result as i32);
    } else {
        // Handle başarıyla edinildi (sonuç pozitif veya sıfırsa handle değeridir)
        console_handle = console_handle_result as u64;
    }

    // Konsola mesaj yazma
    let message = b"Merhaba, Karnal64!\n"; // byte slice olarak mesaj
    let write_result = resource_write(console_handle, message);

    if write_result < 0 {
         // Yazma başarısız oldu.
         // Gerçek uygulamada loglama veya hata işleme yapılabilir.
         // Burada basitçe hata koduyla çıkıyoruz.
         // Önce handle'ı serbest bırakmayı deneyebiliriz.
         let _ = resource_release(console_handle); // Serbest bırakma hatasını şimdilik yoksayalım
         task_exit(write_result as i32);
    }

    // Kullanılan handle'ı serbest bırakma
    let release_result = resource_release(console_handle);

    if release_result < 0 {
        // Serbest bırakma başarısız oldu.
        // Yine hata işleme yapılabilir.
        // Hata koduyla çıkış yapalım.
         task_exit(release_result as i32);
    }

    // Görev başarıyla tamamlandı, çıkış yap
    task_exit(0);
}

// --- Panik İşleyici ---
// no_std ortamında panik (beklenmedik hata) oluştuğunda ne yapılacağını tanımlar.
// Çekirdek ortamında genellikle sonsuz döngüye girmek veya hata ayıklama bilgisi
// yazdırmak gibi basit bir eylem yapılır.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // Panik durumunda konsola yazmaya çalışmak risklidir,
    // çünkü panik genellikle bellek veya thread sorunlarından kaynaklanır.
    // Güvenli bir şekilde panik bilgisini bir yere loglamak veya
    // sadece sonsuz döngüye girmek daha yaygındır.
    loop {}
}
