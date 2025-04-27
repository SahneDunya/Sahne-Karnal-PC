#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz
#![allow(dead_code)] // Henüz kullanılmayan kodlar için uyarı vermesin

#[cfg(any(target_arch = "riscv64", target_arch = "aarch64", target_arch = "x86_64", target_arch = "sparc64", target_arch = "openrisc", target_arch = "powerpc64", target_arch = "loongarch64", target_arch = "elbrus", target_arch = "mips64"))]
pub mod arch {
    // Mimariye özel sistem çağrı numaraları (Sahne64 terminolojisi ile)
    pub const SYSCALL_MEMORY_ALLOCATE: u64 = 1;  // Bellek tahsis et
    pub const SYSCALL_MEMORY_RELEASE: u64 = 2;   // Bellek serbest bırak (Handle ile?) - Şimdilik adres/boyut ile
    pub const SYSCALL_TASK_SPAWN: u64 = 3;       // Yeni bir görev (task) başlat
    pub const SYSCALL_TASK_EXIT: u64 = 4;        // Mevcut görevi sonlandır
    pub const SYSCALL_RESOURCE_ACQUIRE: u64 = 5; // Bir kaynağa erişim tanıtıcısı (Handle) al
    pub const SYSCALL_RESOURCE_READ: u64 = 6;    // Kaynaktan oku (Handle ile)
    pub const SYSCALL_RESOURCE_WRITE: u64 = 7;   // Kaynağa yaz (Handle ile)
    pub const SYSCALL_RESOURCE_RELEASE: u64 = 8; // Kaynak tanıtıcısını serbest bırak
    pub const SYSCALL_GET_TASK_ID: u64 = 9;      // Mevcut görev ID'sini al
    pub const SYSCALL_TASK_SLEEP: u64 = 10;      // Görevi uyut
    pub const SYSCALL_LOCK_CREATE: u64 = 11;     // Kilit (Lock) oluştur
    pub const SYSCALL_LOCK_ACQUIRE: u64 = 12;    // Kilidi al (Bloklayabilir)
    pub const SYSCALL_LOCK_RELEASE: u64 = 13;    // Kilidi bırak
    pub const SYSCALL_THREAD_CREATE: u64 = 14;   // Yeni bir iş parçacığı (thread) oluştur
    pub const SYSCALL_THREAD_EXIT: u64 = 15;     // Mevcut iş parçacığını sonlandır
    pub const SYSCALL_GET_SYSTEM_TIME: u64 = 16; // Sistem saatini al
    pub const SYSCALL_SHARED_MEM_CREATE: u64 = 17; // Paylaşımlı bellek alanı oluştur (Handle döner)
    pub const SYSCALL_SHARED_MEM_MAP: u64 = 18;   // Paylaşımlı belleği adres alanına eşle (Handle ile)
    pub const SYSCALL_SHARED_MEM_UNMAP: u64 = 19; // Paylaşımlı bellek eşlemesini kaldır
    pub const SYSCALL_MESSAGE_SEND: u64 = 20;    // Başka bir göreve mesaj gönder (Task ID veya Handle ile)
    pub const SYSCALL_MESSAGE_RECEIVE: u64 = 21; // Mesaj al (Bloklayabilir)
    pub const SYSCALL_GET_KERNEL_INFO: u64 = 100; // Çekirdek bilgisi al
    pub const SYSCALL_TASK_YIELD: u64 = 101;     // CPU'yu başka bir göreve devret
    pub const SYSCALL_RESOURCE_CONTROL: u64 = 102;// Kaynağa özel kontrol komutu gönder (Handle ile)
}

/// Sahne64 Kaynak Tanıtıcısı (Handle).
/// Kaynaklara (dosyalar, soketler, bellek bölgeleri vb.) erişmek için kullanılır.
/// Bu, Unix'teki file descriptor'ların yerine geçer ve daha soyut bir kavramdır.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)] // Bellekte sadece u64 olarak yer kaplar
pub struct Handle(u64);

impl Handle {
    /// Geçersiz veya boş bir Handle oluşturur.
    pub const fn invalid() -> Self {
        Handle(0) // Veya çekirdeğin belirlediği başka bir geçersiz değer
    }

    /// Handle'ın geçerli olup olmadığını kontrol eder.
    pub fn is_valid(&self) -> bool {
        self.0 != Self::invalid().0
    }

    /// Handle'ın içindeki ham değeri alır (dikkatli kullanılmalı!).
    pub(crate) fn raw(&self) -> u64 {
        self.0
    }
}

/// Sahne64 Görev (Task) Tanımlayıcısı.
/// Süreç (process) yerine kullanılır.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct TaskId(u64);

impl TaskId {
    /// Geçersiz bir TaskId oluşturur.
    pub const fn invalid() -> Self {
        TaskId(0) // Veya çekirdeğin belirlediği başka bir geçersiz değer
    }

    /// TaskId'nin geçerli olup olmadığını kontrol eder.
    pub fn is_valid(&self) -> bool {
        self.0 != Self::invalid().0
    }

    /// TaskId'nin içindeki ham değeri alır (dikkatli kullanılmalı!).
    pub(crate) fn raw(&self) -> u64 {
        self.0
    }
}


// Sahne64 Hata Türleri (Unix errno'larından ziyade Sahne64 konseptlerine odaklı)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SahneError {
    OutOfMemory,          // Yetersiz bellek
    InvalidAddress,       // Geçersiz bellek adresi
    InvalidParameter,     // Fonksiyona geçersiz parametre verildi
    ResourceNotFound,     // Belirtilen kaynak bulunamadı (örn. isimle ararken)
    PermissionDenied,     // İşlem için yetki yok
    ResourceBusy,         // Kaynak şu anda meşgul (örn. kilitli dosya, dolu kuyruk)
    Interrupted,          // İşlem bir sinyal veya başka bir olayla kesildi
    NoMessage,            // Beklenen mesaj yok (non-blocking receive)
    InvalidOperation,     // Kaynak üzerinde geçersiz işlem denendi (örn. okunamaz kaynağı okumak)
    NotSupported,         // İşlem veya özellik desteklenmiyor
    UnknownSystemCall,    // Çekirdek bilinmeyen sistem çağrısı numarası aldı
    TaskCreationFailed,   // Yeni görev (task) oluşturulamadı
    InvalidHandle,        // Geçersiz veya süresi dolmuş Handle kullanıldı
    HandleLimitExceeded,  // Süreç başına düşen Handle limiti aşıldı
    NamingError,          // Kaynak isimlendirme ile ilgili hata
    CommunicationError,   // Mesajlaşma veya IPC hatası
}

// Sistem çağrısı arayüzü (çekirdeğe geçiş mekanizması)
// ABI (Application Binary Interface) genellikle platforma özgüdür, bu nedenle "sysv64"
// yaygın bir 64-bit ABI olduğu için kalabilir, ancak Sahne64 kendi ABI'sini de tanımlayabilir.
extern "sysv64" {
    fn syscall(number: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i64;
}

// Hata Kodu Çevirimi Yardımcı Fonksiyonu
// Çekirdekten dönen negatif sayıları SahneError'a çevirir.
// NOT: Gerçek Sahne64 çekirdeği kendi hata kodlarını tanımlamalıdır. Buradakiler varsayımsaldır.
fn map_kernel_error(code: i64) -> SahneError {
    match code {
        -1 => SahneError::PermissionDenied,     // EPERM gibi
        -2 => SahneError::ResourceNotFound,    // ENOENT gibi
        -3 => SahneError::TaskCreationFailed, // ESRCH gibi (belki?)
        -4 => SahneError::Interrupted,        // EINTR gibi
        -9 => SahneError::InvalidHandle,        // EBADF gibi
        -11 => SahneError::ResourceBusy,         // EAGAIN gibi
        -12 => SahneError::OutOfMemory,          // ENOMEM gibi
        -13 => SahneError::PermissionDenied,     // EACCES gibi
        -14 => SahneError::InvalidAddress,     // EFAULT gibi
        -17 => SahneError::NamingError,         // EEXIST gibi (belki?)
        -22 => SahneError::InvalidParameter,     // EINVAL gibi
        -38 => SahneError::NotSupported,        // ENOSYS gibi
        -61 => SahneError::NoMessage,           // ENOMSG gibi
        // ... diğer Sahne64'e özel hata kodları ...
        _ => SahneError::UnknownSystemCall, // Bilinmeyen veya eşlenmemiş hata
    }
}


// Bellek yönetimi modülü
pub mod memory {
    use super::{SahneError, arch, syscall, map_kernel_error, Handle};

    /// Belirtilen boyutta bellek ayırır.
    /// Başarılı olursa, ayrılan belleğe işaretçi döner.
    pub fn allocate(size: usize) -> Result<*mut u8, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_MEMORY_ALLOCATE, size as u64, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(result as *mut u8)
        }
    }

    /// Daha önce `allocate` ile ayrılmış bir belleği serbest bırakır.
    /// NOT: Sahne64, belki de bellek blokları için de Handle kullanabilir,
    /// bu durumda imza `release(handle: Handle)` şeklinde olabilirdi.
    /// Şimdilik klasik adres/boyut yaklaşımı korunuyor.
    pub fn release(ptr: *mut u8, size: usize) -> Result<(), SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_MEMORY_RELEASE, ptr as u64, size as u64, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(())
        }
    }

    /// Belirtilen boyutta paylaşımlı bellek alanı oluşturur ve bir Handle döner.
    pub fn create_shared(size: usize) -> Result<Handle, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_SHARED_MEM_CREATE, size as u64, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(Handle(result as u64))
        }
    }

    /// Paylaşımlı bellek Handle'ını mevcut görevin adres alanına eşler.
    pub fn map_shared(handle: Handle, offset: usize, size: usize) -> Result<*mut u8, SahneError> {
          if !handle.is_valid() {
              return Err(SahneError::InvalidHandle);
          }
        let result = unsafe {
            syscall(arch::SYSCALL_SHARED_MEM_MAP, handle.raw(), offset as u64, size as u64, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(result as *mut u8)
        }
    }

    /// Eşlenmiş paylaşımlı bellek alanını adres alanından kaldırır.
    pub fn unmap_shared(addr: *mut u8, size: usize) -> Result<(), SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_SHARED_MEM_UNMAP, addr as u64, size as u64, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(())
        }
    }
}

// Görev (Task) yönetimi modülü (Süreç yerine)
pub mod task {
    use super::{SahneError, arch, syscall, map_kernel_error, Handle, TaskId};

    /// Yeni bir görev (task) başlatır.
    /// Çalıştırılacak kod bir Handle ile temsil edilir (örn. bir kod kaynağı Handle'ı).
    /// Argümanlar opak bir byte dizisi olarak geçirilir.
    /// Başarılı olursa, yeni görevin TaskId'sini döner.
    ///
    /// # Argümanlar
    /// * `code_handle`: Çalıştırılacak kodu içeren kaynağın Handle'ı.
    /// * `args`: Göreve başlangıçta iletilecek argüman verisi.
    /// * `capabilities`: (Opsiyonel) Göreve verilecek başlangıç yetenekleri/handle'ları listesi.
    pub fn spawn(code_handle: Handle, args: &[u8]) -> Result<TaskId, SahneError> {
          if !code_handle.is_valid() {
              return Err(SahneError::InvalidHandle);
          }
        let args_ptr = args.as_ptr() as u64;
        let args_len = args.len() as u64;
        let result = unsafe {
            syscall(arch::SYSCALL_TASK_SPAWN, code_handle.raw(), args_ptr, args_len, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(TaskId(result as u64))
        }
    }

    /// Mevcut görevi belirtilen çıkış koduyla sonlandırır. Bu fonksiyon geri dönmez.
    pub fn exit(code: i32) -> ! {
        unsafe {
            syscall(arch::SYSCALL_TASK_EXIT, code as u64, 0, 0, 0, 0);
        }
        // Syscall başarısız olsa bile (ki olmamalı), görevi sonlandırmak için döngü.
        loop { core::hint::spin_loop(); }
    }

    /// Mevcut görevin TaskId'sini alır.
    pub fn current_id() -> Result<TaskId, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_GET_TASK_ID, 0, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(TaskId(result as u64))
        }
    }

    /// Mevcut görevi belirtilen milisaniye kadar uyutur.
    pub fn sleep(milliseconds: u64) -> Result<(), SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_TASK_SLEEP, milliseconds, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(())
        }
    }

    /// Yeni bir iş parçacığı (thread) oluşturur.
    /// İş parçacıkları aynı görev adres alanını paylaşır.
    /// `entry_point`: Yeni iş parçacığının başlangıç fonksiyon adresi.
    /// `stack_size`: Yeni iş parçacığı için ayrılacak yığın boyutu.
    /// `arg`: Başlangıç fonksiyonuna geçirilecek argüman.
    /// Başarılı olursa, yeni iş parçacığının ID'sini (belki bir ThreadId türü?) döner.
    pub fn create_thread(entry_point: u64, stack_size: usize, arg: u64) -> Result<u64, SahneError> { // Belki Result<ThreadId, SahneError> olmalı
        let result = unsafe {
            syscall(arch::SYSCALL_THREAD_CREATE, entry_point, stack_size as u64, arg, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(result as u64) // Thread ID
        }
    }

    /// Mevcut iş parçacığını sonlandırır. Bu fonksiyon geri dönmez.
    pub fn exit_thread(code: i32) -> ! {
        unsafe {
            syscall(arch::SYSCALL_THREAD_EXIT, code as u64, 0, 0, 0, 0);
        }
        loop { core::hint::spin_loop(); }
    }

    /// CPU'yu gönüllü olarak başka bir çalıştırılabilir göreve bırakır.
    pub fn yield_now() -> Result<(), SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_TASK_YIELD, 0, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(())
        }
    }
}

// Kaynak yönetimi modülü (Dosya sistemi yerine)
pub mod resource {
    use super::{SahneError, arch, syscall, map_kernel_error, Handle};

    // Kaynak açma/edinme modları için Sahne64'e özgü bayraklar
    // Bunlar Unix O_* bayraklarından farklı anlamlara gelebilir.
    pub const MODE_READ: u32 = 1 << 0;    // Kaynaktan okuma yeteneği iste
    pub const MODE_WRITE: u32 = 1 << 1;   // Kaynağa yazma yeteneği iste
    pub const MODE_CREATE: u32 = 1 << 2;  // Kaynak yoksa oluşturulsun
    pub const MODE_EXCLUSIVE: u32 = 1 << 3; // Kaynak zaten varsa hata ver (CREATE ile kullanılır)
    pub const MODE_TRUNCATE: u32 = 1 << 4; // Kaynak açılırken içeriğini sil (varsa ve yazma izni varsa)
    // ... Sahne64'e özel diğer modlar eklenebilir (örn. Append, NonBlocking vb.)

    /// Sahne64'e özgü bir kaynak adı veya tanımlayıcısı.
    /// Bu, bir string path olabileceği gibi, UUID veya başka bir yapı da olabilir.
    /// Şimdilik basitlik adına string slice kullanıyoruz.
    pub type ResourceId<'a> = &'a str;

    /// Belirtilen ID'ye sahip bir kaynağa erişim Handle'ı edinir.
    /// `id`: Kaynağı tanımlayan Sahne64'e özgü tanımlayıcı.
    /// `mode`: Kaynağa nasıl erişileceğini belirten bayraklar (MODE_*).
    pub fn acquire(id: ResourceId, mode: u32) -> Result<Handle, SahneError> {
        let id_ptr = id.as_ptr() as u64;
        let id_len = id.len() as u64;
        let result = unsafe {
            syscall(arch::SYSCALL_RESOURCE_ACQUIRE, id_ptr, id_len, mode as u64, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(Handle(result as u64))
        }
    }

    /// Belirtilen Handle ile temsil edilen kaynaktan veri okur.
    /// Okunan byte sayısını döner.
    pub fn read(handle: Handle, buffer: &mut [u8]) -> Result<usize, SahneError> {
        if !handle.is_valid() {
            return Err(SahneError::InvalidHandle);
        }
        let buffer_ptr = buffer.as_mut_ptr() as u64;
        let buffer_len = buffer.len() as u64;
        let result = unsafe {
            syscall(arch::SYSCALL_RESOURCE_READ, handle.raw(), buffer_ptr, buffer_len, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(result as usize)
        }
    }

    /// Belirtilen Handle ile temsil edilen kaynağa veri yazar.
    /// Yazılan byte sayısını döner.
    pub fn write(handle: Handle, buffer: &[u8]) -> Result<usize, SahneError> {
          if !handle.is_valid() {
              return Err(SahneError::InvalidHandle);
          }
        let buffer_ptr = buffer.as_ptr() as u64;
        let buffer_len = buffer.len() as u64;
        let result = unsafe {
            syscall(arch::SYSCALL_RESOURCE_WRITE, handle.raw(), buffer_ptr, buffer_len, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(result as usize)
        }
    }

    /// Belirtilen Handle'ı serbest bırakır, kaynağa erişimi sonlandırır.
    /// Kaynağın kendisi (eğer kalıcıysa) silinmeyebilir, sadece bu Handle geçersizleşir.
    pub fn release(handle: Handle) -> Result<(), SahneError> {
          if !handle.is_valid() {
              return Err(SahneError::InvalidHandle); // Zaten geçersiz handle'ı bırakmaya çalışma
          }
        let result = unsafe {
            syscall(arch::SYSCALL_RESOURCE_RELEASE, handle.raw(), 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(())
        }
    }

    /// Kaynağa özel kontrol komutları göndermek için kullanılır (Unix `ioctl` benzeri).
    /// `request`: Gönderilecek komutun Sahne64'e özgü kodu.
    /// `arg`: Komuta eşlik eden veri (yorumu komuta bağlı).
    pub fn control(handle: Handle, request: u64, arg: u64) -> Result<i64, SahneError> {
          if !handle.is_valid() {
              return Err(SahneError::InvalidHandle);
          }
        let result = unsafe {
            syscall(arch::SYSCALL_RESOURCE_CONTROL, handle.raw(), request, arg, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(result) // Kontrol komutunun dönüş değeri (yorumu komuta bağlı)
        }
    }
}

// Çekirdek ile genel etkileşim modülü
pub mod kernel {
    use super::{SahneError, arch, syscall, map_kernel_error};

    // Çekirdek bilgi türleri için Sahne64'e özgü sabitler
    pub const KERNEL_INFO_VERSION_MAJOR: u32 = 1;
    pub const KERNEL_INFO_VERSION_MINOR: u32 = 2;
    pub const KERNEL_INFO_BUILD_ID: u32 = 3;
    pub const KERNEL_INFO_UPTIME_SECONDS: u32 = 4; // Sistem çalışma süresi (saniye)
    pub const KERNEL_INFO_ARCHITECTURE: u32 = 5;   // Çalışan mimari (örn. ARCH_X86_64 sabiti dönebilir)
    // ... diğer Sahne64'e özgü kernel bilgileri

    /// Çekirdekten belirli bir bilgiyi alır.
    pub fn get_info(info_type: u32) -> Result<u64, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_GET_KERNEL_INFO, info_type as u64, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(result as u64)
        }
    }

    /// Sistem saatini (örneğin, epoch'tan beri geçen nanosaniye olarak) alır.
    pub fn get_time() -> Result<u64, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_GET_SYSTEM_TIME, 0, 0, 0, 0, 0)
        };
          if result < 0 {
              Err(map_kernel_error(result))
          } else {
              Ok(result as u64)
          }
    }
}

// Senkronizasyon araçları modülü (Mutex -> Lock)
pub mod sync {
    use super::{SahneError, arch, syscall, map_kernel_error, Handle};

    /// Yeni bir kilit (Lock) kaynağı oluşturur ve bunun için bir Handle döner.
    /// Başlangıçta kilit serbesttir.
    pub fn lock_create() -> Result<Handle, SahneError> {
        let result = unsafe {
            syscall(arch::SYSCALL_LOCK_CREATE, 0, 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(Handle(result as u64))
        }
    }

    /// Belirtilen Handle'a sahip kilidi almaya çalışır.
    /// Kilit başka bir thread/task tarafından tutuluyorsa, çağıran bloke olur.
    pub fn lock_acquire(lock_handle: Handle) -> Result<(), SahneError> {
          if !lock_handle.is_valid() {
              return Err(SahneError::InvalidHandle);
          }
        let result = unsafe {
            syscall(arch::SYSCALL_LOCK_ACQUIRE, lock_handle.raw(), 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(())
        }
    }

    /// Belirtilen Handle'a sahip kilidi serbest bırakır.
    /// Kilidin çağıran thread/task tarafından tutuluyor olması gerekir.
    pub fn lock_release(lock_handle: Handle) -> Result<(), SahneError> {
          if !lock_handle.is_valid() {
              return Err(SahneError::InvalidHandle);
          }
        let result = unsafe {
            syscall(arch::SYSCALL_LOCK_RELEASE, lock_handle.raw(), 0, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(())
        }
    }

    // NOT: Sahne64'te kilitler de normal kaynaklar gibi `resource::release` ile
    // tamamen yok edilebilir. `lock_release` sadece kilidi serbest bırakır, Handle'ı değil.
}

// Görevler arası iletişim (IPC) modülü (Messaging)
pub mod messaging {
    use super::{SahneError, arch, syscall, map_kernel_error, TaskId, Handle};

    // Sahne64'te mesajlaşma kanalları veya portlar da Handle ile temsil edilebilir.
    // Şimdilik TaskId üzerinden doğrudan mesajlaşmayı varsayalım.

    /// Hedef göreve (Task) bir mesaj gönderir.
    /// `target_task`: Mesajın gönderileceği görevin TaskId'si.
    /// `message`: Gönderilecek veri.
    /// Bu işlem asenkron olabilir veya hedef kuyruk doluysa bloklayabilir/hata verebilir.
    pub fn send(target_task: TaskId, message: &[u8]) -> Result<(), SahneError> {
          if !target_task.is_valid() {
              return Err(SahneError::InvalidParameter); // Veya InvalidTarget gibi özel bir hata
          }
        let msg_ptr = message.as_ptr() as u64;
        let msg_len = message.len() as u64;
        let result = unsafe {
            syscall(arch::SYSCALL_MESSAGE_SEND, target_task.raw(), msg_ptr, msg_len, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(())
        }
    }

    /// Mevcut görev için gelen bir mesajı alır.
    /// `buffer`: Mesajın kopyalanacağı tampon.
    /// Eğer mesaj yoksa, varsayılan olarak bloklar. (Non-blocking için özel flag gerekebilir)
    /// Başarılı olursa, alınan mesajın byte cinsinden boyutunu döner.
    ///
    /// # Dönüş Değeri
    /// Ok(0) genellikle gönderenin bağlantıyı kapattığı anlamına gelebilir (eğer bağlantı odaklıysa).
    /// Ok(n) n byte mesaj alındığını gösterir.
    pub fn receive(buffer: &mut [u8]) -> Result<usize, SahneError> {
        let buffer_ptr = buffer.as_mut_ptr() as u64;
        let buffer_len = buffer.len() as u64;
        let result = unsafe {
            // Belki gönderenin TaskId'sini veya bir mesaj Handle'ını da almak için
            // ek argümanlar veya farklı bir syscall olabilir.
            syscall(arch::SYSCALL_MESSAGE_RECEIVE, buffer_ptr, buffer_len, 0, 0, 0)
        };
        if result < 0 {
            Err(map_kernel_error(result))
        } else {
            Ok(result as usize)
        }
    }

     TODO: Sahne64'te mesaj kuyrukları, portlar veya kanallar için `resource::acquire`
    // benzeri bir mekanizma ve bunlara özel Handle'lar tanımlanabilir. Bu, daha yapılandırılmış
    // bir IPC sağlar. Örneğin:
     fn create_channel() -> Result<Handle, SahneError>`
     fn connect(channel_id: ResourceId) -> Result<Handle, SahneError>`
     fn send_via(handle: Handle, message: &[u8]) -> Result<(), SahneError>`
     fn receive_from(handle: Handle, buffer: &mut [u8]) -> Result<usize, SahneError>`
}


// --- Buradan itibaren C API katmanı kodunu ekliyoruz ---

// Rust'taki size_t'ye karşılık gelen C tipi için import. no_std olduğu için core kullanıyoruz.
use core::ffi::c_void; // C'deki void*'a karşılık gelir
use core::ptr; // İşaretçi operasyonları için

// --- Add C-compatible Error Codes ---
// These correspond to the SAHNE_ERROR_* defines in sahne.h
#[repr(i32)] // Ensure these are 32-bit integers
#[allow(non_camel_case_types)] // Allow C-style names
pub enum sahne_error_t {
    SAHNE_SUCCESS = 0,
    SAHNE_ERROR_OUT_OF_MEMORY = 1,
    SAHNE_ERROR_INVALID_ADDRESS = 2,
    SAHNE_ERROR_INVALID_PARAMETER = 3,
    SAHNE_ERROR_RESOURCE_NOT_FOUND = 4,
    SAHNE_ERROR_PERMISSION_DENIED = 5,
    SAHNE_ERROR_RESOURCE_BUSY = 6,
    SAHNE_ERROR_INTERRUPTED = 7,
    SAHNE_ERROR_NO_MESSAGE = 8,
    SAHNE_ERROR_INVALID_OPERATION = 9,
    SAHNE_ERROR_NOT_SUPPORTED = 10,
    SAHNE_ERROR_UNKNOWN_SYSCALL = 11,
    SAHNE_ERROR_TASK_CREATION_FAILED = 12,
    SAHNE_ERROR_INVALID_HANDLE = 13,
    SAHNE_ERROR_HANDLE_LIMIT_EXCEEDED = 14,
    SAHNE_ERROR_NAMING_ERROR = 15,
    SAHNE_ERROR_COMMUNICATION_ERROR = 16,
    // Add more mappings here if SahneError gets new variants
    SAHNE_ERROR_OTHER = 255, // A catch-all for unmapped kernel errors
}

// Helper to map Rust SahneError to C error code
fn map_sahne_error_to_c(err: SahneError) -> sahne_error_t {
    match err {
        SahneError::OutOfMemory => sahne_error_t::SAHNE_ERROR_OUT_OF_MEMORY,
        SahneError::InvalidAddress => sahne_error_t::SAHNE_ERROR_INVALID_ADDRESS,
        SahneError::InvalidParameter => sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER,
        SahneError::ResourceNotFound => sahne_error_t::SAHNE_ERROR_RESOURCE_NOT_FOUND,
        SahneError::PermissionDenied => sahne_error_t::SAHNE_ERROR_PERMISSION_DENIED,
        SahneError::ResourceBusy => sahne_error_t::SAHNE_ERROR_RESOURCE_BUSY,
        SahneError::Interrupted => sahne_error_t::SAHNE_ERROR_INTERRUPTED,
        SahneError::NoMessage => sahne_error_t::SAHNE_ERROR_NO_MESSAGE,
        SahneError::InvalidOperation => sahne_error_t::SAHNE_ERROR_INVALID_OPERATION,
        SahneError::NotSupported => sahne_error_t::SAHNE_ERROR_NOT_SUPPORTED,
        SahneError::UnknownSystemCall => sahne_error_t::SAHNE_ERROR_UNKNOWN_SYSCALL,
        SahneError::TaskCreationFailed => sahne_error_t::SAHNE_ERROR_TASK_CREATION_FAILED,
        SahneError::InvalidHandle => sahne_error_t::SAHNE_ERROR_INVALID_HANDLE,
        SahneError::HandleLimitExceeded => sahne_error_t::SAHNE_ERROR_HANDLE_LIMIT_EXCEEDED,
        SahneError::NamingError => sahne_error_t::SAHNE_ERROR_NAMING_ERROR,
        SahneError::CommunicationError => sahne_error_t::SAHNE_ERROR_COMMUNICATION_ERROR,
        // Add more mappings here
    }
}

// Helper to map the raw kernel i64 result to a C error code
fn map_raw_result_to_c_error(result: i64) -> sahne_error_t {
    if result >= 0 {
        sahne_error_t::SAHNE_SUCCESS
    } else {
        // Map the *negative* kernel code to a SahneError first, then to C error
        let sahne_err = map_kernel_error(result);
        map_sahne_error_to_c(sahne_err)
    }
}


// --- Expose C API Functions ---

#[no_mangle] // Prevent Rust from mangling the function name
pub extern "C" fn sahne_mem_allocate(size: usize, out_ptr: *mut *mut u8) -> sahne_error_t { usize ~ size_t
    if out_ptr.is_null() {
        return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
    match memory::allocate(size) {
        Ok(ptr) => {
            unsafe { *out_ptr = ptr; }
            sahne_error_t::SAHNE_SUCCESS
        }
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_mem_release(ptr: *mut u8, size: usize) -> sahne_error_t { usize ~ size_t
     match memory::release(ptr, size) {
        Ok(()) => sahne_error_t::SAHNE_SUCCESS,
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_mem_create_shared(size: usize, out_handle: *mut u64) -> sahne_error_t { usize ~ size_t, u64 ~ sahne_handle_t
    if out_handle.is_null() {
        return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
    match memory::create_shared(size) {
        Ok(handle) => {
            unsafe { *out_handle = handle.0; }
            sahne_error_t::SAHNE_SUCCESS
        }
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_mem_map_shared(handle: u64, offset: usize, size: usize, out_ptr: *mut *mut u8) -> sahne_error_t { u64 ~ sahne_handle_t, usize ~ size_t
     if out_ptr.is_null() {
        return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
    match memory::map_shared(Handle(handle), offset, size) {
        Ok(ptr) => {
            unsafe { *out_ptr = ptr; }
            sahne_error_t::SAHNE_SUCCESS
        }
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_mem_unmap_shared(addr: *mut u8, size: usize) -> sahne_error_t { usize ~ size_t
     match memory::unmap_shared(addr, size) {
        Ok(()) => sahne_error_t::SAHNE_SUCCESS,
        Err(e) => map_sahne_error_to_c(e),
    }
}


#[no_mangle]
pub extern "C" fn sahne_task_spawn(code_handle: u64, args_ptr: *const u8, args_len: usize, out_task_id: *mut u64) -> sahne_error_t { u64 ~ sahne_handle_t, usize ~ size_t, u64 ~ sahne_task_id_t
    if args_ptr.is_null() && args_len > 0 {
         return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
    if out_task_id.is_null() {
        return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
    let args_slice = if args_ptr.is_null() {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(args_ptr, args_len) }
    };

    match task::spawn(Handle(code_handle), args_slice) {
        Ok(tid) => {
            unsafe { *out_task_id = tid.0; }
            sahne_error_t::SAHNE_SUCCESS
        }
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_task_exit(code: i32) -> ! {
    task::exit(code)
}

#[no_mangle]
pub extern "C" fn sahne_task_current_id(out_task_id: *mut u64) -> sahne_error_t { u64 ~ sahne_task_id_t
    if out_task_id.is_null() {
        return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
    match task::current_id() {
        Ok(tid) => {
            unsafe { *out_task_id = tid.0; }
            sahne_error_t::SAHNE_SUCCESS
        }
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_task_sleep(milliseconds: u64) -> sahne_error_t {
    match task::sleep(milliseconds) {
        Ok(()) => sahne_error_t::SAHNE_SUCCESS,
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_thread_create(entry_point: u64, stack_size: usize, arg: u64, out_thread_id: *mut u64) -> sahne_error_t { usize ~ size_t, u64 ~ thread ID
     if out_thread_id.is_null() {
        return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
     match task::create_thread(entry_point, stack_size, arg) {
        Ok(thread_id) => {
            unsafe { *out_thread_id = thread_id; }
            sahne_error_t::SAHNE_SUCCESS
        }
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_thread_exit(code: i32) -> ! {
    task::exit_thread(code)
}

#[no_mangle]
pub extern "C" fn sahne_task_yield() -> sahne_error_t {
    match task::yield_now() {
        Ok(()) => sahne_error_t::SAHNE_SUCCESS,
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_resource_acquire(id_ptr: *const u8, id_len: usize, mode: u32, out_handle: *mut u64) -> sahne_error_t { usize ~ size_t, u64 ~ sahne_handle_t
    if id_ptr.is_null() && id_len > 0 {
         return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
    if out_handle.is_null() {
        return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
    let id_slice = if id_ptr.is_null() {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(id_ptr, id_len) }
    };

    // Convert &[u8] to &str for the Rust API call if needed, handle errors.
    // If Sahne64 resource IDs are arbitrary bytes, modify the Rust API to take &[u8].
    // Assuming &[u8] mapping to &str and checking UTF-8 here.
    let id_str_res = core::str::from_utf8(id_slice);
    let id_str = match id_str_res {
        Ok(s) => s,
        Err(_) => return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER, // Or a specific NamingError for invalid UTF-8
    };

    match resource::acquire(id_str, mode) {
        Ok(handle) => {
            unsafe { *out_handle = handle.0; }
            sahne_error_t::SAHNE_SUCCESS
        }
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_resource_read(handle: u64, buffer_ptr: *mut u8, buffer_len: usize, out_bytes_read: *mut usize) -> sahne_error_t { u64 ~ sahne_handle_t, usize ~ size_t
    if buffer_ptr.is_null() && buffer_len > 0 {
         return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
    if out_bytes_read.is_null() {
        return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
    let buffer_slice = if buffer_ptr.is_null() {
        &mut []
    } else {
        unsafe { core::slice::from_raw_parts_mut(buffer_ptr, buffer_len) }
    };

    match resource::read(Handle(handle), buffer_slice) {
        Ok(bytes_read) => {
            unsafe { *out_bytes_read = bytes_read; }
            sahne_error_t::SAHNE_SUCCESS
        }
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_resource_write(handle: u64, buffer_ptr: *const u8, buffer_len: usize, out_bytes_written: *mut usize) -> sahne_error_t { // u64 ~ sahne_handle_t, usize ~ size_t
     if buffer_ptr.is_null() && buffer_len > 0 {
         return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
    // out_bytes_written can be null if the caller doesn't care about the count. Check before dereferencing.
     if out_bytes_written.is_null() { return SAHNE_ERROR_INVALID_PARAMETER; } // Depending on desired strictness

    let buffer_slice = if buffer_ptr.is_null() {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(buffer_ptr, buffer_len) }
    };

    match resource::write(Handle(handle), buffer_slice) {
        Ok(bytes_written) => {
             if !out_bytes_written.is_null() {
                 unsafe { *out_bytes_written = bytes_written; }
             }
            sahne_error_t::SAHNE_SUCCESS
        }
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_resource_release(handle: u64) -> sahne_error_t { u64 ~ sahne_handle_t
    match resource::release(Handle(handle)) {
        Ok(()) => sahne_error_t::SAHNE_SUCCESS,
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_resource_control(handle: u64, request: u64, arg: u64, out_result: *mut i64) -> sahne_error_t { u64 ~ sahne_handle_t
    if out_result.is_null() {
        return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
     match resource::control(Handle(handle), request, arg) {
        Ok(result_val) => {
            unsafe { *out_result = result_val; }
            sahne_error_t::SAHNE_SUCCESS
        }
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_kernel_get_info(info_type: u32, out_value: *mut u64) -> sahne_error_t {
    if out_value.is_null() {
        return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
    match kernel::get_info(info_type) {
        Ok(value) => {
            unsafe { *out_value = value; }
            sahne_error_t::SAHNE_SUCCESS
        }
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_kernel_get_time(out_time: *mut u64) -> sahne_error_t {
    if out_time.is_null() {
        return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
     match kernel::get_time() {
        Ok(time) => {
            unsafe { *out_time = time; }
            sahne_error_t::SAHNE_SUCCESS
        }
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_sync_lock_create(out_handle: *mut u64) -> sahne_error_t { u64 ~ sahne_handle_t
    if out_handle.is_null() {
        return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
    match sync::lock_create() {
        Ok(handle) => {
            unsafe { *out_handle = handle.0; }
            sahne_error_t::SAHNE_SUCCESS
        }
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_sync_lock_acquire(handle: u64) -> sahne_error_t { u64 ~ sahne_handle_t
    match sync::lock_acquire(Handle(handle)) {
        Ok(()) => sahne_error_t::SAHNE_SUCCESS,
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_sync_lock_release(handle: u64) -> sahne_error_t { u64 ~ sahne_handle_t
     match sync::lock_release(Handle(handle)) {
        Ok(()) => sahne_error_t::SAHNE_SUCCESS,
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_msg_send(target_task: u64, message_ptr: *const u8, message_len: usize) -> sahne_error_t { u64 ~ sahne_task_id_t, usize ~ size_t
     if message_ptr.is_null() && message_len > 0 {
         return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
    let message_slice = if message_ptr.is_null() {
        &[]
    } else {
        unsafe { core::slice::from_raw_parts(message_ptr, message_len) }
    };
    match messaging::send(TaskId(target_task), message_slice) {
        Ok(()) => sahne_error_t::SAHNE_SUCCESS,
        Err(e) => map_sahne_error_to_c(e),
    }
}

#[no_mangle]
pub extern "C" fn sahne_msg_receive(buffer_ptr: *mut u8, buffer_len: usize, out_bytes_received: *mut usize) -> sahne_error_t { // usize ~ size_t
     if buffer_ptr.is_null() && buffer_len > 0 {
         return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
    if out_bytes_received.is_null() {
        return sahne_error_t::SAHNE_ERROR_INVALID_PARAMETER;
    }
    let buffer_slice = if buffer_ptr.is_null() {
        &mut []
    } else {
        unsafe { core::slice::from_raw_parts_mut(buffer_ptr, buffer_len) }
    };

    match messaging::receive(buffer_slice) {
        Ok(bytes_received) => {
            unsafe { *out_bytes_received = bytes_received; }
            sahne_error_t::SAHNE_SUCCESS
        }
        Err(e) => map_sahne_error_to_c(e),
    }
}

// Optional: Expose the raw syscall for advanced users or specific needs, though generally discouraged.
 #[no_mangle]
 pub extern "C" fn sahne_raw_syscall(number: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i64 {
       unsafe { syscall(number, arg1, arg2, arg3, arg4, arg5) }
 }


// --- Re-export types for Rust users ---
// (Bu kısım sizin orijinal kodunuzun sonuna yakındı, burada bırakmak uygun)
pub use crate::arch;
pub use crate::memory;
pub use crate::task;
pub use crate::resource;
pub use crate::kernel;
pub use crate::sync;
pub use crate::messaging;
pub use crate::{Handle, TaskId, SahneError}; // Export Rust-idiomatic types
// C API hata tipi de Rust tarafından kullanılabilir hale getirilebilir (isteğe bağlı)
pub use crate::sahne_error_t;


// --- Örnek Kullanım (main fonksiyonu std gerektirir veya no_std ortamında özel entry point gerekir) ---
// Bu kısım, kütüphanenin nasıl kullanılacağını gösterir ve Sahne64 prensiplerini yansıtır.
#[cfg(feature = "std")] // Sadece standart kütüphane varsa derlenir (test/örnek amaçlı)
fn main() {
    // no_std ortamında println! için kendi implementasyonumuz lazım.
    // Şimdilik std ortamında olduğumuzu varsayalım.
    use crate::{task, memory, resource, kernel, sync, messaging, Handle, TaskId, SahneError};

    println!("Sahne64 Kullanıcı Alanı Programı Başlatıldı!");

    match task::current_id() {
        Ok(tid) => println!("Mevcut Görev ID: {:?}", tid),
        Err(e) => eprintln!("Görev ID alınamadı: {:?}", e),
    }

    // Bellek Ayırma ve Bırakma
    match memory::allocate(2048) {
        Ok(ptr) => {
            println!("2048 byte bellek ayrıldı: {:p}", ptr);
            // Belleği kullan... (örneğin ilk byte'a yaz)
            unsafe { *ptr = 64; }
            match memory::release(ptr, 2048) {
                Ok(_) => println!("Bellek serbest bırakıldı."),
                Err(e) => eprintln!("Bellek bırakma hatası: {:?}", e),
            }
        }
        Err(e) => eprintln!("Bellek ayırma hatası: {:?}", e),
    }

    // Kaynak Edinme, Okuma/Yazma, Bırakma (Dosya yerine)
    let resource_name = "sahne://config/settings.dat"; // Sahne64'e özgü bir URI/path formatı?
    match resource::acquire(resource_name, resource::MODE_READ | resource::MODE_CREATE) {
        Ok(handle) => {
            println!("Kaynak edinildi ('{}'), Handle: {:?}", resource_name, handle);
            let mut buffer = [0u8; 256];
            match resource::read(handle, &mut buffer) {
                Ok(bytes_read) => println!("Kaynaktan {} byte okundu.", bytes_read),
                Err(e) => eprintln!("Kaynak okuma hatası: {:?}", e),
            }
            // Yazma denemesi (eğer MODE_WRITE de istenseydi)
             match resource::write(handle, b"Merhaba Sahne64!") { ... }

            match resource::release(handle) {
                Ok(_) => println!("Kaynak Handle'ı serbest bırakıldı."),
                Err(e) => eprintln!("Kaynak bırakma hatası: {:?}", e),
            }
        }
        Err(SahneError::ResourceNotFound) => eprintln!("Kaynak bulunamadı: {}", resource_name),
        Err(e) => eprintln!("Kaynak edinme hatası ('{}'): {:?}", resource_name, e),
    }

    // Yeni Görev Başlatma (Çalıştırılabilir kodun Handle'ı lazım)
    // Gerçek sistemde bu handle başka bir `resource::acquire` ile alınır.
    let code_handle = resource::acquire("sahne://bin/hesaplayici", resource::MODE_READ)?;
    let dummy_code_handle = Handle(10); // Varsayımsal handle
    let task_args = b"arg1 arg2";
    match task::spawn(dummy_code_handle, task_args) {
        Ok(new_tid) => println!("Yeni görev başlatıldı, TaskId: {:?}", new_tid),
        Err(e) => eprintln!("Görev başlatma hatası: {:?}", e),
    }

    // Çekirdek Bilgisi Alma
    match kernel::get_info(kernel::KERNEL_INFO_VERSION_MAJOR) {
        Ok(ver) => println!("Çekirdek Ana Versiyon: {}", ver),
        Err(e) => eprintln!("Çekirdek bilgisi alma hatası: {:?}", e),
    }
    match kernel::get_time() {
        Ok(time) => println!("Sistem Zamanı (nanosaniye?): {}", time),
        Err(e) => eprintln!("Zaman bilgisi alma hatası: {:?}", e),
    }

    // Kilit (Mutex) Kullanımı
    match sync::lock_create() {
        Ok(lock_handle) => {
            println!("Kilit oluşturuldu, Handle: {:?}", lock_handle);
            match sync::lock_acquire(lock_handle) {
                Ok(_) => {
                    println!("Kilit alındı.");
                    // ... Kritik bölge ...
                    println!("Kritik bölge bitti.");
                    match sync::lock_release(lock_handle) {
                        Ok(_) => println!("Kilit bırakıldı."),
                        Err(e) => eprintln!("Kilit bırakma hatası: {:?}", e),
                    }
                }
                Err(e) => eprintln!("Kilit alma hatası: {:?}", e),
            }
            // Kilidi tamamen yok etmek için resource::release kullanılır (opsiyonel)
            match resource::release(lock_handle) {
                Ok(_) => println!("Kilit kaynağı serbest bırakıldı."),
                Err(e) => eprintln!("Kilit kaynağı bırakma hatası: {:?}", e),
            }
        }
        Err(e) => eprintln!("Kilit oluşturma hatası: {:?}", e),
    }

    // Mesajlaşma Örneği
    let target_task_id = TaskId(2); // Hedef görevin ID'si (varsayımsal)
    let message_data = b"Merhaba Task 2!";
    match messaging::send(target_task_id, message_data) {
        Ok(_) => println!("{:?} ID'li göreve mesaj gönderildi.", target_task_id),
        Err(e) => eprintln!("Mesaj gönderme hatası: {:?}", e),
    }

    let mut recv_buffer = [0u8; 64];
    println!("Gelen mesaj bekleniyor...");
    match messaging::receive(&mut recv_buffer) {
        Ok(received_len) => {
            if received_len > 0 {
                println!("Mesaj alındı ({} byte): {:?}", received_len, &recv_buffer[..received_len]);
                // Belki string'e çevirme?
                if let Ok(s) = core::str::from_utf8(&recv_buffer[..received_len]) {
                    println!("  Mesaj (metin): {}", s);
                }
            } else {
                  println!("Boş mesaj alındı veya bağlantı kapandı?");
            }
        }
        Err(SahneError::NoMessage) => eprintln!("Mesaj yok (non-blocking olsaydı)."), // Bu senaryo blocking receive'de zor
        Err(e) => eprintln!("Mesaj alma hatası: {:?}", e),
    }

    println!("Görev uykuya dalıyor (1 saniye)...");
    let _ = task::sleep(1000);
    println!("Görev uyandı.");


    println!("Sahne64 programı normal şekilde sonlanıyor.");
    task::exit(0); // Görevi 0 koduyla sonlandır
}


// --- no_std için Gerekli Olabilecekler ---

// Panik durumunda ne yapılacağı (no_std)
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // Gerçek bir sistemde burada belki hata mesajı bir porta yazılır,
    // sistem yeniden başlatılır veya sadece sonsuz döngüye girilir.
    // Örn:
     print!("PANIC: {}", info);
     system_reset();
    loop {
        core::hint::spin_loop(); // İşlemciyi meşgul etmeden bekle
    }
}

// `println!` gibi makroların `no_std` ortamında çalışması için basit bir implementasyon.
// Gerçek bir Sahne64'te bu, çekirdeğe `resource::write` sistem çağrısı yaparak
// bir konsol Handle'ına yazmayı içerecektir.
#[cfg(not(feature = "std"))]
mod stdio_impl {
    use core::fmt;

    // Bu struct, çıktının nereye yazılacağını temsil eder.
    // Gerçek sistemde bu bir UART, VGA buffer veya debug portu olabilir.
    // Şimdilik hiçbir şey yapmıyor.
    struct SahneWriter;

    impl fmt::Write for SahneWriter {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            // BURASI ÖNEMLİ: Gerçek Sahne64 çekirdeğinde, bu fonksiyon
             `syscall(SYSCALL_RESOURCE_WRITE, CONSOLE_HANDLE, s.ptr, s.len, ...)`
            // gibi bir sistem çağrısı yapmalıdır. CONSOLE_HANDLE, görevin
            // başlangıçta aldığı standart çıktı Handle'ı olabilir.
            // Şimdilik sadece başarılı olduğunu varsayıyoruz.
            Ok(())
        }
    }

    #[macro_export]
    macro_rules! print {
        ($($arg:tt)*) => ({
            use core::fmt::Write;
            let mut writer = $crate::stdio_impl::SahneWriter;
            let _ = write!(writer, $($arg)*); // Hata durumunu yoksay (basit örnek)
        });
    }

    #[macro_export]
    macro_rules! println {
        () => ($crate::print!("\n"));
        ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
    }

    #[macro_export]
    macro_rules! eprintln {
        () => ($crate::print!("\n")); // Şimdilik stderr yok, stdout'a yaz
        ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
    }

    // Bu modülü ana scope'a eklemek için:
     use crate::stdio_impl; // main veya lib.rs içinde
}

// Eğer bu bir kütüphane ise ve `main` sadece örnekse, aşağıdaki gibi bir `lib.rs` yapısı olur:
pub use crate::arch;
pub use crate::memory;
// // ... diğer modüller ...
pub use crate::task;
pub use crate::resource;
pub use crate::kernel;
pub use crate::sync;
pub use crate::messaging;

// Rust API tiplerini ve C API hata tipini dışa aktar
pub use crate::{Handle, TaskId, SahneError, sahne_error_t}; // sahne_error_t de artık kullanılabilir

// (Bu API için programlama dili uyumluluk katmanı oluşturmak istiyorum çünkü bu API sadece Rust ile tam uyumlu olabiliyor)
// (Ben bu katma sayesinde C, C++, D ve Rust olmak üzere toplamda 4 tane programlama dilinde kullanılabilmesini sağlayacak!)
