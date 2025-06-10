#ifndef HARDWARE_SPECIFIC_H
#define HARDWARE_SPECIFIC_H

#include <stdint.h> // uint*_t
#include <stddef.h> // size_t

#ifdef __cplusplus
extern "C" {
#endif

// --- Temel Donanım Tipleri ---

// Fiziksel adres tipi (Hedef mimarinin adres genişliğine göre uint32_t veya uint64_t)
typedef uint64_t paddr_t;

// Sanal adres tipi (Genellikle fiziksel adres ile aynı genişlikte)
typedef uint64_t vaddr_t;

// Kesme numarası tipi (Mimarinin desteklediği kesme sayısına göre)
typedef uint32_t interrupt_id_t;

// --- Düşük Seviye Başlatma Fonksiyonları ---
// Bu fonksiyonların implementasyonları assembly veya mimariye özgü C kodlarında bulunur.

/**
 * Temel donanım bileşenlerini başlatır.
 * CPU özellikleri, MMU'nun erken başlatılması (minimal mapping),
 * temel saatler, ilk seri port vb.
 */
void low_level_hardware_init(void);

/**
 * Kesme denetleyicisini ve temel kesme işleme mekanizmasını başlatır.
 */
void low_level_interrupt_init(void);

/**
 * Sistem timer'ını başlatır.
 */
void low_level_timer_init(void);

// --- Temel Çıktı Fonksiyonu (Çekirdek Debug/Panik için) ---
// Karnal64 konsol sürücüsü hazır olmadan önce kullanılır.

/**
 * Düşük seviye donanım kullanarak bir karakteri çıktıya yazar (örn. seri port).
 * @param c Yazılacak karakter.
 */
void low_level_console_putc(char c);


// --- CPU Kontrol/Özel Fonksiyonlar ---
// Mimarinin özel komutları veya register erişimleri için.

/**
 * Tüm kesmeleri etkinleştirir.
 */
void low_level_interrupt_enable(void);

/**
 * Tüm kesmeleri devre dışı bırakır.
 */
void low_level_interrupt_disable(void);

/**
 * CPU'yu bekletir (genellikle bir kesme olana kadar).
 */
void low_level_cpu_halt(void);

// TODO: Mimariye özel register okuma/yazma fonksiyonları veya makroları
// TODO: Bağlam değiştirme (context switching) ile ilgili assembly/intrinsics arayüzleri

#ifdef __cplusplus
} // extern "C"
#endif

#endif // HARDWARE_SPECIFIC_H
