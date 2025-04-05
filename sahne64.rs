#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz
#![allow(dead_code)] // Henüz kullanılmayan kodlar için uyarı vermesin

// Çeşitli platformlar ve mimariler için konfigürasyon (örnek)
#[cfg(any(target_arch = "riscv64", target_arch = "aarch64", target_arch = "x86_64", target_arch = "sparc64", target_arch = "openrisc", target_arch = "powerpc64", target_arch = "loongarch64", target_arch = "elbrus", target_arch = "mips64"))]
pub mod arch {
    // Mimariye özel detaylar buraya gelebilir (örneğin, sistem çağrı numaraları)
    pub const SYSCALL_MEMORY_ALLOCATE: u64 = 1;
    pub const SYSCALL_MEMORY_FREE: u64 = 2;
    pub const SYSCALL_CREATE_PROCESS: u64 = 3;
    pub const SYSCALL_EXIT_PROCESS: u64 = 4;
    pub const SYSCALL_FILE_OPEN: u64 = 5; // Dosya açma sistem çağrısı numarası
    pub const SYSCALL_FILE_READ: u64 = 6; // Dosyadan okuma sistem çağrısı numarası
    pub const SYSCALL_FILE_WRITE: u64 = 7; // Dosyaya yazma sistem çağrısı numarası
    pub const SYSCALL_FILE_CLOSE: u64 = 8; // Dosyayı kapatma sistem çağrısı numarası
    pub const SYSCALL_GET_PID: u64 = 9; // Süreç ID'sini alma
    pub const SYSCALL_SLEEP: u64 = 10; // Süreci uyutma
    pub const SYSCALL_MUTEX_CREATE: u64 = 11; // Muteks oluşturma
    pub const SYSCALL_MUTEX_LOCK: u64 = 12; // Muteksi kilitleme
    pub const SYSCALL_MUTEX_UNLOCK: u64 = 13; // Muteksi kilidini açma
    pub const SYSCALL_THREAD_CREATE: u64 = 14; // Yeni bir thread oluşturma
    pub const SYSCALL_THREAD_EXIT: u64 = 15; // Mevcut thread'i sonlandırma
    pub const SYSCALL_GET_TIME: u64 = 16; // Sistem saatini alma
    pub const SYSCALL_CREATE_SHARED_MEMORY: u64 = 17; // Paylaşımlı bellek oluşturma
    pub const SYSCALL_MAP_SHARED_MEMORY: u64 = 18; // Paylaşımlı belleği adres alanına eşleme
    pub const SYSCALL_UNMAP_SHARED_MEMORY: u64 = 19; // Paylaşımlı belleğin eşlemesini kaldırma
    pub const SYSCALL_SEND_MESSAGE: u64 = 20; // Süreçler arası mesaj gönderme
    pub const SYSCALL_RECEIVE_MESSAGE: u64 = 21; // Süreçler arası mesaj alma
    pub const SYSCALL_GET_KERNEL_INFO: u64 = 100; // Çekirdek bilgisi alma sistem çağrısı numarası
    pub const SYSCALL_YIELD: u64 = 101; // CPU'yu başka bir sürece bırakma
    pub const SYSCALL_IOCTL: u64 = 102; // Aygıt sürücülerine özel komutlar gönderme
}

// Hata türü
#[derive(Debug)]
pub enum SahneError {
    OutOfMemory,
    InvalidAddress,
    InvalidParameter,
    FileNotFound,
    PermissionDenied,
    FileAlreadyExists,
    InvalidFileDescriptor,
    ResourceBusy,
    Interrupted,
    NoMessage,
    InvalidOperation,
    NotSupported,
    // ... diğer hata türleri
    UnknownSystemCall,
    ProcessCreationFailed, // Süreç oluşturma hatası için daha spesifik bir tür
}

// Sistem çağrısı arayüzü (çekirdeğe geçiş mekanizması)
extern "sysv64" { // veya işletim sisteminizin kullandığı çağrı standardı
    fn syscall(number: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i64;
}

// Bellek yönetimi modülü
pub mod memory {
    use super::{SahneError, arch, syscall};

    /// Belirtilen boyutta bellek ayırır.
    ///
    /// # Hatalar
    ///
    /// * `SahneError::OutOfMemory`: Yeterli bellek yoksa döner.
    pub fn allocate(size: usize) -> Result<*mut u8, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_MEMORY_ALLOCATE, size as u64, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(SahneError::OutOfMemory)
        } else {
            Ok(result as *mut u8)
        }
    }

    /// Daha önce ayrılmış bir belleği serbest bırakır.
    ///
    /// # Hatalar
    ///
    /// * `SahneError::InvalidAddress`: Geçersiz bir adres verilirse döner.
    pub fn free(ptr: *mut u8, size: usize) -> Result<(), SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_MEMORY_FREE, ptr as u64, size as u64, 0, 0, 0)
        };
        if result < 0 {
            Err(SahneError::InvalidAddress)
        } else {
            Ok(())
        }
    }

    /// Belirtilen boyutta paylaşımlı bellek alanı oluşturur.
    ///
    /// # Hatalar
    ///
    /// * `SahneError::OutOfMemory`: Yeterli bellek yoksa döner.
    pub fn create_shared(size: usize) -> Result<u64, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_CREATE_SHARED_MEMORY, size as u64, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(SahneError::OutOfMemory)
        } else {
            Ok(result as u64) // Paylaşımlı bellek ID'si döner
        }
    }

    /// Paylaşımlı bellek alanını mevcut sürecin adres alanına eşler.
    ///
    /// # Hatalar
    ///
    /// * `SahneError::InvalidParameter`: Geçersiz bir ID verilirse döner.
    pub fn map_shared(id: u64, offset: usize, size: usize) -> Result<*mut u8, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_MAP_SHARED_MEMORY, id, offset as u64, size as u64, 0, 0)
        };
        if result < 0 {
            Err(SahneError::InvalidParameter)
        } else {
            Ok(result as *mut u8)
        }
    }

    /// Paylaşımlı bellek alanının eşlemesini kaldırır.
    ///
    /// # Hatalar
    ///
    /// * `SahneError::InvalidAddress`: Geçersiz bir adres verilirse döner.
    pub fn unmap_shared(addr: *mut u8, size: usize) -> Result<(), SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_UNMAP_SHARED_MEMORY, addr as u64, size as u64, 0, 0, 0)
        };
        if result < 0 {
            Err(SahneError::InvalidAddress)
        } else {
            Ok(())
        }
    }
}

// Süreç yönetimi modülü
pub mod process {
    use super::{SahneError, arch, syscall};

    /// Yeni bir süreç oluşturur.
    ///
    /// # Hatalar
    ///
    /// * `SahneError::InvalidParameter`: Geçersiz bir parametre verilirse döner.
    /// * `SahneError::ProcessCreationFailed`: Süreç oluşturulamazsa döner.
    pub fn create(path: &str) -> Result<u64, SahneError> {
        let path_ptr = path.as_ptr() as u64;
        let path_len = path.len() as u64;
        let result = unsafe {
            syscall(arch::SYSCALL_CREATE_PROCESS, path_ptr, path_len, 0, 0, 0)
        };
        if result < 0 {
            // İşletim sisteminden dönen hata koduna göre daha spesifik hatalar döndürülebilir.
            // Şimdilik genel bir hata dönülüyor.
            if result == -22 { // Örnek hata kodu: EINVAL (Geçersiz argüman)
                Err(SahneError::InvalidParameter)
            } else {
                Err(SahneError::ProcessCreationFailed)
            }
        } else {
            Ok(result as u64)
        }
    }

    /// Mevcut süreci sonlandırır.
    pub fn exit(code: i32) -> ! {
        unsafe {
            syscall(arch::SYSCALL_EXIT_PROCESS, code as u64, 0, 0, 0, 0);
        }
        loop {} // Sürecin gerçekten sonlandığından emin olmak için sonsuz döngü
    }

    /// Mevcut sürecin ID'sini (PID) alır.
    pub fn get_pid() -> Result<u64, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_GET_PID, 0, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(SahneError::UnknownSystemCall)
        } else {
            Ok(result as u64)
        }
    }

    /// Mevcut süreci belirtilen milisaniye kadar uyutur.
    pub fn sleep(milliseconds: u64) -> Result<(), SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_SLEEP, milliseconds, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(SahneError::Interrupted) // Uyku kesintiye uğradıysa
        } else {
            Ok(())
        }
    }

    /// Yeni bir thread oluşturur.
    ///
    /// # Hatalar
    ///
    /// * `SahneError::InvalidParameter`: Geçersiz bir parametre verilirse döner.
    pub fn create_thread(entry_point: u64, stack_size: usize, arg: u64) -> Result<u64, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_THREAD_CREATE, entry_point, stack_size as u64, arg, 0, 0)
        };
        if result < 0 {
            Err(SahneError::InvalidParameter)
        } else {
            Ok(result as u64) // Yeni thread'in ID'si
        }
    }

    /// Mevcut thread'i sonlandırır.
    pub fn exit_thread(code: i32) -> ! {
        unsafe {
            syscall(arch::SYSCALL_THREAD_EXIT, code as u64, 0, 0, 0, 0);
        }
        loop {}
    }

    // ... diğer süreç yönetimi fonksiyonları (IPC vb.)
}

// Dosya sistemi modülü
pub mod fs {
    use super::{SahneError, arch, syscall};

    // Dosya açma modları için sabit tanımlar (örnek)
    pub const O_RDONLY: u32 = 0;
    pub const O_WRONLY: u32 = 1;
    pub const O_RDWR: u32 = 2;
    pub const O_CREAT: u32 = 0x0100; // Dosya yoksa oluştur
    pub const O_EXCL: u32 = 0x0200;  // Dosya varsa hata döndür
    pub const O_TRUNC: u32 = 0x0400; // Dosyayı sıfır uzunlukta aç

    /// Belirtilen yolda bir dosya açar.
    ///
    /// # Hatalar
    ///
    /// * `SahneError::FileNotFound`: Dosya bulunamazsa döner.
    /// * `SahneError::PermissionDenied`: Erişim izni yoksa döner.
    pub fn open(path: &str, flags: u32) -> Result<u64, SahneError> {
        let path_ptr = path.as_ptr() as u64;
        let path_len = path.len() as u64;
        let result = unsafe {
            syscall(arch::SYSCALL_FILE_OPEN, path_ptr, path_len, flags as u64, 0, 0)
        };
        if result < 0 {
            // İşletim sisteminden dönen hata kodlarına göre hataları çeviriyoruz.
            // Bu örnek kodlar tamamen varsayımsaldır. Gerçek hata kodları işletim sistemine özeldir.
            if result == -2 { // Örnek hata kodu: ENOENT (Dosya veya dizin yok)
                Err(SahneError::FileNotFound)
            } else if result == -13 { // Örnek hata kodu: EACCES (İzin reddedildi)
                Err(SahneError::PermissionDenied)
            } else {
                Err(SahneError::UnknownSystemCall)
            }
        } else {
            Ok(result as u64)
        }
    }

    /// Belirtilen dosya tanımlayıcısından (file descriptor) veri okur.
    ///
    /// # Hatalar
    ///
    /// * `SahneError::InvalidFileDescriptor`: Geçersiz bir dosya tanımlayıcısı verilirse döner.
    pub fn read(fd: u64, buffer: &mut [u8]) -> Result<usize, SahneError> {
        let buffer_ptr = buffer.as_mut_ptr() as u64;
        let buffer_len = buffer.len() as u64;
        let result = unsafe {
            syscall(arch::SYSCALL_FILE_READ, fd, buffer_ptr, buffer_len, 0, 0)
        };
        if result < 0 {
            if result == -9 { // Örnek hata kodu: EBADF (Kötü dosya numarası)
                Err(SahneError::InvalidFileDescriptor)
            } else {
                Err(SahneError::UnknownSystemCall)
            }
        } else {
            Ok(result as usize)
        }
    }

    /// Belirtilen dosya tanımlayıcısına (file descriptor) veri yazar.
    ///
    /// # Hatalar
    ///
    /// * `SahneError::InvalidFileDescriptor`: Geçersiz bir dosya tanımlayıcısı verilirse döner.
    pub fn write(fd: u64, buffer: &[u8]) -> Result<usize, SahneError> {
        let buffer_ptr = buffer.as_ptr() as u64;
        let buffer_len = buffer.len() as u64;
        let result = unsafe {
            syscall(arch::SYSCALL_FILE_WRITE, fd, buffer_ptr, buffer_len, 0, 0)
        };
        if result < 0 {
            if result == -9 { // Örnek hata kodu: EBADF (Kötü dosya numarası)
                Err(SahneError::InvalidFileDescriptor)
            } else {
                Err(SahneError::UnknownSystemCall)
            }
        } else {
            Ok(result as usize)
        }
    }

    /// Belirtilen dosya tanımlayıcısını (file descriptor) kapatır.
    ///
    /// # Hatalar
    ///
    /// * `SahneError::InvalidFileDescriptor`: Geçersiz bir dosya tanımlayıcısı verilirse döner.
    pub fn close(fd: u64) -> Result<(), SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_FILE_CLOSE, fd, 0, 0, 0, 0)
        };
        if result < 0 {
            if result == -9 { // Örnek hata kodu: EBADF (Kötü dosya numarası)
                Err(SahneError::InvalidFileDescriptor)
            } else {
                Err(SahneError::UnknownSystemCall)
            }
        } else {
            Ok(())
        }
    }

    // ... diğer dosya sistemi fonksiyonları (read, write, close vb.)
}

// Çekirdek içi bileşenlerle iletişim (örnek)
pub mod kernel {
    use super::{SahneError, arch, syscall};

    // Çekirdek bilgi türleri için sabit tanımlar (örnek)
    pub const KERNEL_INFO_VERSION: u32 = 1;
    pub const KERNEL_INFO_UPTIME: u32 = 2;
    pub const KERNEL_INFO_ARCHITECTURE: u32 = 3;

    /// Çekirdekten belirli bir bilgiyi alır.
    pub fn get_kernel_info(info_type: u32) -> Result<u64, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_GET_KERNEL_INFO, info_type as u64, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(SahneError::UnknownSystemCall)
        } else {
            Ok(result as u64)
        }
    }

    /// CPU'yu başka bir sürece bırakır.
    pub fn yield_cpu() -> Result<(), SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_YIELD, 0, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(SahneError::UnknownSystemCall)
        } else {
            Ok(())
        }
    }

    /// Aygıt sürücülerine özel komutlar göndermek için kullanılır.
    /// `ioctl` sistem çağrısını temsil eder.
    ///
    /// # Hatalar
    ///
    /// * `SahneError::InvalidFileDescriptor`: Geçersiz bir dosya tanımlayıcısı verilirse döner.
    /// * `SahneError::InvalidParameter`: Geçersiz bir komut veya argüman verilirse döner.
    /// * `SahneError::PermissionDenied`: Gerekli izinler yoksa döner.
    pub fn ioctl(fd: u64, request: u64, arg: u64) -> Result<i64, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_IOCTL, fd, request, arg, 0, 0)
        };
        if result < 0 {
            match result {
                -9 => Err(SahneError::InvalidFileDescriptor),
                -22 => Err(SahneError::InvalidParameter),
                -13 => Err(SahneError::PermissionDenied),
                _ => Err(SahneError::UnknownSystemCall),
            }
        } else {
            Ok(result)
        }
    }

    // ... diğer çekirdek iletişim fonksiyonları
}

// Senkronizasyon ilkel araçları
pub mod sync {
    use super::{SahneError, arch, syscall};

    /// Yeni bir muteks oluşturur. Başlangıçta kilidi açıktır.
    pub fn mutex_create() -> Result<u64, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_MUTEX_CREATE, 0, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(SahneError::OutOfMemory) // Muteks oluşturmak için yeterli kaynak yoksa
        } else {
            Ok(result as u64) // Muteks ID'si döner
        }
    }

    /// Belirtilen muteksi kilitler. Eğer muteks zaten kilitliyse, çağıran thread bloke olur.
    ///
    /// # Hatalar
    ///
    /// * `SahneError::InvalidParameter`: Geçersiz bir muteks ID'si verilirse döner.
    /// * `SahneError::Interrupted`: İşlem sinyal tarafından kesilirse döner.
    pub fn mutex_lock(mutex_id: u64) -> Result<(), SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_MUTEX_LOCK, mutex_id, 0, 0, 0, 0)
        };
        if result < 0 {
            match result {
                -22 => Err(SahneError::InvalidParameter),
                -4 => Err(SahneError::Interrupted),
                _ => Err(SahneError::UnknownSystemCall),
            }
        } else {
            Ok(())
        }
    }

    /// Belirtilen muteksin kilidini açar.
    ///
    /// # Hatalar
    ///
    /// * `SahneError::InvalidParameter`: Geçersiz bir muteks ID'si verilirse döner.
    /// * `SahneError::InvalidOperation`: Muteks çağıran thread tarafından tutulmuyorsa döner.
    pub fn mutex_unlock(mutex_id: u64) -> Result<(), SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_MUTEX_UNLOCK, mutex_id, 0, 0, 0, 0)
        };
        if result < 0 {
            match result {
                -22 => Err(SahneError::InvalidParameter),
                -1 => Err(SahneError::InvalidOperation), // Örnek hata kodu: EPERM
                _ => Err(SahneError::UnknownSystemCall),
            }
        } else {
            Ok(())
        }
    }
}

// Süreçler arası iletişim (IPC) modülü
pub mod ipc {
    use super::{SahneError, arch, syscall};

    /// Belirli bir boyutta bir mesaj kuyruğu oluşturur.
    pub fn create_message_queue(capacity: usize) -> Result<u64, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_CREATE_SHARED_MEMORY, capacity as u64, 0, 0, 0, 0) // Mesaj kuyruğu için paylaşımlı bellek kullanabiliriz
        };
        if result < 0 {
            Err(SahneError::OutOfMemory)
        } else {
            Ok(result as u64) // Paylaşımlı bellek ID'si (mesaj kuyruğu olarak kullanılacak)
        }
    }

    /// Belirtilen süreç ID'sine bir mesaj gönderir.
    ///
    /// # Hatalar
    ///
    /// * `SahneError::InvalidParameter`: Geçersiz bir süreç ID'si veya mesaj verilirse döner.
    /// * `SahneError::ResourceBusy`: Alıcı süreç mesaj kuyruğu doluysa döner.
    pub fn send_message(pid: u64, message_ptr: *const u8, message_len: usize) -> Result<(), SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_SEND_MESSAGE, pid, message_ptr as u64, message_len as u64, 0, 0)
        };
        if result < 0 {
            match result {
                -22 => Err(SahneError::InvalidParameter),
                -11 => Err(SahneError::ResourceBusy), // Örnek hata kodu: EAGAIN
                _ => Err(SahneError::UnknownSystemCall),
            }
        } else {
            Ok(())
        }
    }

    /// Mevcut süreç için bir mesaj alır. Eğer mesaj yoksa, işlem bloke olabilir.
    ///
    /// # Hatalar
    ///
    /// * `SahneError::NoMessage`: Mesaj kuyruğu boşsa ve bloke etme istenmediyse döner.
    /// * `SahneError::InvalidAddress`: Geçersiz bir arabellek adresi verilirse döner.
    /// * `SahneError::Interrupted`: İşlem sinyal tarafından kesilirse döner.
    pub fn receive_message(buffer_ptr: *mut u8, buffer_len: usize) -> Result<usize, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_RECEIVE_MESSAGE, buffer_ptr as u64, buffer_len as u64, 0, 0, 0)
        };
        if result < 0 {
            match result {
                -61 => Err(SahneError::NoMessage), // Örnek hata kodu: ENOMSG
                -14 => Err(SahneError::InvalidAddress), // Örnek hata kodu: EFAULT
                -4 => Err(SahneError::Interrupted),
                _ => Err(SahneError::UnknownSystemCall),
            }
        } else {
            Ok(result as usize) // Alınan mesajın boyutu
        }
    }
}

// Örnek bir kullanıcı alanı programı
#[cfg(feature = "std")] // Standart kütüphane özelliği aktifse çalışır
fn main() {
    println!("Sahne64 kullanıcı alanı programı çalışıyor!");

    println!("Süreç ID: {:?}", process::get_pid());

    match memory::allocate(1024) {
        Ok(ptr) => {
            println!("1024 byte bellek ayrıldı: {:?}", ptr);
            match memory::free(ptr, 1024) {
                Ok(_) => println!("Bellek serbest bırakıldı."),
                Err(e) => eprintln!("Bellek serbest bırakılırken hata oluştu: {:?}", e),
            }
        }
        Err(e) => eprintln!("Bellek ayırma hatası: {:?}", e),
    }

    match process::create("/path/to/another_program") {
        Ok(pid) => println!("Yeni süreç oluşturuldu, PID: {}", pid),
        Err(e) => eprintln!("Süreç oluşturma hatası: {:?}", e),
    }

    match fs::open("/path/to/a_file.txt", fs::O_RDONLY) {
        Ok(fd) => {
            println!("Dosya açıldı, dosya tanımlayıcısı: {}", fd);
            let mut buffer = [0u8; 128];
            match fs::read(fd, &mut buffer) {
                Ok(bytes_read) => println!("Dosyadan {} byte okundu.", bytes_read),
                Err(e) => eprintln!("Dosyadan okuma hatası: {:?}", e),
            }
            match fs::close(fd) {
                Ok(_) => println!("Dosya kapatıldı."),
                Err(e) => eprintln!("Dosya kapatma hatası: {:?}", e),
            }
        }
        Err(e) => eprintln!("Dosya açma hatası: {:?}", e),
    }

    match kernel::get_kernel_info(kernel::KERNEL_INFO_VERSION) {
        Ok(info) => println!("Çekirdek versiyonu: {}", info),
        Err(e) => eprintln!("Çekirdek bilgisi alınırken hata oluştu: {:?}", e),
    }

    match sync::mutex_create() {
        Ok(mutex_id) => {
            println!("Muteks oluşturuldu, ID: {}", mutex_id);
            match sync::mutex_lock(mutex_id) {
                Ok(_) => {
                    println!("Muteks kilitlendi.");
                    match sync::mutex_unlock(mutex_id) {
                        Ok(_) => println!("Muteks kilidi açıldı."),
                        Err(e) => eprintln!("Muteks kilidi açılırken hata: {:?}", e),
                    }
                }
                Err(e) => eprintln!("Muteks kilitlenirken hata: {:?}", e),
            }
        }
        Err(e) => eprintln!("Muteks oluşturma hatası: {:?}", e),
    }

    match ipc::create_message_queue(16) {
        Ok(queue_id) => {
            println!("Mesaj kuyruğu oluşturuldu, ID: {}", queue_id);
            let message = b"Merhaba Kernel!";
            match ipc::send_message(1, message.as_ptr(), message.len()) {
                Ok(_) => println!("Çekirdeğe mesaj gönderildi."),
                Err(e) => eprintln!("Mesaj gönderme hatası: {:?}", e),
            }
            let mut buffer = [0u8; 32];
            match ipc::receive_message(buffer.as_mut_ptr(), buffer.len()) {
                Ok(received_len) => {
                    println!("Alınan mesaj: {:?}", &buffer[..received_len]);
                }
                Err(e) => eprintln!("Mesaj alma hatası: {:?}", e),
            }
        }
        Err(e) => eprintln!("Mesaj kuyruğu oluşturma hatası: {:?}", e),
    }

    process::exit(0);
}

// Standart kütüphanenin bazı temel fonksiyonlarının (örneğin println!) kendi implementasyonunuz
// veya harici bir crate (örneğin core::fmt) kullanılarak sağlanması gerekebilir.
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// Bu kısım, no_std ortamında println! gibi makroların çalışması için gereklidir.
// Gerçek bir CustomOS ortamında, bu işlevselliği çekirdek üzerinden bir sistem çağrısı ile
// veya özel bir donanım sürücüsü ile sağlamanız gerekebilir.
// Aşağıdaki kod, core::fmt kütüphanesini kullanarak basit bir formatlama örneği sunar.
// Ancak, gerçek bir çıktı mekanizması (örneğin, UART) olmadan bu çıktıları göremezsiniz.
#[cfg(not(feature = "std"))]
mod print {
    use core::fmt;
    use core::fmt::Write;

    struct Stdout;

    impl fmt::Write for Stdout {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            // Burada gerçek çıktı mekanizmasına (örneğin, bir UART sürücüsüne) erişim olmalı.
            // Bu örnekte, çıktı kaybolacaktır çünkü gerçek bir çıktı yok.
            // Gerçek bir işletim sisteminde, bu kısım donanıma özel olacaktır.
            Ok(())
        }
    }

    #[macro_export]
    macro_rules! print {
        ($($arg:tt)*) => ({
            let mut stdout = $crate::print::Stdout;
            core::fmt::write(&mut stdout, core::format_args!($($arg)*)).unwrap();
        });
    }

    #[macro_export]
    macro_rules! println {
        () => ($crate::print!("\n"));
        ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
    }
}