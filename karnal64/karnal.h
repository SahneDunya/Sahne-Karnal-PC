#ifndef KARNAL_H
#define KARNAL_H

#include <stdint.h> // uint64_t, int64_t, uint32_t, int32_t
#include <stddef.h> // size_t

#ifdef __cplusplus
extern "C" {
#endif

// --- Dahili Karnal64 Tipleri ---
// Bunlar Rust'taki KError, KTaskId vb. tiplerinin C karşılığıdır.
// Kullanıcı alanındaki Sahne64 tipleriyle aynı temelde (u64) olsa da,
// anlamsal olarak çekirdek içindeki nesneleri temsil ederler.

// Dahili çekirdek hata kodu. Rust'taki KError enum'una karşılık gelir.
// Sistem çağrısı işleyicisi, KError değerlerini kullanıcı alanına dönecek
// negatif i64 değerlerine dönüştürür. Buradaki değerler bu negatif i64 kodlarıdır.
typedef int64_t kerror_t;

// Dahili çekirdek hata kodu sabitleri (negatif değerler olmalıdır)
#define KSUCCESS                0  // Başarı (bu API fonksiyonları i64 döndürdüğünde, başarı durumunda >0 veya =0 döner)
#define KERROR_PERMISSION_DENIED -1
#define KERROR_NOT_FOUND         -2
#define KERROR_INVALID_ARGUMENT  -3
#define KERROR_INTERRUPTED       -4
#define KERROR_BAD_HANDLE        -9
#define KERROR_BUSY             -11
#define KERROR_OUT_OF_MEMORY    -12
#define KERROR_BAD_ADDRESS      -14
#define KERROR_ALREADY_EXISTS   -17
#define KERROR_NOT_SUPPORTED    -38
#define KERROR_NO_MESSAGE       -61
#define KERROR_INTERNAL_ERROR  -255
// ... Rust KError enum'undaki diğer hatalar buraya eklenmeli ...


// Dahili çekirdek Görev Tanımlayıcısı. Rust KTaskId'ye karşılık gelir.
typedef uint64_t ktid_t;

// Dahili çekirdek İş Parçacığı Tanımlayıcısı. Rust KThreadId'ye karşılık gelir.
typedef uint64_t kthread_id_t; // Varsayım: Thread ID'ler de u64

// Dahili çekirdek Handle'ı. Rust KHandle'a karşılık gelir.
// Kullanıcı alanından gelen ham handle değeridir, çekirdek içinde anlamlıdır.
typedef uint64_t khandle_t;


// --- Karnal64 API Fonksiyonları (Çekirdek İçi Çağrılar İçin) ---
// Bu fonksiyonlar, çekirdeğin sistem çağrısı işleyicisi veya diğer çekirdek
// modülleri tarafından çağrılır. Rust'taki `pub fn` fonksiyonlarının C imzalarıdır.
// Rust'taki `Result<T, KError>` dönüş tipi, C'de tek bir `int64_t` olarak temsil edilir:
// - Başarı durumunda, T değerinin i64'e dönüştürülmüş hali (genellikle >= 0) döner.
// - Hata durumunda, KError değerinin i64'e dönüştürülmüş hali (negatif) döner.

/**
 * Karnal64 çekirdek API'sını başlatır. Çekirdek boot sırasında çağrılmalıdır.
 */
void karnal_init(void);


// --- Bellek Yönetimi ---

/**
 * Kullanıcı alanı için bellek tahsis eder.
 * @param size Tahsis edilecek bellek boyutu.
 * @return Başarı durumunda tahsis edilen bellek adresinin u64'e dönüştürülmüş hali (>=0), hata durumunda negatif kerror_t döner.
 */
int64_t karnal_memory_allocate(size_t size);

/**
 * Kullanıcı alanı için daha önce tahsis edilmiş belleği serbest bırakır.
 * @param ptr Serbest bırakılacak bellek adresinin u64'e dönüştürülmüş hali.
 * @param size Bellek boyutu.
 * @return Başarı durumunda 0, hata durumunda negatif kerror_t döner.
 */
int64_t karnal_memory_release(uint64_t ptr, size_t size); // ptr artık u64 olarak alınıyor, doğrulama içeride yapılır


// --- Görev (Task) Yönetimi ---

/**
 * Yeni bir görev (task) başlatır.
 * @param code_handle_value Çalıştırılabilir kod kaynağının handle değeri.
 * @param args_ptr Kullanıcı alanındaki argüman verisi pointer'ı.
 * @param args_len Argüman verisi uzunluğu.
 * @return Başarı durumunda yeni görevin ktid_t değerinin i64'e dönüştürülmüş hali (>=0), hata durumunda negatif kerror_t döner.
 */
int64_t karnal_task_spawn(khandle_t code_handle_value, const uint8_t* args_ptr, size_t args_len); // Pointerlar kullanıcı adresinde, içeride doğrulanmalı

/**
 * Mevcut görevi belirtilen çıkış koduyla sonlandırır. Geri dönmez.
 * @param code Çıkış kodu.
 */
void karnal_task_exit(int32_t code) __attribute__((noreturn));


/**
 * Mevcut görevin Task ID'sini alır.
 * @return Başarı durumunda mevcut görevin ktid_t değerinin i64'e dönüştürülmüş hali (>=0), hata durumunda negatif kerror_t döner.
 */
int64_t karnal_task_current_id(void); // Karnal64 fns are called by handler, they return value directly


/**
 * Mevcut görevi belirtilen milisaniye kadar uyutur.
 * @param milliseconds Uyutulacak süre (milisaniye).
 * @return Başarı durumunda 0, hata durumunda negatif kerror_t döner.
 */
int64_t karnal_task_sleep(uint64_t milliseconds);

/**
 * Yeni bir iş parçacığı (thread) oluşturur.
 * @param entry_point Yeni iş parçacığının başlangıç fonksiyon adresinin u64'e dönüştürülmüş hali.
 * @param stack_size Yeni iş parçacığı için ayrılacak yığın boyutu.
 * @param arg Başlangıç fonksiyonuna geçirilecek argümanın u64'e dönüştürülmüş hali.
 * @return Başarı durumunda yeni iş parçacığının kthread_id_t değerinin i64'e dönüştürülmüş hali (>=0), hata durumunda negatif kerror_t döner.
 */
int64_t karnal_thread_create(uint64_t entry_point, size_t stack_size, uint64_t arg);

/**
 * Mevcut iş parçacığını belirtilen çıkış koduyla sonlandırır. Geri dönmez.
 * @param code Çıkış kodu.
 */
void karnal_thread_exit(int32_t code) __attribute__((noreturn));

/**
 * CPU'yu gönüllü olarak başka bir çalıştırılabilir göreve/iş parçacığına bırakır.
 * @return Başarı durumunda 0, hata durumunda negatif kerror_t döner.
 */
int64_t karnal_task_yield(void);


// --- Kaynak Yönetimi ---

/**
 * Belirtilen ID'ye sahip bir kaynağa erişim handle'ı edinir.
 * @param resource_id_ptr Kullanıcı alanındaki kaynak ID pointer'ı (uint8_t dizisi).
 * @param resource_id_len Kaynak ID uzunluğu.
 * @param mode Talep edilen erişim modları bayrakları.
 * @return Başarı durumunda edinilen khandle_t değerinin i64'e dönüştürülmüş hali (>=0), hata durumunda negatif kerror_t döner.
 */
int64_t karnal_resource_acquire(const uint8_t* resource_id_ptr, size_t resource_id_len, uint32_t mode); // Pointerlar kullanıcı adresinde, içeride doğrulanmalı


/**
 * Belirtilen handle ile temsil edilen kaynaktan veri okur.
 * @param handle_value Kaynak handle değeri.
 * @param user_buffer_ptr Kullanıcı alanındaki okuma tamponu pointer'ı.
 * @param user_buffer_len Kullanıcı tamponunun uzunluğu.
 * @return Başarı durumunda okunan byte sayısı (size_t olarak, i64'e dönüştürülür >=0), hata durumunda negatif kerror_t döner.
 */
int64_t karnal_resource_read(khandle_t handle_value, uint8_t* user_buffer_ptr, size_t user_buffer_len); // Pointerlar kullanıcı adresinde, içeride doğrulanmalı

/**
 * Belirtilen handle ile temsil edilen kaynağa veri yazar.
 * @param handle_value Kaynak handle değeri.
 * @param user_buffer_ptr Kullanıcı alanındaki yazma tamponu pointer'ı.
 * @param user_buffer_len Kullanıcı tamponunun uzunluğu.
 * @return Başarı durumunda yazılan byte sayısı (size_t olarak, i64'e dönüştürülür >=0), hata durumunda negatif kerror_t döner.
 */
int64_t karnal_resource_write(khandle_t handle_value, const uint8_t* user_buffer_ptr, size_t user_buffer_len); // Pointerlar kullanıcı adresinde, içeride doğrulanmalı

/**
 * Belirtilen handle'ı serbest bırakır.
 * @param handle_value Serbest bırakılacak handle değeri.
 * @return Başarı durumunda 0, hata durumunda negatif kerror_t döner.
 */
int64_t karnal_resource_release(khandle_t handle_value);

/**
 * Kaynağa özel kontrol komutu gönderir.
 * @param handle_value Kaynak handle değeri.
 * @param request Komut kodu.
 * @param arg Komut argümanı.
 * @return Başarı durumunda komutun sonucu (i64), hata durumunda negatif kerror_t döner.
 */
int64_t karnal_resource_control(khandle_t handle_value, uint64_t request, uint64_t arg);


// --- Çekirdek Bilgisi ---

/**
 * Çekirdekten belirli bir bilgiyi alır.
 * @param info_type Talep edilen bilgi türü.
 * @return Başarı durumunda bilgi değeri (uint64_t olarak, i64'e dönüştürülür >=0), hata durumunda negatif kerror_t döner.
 */
int64_t karnal_kernel_get_info(uint32_t info_type);

/**
 * Sistem saatini (örneğin, epoch'tan beri geçen nanosaniye) alır.
 * @return Başarı durumunda sistem zamanı değeri (uint64_t olarak, i64'e dönüştürülür >=0), hata durumunda negatif kerror_t döner.
 */
int64_t karnal_kernel_get_time(void);


// --- Senkronizasyon ---

/**
 * Yeni bir kilit (Lock) kaynağı oluşturur.
 * @return Başarı durumunda kilit handle'ının i64'e dönüştürülmüş hali (>=0), hata durumunda negatif kerror_t döner.
 */
int64_t karnal_sync_lock_create(void);

/**
 * Kilidi almaya çalışır. Başka bir görev/iş parçacığı tutuyorsa bloklar.
 * @param handle_value Kilit handle değeri.
 * @return Başarı durumunda 0, hata durumunda negatif kerror_t döner.
 */
int64_t karnal_sync_lock_acquire(khandle_t handle_value);

/**
 * Kilidi serbest bırakır. Çağıranın kilidi tutuyor olması gerekir.
 * @param handle_value Kilit handle değeri.
 * @return Başarı durumunda 0, hata durumunda negatif kerror_t döner.
 */
int64_t karnal_sync_lock_release(khandle_t handle_value);


// --- Mesajlaşma / IPC ---

/**
 * Hedef göreve bir mesaj gönderir.
 * @param target_task_id_value Hedef görevin Task ID değeri.
 * @param message_ptr Kullanıcı alanındaki mesaj verisi pointer'ı.
 * @param message_len Mesaj verisi uzunluğu.
 * @return Başarı durumunda 0, hata durumunda negatif kerror_t döner.
 */
int64_t karnal_messaging_send(ktid_t target_task_id_value, const uint8_t* message_ptr, size_t message_len); // Pointerlar kullanıcı adresinde, içeride doğrulanmalı

/**
 * Mevcut görev için gelen bir mesajı alır.
 * @param user_buffer_ptr Kullanıcı alanındaki tampon pointer'ı.
 * @param user_buffer_len Kullanıcı tamponunun uzunluğu.
 * @return Başarı durumunda alınan mesajın boyutu (size_t olarak, i64'e dönüştürülür >=0), mesaj yoksa KERROR_NO_MESSAGE veya başka bir negatif kerror_t döner.
 */
int64_t karnal_messaging_receive(uint8_t* user_buffer_ptr, size_t user_buffer_len); // Pointer kullanıcı adresinde, içeride doğrulanmalı


// --- Çekirdek Bileşenleri Kayıt API'sı (Örnek) ---
// Bu kısım, başka çekirdek modüllerinin (sürücüler, fs vb.)
// Karnal64'ün yöneticilerine kendilerini kaydetmek için kullanacağı API'dır.
// Rust'taki `register_provider` gibi fonksiyonların C arayüzü burada tanımlanır.
// Rust'taki traitleri C'ye bind etmek karmaşık olabilir (genellikle C fonksiyon pointerları kullanılır).
// Basit bir ResourceProvider kayıt fonksiyonu örneği:

// Çekirdek içindeki C kodları tarafından implemente edilecek ResourceProvider fonksiyon işaretçileri
typedef struct KarnalResourceProviderC {
    // provider_data: Implementasyonun kendi durumunu tuttuğu pointer (Rust tarafında Box<dyn Trait> karşılığı)
    int64_t (*read_fn)(void* provider_data, uint8_t* buffer, size_t size, uint64_t offset);
    int64_t (*write_fn)(void* provider_data, const uint8_t* buffer, size_t size, uint64_t offset);
    int64_t (*control_fn)(void* provider_data, uint64_t request, uint64_t arg);
    // ... diğer trait fonksiyonları ...
    void* provider_data;
} KarnalResourceProviderC_t;

/**
 * Çekirdek içindeki bir C modülü tarafından implemente edilmiş ResourceProvider'ı kaydeder.
 * Rust Karnal64 katmanı bu C fonksiyon işaretçilerini bir dyn ResourceProvider objesine sarmalayacaktır.
 * @param id_ptr Kaydedilecek kaynağın çekirdek içi ID pointer'ı (genellikle statik string).
 * @param id_len Kaynak ID uzunluğu.
 * @param provider_c_fns ResourceProvider fonksiyon işaretçilerini içeren yapı.
 * @return Başarı durumunda kaynağın dahili khandle_t değerinin i64'e dönüştürülmüş hali (>=0), hata durumunda negatif kerror_t döner.
 */
int64_t karnal_resource_register_c_provider(const uint8_t* id_ptr, size_t id_len, const KarnalResourceProviderC_t* provider_c_fns);


// TODO: Diğer yöneticiler için kayıt/etkileşim fonksiyonları
// Örneğin, yeni bellek alanı türü kaydetme, yeni IPC mekanizması kaydetme vb.


#ifdef __cplusplus
} // extern "C"
#endif

#endif // KARNAL_H
