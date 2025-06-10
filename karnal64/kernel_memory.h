#ifndef KERNEL_MEMORY_H
#define KERNEL_MEMORY_H

#include "hardware_specific.h" // paddr_t, vaddr_t tanımları için
#include "karnal.h"            // kerror_t tanımı için (isteğe bağlı, kendi hata türü olabilir)

#include <stddef.h> // size_t
#include <stdint.h> // uint*_t

#ifdef __cplusplus
extern "C" {
#endif

// --- Bellek Sabitleri ve Yapıları ---

// Sayfa boyutu (Mimarinin ve tasarımın belirlediği, örn. 4KB = 4096)
#define KERNEL_PAGE_SIZE 4096

// Çekirdek sanal adres alanı başlangıcı (Tasarımın belirlediği)
#define KERNEL_VIRTUAL_BASE 0xFFFFFF0000000000ULL

// Bellek bölgesi tanımı (Genel bir yapı)
typedef struct MemoryRegion {
    vaddr_t start_vaddr; // Sanal başlangıç adresi
    size_t size;         // Boyut (byte)
    uint32_t flags;      // İzin bayrakları (okuma, yazma, yürütme vb.)
    // TODO: Diğer detaylar (fiziksel karşılığı, cache özellikleri vb.)
} MemoryRegion_t;


// --- Düşük Seviye Bellek Başlatma ---
// Bu fonksiyonun implementasyonu çekirdek başlatma kodunda bulunur.

/**
 * Çekirdek bellek yönetiminin erken aşamasını başlatır.
 * Fiziksel bellek ayırıcısını, sayfa tablolarını ve çekirdek adres alanını ayarlar.
 * low_level_hardware_init() çağrıldıktan sonra çağrılmalıdır.
 */
void low_level_memory_init(void);


// --- Fiziksel Bellek Yönetimi ---
// Çekirdek içindeki fiziksel sayfa/frame yönetimi için.

/**
 * Fiziksel bellekte boş bir sayfa (frame) tahsis eder.
 * @return Tahsis edilen fiziksel sayfanın adresi (paddr_t), hata durumunda özel bir değer (örn. 0 veya NULL_PADDR).
 */
paddr_t kmem_phys_alloc_frame(void);

/**
 * Tahsis edilmiş bir fiziksel sayfayı serbest bırakır.
 * @param frame_addr Serbest bırakılacak fiziksel sayfanın adresi.
 */
void kmem_phys_free_frame(paddr_t frame_addr);

// TODO: Çoklu frame tahsisi, belirli adrese yakın tahsis vb. fonksiyonlar.


// --- Sanal Bellek Yönetimi / Sayfa Tabloları ---
// Sanal adresleri fiziksel adreslere eşlemek ve adres alanlarını yönetmek için.
// Bu fonksiyonlar MMU donanımıyla etkileşime girer.

/**
 * Bir sanal adresi (sayfa) belirli bir fiziksel adrese (frame) eşler
 * mevcut adres alanında, belirli izinlerle.
 * @param vaddr Eşlenecek sanal adres (sayfa hizalı).
 * @param paddr Eşlenecek fiziksel adres (sayfa hizalı).
 * @param flags Sayfa izinleri (okuma, yazma, yürütme, kullanıcı/çekirdek erişimi vb.).
 * @return Başarı durumunda 0, hata durumunda negatif kerror_t (veya başka bir hata kodu).
 */
kerror_t kmem_virt_map_page(vaddr_t vaddr, paddr_t paddr, uint32_t flags);

/**
 * Bir sanal adres eşlemesini kaldırır.
 * @param vaddr Eşlemesi kaldırılacak sanal adres (sayfa hizalı).
 * @return Başarı durumunda 0, hata durumunda negatif kerror_t.
 */
kerror_t kmem_virt_unmap_page(vaddr_t vaddr);

/**
 * Yeni bir boş sanal adres alanı (sayfa tablosu kökü) oluşturur.
 * Genellikle yeni bir görev (task) başlatılırken kullanılır.
 * @return Yeni adres alanını temsil eden bir tanımlayıcı (örn. sayfa tablosu kökünün fiziksel adresi), hata durumunda özel bir değer.
 */
paddr_t kmem_virt_create_address_space(void); // Genellikle page table root PADDR döndürür

/**
 * Belirli bir sanal adres alanını siler.
 * @param address_space_id Silinecek adres alanının tanımlayıcısı.
 */
void kmem_virt_destroy_address_space(paddr_t address_space_id); // Adres alanı ID'si genellikle root PADDR'dır

/**
 * CPU'nun kullanacağı aktif sanal adres alanını değiştirir.
 * @param address_space_id Aktif hale getirilecek adres alanının tanımlayıcısı.
 */
void kmem_virt_activate_address_space(paddr_t address_space_id);


// TODO: Bölge (region) yönetimi, MTRR/PAT gibi cache kontrolü, bellek koruma mekanizmaları.


#ifdef __cplusplus
} // extern "C"
#endif

#endif // KERNEL_MEMORY_H
