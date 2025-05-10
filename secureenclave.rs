#![no_std]
#![allow(dead_code)]
#![allow(unused_variables)]

// Karnal64 API'sından gerekli tipleri ve traitleri içe aktar
use super::{
    KError,
    KHandle,
    KTaskId,
    ResourceProvider, // Karnal64 Kaynak Sağlayıcı trait'i
    // Diğer Karnal64 modüllerinden ihtiyaç duyulabilecek öğeler
    kresource,
    kmemory,
    ktask,
    ksync, // Senkronizasyon primitifleri için
};

// Statik yönetici örneği için senkronizasyon
// Gerçek çekirdekte burası daha sağlam bir spinlock veya mutex olacaktır.
// Basit bir yer tutucu kullanalım.
use ksync::Spinlock; // ksync modülünün Spinlock sağladığını varsayalım.

// --- Güvenli Alan (Secure Enclave) Yönetimine Özgü Tanımlar ---

/// Güvenli Alan Kimliği
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EnclaveId(u64);

/// Güvenli Alan Durumu
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum EnclaveState {
    Created,
    Loading,
    Running,
    Paused,
    Destroyed,
    Error,
}

// Güvenli Alan Kontrol Komutları (ResourceProvider::control için)
// Bu sabitler, kullanıcı alanındaki karşılıklarıyla eşleşmelidir.
pub const SECURE_ENCLAVE_CONTROL_CREATE: u64 = 1;
pub const SECURE_ENCLAVE_CONTROL_DESTROY: u64 = 2;
pub const SECURE_ENCLAVE_CONTROL_LOAD_CODE: u64 = 3; // Kod ve veri yükle
pub const SECURE_ENCLAVE_CONTROL_RUN: u64 = 4; // Alanı çalıştır
pub const SECURE_ENCLAVE_CONTROL_GET_STATE: u64 = 5; // Durumunu sorgula
// ... Diğer komutlar (IPC, bellek yönetimi vb.)

// --- Güvenli Alan Veri Yapıları ---

/// Tek bir Güvenli Alanın Temsili
struct Enclave {
    id: EnclaveId,
    state: EnclaveState,
    /// Enclave'e tahsis edilen bellek alanlarının listesi
    memory_regions: Vec<*mut u8>, // Basit bir yer tutucu
    /// Enclave içinde çalışan ilk görev/iş parçacığının Karnal64 ID'si (varsa)
    associated_task: Option<KTaskId>,
    // Platforma özgü donanım bağlamı/tanıtıcısı
    hardware_context: u64, // Yer tutucu
}

impl Enclave {
    /// Yeni bir boş Enclave örneği oluştur
    fn new(id: EnclaveId, hardware_context: u64) -> Self {
        Self {
            id,
            state: EnclaveState::Created,
            memory_regions: Vec::new(), // Veya sabit boyutlu dizi
            associated_task: None,
            hardware_context,
        }
    }

    // TODO: Enclave yaşam döngüsü ve etkileşim metodları (load, run, terminate vb.)
    // Bu metodlar kmemory, ktask, ksync gibi diğer çekirdek modüllerini kullanacaktır.
}

/// Tüm Güvenli Alanları Yöneten Merkezi Yapı
struct SecureEnclaveManager {
    /// Yönetilen enclave'ler (örneğin ID'ye göre map/vektör)
    enclaves: Vec<Enclave>, // Basitlik için Vec kullanalım, gerçekte Map daha iyi olabilir.
    next_enclave_id: u64,
    /// Yöneticiye erişim için spinlock
    lock: Spinlock, // ksync modülünden geldiğini varsayalım
}

// Statik, global yönetici örneği
// !!! DİKKAT: `static mut` kullanmak Rust'ta genellikle unsafe'dir ve spinlock
// ile korunması ZORUNLUDUR. `once_cell::Once` veya benzeri bir mekanizma
// `no_std` ortamında uygunsa tercih edilmelidir. Basitlik için burada `static mut`
// ve yer tutucu Spinlock kullanılmıştır.
static mut ENCLAVE_MANAGER: Option<SecureEnclaveManager> = None;
static ENCLAVE_MANAGER_LOCK: Spinlock = Spinlock::new(); // ksync modülünden

impl SecureEnclaveManager {
    /// Yöneticiyi başlatır (çekirdek başlatma sırasında bir kez çağrılır)
    fn init() -> Self {
        // TODO: Donanım/firmware ile etkileşime geçerek enclave desteğini kontrol et.
        // Gerekli donanım bağlamlarını başlat.
         println!("SecureEnclave: Yönetici Başlatılıyor..."); // Çekirdek içi print!

        SecureEnclaveManager {
            enclaves: Vec::new(), // Veya fixed-size array / custom map
            next_enclave_id: 1, // ID'ler 1'den başlasın
            lock: Spinlock::new(),
        }
    }

    /// Yeni bir Güvenli Alan oluşturur
    fn create_enclave(&mut self) -> Result<EnclaveId, KError> {
        let _guard = self.lock.lock(); // Lock the manager

        // TODO: Donanım/firmware üzerinden yeni bir enclave bağlamı oluştur
        // Bu, platforma özgü bir işlemdir ve başarısız olabilir.
         let hardware_ctx = 0x100; // Yer tutucu donanım bağlamı

        let id = EnclaveId(self.next_enclave_id);
        self.next_enclave_id += 1;

        let enclave = Enclave::new(id, hardware_ctx);

        self.enclaves.push(enclave); // TODO: Daha verimli depolama yapısı kullan

        println!("SecureEnclave: Yeni alan oluşturuldu {:?}", id);

        Ok(id)
    }

    /// Belirtilen Güvenli Alanı yok eder
    fn destroy_enclave(&mut self, id: EnclaveId) -> Result<(), KError> {
         let _guard = self.lock.lock(); // Lock the manager

        // Enclave'i listede bul
        if let Some(index) = self.enclaves.iter().position(|e| e.id == id) {
            let mut enclave = self.enclaves.swap_remove(index); // Enclave'i çıkar

            // TODO: Bu enclave'e ait kaynakları serbest bırak (bellek, görevler vb.)
            // enclave.memory_regions içindeki tüm bellek alanlarını serbest bırakmak için kmemory kullan
            for region in enclave.memory_regions.drain(..) {
                 // kmemory::free_enclave_memory(region)?; // Varsayımsal free fonksiyonu
            }
            // İlişkili görevi sonlandır (varsa)
            if let Some(task_id) = enclave.associated_task {
                  ktask::terminate_task(task_id)?; // Varsayımsal terminate fonksiyonu
            }

            // TODO: Donanım/firmware üzerinden enclave bağlamını serbest bırak
             println!("SecureEnclave: Alan yok edildi {:?}", id);
            Ok(())
        } else {
             println!("SecureEnclave: Alan bulunamadı {:?}", id);
            Err(KError::NotFound)
        }
    }

    /// Enclave'e kod ve veri yükler
    /// `id`: Hedef Enclave ID
    /// `user_data_ptr`: Kullanıcı alanındaki yüklenecek veri pointer'ı
    /// `user_data_len`: Yüklenecek veri uzunluğu
    /// !!! GÜVENLİK: user_data_ptr/len ÇOK DİKKATLİ DOĞRULANMALIDIR!
    /// resource_control syscall işleyicisinin bu veriyi kernel alanına güvenli
    /// bir şekilde kopyaladığı ve size buraya bir kernel pointer/slice verdiği
    /// varsayılır. Bu örnekte, arg u64 olduğu için pointer doğrudan geçilemez,
    /// farklı bir mekanizma (örn. shared memory handle'ı veya daha karmaşık arg yapısı)
    /// gereklidir. Basitlik için arg'ın yükleme adresi veya başka bir bilgi olduğunu
    /// varsayalım veya verinin farklı bir syscall ile geldiğini not edelim.
    fn load_enclave_code(&mut self, id: EnclaveId, arg: u64) -> Result<(), KError> {
        let _guard = self.lock.lock(); // Lock the manager

        // Enclave'i bul
        if let Some(enclave) = self.enclaves.iter_mut().find(|e| e.id == id) {
            if enclave.state != EnclaveState::Created {
                return Err(KError::InvalidArgument); // Sadece Created durumunda yükleme yapılabilir
            }

            // TODO: arg değerini kullanarak yükleme verisine erişin (örneğin, arg shared memory handle'ı olabilir)
            // TODO: kmemory::allocate_enclave_memory() kullanarak enclave için bellek ayırın.
            // TODO: Kullanıcı verisini (arg'nin işaret ettiği yerdeki veya shared memory'deki)
            // ayrılan enclave belleğine kopyalayın. GÜVENLİ KOPYALAMA ZORUNLU!
            // TODO: Donanım/firmware API'larını kullanarak kodu enclave içine yükleyin.

            // Örnek: Basit durum güncellemesi
            enclave.state = EnclaveState::Loading; // Yükleme başladı
             println!("SecureEnclave: Alan {} için kod yükleniyor (arg: {})", id.0, arg);

            // TODO: Yükleme tamamlandığında durumu Running veya başka bir şeye güncelleyin.
             enclave.state = EnclaveState::Loaded; // Yeni durum

            Ok(())
        } else {
            Err(KError::NotFound)
        }
    }

    /// Enclave'i çalıştırır
    fn run_enclave(&mut self, id: EnclaveId) -> Result<(), KError> {
        let _guard = self.lock.lock(); // Lock the manager

        // Enclave'i bul
        if let Some(enclave) = self.enclaves.iter_mut().find(|e| e.id == id) {
            // TODO: Durum kontrolü yap (örn. Loaded durumunda olmalı)
             if enclave.state != EnclaveState::Loading { // Yükleme bitti/Loaded varsayalım
                 return Err(KError::InvalidArgument);
             }

            // TODO: Donanım/firmware API'larını kullanarak enclave'i çalıştırın.
            // TODO: Enclave'in ilk entry point'ini bir Karnal64 görevine/iş parçacığına bağlayın (isteğe bağlı, modellemeye bağlı)
            // Örneğin, bir ktask::spawn_enclave_task() çağrısı yapılabilir.
             let task_id = ktask::create_enclave_task(id)?; // Varsayımsal görev oluşturma

            enclave.state = EnclaveState::Running;
            enclave.associated_task = Some(task_id);

            println!("SecureEnclave: Alan {} çalıştırılıyor, Görev ID: {:?}", id.0, task_id);

            Ok(())
        } else {
            Err(KError::NotFound)
        }
    }

    /// Enclave'in güncel durumunu döndürür
    fn get_enclave_state(&self, id: EnclaveId) -> Result<EnclaveState, KError> {
         let _guard = self.lock.lock(); // Lock the manager

        // Enclave'i bul
        if let Some(enclave) = self.enclaves.iter().find(|e| e.id == id) {
            Ok(enclave.state)
        } else {
            Err(KError::NotFound)
        }
    }

    // TODO: IPC metodları (send_message, receive_message) implemente et
    // Bu metodlar kmessaging veya enclave'e özgü IPC mekanizmalarını kullanabilir.
    // Yine kullanıcı pointer doğrulaması ve veri kopyalama GÜVENLİK AÇISINDAN KRİTİKTİR.
}

// --- ResourceProvider Trait Implementasyonu ---
// Güvenli Alan Yönetimini Karnal64 Kaynak Yöneticisine Kaydetmek İçin

impl ResourceProvider for SecureEnclaveManager {
    /// Enclave kaynağı doğrudan okunamaz, hata döndür.
    fn read(&self, buffer: &mut [u8], offset: u64) -> Result<usize, KError> {
        // Enclave kaynağı genellikle dosya gibi okunmaz.
        Err(KError::NotSupported)
    }

    /// Enclave kaynağı doğrudan yazılamaz, hata döndür.
    fn write(&self, buffer: &[u8], offset: u64) -> Result<usize, KError> {
        // Enclave kaynağı genellikle dosya gibi yazılamaz.
        Err(KError::NotSupported)
    }

    /// Enclave yönetimine özel kontrol komutlarını işler.
    /// `request`: Komut kodu (SECURE_ENCLAVE_CONTROL_* sabitlerinden biri)
    /// `arg`: Komuta özgü argüman (enclave ID, pointer, değer vb.)
    /// GÜVENLİK: arg değeri bir kullanıcı alanı pointer'ı İSE, bu noktaya gelmeden
    /// sistem çağrısı işleyicisi veya resource_control API fonksiyonu tarafından
    /// doğrulanmış ve içeriği güvenli bir şekilde kernel alanına kopyalanmış olmalıdır!
    /// Burada `arg`'nin ya bir değer (enclave ID gibi) ya da KOPYALANMIŞ veriye işaret
    /// eden bir kernel alanı pointer'ı olduğunu varsayalım.
    fn control(&self, request: u64, arg: u64) -> Result<i64, KError> {
        // Global yönetici örneğine güvenli erişim
         let manager = unsafe {
             // Spinlock ile korunduğu varsayılan statik mut değişkene erişim
             // GERÇEK KODDA ÇOK DİKKATLİ KULLANILMALI
             ENCLAVE_MANAGER.as_mut().ok_or(KError::InternalError)?
         };


        match request {
            SECURE_ENCLAVE_CONTROL_CREATE => {
                // arg kullanılmaz veya ek ayarlar için olabilir, şu an için 0 varsayalım
                if arg != 0 { return Err(KError::InvalidArgument); }
                manager.create_enclave().map(|id| id.0 as i64) // Başarılı olursa enclave ID döner
            }
            SECURE_ENCLAVE_CONTROL_DESTROY => {
                let enclave_id = EnclaveId(arg); // arg'nin enclave ID olduğu varsayılır
                manager.destroy_enclave(enclave_id).map(|_| 0) // Başarılı olursa 0 döner
            }
            SECURE_ENCLAVE_CONTROL_LOAD_CODE => {
                // arg'nin yüklenecek verinin kernel alanı pointer'ı olduğunu varsayalım.
                // SYSCALL işleyici bu pointer'ı kullanıcıdan alıp doğrulayıp veriyi
                // kernel tamponuna kopyalamış ve arg olarak o tamponun adresini
                // veya bir handle'ını geçirmiş olmalı.
                // Bu senaryoda, kontrol komutunun hangi Enclave ID'si için olduğunu
                // başka bir arg ile (veya control fonksiyonuna Enclave handle'ı geçilerek)
                // bilmemiz gerekir. Mevcut ResourceProvider traitinde sadece request ve arg var.
                // Daha gerçekçi bir senaryo: arg'nin Enclave ID'si + ek data pointer'ı
                // içeren bir struct'a kernel pointer'ı olması. Ya da API signature değişmeli.
                // Basitlik için, arg'nin sadece Enclave ID olduğunu ve data'nın başka bir yerden
                // (örn. daha önce map edilmiş shared memory) geldiğini varsayalım.
                // Ya da arg'nin Enclave ID'si, diğer argümanların data pointer/len olduğu bir
                // `resource_control` syscall signature'ı varsayalım, ancak trait buna uymuyor.
                // Geçici çözüm: arg = Enclave ID, ve veri işleme logic'i bu arg'den bağımsız,
                // veya bu control çağrısı sadece yüklemeyi başlatır, veri aktarımı ayrıdır.
                // Veya ResourceProvider::control'un signature'ını değiştirmek gerekir.

                // Geçerli trait signature'ına uyacak şekilde: arg'nin enclave ID olduğunu varsayalım.
                let enclave_id = EnclaveId(arg);
                // Yükleme verisi işi daha karmaşık, bu control çağrısı sadece yükleme komutunu
                // başlatır, veri aktarımı başka bir mekanizma (IPC, başka syscall) ile olur.
                // Ya da arg'nin "yüklenecek veri kaynağının handle'ı" olduğunu varsayalım.
                // Let's assume arg is the Enclave ID for now, and the actual data transfer
                // mechanism needs to be defined elsewhere or requires a different control command signature.
                // Ya da arg'nin Enclave ID'si ve Yüklenecek veriyi içeren bir kernel tampon pointer'ı
                // olduğu karma bir değer olduğunu varsayalım (pratik değil).

                // En mantıklısı: control sadece komutu ve birincil argümanı alır.
                // Load için, arg Enclave ID'si olsun. Verinin nereden geleceği (Shared Memory Handle?)
                // farklı bir mekanizma ile belirlenir veya bu control çağrısı bir başlangıçtır.
                // En basit haliyle: arg = Enclave ID.
                let enclave_id = EnclaveId(arg);
                // Yükleme verisi arg'de değil, bunun başka bir syscall veya shared memory ile gelmesi lazım.
                // Şimdilik, sadece Enclave ID'sini alıp load_enclave_code'u çağıralım, veri işleme TODO olarak kalsın.
                manager.load_enclave_code(enclave_id, 0) // 0: Veri argümanı yer tutucusu
                   .map(|_| 0) // Başarılı olursa 0 döner
            }
            SECURE_ENCLAVE_CONTROL_RUN => {
                let enclave_id = EnclaveId(arg); // arg'nin enclave ID olduğu varsayılır
                manager.run_enclave(enclave_id).map(|_| 0) // Başarılı olursa 0 döner
            }
             SECURE_ENCLAVE_CONTROL_GET_STATE => {
                 let enclave_id = EnclaveId(arg); // arg'nin enclave ID olduğu varsayılır
                 manager.get_enclave_state(enclave_id)
                    .map(|state| {
                        // Durumu i64 olarak döndür (eşleme yapılması gerekir)
                        match state {
                            EnclaveState::Created => 1,
                            EnclaveState::Loading => 2,
                            EnclaveState::Running => 3,
                            EnclaveState::Paused => 4,
                            EnclaveState::Destroyed => 5, // Aslında bu durumda Not Found dönmeliydi
                            EnclaveState::Error => -1, // Hata durumunu negatif ile işaretle
                        } as i64
                    })
             }
            // TODO: Diğer kontrol komutlarını ekle

            _ => Err(KError::InvalidArgument), // Bilinmeyen komut
        }
    }

     fn seek(&self, position: KseekFrom) -> Result<u64, KError> {
         // Enclave kaynağı seekable değil
         Err(KError::NotSupported)
     }

     fn get_status(&self) -> Result<KResourceStatus, KError> {
         // Enclave yöneticisinin genel durumu hakkında bilgi verebilir
         // Örneğin kaç enclave olduğu vb.
         // Bu trait'in KResourceStatus tanımına bağlıdır.
         // Şimdilik NotSupported diyelim.
         Err(KError::NotSupported)
     }
}

// --- Modül Başlatma ---

/// Secure Enclave yönetim modülünü başlatır.
/// Karnal64 init fonksiyonu tarafından çağrılmalıdır.
pub fn init() {
    println!("SecureEnclave: Modül Başlatılıyor..."); // Çekirdek içi print!

    // Yönetici örneğini oluştur ve statik değişkene yerleştir.
    // Sadece bir kez ve güvenli bir şekilde yapılmalı.
     let manager = SecureEnclaveManager::init();
     let manager_box = Box::new(manager); // Heap'te sakla

    // Statik mut değişkene erişim güvenli olmalı (örneğin once_cell veya lock ile)
     unsafe {
         // İlk başlatmada ENCLAVE_MANAGER None olmalı
         if ENCLAVE_MANAGER.is_some() {
              panic!("SecureEnclave Manager zaten başlatıldı!"); // Hata durumu
         }
         // Box<dyn ResourceProvider> olarak kaydetmek için Box'u kullanıyoruz
         // Ancak ENCLAVE_MANAGER static mut Option<Self> olduğu için,
         // burada manager_box'ı static mut değişkenin yerine koymak değil,
         // manager_box'ın *işaret ettiği yere* bir şekilde ENCLAVE_MANAGER'ı kurmak
         // veya ENCLAVE_MANAGER'ın kendisini Box içine almak gibi bir mekanizma lazım.
         // `Box::leak` veya `static mut`'a doğrudan yerleştirme (güvenli olmayabilir)

         // Basitlik için: Manager'ı statik mut değişkene atayalım
         // GERÇEKTE: Box<dyn ResourceProvider + Send + Sync> yapılıp kaydedilmesi lazım.
         // ENCLAVE_MANAGER = Some(*manager_box); // Bu taşınma hatası verir

         // Global static instance pattern using `static mut` and a lock:
         // The `init` function itself is called inside the lock or guaranteed single-threaded.
         // Let's assume init guarantees single-threaded access.
         ENCLAVE_MANAGER = Some(SecureEnclaveManager::init()); // Initialize directly


     }

    // Kaynak sağlayıcıyı Karnal64 Kaynak Yöneticisine kaydet.
    // Burada kaydedilen şey, ResourceProvider traitini implemente eden
    // SecureEnclaveManager'ın global örneğine erişim sağlamalıdır.
    // kresource::register_provider fonksiyonunun Box<dyn ResourceProvider> aldığını varsayalım.

    // Kaydedilecek nesne global, statik yönetici olmalı.
    // Bunun için yönetici struct'ın kendisinin ResourceProvider olması ve
    // global statik örneğin bir referansının (veya Box'unun) kaydedilmesi gerekir.
    // ResourceProvider trait'i &self aldığı için, kaydedilen nesne statik ömre sahip
    // veya heap'te yaşayan bir nesne olmalı. Global statik mut nesne bu işe yarar.

    // `register_provider` muhtemelen `Box<dyn ResourceProvider>` alır.
    // Global static mut'tan Box<dyn ResourceProvider> oluşturmak karmaşıktır
    // ve `static mut`'ın ömrü ile ilgili sorunlar yaratabilir.
    // Daha iyi yaklaşım: Global statik mut `SecureEnclaveManager`'ın kendisinin
    // ResourceProvider implementasyonu olması ve register_provider'ın
    // `&'static dyn ResourceProvider` veya `KHandle` dönmesi.
    // Önceki Karnal64 kodunda register_provider KHandle dönüyordu ve Box<dyn ResourceProvider> alıyordu.
    // Bu durumda, global statik mut'ı bir Box'a alıp `leak` etmek gerekebilir (unsafe).

    // Let's refine the static instance access and registration.
    // A common kernel pattern is to use a global static reference behind a lock.
     let provider: &'static dyn ResourceProvider = unsafe {
         // Dereference and get a static reference to the initialized manager.
         // This is safe *only* if init guarantees initialization and the lock
         // is used for all accesses via `control`.
         ENCLAVE_MANAGER.as_ref().expect("Enclave Manager should be initialized")
     };

    // Kaynak adını tanımla
    let resource_name = "karnal://device/secure_enclave";

    // Kaynak Yöneticisine kaydet (ResourceProvider traitini implemente eden nesneyi)
    // kresource::register_provider'ın &str ve &'static dyn ResourceProvider alması
    // veya Box<dyn ResourceProvider> alması senaryosuna göre burası değişir.
    // Önceki Karnal64 taslağında Box alıyordu.

    // Box alıyorsa ve statik referansımız varsa:
    // Bunu Box'a sarmalamanın doğru yolu `Box::new(provider)` değildir,
    // çünkü bu provider'ın referansını kopyalar, asıl nesneyi Box'a almaz.
    // `Box::leak` unsafe kullanılarak statik mut'tan oluşturulan Box sızdırılabilir.
    // Bu karmaşıklık yerine, farz edelim ki `kresource::register_provider`
    // &'static dyn ResourceProvider alabilir veya içerde kendisi yönetebilir.

     let registration_result = kresource::register_provider(resource_name, provider);

     match registration_result {
         Ok(_) => println!("SecureEnclave: '{}' kaynağı başarıyla kaydedildi.", resource_name),
         Err(e) => println!("SecureEnclave: '{}' kaynağı kaydedilirken hata: {:?}", resource_name, e),
     }
}

// --- Yer Tutucu Diğer Çekirdek Modüllerinden İhtiyaç Duyulan Öğeler ---
// Bunlar normalde ilgili modül dosyalarında (ksync.rs, kresource.rs, ktask.rs) tanımlanır.
// secureenclave.rs'nin derlenmesi için burada temel yer tutucular ekleyelim.

mod ksync {
    use super::KError;
    use core::cell::UnsafeCell;
    use core::ops::{Deref, DerefMut};

    /// Basit Yer Tutucu Spinlock (Gerçek implementasyon platforma/mimarisine bağlıdır)
    /// Çekirdek kodunda, kesmeleri devre dışı bırakarak veya atomik işlemlerle implemente edilir.
    pub struct Spinlock {
        locked: UnsafeCell<bool>, // bool yerine bir mimariye özgü kilit tipi daha uygun
    }

    // `Sync` implementasyonu, içeriğin Send olması veya Spinlock'ın kendisinin
    // atomik/güvenli olmasını gerektirir. Basitlik için `unsafe impl Sync` kullanıyoruz.
    // Gerçek Spinlock implementasyonu genellikle Sync'tir.
    unsafe impl Sync for Spinlock {}

    impl Spinlock {
        pub const fn new() -> Self {
            Self { locked: UnsafeCell::new(false) }
        }

        /// Kilidi almaya çalışır, alınana kadar bekler.
        /// Kesmeleri devre dışı bırakmalıdır (çekirdek ortamında).
        pub fn lock(&self) -> SpinlockGuard {
            // TODO: Atomik döngü kullanarak veya kesmeleri devre dışı bırakarak kilidi al.
             //println!("Spinlock: Kilitleniyor..."); // Debug çıktısı
            let locked_ptr = self.locked.get();
            loop {
                unsafe {
                    // Basit atomik olmayan kontrol, gerçekte yanlış
                    if !*locked_ptr {
                        *locked_ptr = true;
                        break;
                    }
                }
                 // TODO: Kısa bir süre bekle (yield veya spin)
            }
             //println!("Spinlock: Kilit Alındı.");
            SpinlockGuard { lock: self }
        }

        /// Kilidi serbest bırakır.
        /// Kesmeleri geri açmalıdır (çekirdek ortamında).
        #[allow(clippy::drop_copy)] // Guard ile drop çağrısı beklendiği için
        fn unlock(&self) {
            // TODO: Atomik olarak veya kesmeleri geri açarak kilidi serbest bırak.
             //println!("Spinlock: Kilit Serbest Bırakılıyor..."); // Debug çıktısı
            let locked_ptr = self.locked.get();
            unsafe {
                *locked_ptr = false;
            }
             // TODO: Kesmeleri geri aç
             //println!("Spinlock: Kilit Serbest Bırakıldı.");
        }
    }

    /// Spinlock'ın Kilit Kalkanı (RAII deseni)
    pub struct SpinlockGuard<'a> {
        lock: &'a Spinlock,
    }

    // Deref ve DerefMut implementasyonları, guard'ın içeriğe (kilitlenen veri yapısına)
    // erişim sağlamasını sağlar. Ancak bu Spinlock örneği sadece bir `bool` kilitlediği için
    // içeriğe doğrudan erişim sağlamaz, sadece kilidin kendisini yönetir.
    // Gerçekte Guard, kilitlenen veriyi (örn. `&'a mut T` eğer `Spinlock<T>` olsaydı) tutar.
    // Bu yer tutucuda Deref uygulamak mantıklı değil.

    impl Drop for SpinlockGuard<'_> {
        fn drop(&mut self) {
            self.lock.unlock();
        }
    }
}

mod kresource {
    use super::{KError, KHandle, ResourceProvider};
    use alloc::boxed::Box;
    // alloc kütüphanesine ihtiyacımız var, ya feat = "alloc" kullanılacak ya da
    // kernel'in kendi allocator'ı impl edilecek. `Box::new` kullanabilmek için alloc gerekli.

    // ResourceProvider traitini implemente eden nesneyi saklayan yapı
    struct ProviderEntry {
         name: &'static str, // Kaynak adı
         provider: Box<dyn ResourceProvider + Send + Sync>, // Provider nesnesi
         // TODO: Handle eşlemeleri, izinler vb.
    }

    // Kayıtlı provider'ları saklayan global liste/map
    // Yine static mut ve lock gerekli
    static mut REGISTERED_PROVIDERS: Vec<ProviderEntry> = Vec::new(); // alloc gerektirir
    static PROVIDER_REGISTRY_LOCK: super::ksync::Spinlock = super::ksync::Spinlock::new();

    pub fn init_manager() {
         println!("KResource: Yönetici Başlatılıyor...");
         // Vector'ü başlatmaya gerek yok, lazy init gibi düşünülebilir veya fixed-size array kullanılır.
    }

    // Kaynak sağlayıcıyı kaydetme fonksiyonu
    // `provider`: ResourceProvider traitini implemente eden nesne
    pub fn register_provider(name: &'static str, provider: &'static dyn ResourceProvider) -> Result<KHandle, KError> {
         let _guard = PROVIDER_REGISTRY_LOCK.lock(); // Lock registry

        // TODO: name çakışması kontrolü
        // TODO: provider nesnesini sakla ve buna bir handle ata

        // Provider referansını Box'a almak zor.
        // Alternatif olarak, ProviderEntry struct'ı Box<dyn ResourceProvider> yerine
        // &'static dyn ResourceProvider tutabilir, eğer provider nesnesi statik ömürlüyse
        // (bizim SecureEnclaveManager örneği gibi).

        // Basitlik için, name'i kaydedelim ve dummy handle dönelim
         println!("KResource: '{}' kaydediliyor (Yer Tutucu)", name);

        // Gerçekte burada yeni bir KHandle oluşturulur ve bu handle ile provider arasında
        // bir eşleme (mapping) kaydedilir.
         let dummy_handle = KHandle(1); // Dummy handle değeri

        // ProviderEntry'yi oluşturup listeye ekleyelim (varsaalloc kullanılırsa)
        // let entry = ProviderEntry { name, provider: Box::new(provider), /* ... */ };
        // unsafe { REGISTERED_PROVIDERS.push(entry); } // Provider'ın kendisi Box'a alınmalıydı

        // Eğer provider &'static dyn ResourceProvider ise ve bunu saklarsak:
        struct StaticProviderEntry { name: &'static str, provider: &'static dyn ResourceProvider, handle: KHandle }
        static mut STATIC_REGISTERED_PROVIDERS: Vec<StaticProviderEntry> = Vec::new(); // alloc gerektirir
        static NEXT_HANDLE_VALUE: super::ksync::Spinlock = super::ksync::Spinlock::new();
        static mut NEXT_HANDLE_COUNTER: u64 = 1;

         let _handle_guard = NEXT_HANDLE_VALUE.lock();
         let handle_value = unsafe {
             let h = NEXT_HANDLE_COUNTER;
             NEXT_HANDLE_COUNTER += 1;
             h
         };
         let new_handle = KHandle(handle_value);

         unsafe {
             STATIC_REGISTERED_PROVIDERS.push(StaticProviderEntry {
                 name,
                 provider,
                 handle: new_handle,
             });
         }

         Ok(new_handle) // Kaydedilen handle'ı döndür
    }


    // Diğer kresource fonksiyonları yer tutucu
     pub fn lookup_provider_by_name(name: &str) -> Result<&'static dyn ResourceProvider, KError> {
         let _guard = PROVIDER_REGISTRY_LOCK.lock();
         unsafe {
             STATIC_REGISTERED_PROVIDERS.iter()
                 .find(|entry| entry.name == name)
                 .map(|entry| entry.provider)
                 .ok_or(KError::NotFound)
         }
     }

     pub fn get_provider_by_handle(handle: &KHandle) -> Result<&'static dyn ResourceProvider, KError> {
         let _guard = PROVIDER_REGISTRY_LOCK.lock();
         unsafe {
             STATIC_REGISTERED_PROVIDERS.iter()
                 .find(|entry| entry.handle == *handle)
                 .map(|entry| entry.provider)
                 .ok_or(KError::BadHandle)
         }
     }

     pub fn issue_handle(_provider: &'static dyn ResourceProvider, mode: u32) -> KHandle {
          // Bu fonksiyon register_provider içinde handle ataması yapıyorsa burada boş olabilir
          // Veya farklı handle tipleri için genel bir issue_handle olabilir.
          // secureenclave manager bir kaynak sağlayıcı (provider) olarak kaydedildiğinde bir handle alır.
          // Belki bu handle, secure_enclave kaynağının kendisini temsil eder.
          // İçerdeki individual enclaves için handle vermek farklı bir konu olabilir.
          // Karnal64 API'sında resource_acquire handle döndürüyor. Bu handle'ı bir provider'a eşliyoruz.
          // Bu handle, register_provider tarafından dönülen handle ile aynı mekanizmadan gelmeli.
          // Şu anki static registry modelinde handle register anında veriliyor.
          // resource_acquire bu handle'ı bulup dönmeli.

          // Bu fonksiyon sanırım resource_acquire'ın dahili helper'ı olmalıydı.
          // Eğer öyleyse, provider lookup yapıldıktan sonra handle burada üretilir ve kaydedilir.
          // resource_acquire'ın içindeki yer tutucu kresource::issue_handle çağrısına karşılık gelir.
          // Static registry modelimizde handle, register anında oluşuyor ve isme veya handle değerine göre bulunuyor.
          // issue_handle'ın rolü net değil bu modelde. Belki HandleManager'ın içindedir?

          // Şimdilik dummy handle dönelim
          KHandle(0) // Dummy handle
     }

     pub fn handle_has_permission(handle: &KHandle, mode: u32) -> bool {
         // TODO: Handle ile ilişkili izinleri kontrol et
         true // Yer tutucu: Her zaman izin ver
     }

     pub fn release_handle(handle_value: u64) -> Result<(), KError> {
         let _guard = PROVIDER_REGISTRY_LOCK.lock();
         unsafe {
             if let Some(index) = STATIC_REGISTERED_PROVIDERS.iter().position(|entry| entry.handle.0 == handle_value) {
                 // TODO: Handle ile ilişkili kaynakları (varsa) serbest bırak
                 // SecureEnclaveManager kaynağının handle'ı serbest bırakıldığında ne olur?
                 // Yönetici nesnesinin kendisi static olduğu için yok edilmez, sadece handle geçersiz olur.
                 STATIC_REGISTERED_PROVIDERS.swap_remove(index);
                  println!("KResource: Handle {} serbest bırakıldı.", handle_value);
                 Ok(())
             } else {
                 Err(KError::BadHandle)
             }
         }
     }

     // KResourceStatus ve KseekFrom yer tutucuları
     pub struct KResourceStatus;
     pub enum KseekFrom { Start, Current, End }

     // Dummy ResourceProvider implementasyonu (Karnal64 init içinde kullanılmıştı)
     pub mod implementations {
         use super::*; // kresource scope'undaki tipleri kullan
         use alloc::boxed::Box;

         pub struct DummyConsole;

         impl ResourceProvider for DummyConsole {
             fn read(&self, buffer: &mut [u8], offset: u64) -> Result<usize, KError> { Err(KError::NotSupported) }
             fn write(&self, buffer: &[u8], offset: u64) -> Result<usize, KError> {
                 // Basitçe ekrana yaz (varsayımsal çekirdek print!)
                  let s = core::str::from_utf8(buffer).unwrap_or("<geçersiz utf8>");
                  println!("DummyConsole: {}", s);
                 Ok(buffer.len())
             }
             fn control(&self, request: u64, arg: u64) -> Result<i64, KError> { Err(KError::NotSupported) }
             fn seek(&self, position: KseekFrom) -> Result<u64, KError> { Err(KError::NotSupported) }
             fn get_status(&self) -> Result<KResourceStatus, KError> { Err(KError::NotSupported) }
         }
     }
}

// Diğer modüller için temel yer tutucular

mod ktask {
    use super::{KError, KTaskId};

    pub fn init_manager() { println!("KTask: Yönetici Başlatılıyor..."); }
    pub fn create_enclave_task(enclave_id: super::EnclaveId) -> Result<KTaskId, KError> {
         println!("KTask: Enclave {:?} için görev oluşturuluyor...", enclave_id);
        // TODO: Gerçek görev oluşturma mantığı
         Ok(KTaskId(100)) // Dummy ID
    }
    // TODO: task_spawn, task_exit, get_current_task_id vb. fonksiyonlar
    pub fn get_current_task_id() -> Result<KTaskId, KError> { Ok(KTaskId(core::process::id() as u64)) } // core::process::id() yer tutucu
    pub fn task_sleep(ms: u64) -> Result<(), KError> {
         println!("KTask: Görev {}ms uyutuluyor...", ms); // Yer tutucu
         // TODO: Zamanlayıcı ile entegrasyon
         Ok(())
    }
     pub fn yield_now() -> Result<(), KError> {
          println!("KTask: Görev CPU'yu bırakıyor..."); // Yer tutucu
          // TODO: Zamanlayıcı yield işlemi
          Ok(())
     }
}

mod kmemory {
     use super::KError;

     pub fn init_manager() { println!("KMemory: Yönetici Başlatılıyor..."); }
     // TODO: Kullanıcı/enclave belleği tahsis/serbest bırakma, haritalama fonksiyonları
     pub fn allocate_user_memory(size: usize) -> Result<*mut u8, KError> {
          println!("KMemory: {} byte kullanıcı belleği tahsis ediliyor...", size);
         // TODO: Gerçek bellek tahsisi
         Err(KError::OutOfMemory) // Yer tutucu
     }
     pub fn free_user_memory(ptr: *mut u8, size: usize) -> Result<(), KError> {
          println!("KMemory: Kullanıcı belleği serbest bırakılıyor ({:?}, {} byte)...", ptr, size);
          // TODO: Gerçek bellek serbest bırakma
         Ok(()) // Yer tutucu
     }
     pub fn map_shared(_handle: u64, _offset: usize, _size: usize) -> Result<*mut u8, KError> { Err(KError::NotSupported) } // Yer tutucu
}

 mod kmessaging {
      use super::KError;

      pub fn init_manager() { println!("KMessaging: Yönetici Başlatılıyor..."); }
      // TODO: Mesaj gönderme/alma fonksiyonları
      pub fn send(_target_handle: u64, _buffer_ptr: *const u8, _buffer_len: usize) -> Result<(), KError> { Err(KError::NotSupported) } // Yer tutucu
      pub fn receive(_buffer_ptr: *mut u8, _buffer_len: usize) -> Result<usize, KError> { Err(KError::NoMessage) } // Yer tutucu
 }

 mod kkernel {
      use super::KError;

      pub fn init_manager() { println!("KKernel: Yönetici Başlatılıyor..."); }
      pub fn get_info(_req: u32) -> Result<u64, KError> { Ok(1) } // Yer tutucu
 }


// --- Dummy Allocator ---
// Box ve Vec kullanabilmek için çok temel bir global allocator tanımı.
// GERÇEK ÇEKİRDEKTE BURASI UYGUN BİR ALLOCATOR İLE DEĞİŞTİRİLMELİDİR.
extern crate alloc;
use core::alloc::{GlobalAlloc, Layout};

struct DummyKernelAllocator;

unsafe impl GlobalAlloc for DummyKernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
         //println!("ALLOC: size={}, align={}", layout.size(), layout.align());
        // Normalde burada fiziksel bellekten sayfa tahsis edilir,
        // sanal adrese eşlenir vb.
        // Bu sadece bir yer tutucu, ASLA GERÇEK KULLANILMAMALI.
        // Çok basit bir statik tampon üzerinden tahsis simülasyonu yapalım.
         static mut HEAP: [u8; 4096] = [0; 4096]; // 4KB heap
         static mut HEAP_NEXT: usize = 0;

         let mut current_heap_next = HEAP_NEXT;
         let padding = (layout.align() - (current_heap_next % layout.align())) % layout.align();
         let start = current_heap_next + padding;
         let end = start + layout.size();

         if end > HEAP.len() {
             core::ptr::null_mut() // Yetersiz bellek
         } else {
             HEAP_NEXT = end;
             HEAP.as_mut_ptr().add(start)
         }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
         println!("DEALLOC: ptr={:?}, size={}, align={}", ptr, layout.size(), layout.align());
        // Gerçekte belleği serbest bırakır.
        // Dummy allocator'da serbest bırakma implemente edilmemiştir.
        // Bu allocator sadece test/basit kullanım içindir.
    }
}

#[global_allocator]
static ALLOCATOR: DummyKernelAllocator = DummyKernelAllocator;


// KError'ın i64'e dönüşümü için trait implementasyonu
impl From<KError> for i64 {
    fn from(err: KError) -> i64 {
        // KError enum'u zaten #[repr(i64)] olduğu için doğrudan dönüşüm güvenlidir.
        err as i64
    }
}
