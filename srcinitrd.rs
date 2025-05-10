#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz, ilk kullanıcı süreci için tipik
#![no_main] // Kendi giriş noktamızı tanımlıyoruz (main fonksiyonu)

// Bu initrd süreci, Karnal64 çekirdeği tarafından sağlanan sistem çağrılarını kullanır.
// Aşağıdaki sabitler, Karnal64'ün handle_syscall fonksiyonunda tanımlanan
// sistem çağrısı numaralarıyla EŞLEŞMELİDİR.
// ÖNEMLİ: Karnal64 kodundaki syscall numaralarını kontrol edin ve gerekirse burayı güncelleyin.
const SYSCALL_TASK_EXIT: u64 = 4; // Karnal64::handle_syscall'daki 4'e karşılık gelir
const SYSCALL_RESOURCE_ACQUIRE: u64 = 5; // Karnal64::handle_syscall'daki 5'e karşılık gelir
const SYSCALL_RESOURCE_WRITE: u64 = 7; // Karnal64::handle_syscall'daki 7'ye karşılık gelir
const SYSCALL_TASK_SPAWN: u64 = 3; // Karnal64::handle_syscall'daki 3'e karşılık gelir

// Kaynak edinme (acquire) modları için basit bit bayrakları.
// Bunlar da Karnal64::kresource modülündeki tanımlarla eşleşmeli.
const MODE_READ: u32 = 1 << 0; // Genellikle 1
const MODE_WRITE: u32 = 1 << 1; // Genellikle 2
const MODE_EXECUTE: u32 = 1 << 2; // Genellikle 4 (Varsayım)

// Çekirdek tarafından sağlanan ham sistem çağrısı fonksiyonlarının dış bildirimleri.
// Bu fonksiyonlar doğrudan işlemci seviyesindeki 'syscall' komutunu tetikler
// ve kontrolü çekirdekteki handle_syscall fonksiyonuna devreder.
extern "C" {
    // arg1: id_ptr (*const u8), arg2: id_len (usize), arg3: mode (u32)
    // Başarı: handle (u64 pozitif), Hata: KError (-i64 negatif)
    fn sys_resource_acquire(id_ptr: *const u8, id_len: usize, mode: u32) -> i64;

    // arg1: handle (u64), arg2: buf_ptr (*const u8), arg3: buf_len (usize)
    // Başarı: yazılan_byte_sayısı (usize pozitif), Hata: KError (-i64 negatif)
    fn sys_resource_write(handle: u64, buf_ptr: *const u8, buf_len: usize) -> i64;

    // arg1: code_handle (u64), arg2: args_ptr (*const u8), arg3: args_len (usize)
    // Başarı: task_id (u64 pozitif), Hata: KError (-i64 negatif)
    fn sys_task_spawn(code_handle: u64, args_ptr: *const u8, args_len: usize) -> i64;

     arg1: exit_code (i32)
    // Bu fonksiyon geri dönmez (!), doğrudan görevi sonlandırır.
    fn sys_task_exit(code: i32) -> !;

    // TODO: İhtiyaç duyuldukça diğer sistem çağrıları buraya eklenecek (memory, messaging vb.)
}

// Panic durumunda çağrılacak fonksiyon.
// initrd paniklerse genellikle yapılabilecek pek bir şey yoktur,
// sistemi durdurmak en güvenli eylemdir.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // Eğer panik noktasına konsol handle'ı edinebildiysek, panik bilgisini yazabiliriz.
    // Ancak bu basit implementasyonda, panik anında konsolun kullanılabilir olduğunu
    // garanti etmek zordur. En basit yaklaşım süresiz döngüye girmektir.
    loop {}
}

// Temel konsol yazma fonksiyonu.
// Güvenlik notu: Çekirdek sistem çağrısı `sys_resource_write` zaten kullanıcı buffer'ını doğrulamalıdır.
fn write_to_console(console_handle: u64, message: &str) {
    let bytes = message.as_bytes();
    // sys_resource_write hata dönebilir, ancak initrd için temel yazdırmada
    // hataları ele almak genelde karmaşıktır. Şimdilik sonucu yoksayıyoruz.
    let _ = unsafe { sys_resource_write(console_handle, bytes.as_ptr(), bytes.len()) };
}

// Initrd'nin ana giriş noktası.
// Çekirdek boot işlemi bittikten sonra ilk bu fonksiyon çalıştırılır.
#[no_mangle]
pub extern "C" fn main() -> ! {
    let mut console_handle: u64 = 0; // Başlangıçta geçersiz handle değeri

    // 1. Konsol kaynağını edin. Bu, çıktı alabilmemizi sağlar.
    let console_path = "karnal://device/console"; // Çekirdeğin konsol kaynağını bu isimle kaydettiğini varsayıyoruz.
    let path_bytes = console_path.as_bytes();

    // Konsol için yazma izniyle bir handle talep et.
    let result = unsafe {
        sys_resource_acquire(path_bytes.as_ptr(), path_bytes.len(), MODE_WRITE)
    };

    if result < 0 {
        // Eğer konsol handle'ı edinemezsek, hata mesajı yazdıramayız.
        // Bu durumda yapılabilecek en iyi şey bir hata koduna dönmek (initrd için geçerli değil, çünkü geri dönmez)
        // veya basitçe süresiz döngüye girmektir.
        loop {}
    }

    console_handle = result as u64;
    write_to_console(console_handle, "Karnal64 Initrd: Konsol edinildi.\n");
    write_to_console(console_handle, "Karnal64 Initrd: Basit Dahili Init Sistemi başlatılıyor...\n");


    // 2. Asıl 'init' programının kaynağını edin.
    // Bu genellikle initrd imajının içine gömülmüş bir program olacaktır,
    // veya çekirdeğin basit bir sanal dosya sistemi sağlayıp sağlamadığına bağlıdır.
    // Yaygın bir yol, initrd içine `/sbin/init` adıyla paketlenmiş bir programın olmasıdır.
    let main_init_path = "/sbin/init"; // Başlatılacak asıl init programının yolu
    let init_path_bytes = main_init_path.as_bytes();

    // Programı çalıştırmak için execute izniyle bir handle talep et.
    let init_handle_result = unsafe {
        sys_resource_acquire(init_path_bytes.as_ptr(), init_path_bytes.len(), MODE_EXECUTE)
    };

    if init_handle_result < 0 {
        write_to_console(console_handle, "Karnal64 Initrd: '/sbin/init' kaynağı edinilemedi! Hata!\n");
        // Kaynak edinilemezse devam edemeyiz, sistemi durdur.
        loop {}
    }

    let init_handle = init_handle_result as u64;
    write_to_console(console_handle, "Karnal64 Initrd: '/sbin/init' kaynağı edinildi.\n");


    // 3. '/sbin/init' programını yeni bir görev (task) olarak başlat.
    // Şu an için argüman veya ortam değişkeni geçmiyoruz.
    let args: [u8; 0] = []; // Argümanlar için boş byte slice

    let spawn_result = unsafe {
        sys_task_spawn(init_handle, args.as_ptr(), args.len())
    };

    if spawn_result < 0 {
        write_to_console(console_handle, "Karnal64 Initrd: '/sbin/init' görevi başlatılamadı! Hata!\n");
        // Görev başlatılamazsa devam edemeyiz, sistemi durdur.
        loop {}
    }

    let init_task_id = spawn_result as u64; // Başarılı olursa TaskID döner
    write_to_console(console_handle, "Karnal64 Initrd: '/sbin/init' görevi başlatıldı. (TaskID: ");
    // TODO: TaskID'yi yazdırmak için sayıdan stringe çevirme fonksiyonu gerekli (şu an yok)
    // write_to_console(console_handle, &init_task_id.to_string());
    write_to_console(console_handle, "...)\n");


    // 4. Initrd görevi amacına ulaştı (asıl init sürecini başlattı).
    // Artık initrd görevi sonlanabilir.
    write_to_console(console_handle, "Karnal64 Initrd: Initrd görevi sonlanıyor.\n");
    unsafe {
        sys_task_exit(0); // Başarılı çıkış kodu (0) ile görevi sonlandır.
    }

    // Bu noktaya asla ulaşılmamalıdır. Eğer ulaşılırsa, bir hata var demektir.
    write_to_console(console_handle, "Karnal64 Initrd: Hata: Initrd sonlanamadı! Süresiz döngüye giriliyor.\n");
    loop {}
}
