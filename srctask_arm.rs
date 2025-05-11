#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz, çekirdek alanında çalışırız
#![allow(dead_code)] // Geliştirme sırasında kullanılmayan kod veya argümanlar için izinler
#![allow(unused_variables)] // Geliştirme sırasında kullanılmayan değişkenler için izinler
#![feature(asm_experimental_inline)] // Inline assembly için gerekli (ARM bağlam değiştirme için)
// #![feature(naked_functions)] // Bağlam değiştirme fonksiyonu için gerekebilir, duruma göre


// Karnal64'ten temel tipleri al
// Varsayım: karnal64.rs ayrı bir crate veya module olarak build ediliyor ve bu dosyadan erişilebilir.
// Projenizin build sistemi, karnal64.rs'yi ktask içinde kullanılabilecek şekilde ayarlamalıdır.
use karnal64::{KError, KTaskId};
// KThreadId eğer threadler görevlerden ayrı varlıklarsa kullanılabilir.


// --- Yer Tutucu Diğer Çekirdek Modülleri ---
// ktask modülünün ihtiyaç duyduğu diğer çekirdek modüllerinin dummy veya minimal tanımları.
// Gerçek implementasyonda bu modüllerin API'ları doğru şekilde kullanılır.
// Bu tanımlar, derleyicinin ktask içinde yapılan çağrıları çözebilmesi için buraya eklendi.

mod kresource {
    use super::KError;
    pub const MODE_READ: u32 = 1; // Örnek mod
    pub fn init_manager() { /* Gerçek başlatma mantığı */ }
    // TODO: lookup_provider_by_name, issue_handle, release_handle, handle_has_permission fonksiyonları
}
mod kmemory {
    use super::KError;
    // Basit yer tutucu bellek ayırıcı fonksiyonları
    pub fn allocate_stack(size: usize) -> Result<*mut u8, KError> {
        // TODO: Gerçek, güvenli, hizalanmış bellek ayırma implementasyonu (örn. heap ayırıcıdan)
        // Bu fonksiyon, görevin stack'i için kernel belleğinden yer ayırmalıdır.
         println!("KMemory: Allocating stack of size {}", size); // Çekirdek içi print! gerekiyor
        Err(KError::OutOfMemory) // Başlangıçta bellek ayırma başarısızlığı simülasyonu
          Ok(unsafe { my_heap_allocator::allocate(size, align)? })
    }
    pub fn free_stack(ptr: *mut u8, size: usize) -> Result<(), KError> {
         // TODO: Gerçek bellek serbest bırakma implementasyonu
         println!("KMemory: Freeing stack at {:?}", ptr); // Çekirdek içi print! gerekiyor
        Ok(()) // Başarı simülasyonu
    }
     // TODO: allocate_user_memory, free_user_memory, shared_mem_map/unmap gibi kullanıcı alanı bellek fonksiyonları
    pub fn init_manager() { /* Gerçek başlatma mantığı */ }
}
mod ksync {
    use super::KError;
    pub fn init_manager() { /* Gerçek başlatma mantığı */ }
    // TODO: sleep_for, acquire, release gibi senkronizasyon primitifi fonksiyonları
     pub fn sleep_for(duration_ms: u64) -> Result<(), KError> {
        // TODO: Gerçek uyku implementasyonu (zamanlayıcı kesmeleri, bekleme kuyrukları)
         println!("KSync: Sleeping for {}ms", duration_ms); // Çekirdek içi print! gerekiyor
        Err(KError::NotSupported) // Yer tutucu hata
    }
    // TODO: lock_create, lock_acquire, lock_release gibi kilit fonksiyonları
}
mod kmessaging {
    use super::KError;
    pub fn init_manager() { /* Gerçek başlatma mantığı */ }
    // TODO: send, receive gibi mesajlaşma fonksiyonları
     // Dummy send/receive (prototipler mevcut koddaki handle_syscall'da kullanılıyor)
     pub fn send(target_handle: u64, buffer_ptr: *const u8, buffer_len: usize) -> Result<(), KError> {
         // TODO: Gerçek mesaj gönderme mantığı
         Err(KError::NotSupported)
     }
      pub fn receive(buffer_ptr: *mut u8, buffer_len: usize) -> Result<usize, KError> {
         // TODO: Gerçek mesaj alma mantığı
         Err(KError::NoMessage) // Mesaj yok veya desteklenmiyor
     }
}
mod kkernel {
    use super::KError;
     pub fn init_manager() { /* Gerçek başlatma mantığı */ }
    // TODO: get_info, get_time gibi kernel bilgi fonksiyonları
     pub fn get_info(_request: u32) -> Result<u64, KError> { Err(KError::NotSupported) } // Dummy
}


// Çekirdek içi print! makrosu için bir yer tutucu (gerçek çekirdekte implemente edilmeli)
// Hata ayıklama sırasında çıktı görmek için gereklidir.
macro_rules! println {
    ($($arg:tt)*) => ({
        // TODO: Gerçek konsol/seri port çıktı implementasyonu
         unsafe { core::fmt::write(&mut YourKernelConsoleWriter, format_args!($($arg)*)).unwrap(); }
        // Şimdilik çıktı üretmeyecek dummy bir implementasyon.
         use core::fmt::Write;
         struct DummyConsoleWriter;
         impl core::fmt::Write for DummyConsoleWriter {
             fn write_str(&mut self, s: &str) -> core::fmt::Result { Ok(()) } // Yazma işlemi yapmaz
         }
          DummyConsoleWriter.write_fmt(format_args!($($arg)*)).ok(); // Sadece çağrı simülasyonu
    });
}


// --- ARM Özel Görev/İş Parçacığı Yapıları ve Mantığı ---

/// Görev/İş Parçacığı Durumu
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TaskState {
    Ready,      // Çalışmaya hazır, zamanlayıcı tarafından seçilmeyi bekliyor
    Running,    // Şu anda işlemcide çalışıyor
    Sleeping,   // Belirli bir süre veya olayı bekliyor
    Blocked,    // Bir kaynağı (kilit, mesaj) bekliyor
    Exited,     // Çalışması tamamlandı veya sonlandırıldı
    // Diğer durumlar eklenebilir (örn. Stopped, Zombie)
}

/// ARM Görev Kontrol Bloğu (Task Control Block - TCB)
/// Bu yapı, bir görevin (veya iş parçacığının) durumunu (kaydedilmiş registerlar, stack bilgisi vb.) tutar.
/// ARM AArch64 mimarisine özel register setini içerir.
/// Bağlam değiştirme assembly kodumuzun bu yapıya erişmesi gerekecektir.
#[repr(C)] // C ABI uyumluluğu için, assembly/düşük seviye koddan erişim gerekebilir
#[derive(Debug)]
pub struct TaskControlBlock {
    // Bağlam değiştirme sırasında kaydedilip geri yüklenen ARM AArch64 registerları.
    // RFC (Procedure Call Standard for the ARM 64-bit Architecture) tarafından
    // "Non-volatile" olarak belirlenen ve çağıran (caller) tarafından korunması gereken registerlar
    // (x19-x29, FP, LR) ve ayrıca SP ve PSTATE (SPSR_EL1) kaydedilir.
    // x0-x18 ve x30 (LR) genellikle "volatile" kabul edilir ve fonksiyon çağrıları arasında korunmaz.
    // PC (Program Counter) genellikle doğrudan TCB'de tutulmaz, bağlam değiştirme rutinindeki
    // RET (Return) veya BR (Branch) komutları ile kontrol edilir (genellikle LR veya TCB'deki PC alanı kullanılır).
    // Burada TCB'de PC alanı tutarak daha esnek bir model izleyelim.

    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub x29: u64, // Frame Pointer (FP)
    pub x30: u64, // Link Register (LR) - Görev fonksiyonu bittiğinde dönülecek adres
    pub sp: u64,  // Stack Pointer
    pub elr_el1: u64, // Exception Link Register (EL1) - Exception'dan dönüldüğünde çalışacak adres (PC karşılığı)
    pub spsr_el1: u64, // Saved Program Status Register (EL1) - İmtiyaz seviyesi, bayraklar vb.

    // Görev/İş Parçacığı Bilgileri
    pub id: KTaskId,
    pub state: TaskState,
    pub stack_base: *mut u8, // Ayrılan stack bloğunun başlangıç adresi
    pub stack_size: usize,   // Ayrılan stack bloğunun boyutu

    // TODO: Görev adres alanı (MMU bağlamı/Page Table Base Register - TTBR0_EL1/TTBR1_EL1) bilgisi
    // TODO: Mesaj kuyruğu, beklenen olaylar, sinyaller gibi IPC/senkronizasyon alanları
    // TODO: Görevin açık kaynaklarının listesi (handle tablosu)
}

// TCB alanlarının assembly içinde kullanılacak offsetlerini belirleyelim.
// `memoffset` crate'i `no_std` ortamında bu offsetleri güvenli bir şekilde hesaplamak için kullanılabilir.
// Varsayımsal manuel offset hesaplaması (u64 alanlar 8 byte):
const TCB_X19_OFFSET: usize = 0 * 8;
const TCB_X20_OFFSET: usize = 1 * 8;
const TCB_X21_OFFSET: usize = 2 * 8;
const TCB_X22_OFFSET: usize = 3 * 8;
const TCB_X23_OFFSET: usize = 4 * 8;
const TCB_X24_OFFSET: usize = 5 * 8;
const TCB_X25_OFFSET: usize = 6 * 8;
const TCB_X26_OFFSET: usize = 7 * 8;
const TCB_X27_OFFSET: usize = 8 * 8;
const TCB_X28_OFFSET: usize = 9 * 8;
const TCB_X29_OFFSET: usize = 10 * 8; // FP
const TCB_X30_OFFSET: usize = 11 * 8; // LR
const TCB_SP_OFFSET: usize = 12 * 8;  // Stack Pointer
const TCB_ELR_OFFSET: usize = 13 * 8; // Exception Link Register (PC karşılığı)
const TCB_SPSR_OFFSET: usize = 14 * 8; // Saved Program Status Register

// --- Çekirdek Görev/İş Parçacığı Yöneticisi (ktask implementasyonu) ---

// Güvenli olmayan statik veri yapıları (yalnızca konsepti göstermek için)
// Gerçek çekirdekte spinlock veya başka bir senkronizasyon mekanizması ile korunmalıdır.
const MAX_TASKS: usize = 32; // Görev havuzu boyutu
static mut TASK_POOL: [Option<TaskControlBlock>; MAX_TASKS] = [None; MAX_TASKS];
static mut NEXT_TASK_ID_COUNTER: u64 = 1; // Görev ID'leri 1'den başlar (0 genellikle kernel/idle için)
static mut CURRENT_TASK_INDEX: usize = 0; // Şu anda çalışan görevin TASK_POOL indeksi

// Basit bir hazır kuyruğu (dairesel tampon)
static mut READY_QUEUE: [usize; MAX_TASKS] = [0; MAX_TASKS];
static mut READY_QUEUE_HEAD: usize = 0; // Kuyruk başı indeksi (çıkarılacak eleman)
static mut READY_QUEUE_TAIL: usize = 0; // Kuyruk sonu indeksi (yeni eleman buraya eklenir)

// DİKKAT: Bu statik değişkenlere erişim, kesmelerin kapalı olması veya spinlock kullanılması gibi
// senkronizasyon mekanizmalarıyla korunmalıdır. Bu örnekte güvenlik mekanizması YOKTUR.

/// Çekirdek Görev Yöneticisini başlatır.
/// Çekirdek boot sürecinin başlarında çağrılır.
pub fn init_manager() {
    unsafe {
        // Tüm görev havuzunu ve hazır kuyruğunu temizle
        for i in 0..MAX_TASKS {
            TASK_POOL[i] = None;
            READY_QUEUE[i] = 0;
        }

        // İlk görev olarak şu anda çalışan kernel bağlamını ayarla (Genellikle idle görev veya init görevi)
        // Bu TCB, bağlam değiştirmeden DÖNÜLECEK yerin durumunu saklar.
        // Bağlam değiştirme rutinimiz, ilk çağrıldığında `prev_tcb_ptr` null olacağı durumu işlemelidir.
        // Burada 0 ID'li bir dummy kernel/idle görevi oluşturuyoruz.
        TASK_POOL[0] = Some(TaskControlBlock {
            id: KTaskId(0), // Kernel veya Idle görevi için 0 ID
            state: TaskState::Running, // Başlangıçta çalışıyor kabul edelim
            // Bu TCB'nin register değerleri bağlam değiştirmede doldurulacaktır (prev_tcb_ptr olarak verildiğinde).
            sp: 0, lr: 0, elr_el1: 0, spsr_el1: 0,
            x19: 0, x20: 0, x21: 0, x22: 0, x23: 0, x24: 0, x25: 0, x26: 0, x27: 0, x28: 0, x29: 0, x30: 0, // FP, LR(X30)
            stack_base: core::ptr::null_mut(), // Kernel stack'i bilgisi buraya eklenebilir
            stack_size: 0,
            // TODO: Diğer alanları ayarla (MMU bağlamı vb.)
        });

        CURRENT_TASK_INDEX = 0; // İlk görev şu anda çalışıyor kabul edildi
        READY_QUEUE_HEAD = 0;
        READY_QUEUE_TAIL = 0;

        println!("Karnal64 ARM: Görev Yöneticisi Başlatıldı. İlk (Kernel/Idle) Görev ID: {}", TASK_POOL[0].as_ref().unwrap().id.0);

        // TODO: Gerekirse bir idle görevi TCB'sini oluşturup Ready kuyruğuna ekle
    }
}

/// Yeni bir kullanıcı alanı görevi/süreci oluşturur ve başlatılmaya hazırlar.
/// Bu fonksiyon genellikle sistem çağrısı işleyicisi tarafından çağrılır (SYSCALL_TASK_SPAWN).
/// `code_handle_value`: Çalıştırılacak kodun ResourceProvider handle değeri.
/// `args_ptr`, `args_len`: Göreve iletilecek argüman verisi (kullanıcı alanı pointerı).
///
/// # Güvenlik (Unsafe)
/// `args_ptr` ve `args_len` sistem çağrısı işleyicisinde kullanıcı adres alanında
/// geçerli ve okunabilir oldukları doğrulanmış OLMALIDIR.
/// `code_handle_value` da geçerli bir Resource handle olmalıdır.
///
/// Başarı durumunda yeni görevin KTaskId'sini, hata durumunda KError döner.
pub fn task_spawn(code_handle_value: u64, args_ptr: *const u8, args_len: usize) -> Result<KTaskId, KError> {
    unsafe {
        // TODO: code_handle_value ile code ResourceProvider'a erişim ve yürütülebilir kodu yükleme mantığı.
        // Bu, görev için yeni bir adres alanı oluşturma, yürütülebilir segmentleri bu alana haritalama,
        // programın giriş noktasını belirleme gibi adımlar içerir ve kmemory ile kresource gerektirir.
        // Burası çekirdeğin en karmaşık kısımlarından biridir (ELF/Mach-O/PE parsing, MMU yönetimi).
        // Şimdilik, yüklenecek kodun giriş noktasını ve stack'i ayarlamaya odaklanalım ve
        // kodun zaten bellekte bir yerde olduğunu ve giriş noktasını bildiğimizi varsayalım.

        // TODO: Görev için bir kullanıcı alanı stack'i tahsis et (kmemory::allocate_user_memory veya allocate_stack).
        // Kullanıcı stack'i, kullanıcının kendi adres alanında olmalıdır.
        let stack_size: usize = 8 * 1024; // 8KB kullanıcı stack örneği
        let stack_base_user: *mut u8 = match kmemory::allocate_stack(stack_size) { // Dummy çağrı, gerçekte kullanıcı alanı belleği olmalı
             Ok(ptr) => ptr,
             Err(e) => {
                 println!("Karnal64 ARM: Görev stack ayırma hatası: {:?}", e);
                 return Err(e); // Bellek hatası
            }
        };
        // ARM AArch64'te stack yüksek adresten aşağıya büyür. SP, stack'in en üstünde (stack_base + stack_size) başlar.
        let initial_sp_user = stack_base_user.add(stack_size);

        // TODO: Yüklenen yürütülebilir koddan gerçek kullanıcı alanı giriş noktasını al
        let entry_point_addr_user: u64 = 0x400000; // Örnek kullanıcı alanı program giriş adresi

        // Boş bir TCB girişi bulmak için havuzu tara
        let mut new_task_index = None;
        for i in 0..MAX_TASKS {
            if TASK_POOL[i].is_none() {
                new_task_index = Some(i);
                break;
            }
        }

        let new_task_index = match new_task_index {
            Some(idx) => idx,
            None => {
                println!("Karnal64 ARM: Görev havuzu dolu!");
                kmemory::free_stack(stack_base_user, stack_size).ok(); // Ayrılan belleği geri ver
                return Err(KError::OutOfMemory); // Görev havuzu dolu
            }
        };

        // Yeni Görev ID'si atama
        let task_id = KTaskId(NEXT_TASK_ID_COUNTER);
        NEXT_TASK_ID_COUNTER += 1;
        // TODO: Taşma kontrolü yapılmalı! ID havuzu biterse ne olacak?

        // Yeni TCB'yi oluştur ve başlangıç durumunu ayarla
        // Bu TCB, görev ilk kez çalışmaya başladığında ARM bağlam değiştirme rutini tarafından yüklenecek register değerlerini belirler.
        // Bağlam değiştirme rutini genellikle EL1'den (kernel modu) EL0'a (kullanıcı modu) dönecektir.
        // Bu dönüş, ERET (Exception Return) komutu ile yapılır ve ELR_EL1 ve SPSR_EL1 registerlarının değerlerini kullanır.
        // Bu yüzden TCB'de ELR_EL1 ve SPSR_EL1 alanları bulunur.

        // ARM AArch64 kullanıcı modu (EL0) SPSR_EL1 değeri:
        // Genellikle 0b0000_0000_0000_0000_0000_0000_1000_0000
        // State (AArch64): 0b100000 (EL0t)
        // D, A, I, F (masklar): 0 (maskeli değil)
        // E (Endianness): 0 (Little-endian)
        // J, T (ARM/Thumb state): 0 (AArch64'te geçerli değil)
        // PSTATE alanları: N, Z, C, V bayrakları vb.
        let user_mode_spsr_el1: u64 = 0b0000_0000_0000_0000_0000_0000_0000_0000; // SPSR[31:28] = NZCV = 0
         SPSR_EL1[20] (D) = 0 (Debug mask off)
         SPSR_EL1[9] (A) = 0 (SError mask off)
         SPSR_EL1[8] (I) = 0 (IRQ mask off)
         SPSR_EL1[7] (F) = 0 (FIQ mask off)
         SPSR_EL1[6] (A) = 0 (AArch64 state)
         SPSR_EL1[4] (EL) = 0 (EL0)
         SPSR_EL1[3:2] (SP) = 00 (EL0t - SP_EL0)
         SPSR_EL1 = ... | (EL << 2) | (SP << 0)
        // Genellikle SPSR_EL1 = 0b0000_0000_0000_0000_0000_0000_0000_0000 | (0 << 2) | (0b00 << 0) ...
        // Basit bir başlangıç SPSR değeri (IRQ/FIQ maskeli, EL0t): 0x3c0 (EL0t, all interrupts masked) veya 0x300 (EL0t, unmasked)
        let initial_spsr_el1: u64 = 0x300; // EL0t, IRQ/FIQ/SError unmasked

        let mut new_tcb = TaskControlBlock {
            id: task_id,
            state: TaskState::Ready, // Başlangıçta çalışmaya hazır
            stack_base: stack_base_user,
            stack_size: stack_size,

            // Bağlam değiştirme tarafından yüklenecek başlangıç register değerleri
            sp: initial_sp_user,    // Stack Pointer: Ayrılan stack'in en üstü
            x30: task_exit_handler as u64, // LR (X30): Görev fonksiyonu bittiğinde buraya gider (task_exit_handler'ı çağıran adres)
            elr_el1: entry_point_addr_user, // ELR_EL1: Exception'dan dönüldüğünde PC'ye yüklenecek adres (Giriş Noktası)
            spsr_el1: initial_spsr_el1, // SPSR_EL1: Exception'dan dönüldüğünde PSTATE'e yüklenecek değer (Kullanıcı modu)

            // Non-volatile GPR'lar (x19-x29) genellikle başlangıçta 0 veya önemsiz değerlerle başlatılır.
            x19: 0, x20: 0, x21: 0, x22: 0, x23: 0, x24: 0, x25: 0, x26: 0, x27: 0, x28: 0, x29: 0, // FP
            // x0-x18 ve x30 (LR) bu TCB layoutunda yok, bağlam değiştirme rutini ve çağrı kuralı yönetir
        };

        // Eğer argümanlar varsa, bunları kullanıcının stack'ine kopyalayabiliriz veya
        // kullanıcı alanındaki giriş noktasına registerlar (x0-x7) aracılığıyla geçirebiliriz.
        // ARGÖrnek: Argüman pointer ve uzunluğunu x0 ve x1 registerlarına koymak (ARM64 PCS)
        // Bu, TCB'de x0, x1 alanları olmasını gerektirir veya bağlam değiştirme sırasında
        // bu registerları özel olarak yüklememiz gerekir.
        // Basitlik için, TCB'ye x0-x7 alanlarını ekleyelim:
         pub x0: u64, ... pub x7: u64,
         new_tcb.x0 = args_ptr as u64;
         new_tcb.x1 = args_len as u64;
        // TCB struct'ına x0-x7 eklenmediyse, bu kısım iptal edilmeli veya farklı işlenmeli.

        // TCB'yi havuza yerleştir
        TASK_POOL[new_task_index] = Some(new_tcb);

        // Görevi hazır kuyruğuna ekle
        // Kuyruk dolu mu kontrolü
        let next_tail = (READY_QUEUE_TAIL + 1) % MAX_TASKS;
        if next_tail == READY_QUEUE_HEAD {
            // Kuyruk dolu! Bu, MAX_TASKS'tan fazla görev yaratmaya çalıştığımız anlamına gelir.
            println!("Karnal64 ARM: Hazır kuyruğu dolu, yeni görev eklenemedi!");
            kmemory::free_stack(stack_base_user, stack_size).ok(); // Belleği geri ver
            TASK_POOL[new_task_index] = None; // TCB'yi boşalt
            NEXT_TASK_ID_COUNTER -= 1; // ID'yi geri al
            return Err(KError::OutOfMemory); // Uygun hata kodu (örn. TooManyTasks)
        }
        READY_QUEUE[READY_QUEUE_TAIL] = new_task_index;
        READY_QUEUE_TAIL = next_tail;


        println!("Karnal64 ARM: Yeni görev oluşturuldu. ID: {}. Index: {}", task_id.0, new_task_index);

        Ok(task_id) // Başarı, görev ID'sini döndür
    }
}

/// Mevcut görevi sonlandırır. Bu fonksiyon geri dönmez (noreturn).
/// `exit_code`: Görev çıkış kodu (şu an kullanılmıyor olabilir, gelecekte bekleme mekanizmaları için).
/// Bu fonksiyon genellikle kullanıcı alanından SYSCALL_TASK_EXIT ile çağrılır veya
/// task_exit_handler tarafından görev fonksiyonu normal döndüğünde çağrılır.
pub fn task_exit(exit_code: i32) -> ! {
    unsafe {
        let current_id = get_current_task_id().0; // Sonlanacak görevin ID'sini al
        let current_idx = CURRENT_TASK_INDEX;
        println!("Karnal64 ARM: Görev {} (Index {}) sonlanıyor, çıkış kodu: {}", current_id, current_idx, exit_code);

        // Mevcut görevi Exited olarak işaretle ve kaynaklarını serbest bırak
        // TCB'yi havuzdan kaldırmak için `take()` kullanabiliriz.
        if let Some(current_task_tcb) = TASK_POOL[current_idx].take() {
            // TCB state'i artık None olduğu için Exited olarak ayarlamaya gerek yok.
            // TODO: Görevle ilişkili tüm kaynakları temizle:
            // - Açık Resource handle'larını kapat (kresource::release_handle kullanarak)
            // - Göreve ait tüm bellek haritalarını iptal et (kmemory kullanarak)
            // - Göreve ait diğer çekirdek kaynaklarını (kilitler, mesaj kuyrukları vb.) serbest bırak
            kmemory::free_stack(current_task_tcb.stack_base, current_task_tcb.stack_size).ok(); // Stack'i serbest bırak

            // TODO: Eğer threadler görevlerden ayrıysa, görevin tüm threadlerinin sonlanmasını bekle
            // TODO: Ebeveyn göreve sonlanma bilgisini ilet (waitpid/task_wait benzeri mekanizma için)

             // Görev Ready kuyruğundaysa buradan çıkarılmalıdır.
             // Basit kuyruk implementasyonumuzda bu manuel yapılmalı veya scheduler'ın filtrelemesi lazım.
             // Scheduler Ready state'e bakarak çıkmış görevleri atlayacaktır.

        } else {
             // Bu durum olmamalı; task_exit'i çağıran bir görev her zaman geçerli bir TCB'ye sahip olmalı.
             // Ciddi bir çekirdek hatasıdır.
             println!("Karnal64 ARM: Ciddi Hata: Sonlanan görev TCB'si havuzda bulunamadı! Index: {}", current_idx);
             // Hata durumunu kernel log'larına kaydetmek veya kilitlenmek gerekebilir.
        }

        // Bir sonraki göreve bağlam değiştir.
        // schedule fonksiyonu geri dönmez.
        schedule();

        // schedule() başarılı olursa buradan asla dönülmez.
        // Dönülürse ciddi bir hata var demektir.
        println!("Karnal64 ARM: Ciddi Hata: schedule() geri döndü!");
        loop { /* Güvenlik için sonsuz döngü - işlemciyi durdurabilir (wfi) */ }
    }
}

/// Şu anda çalışan görevin ID'sini döndürür.
/// Bu fonksiyon genellikle kullanıcı alanından SYSCALL_GET_TASK_ID ile çağrılır.
/// # Güvenlik (Unsafe)
/// `CURRENT_TASK_INDEX` statik değişkenine erişim senkronizasyon gerektirir.
pub fn get_current_task_id() -> KTaskId {
    unsafe {
        // CURRENT_TASK_INDEX'teki görevin TCB'sinden ID'yi okur
        // Bu alana erişim de senkronizasyon gerektirir (kesmeler kapalı olmalı veya spinlock).
        TASK_POOL[CURRENT_TASK_INDEX].as_ref()
            .map(|tcb| tcb.id)
            // Eğer mevcut görev TCB'si None ise (ki olmamalı), KError::InternalError dönebiliriz
            // Ancak bu fonksiyon Result değil KTaskId döndürüyor, dummy 0 ID döndürelim.
            .unwrap_or(KTaskId(0)) // Hata durumunda (TCB yoksa) 0 dönebiliriz (kernel/idle)
    }
}

/// Mevcut görevi belirtilen süre kadar uykuya alır.
/// Bu fonksiyon genellikle kullanıcı alanından SYSCALL_TASK_SLEEP ile çağrılır.
/// `duration_ms`: Uyku süresi (milisaniye).
/// # Güvenlik (Unsafe)
/// Statik veri yapılarına erişim senkronizasyon gerektirir.
pub fn task_sleep(duration_ms: u64) -> Result<(), KError> {
    unsafe {
         let current_id = get_current_task_id().0;
         let current_idx = CURRENT_TASK_INDEX;
        println!("Karnal64 ARM: Görev {} (Index {}) {}ms uykuya geçiyor.", current_id, current_idx, duration_ms);

        // Mevcut görevin durumunu Sleeping olarak ayarla
         let current_task_tcb = TASK_POOL[current_idx].as_mut()
            .ok_or(KError::InternalError)?; // TCB yoksa hata

        current_task_tcb.state = TaskState::Sleeping;

        // TODO: Uyku süresini zamanlayıcıya kaydet ve görev uyandırılacaksa işaretle
        // TODO: Görevi hazır kuyruğundan kaldır (eğer oradaysa)
         ksync::sleep_for(duration_ms)?; // Gerçek uyku mekanizması çağrısı (bloklama yapabilir)

        // Bir sonraki göreve bağlam değiştir
        schedule();

        // Görev uyandırıldığında (başka bir çekirdek modülü veya kesme tarafından)
        // buradan devam eder (scheduler tarafından tekrar çalıştırıldığında).

        Ok(())
    }
}

/// Mevcut görevin işlemciyi gönüllü olarak bırakmasını sağlar (yield).
/// Görev, Ready kuyruğunun sonuna konur ve zamanlayıcı bir sonraki görevi seçer.
/// Bu fonksiyon genellikle kullanıcı alanından SYSCALL_TASK_YIELD ile çağrılır.
/// # Güvenlik (Unsafe)
/// Statik veri yapılarına erişim senkronizasyon gerektirir.
pub fn yield_now() -> Result<(), KError> {
    unsafe {
         let current_id = get_current_task_id().0;
         let current_idx = CURRENT_TASK_INDEX;
        println!("Karnal64 ARM: Görev {} (Index {}) gönüllü olarak işlemciyi bırakıyor (yield).", current_id, current_idx);

        // Mevcut görevin durumunu Ready olarak ayarla (eğer Running ise)
         let current_task_tcb = TASK_POOL[current_idx].as_mut()
            .ok_or(KError::InternalError)?; // TCB yoksa hata

        if current_task_tcb.state == TaskState::Running {
             current_task_tcb.state = TaskState::Ready;

            // Görevi hazır kuyruğunun sonuna ekle (Basit Round Robin için)
            // Kuyruk doluysa hata veya farklı davranış gerekebilir.
             let next_tail = (READY_QUEUE_TAIL + 1) % MAX_TASKS;
             if next_tail == READY_QUEUE_HEAD {
                 // Kuyruk dolu! Bu durum yield için ideal değil. Görev zaten Running.
                 // Hazır kuyruğuna ekleyemediğimiz için state'ini Ready yapmanın anlamı kalmaz.
                 // Durumu Running'e geri çekip hata dönebilir veya sadece loglayabiliriz.
                  println!("Karnal64 ARM: Görev {} yield sırasında hazır kuyruğu dolu! Bağlam değiştirilmedi.", current_id);
                 current_task_tcb.state = TaskState::Running; // Durumu geri al
                 return Err(KError::Busy); // Kuyruk meşgul hatası
             } else {
                 READY_QUEUE[READY_QUEUE_TAIL] = current_idx; // Görev indeksini kuyruğa ekle
                 READY_QUEUE_TAIL = next_tail; // Kuyruk sonunu ilerlet
             }
        } else {
            // Zaten Running değilse (örn. Sleeping, Blocked), yield çağırmak mantıksız olabilir.
            // Ya hata dönülür ya da hiçbir şey yapılmaz.
             println!("Karnal64 ARM: Görev {} yield çağrısı yaptı ama Running değildi ({:?}).", current_id, current_task_tcb.state);
             return Err(KError::InvalidArgument); // Veya Ok(())
        }


        // Bir sonraki göreve bağlam değiştir
        // Bu fonksiyon, yield çağrısı başarılı olduğunda mevcut görev tekrar çalışmaya başladığında geri dönecektir.
        schedule();

        // Görev tekrar çalışmaya başladığında buradan devam eder
        Ok(())
    }
}

 thread_create(task_id, entry_point, args) -> Result<KThreadId, KError>; // Eğer threadler görevlerden ayrıysa
 thread_exit(exit_code) -> !; // Eğer threadler görevlerden ayrıysa


// --- ARM Bağlam Değiştirme ve Zamanlayıcı ---

/// Görevleri zamanlayan ve bağlam değiştirmeyi başlatan fonksiyon.
/// Bu fonksiyon, mevcut görevin durumunu kaydeder, çalıştırılacak bir sonraki görevi seçer
/// ve ARM'e özel bağlam değiştirme assembly kodunu (`arm_context_switch`) çağırır.
/// `arm_context_switch` fonksiyonu geri dönmez; yürütme akışı yeni göreve geçer.
/// Bu fonksiyon kesme işleyicilerinden veya görevlerin gönüllü olarak işlemciyi bıraktığı yerlerden (yield, sleep, block) çağrılır.
///
/// # Güvenlik (Unsafe)
/// Bu fonksiyon kritik bir bölümdür. Çağrıldığında kesmeler kapatılmış veya uygun spinlock alınmış OLMALIDIR.
/// Statik veri yapılarına (`TASK_POOL`, `READY_QUEUE`, `CURRENT_TASK_INDEX`) erişim senkronizasyon gerektirir.
///
fn schedule() -> ! {
    unsafe {
        // CRITICAL SECTION START - Kesmeler kapalı olmalı veya spinlock alınmış olmalı
        // (Bu örnekte eksik)

        let prev_task_index = CURRENT_TASK_INDEX; // Bağlam değiştirmeden önceki görevin indeksi

        // Hazır kuyruğundan çalışacak bir sonraki görevi bul
        let mut next_task_index = READY_QUEUE_HEAD; // Başlangıç adayı
        let mut found_ready = false;

        // Basit Round Robin zamanlayıcı mantığı: Hazır kuyruğunda Ready durumda olan ilk görevi bul.
        // Kuyruğun tamamen boş olma ihtimali varsa, idle göreve geçiş mantığı eklenmelidir.
         let mut attempt_count = 0;
         while attempt_count < MAX_TASKS { // Sonsuz döngüden kaçınmak için max deneme sayısı
             let candidate_queue_index = (READY_QUEUE_HEAD + attempt_count) % MAX_TASKS;
             let candidate_task_pool_index = READY_QUEUE[candidate_queue_index];

             if let Some(ref tcb) = TASK_POOL[candidate_task_pool_index] {
                 if tcb.state == TaskState::Ready {
                     // Hazır bir görev bulduk!
                     next_task_index = candidate_task_pool_index; // Çalışacak görev indexi
                     // Kuyruktan çıkar (Mantıksal olarak, gerçekte sadece head'i ilerletiyoruz)
                     // Birden fazla elemanı atlamış olabiliriz, bu yüzden head'i bulduğumuz yere kadar ilerletelim.
                     READY_QUEUE_HEAD = (candidate_queue_index + 1) % MAX_TASKS;
                     found_ready = true;
                     break; // Döngüyü kır
                 }
             }
              attempt_count += 1;
         }


        // Eğer Ready durumda bir görev bulunamadıysa
        if !found_ready {
             // Hazır görev yok. İdeal olarak idle göreve geçilmeli (TASK_POOL[0] idle olsun).
            if let Some(ref tcb) = TASK_POOL[0] {
                if tcb.id.0 == 0 && (tcb.state == TaskState::Ready || tcb.state == TaskState::Running) {
                    // Idle görev Ready veya Running ise ona geç.
                    next_task_index = 0;
                    // Idle görev Ready kuyruğunda OLMAMALIDIR, her zaman çalıştırılabilir olmalıdır.
                    // Dolayısıyla kuyruktan çıkarmaya çalışmayız.
                    println!("Karnal64 ARM: Hazır görev yok, idle göreve geçiliyor (Index 0).");
                } else {
                     // Idle görev bile Ready/Running değilse, çekirdek kilitlenmiştir.
                     println!("Karnal64 ARM: Ciddi Hata: Hazır görev ve idle görev yok! Çekirdek kilitlendi.");
                     loop { /* Halted state veya panik */ }
                }
            } else {
                 // Idle görev TCB'si bile yoksa, ciddi hata.
                 println!("Karnal64 ARM: Ciddi Hata: Idle görev TCB'si (Index 0) bulunamadı! Çekirdek kilitlendi.");
                 loop { /* Halted state veya panik */ }
            }
        }

        // Çalışacak bir sonraki görevin index'ini belirledik.
        CURRENT_TASK_INDEX = next_task_index; // Şu anda çalışacak görevi güncelle

        // Önceki görevin TCB pointer'ı (eğer varsa)
        // task_exit durumunda prev_task_index'teki TCB None olabilir.
        let prev_tcb_ptr: *mut TaskControlBlock = match TASK_POOL[prev_task_index].as_mut() {
            Some(tcb) => {
                 // Önceki görevin durumunu Running'den Ready'ye veya başka bir duruma güncelle
                 // (Eğer task_exit ile çıkmadıysa ve Running ise)
                 if tcb.state == TaskState::Running {
                      tcb.state = TaskState::Ready; // Yield/Timer kesmesi vb. ile geldiyse
                 }
                tcb as *mut TaskControlBlock
            },
            None => {
                 // Önceki görev task_exit ile çıkmış ve TCB None yapılmışsa
                 println!("Karnal64 ARM: Önceki görev TCB'si (Index {}) task_exit ile kaldırılmış.", prev_task_index);
                 core::ptr::null_mut() // Null pointer geçir, arm_context_switch bunu işlemeli
            }
        };

        // Sonraki görevin TCB pointer'ı
        let next_task_tcb = TASK_POOL[CURRENT_TASK_INDEX].as_mut()
            .expect("Kritik Hata: Bağlam değiştirilecek sonraki görev TCB'si yok!");
        next_task_tcb.state = TaskState::Running; // Durumunu Running olarak ayarla
        let next_tcb_ptr: *mut TaskControlBlock = next_task_tcb as *mut TaskControlBlock;

        println!("Karnal64 ARM: Bağlam değiştirme başlatılıyor: Index {} -> Index {}",
                 prev_task_index, CURRENT_TASK_INDEX);

        // ARM özel bağlam değiştirme fonksiyonunu çağır
        // Bu fonksiyon mevcut registerları prev_tcb_ptr adresine kaydeder (prev_tcb_ptr null değilse)
        // ve next_tcb_ptr adresinden registerları yükler.
        // Fonksiyon döndüğünde, CURRENT_TASK_INDEX tarafından temsil edilen görevin bağlamında oluruz.
        arm_context_switch(prev_tcb_ptr, next_tcb_ptr);

        // CRITICAL SECTION END - Kesmeler tekrar açılmalı veya spinlock serbest bırakılmalı
        // (Bu örnekte eksik)

        // arm_context_switch geri dönmez. Yürütme akışı değişir.
        // Aşağıdaki kod buradan asla çalışmaz.
        loop {} // Güvenlik için sonsuz döngü
    }
}

/// ARM mimarisine özel bağlam değiştirme assembly/intrinsics fonksiyonu.
/// Bu fonksiyon, ARM AArch64 işlemcisinin registerlarını doğrudan manipüle eder.
/// Görevin durumunu kaydeder ve bir sonraki görevin durumunu yükler.
///
/// # Güvenlik (Unsafe)
/// Bu fonksiyon, registerları doğrudan manipüle eder ve pointerlarla çalışır.
/// Doğru kullanılmazsa ciddi kararsızlıklara, güvenlik açıklarına veya donanım kilitlemesine yol açabilir.
/// Çağıranın (schedule fonksiyonu) geçerli TCB pointerları sağladığından emin olması gerekir.
///
/// # Implementasyon Notu
/// Gerçek implementasyon, hedef işlemcinin ISA'sına (Instruction Set Architecture) ve
/// çağırma kuralına (Procedure Call Standard) tam uygun ARM assembly kodunda yapılır.
/// Buradaki inline assembly taslağı AArch64 için temel kaydetme/yükleme adımlarını gösterir.
///
/// `prev_tcb_ptr`: Mevcut (ayrılacak) görevin TCB pointer'ı. Registerlar buraya kaydedilir. Null olabilir (ilk schedule).
/// `next_tcb_ptr`: Sonraki (çalışacak) görevin TCB pointer'ı. Registerlar buradan yüklenir. Null OLMAMALIDIR.
#[inline(never)] // Derleyicinin bu fonksiyonu satır içine almasını engelle, net bir çağrı noktası olsun.
#[no_mangle] // Gerekirse assembly'den doğrudan çağrılabilmesi için (örn. düşük seviye trap işleyici)
unsafe extern "C" fn arm_context_switch(prev_tcb_ptr: *mut TaskControlBlock, next_tcb_ptr: *mut TaskControlBlock) {
    // Inline assembly (AArch64 syntax)
    // __current_task_tcb ve __next_task_tcb değişkenleri, Rust'ın inline assembly'ye
    // sağladığı isimlendirilmiş argümanlardır. `in(reg)` kısıtlaması, pointerların
    // bir genel amaçlı register'a yerleştirilmesini ister.

    asm!(
        // 1. Mevcut görevin (prev_tcb_ptr) durumunu kaydet
        // prev_tcb_ptr null değilse kaydet (İlk schedule çağrısında kernel bağlamı için null olabilir)
        "cmp {prev_tcb_ptr}, #0", // prev_tcb_ptr 0 mı?
        "b.eq 2f",                // Eşitse (0 ise) kaydetme adımını atla (etiket 2'ye dallan)

        // Non-volatile GPR'ları kaydet (x19 - x28)
        "stp x19, x20, [{prev_tcb_ptr}, #{x19_offset}]", // Store Pair (x19 ve x20'yi kaydet)
        "stp x21, x22, [{prev_tcb_ptr}, #{x21_offset}]",
        "stp x23, x24, [{prev_tcb_ptr}, #{x23_offset}]",
        "stp x25, x26, [{prev_tcb_ptr}, #{x25_offset}]",
        "stp x27, x28, [{prev_tcb_ptr}, #{x27_offset}]",
        // FP (x29) ve LR (x30) kaydet
        "stp x29, x30, [{prev_tcb_ptr}, #{x29_offset}]",

        // SP (Stack Pointer) kaydet
        // Mevcut SP'yi okuyup TCB'deki SP alanına kaydediyoruz.
        "mov x17, sp", // SP'yi geçici x17 register'ına taşı
        "str x17, [{prev_tcb_ptr}, #{sp_offset}]", // x17'yi TCB'deki SP alanına kaydet

        // SPSR_EL1 ve ELR_EL1 kaydet (Exception dönüşü için gerekli durum registerları)
        // Bu registerlar sadece EL1'den (kernel) EL0'a (user) dönerken geçerlidir.
        // Eğer bağlam değiştirme EL1 içinde kalıyorsa (kernel threadleri arası) farklı registerlar (SP_EL1 gibi) gerekebilir.
        // Bu taslak, EL1'den EL0'a dönüşü hedefliyor gibi yapılandırıldı.
        "mrs x17, spsr_el1", // SPSR_EL1'i x17'ye oku
        "str x17, [{prev_tcb_ptr}, #{spsr_offset}]", // x17'yi TCB'deki SPSR alanına kaydet
        "mrs x17, elr_el1",  // ELR_EL1'i x17'ye oku (Exception'a girerken PC'nin kaydedildiği yer)
        "str x17, [{prev_tcb_ptr}, #{elr_offset}]", // x17'yi TCB'deki ELR alanına kaydet


        "2:", // Kaydetme bitti (veya atlandı)

        // 2. Sonraki görevin (next_tcb_ptr) durumunu yükle
        // next_tcb_ptr'nin null OLMAMASI beklenir (scheduler geçerli bir görev bulmalı).
        // Gerekirse burada null kontrolü ve hata işleme eklenebilir, ama schedule bunu garanti etmeli.

        // Non-volatile GPR'ları yükle (x19 - x28)
        "ldp x19, x20, [{next_tcb_ptr}, #{x19_offset}]", // Load Pair
        "ldp x21, x22, [{next_tcb_ptr}, #{x21_offset}]",
        "ldp x23, x24, [{next_tcb_ptr}, #{x23_offset}]",
        "ldp x25, x26, [{next_tcb_ptr}, #{x25_offset}]",
        "ldp x27, x28, [{next_tcb_ptr}, #{x27_offset}]",
        // FP (x29) ve LR (x30) yükle
        "ldp x29, x30, [{next_tcb_ptr}, #{x29_offset}]",

        // SP (Stack Pointer) yükle
        "ldr x17, [{next_tcb_ptr}, #{sp_offset}]", // TCB'deki SP alanını x17'ye oku
        "mov sp, x17", // x17'yi SP'ye taşı

        // SPSR_EL1 ve ELR_EL1 yükle
        "ldr x17, [{next_tcb_ptr}, #{spsr_offset}]", // TCB'deki SPSR alanını x17'ye oku
        "msr spsr_el1, x17", // x17'yi SPSR_EL1'e yaz (bu işlem imtiyaz seviyesini, bayrakları vb. ayarlar)
        "ldr x17, [{next_tcb_ptr}, #{elr_offset}]",  // TCB'deki ELR alanını x17'ye oku (Çalışmaya devam edilecek adres)
        "msr elr_el1, x17", // x17'yi ELR_EL1'e yaz

        // TODO: Eğer TCB'de MMU bağlamı (TTBR0_EL1, TTBR1_EL1) saklanıyorsa, onları da yükle.
        // Örneğin: "ldr x17, [{next_tcb_ptr}, #ttbr0_offset]", "msr ttbr0_el1, x17" vb.
        // TLB invalidation (geçersiz kılma) da gerekebilir.

        // 3. Yürütmeyi yeni görevin bağlamında başlat
        // EL1'den EL0'a (Kernel'den Kullanıcıya) dönmek için `eret` (Exception Return) kullanılır.
        // `eret` komutu, ELR_EL1'deki adrese dallanır ve SPSR_EL1'deki durumu (imtiyaz seviyesi, bayraklar) yükler.
        // Biz ELR_EL1'e kullanıcının programının giriş noktasını ayarlamıştık (`task_spawn` içinde).
        "eret", // Exception'dan dön ve ELR_EL1/SPSR_EL1 tarafından belirlenen yerde çalışmaya başla.


        // Argümanlar:
        prev_tcb_ptr = in(reg) prev_tcb_ptr, // Rust pointer'ı register'a koy
        next_tcb_ptr = in(reg) next_tcb_ptr, // Rust pointer'ı register'a koy

        // Sabit offset'ler (assembly'nin literal sayıları kabul etmesi için const)
        x19_offset = const TCB_X19_OFFSET,
        x21_offset = const TCB_X21_OFFSET,
        x23_offset = const TCB_X23_OFFSET,
        x25_offset = const TCB_X25_OFFSET,
        x27_offset = const TCB_X27_OFFSET,
        x29_offset = const TCB_X29_OFFSET,
        sp_offset = const TCB_SP_OFFSET,
        spsr_offset = const TCB_SPSR_OFFSET,
        elr_offset = const TCB_ELR_OFFSET,
        ttbr0_offset = const TCB_TTBR0_OFFSET, // MMU bağlamı varsa

        // Clobber listesi: Bağlam değiştirmede kullanılan geçici registerlar
        // Bu registerların değerleri kaydedilmediği için derleyiciye kirletileceği söylenir.
        // x17 burada geçici register olarak kullanıldı.
        out("x17") _,
         out("x18") _, // x18 bazı ABIs'de Platform Register olabilir, dikkatli kullanın
         out("memory") // Bellek durumu değişti, derleyiciye cache'leri güncellemesini söyle (genellikle bağlam değiştirmede kullanılır)
    );

    // `eret` komutu bu fonksiyondan asla geri dönmez. Yürütme akışı yeni göreve geçer.
    // Bu satır unreachable olmalıdır.
     panic!("Karnal64 ARM: Context switch failed to return to a task!"); // Hata durumunda panik (debugging için)
}

/// Görev sonlandığında, TCB'nin X30 (LR) registerına ayarlanmış olan dahili işleyici.
/// Görev fonksiyonu normal olarak (return ile) bittiğinde bu fonksiyona dallanılır.
#[inline(never)] // Satır içine alınmasını engelle
#[no_mangle] // Gerekirse assembly'den veya düşük seviyeden çağrılabilmesi için
unsafe extern "C" fn task_exit_handler() {
    // Görev fonksiyonu normal olarak (return ile) buradan döndü.
    // Görev sonlandırma sürecini başlatmak için `task_exit` çekirdek fonksiyonunu çağır.
    // `task_exit` fonksiyonu noreturn olduğu için güvenlidir ve schedule'ı çağırarak başka göreve geçer.
    task_exit(0); // Varsayılan çıkış kodu 0 ile task_exit'i çağır.
    // task_exit çağrısı schedule'ı çağırır ve buradan asla geri dönülmez.
    // Bu satırın altındaki kod unreachable olmalıdır.
     panic!("Karnal64 ARM: task_exit_handler returned!"); // Hata ayıklama için
}
