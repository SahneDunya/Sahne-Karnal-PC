#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz, kernel alanında çalışıyoruz

// Geliştirme sırasında kullanılmayan kod veya argümanlar için izinler
#![allow(dead_code)]
#![allow(unused_variables)]

use riscv::register::{
    mcause::{self, Exception, Trap}, // mcause: Trap'in nedenini oku
    mepc,   // mepc: Trap'in olduğu komutun adresi
    mtval,  // mtval: Trap ile ilgili ek bilgi (örn. hatalı adres)
    mtvec,  // mtvec: Trap işleyicisinin adresi
    satp,   // satp: Sayfa tablosu taban adresi ve ASID
    mstatus, // mstatus: Makine durumu, önceki ayrıcalık seviyesi vb.
};
use core::arch::asm; // Inline assembly için

// Karnal64 API'sını kullanabilmek için dışarıdan 'karnal64' crate'ini/modülünü almamız gerekiyor.
// main.rs veya lib.rs dosyanızda 'mod karnal64;' veya 'extern crate karnal64;'
// şeklinde tanımlanmış ve Karnal64 koduyla build edilmiş olmalıdır.
// Bu kod, karnal64 API'sındaki pub fonksiyonları çağıracaktır.
#[path = "../karnal64/karnal64.rs"] // Örnek yol, projenizin yapısına göre ayarlayın
mod karnal64;

// Sistem çağrısı numaraları ve diğer mimariye özgü sabitler (Sahne64 ile uyumlu olmalı)
// Normalde bu sabitler Sahne64 tarafında veya ortak bir dosyada tanımlanabilir.
// Şimdilik burada yer tutucu olarak tanımlayalım:
const SYSCALL_MEMORY_ALLOCATE: u64 = 1;
const SYSCALL_MEMORY_RELEASE: u64 = 2;
const SYSCALL_TASK_SPAWN: u64 = 3;
const SYSCALL_TASK_EXIT: u64 = 4;
const SYSCALL_RESOURCE_ACQUIRE: u64 = 5;
const SYSCALL_RESOURCE_READ: u64 = 6;
const SYSCALL_RESOURCE_WRITE: u64 = 7;
const SYSCALL_RESOURCE_RELEASE: u64 = 8;
// ... diğer SYSCALL_ numaraları ...
const SYSCALL_GET_TASK_ID: u64 = 9;
const SYSCALL_TASK_SLEEP: u64 = 10;
const SYSCALL_LOCK_CREATE: u64 = 11;
const SYSCALL_LOCK_ACQUIRE: u64 = 12;
const SYSCALL_LOCK_RELEASE: u64 = 13;
const SYSCALL_MESSAGE_SEND: u64 = 14;
const SYSCALL_MESSAGE_RECEIVE: u64 = 15;
const SYSCALL_GET_KERNEL_INFO: u64 = 16;
const SYSCALL_TASK_YIELD: u64 = 17;


// Sistem çağrısı argümanlarının ve dönüş değerlerinin tutulduğu register'ları
// temsil eden yapı. Trap anında bu registerlar kaydedilir/yüklenir.
// RISC-V RV64 calling convention'a göre:
// a0-a7: Fonksiyon argümanları
// a0-a1: Dönüş değerleri
// t0-t6: Geçici registerlar (fonksiyon çağrıları arasında korunmaz)
// s0-s11: Kayıtlı registerlar (çağıran tarafından korunmalı)
// sp: Stack pointer
// ra: Return address
// gp: Global pointer
// tp: Thread pointer
// ... diğerleri
#[repr(C)] // C ABI uyumluluğu
#[derive(Debug, Clone, Copy)]
pub struct TrapFrame {
    // Kayıtlı registerlar (s0-s11)
    s0: u664, s1: u64,
    s2: u64, s3: u64, s4: u64, s5: u64, s6: u64, s7: u64, s8: u64, s9: u64, s10: u64, s11: u64,
    // Fonksiyon argümanları/dönüş değerleri (a0-a7) ve geçici registerlar (t0-t6)
    a0: u64, a1: u64, a2: u64, a3: u64, a4: u64, a5: u64, a6: u64, a7: u64,
    t0: u64, t1: u64, t2: u64, t3: u64, t4: u64, t5: u64, t6: u64,
    // Temel registerlar
    gp: u64, tp: u64, sp: u64, ra: u64,
    // Trap/Exception ile ilgili registerlar
    mepc: u64, // veya sepc S-mode için
    mstatus: u64, // veya sstatus S-mode için
    // mtval/stval trapframe içinde genellikle tutulmaz, trap handler içinde okunur.
}


/// RISC-V'de trap vektörünü ayarlar (genellikle mtvec/stvec register'ı).
/// Bu, bir trap oluştuğunda işlemcinin hangi adrese (fonksiyona) sıçrayacağını belirler.
/// `trap_vector`: Trap işleyicisinin bellek adresi.
pub fn set_trap_vector(trap_vector: usize) {
    unsafe {
        // mtvec register'ına trap işleyicisinin adresini yaz.
        // MODE: Vectored (1) veya Direct (0). Genellikle Direct kullanılır.
        mtvec::write(trap_vector, mtvec::TrapMode::Direct);
        // Eğer Supervisor mode kullanılıyorsa stvec::write() kullanılır.
    }
}

/// Temel bellek yönetim birimi (MMU) ayarlarını yapar.
/// Bu genellikle çekirdek için sayfa tablolarının kurulmasını ve MMU'nun aktifleştirilmesini içerir.
/// Bu fonksiyon, çekirdek başlatılırken çağrılmalıdır.
pub fn mmu_init() {
    // TODO: Çekirdek için temel sayfa tablolarını oluştur (örn. kimlik eşleme veya offset eşleme).
    // TODO: Satp register'ını çekirdek sayfa tablosunun adresi ve uygun moda ayarla.
    // satp register formatı SV39 için: MODE(4) | ASID(16) | PPN(44)
    // ASID genellikle çoklu görevde kullanılır, başlangıçta 0 olabilir.
    // PPN: Fiziksel sayfa numarası (sayfa tablosunun kökünün adresi >> 12)

    // Örnek (yer tutucu): satp'yi ayarlayarak MMU'yu etkinleştir.
    // Satp'ye yazmak, MMU'yu belirli bir sayfa tablosuyla etkinleştirir.
    // Buradaki 0 değeri ve satp::Mode::Sv39 sadece bir örnektir.
    // Gerçek PPN değeri ve ASID, bellek yöneticinizden gelmelidir.
    let kernel_page_table_ppn = 0; // Gerçek PPN değeri buraya gelecek
    let kernel_asid = 0; // ASID değeri buraya gelecek

    unsafe {
        satp::write(
            satp::Mode::Sv39, // Kullanılan sanal adresleme modu (örn. Sv39)
            kernel_asid,      // ASID (Address Space ID)
            kernel_page_table_ppn // Sayfa tablosunun kökünün fiziksel adresinin PPN'i
        );
        // TLB'yi temizle (gerekli olmayabilir ama güvenli bir adımdır)
         tlbflush(); // riscv crate'inde veya inline asm ile
    }

    karnal64::kmemory::init_manager(); // Karnal64 bellek yöneticisini başlat
    println!("RISC-V MMU ayarlandı ve Bellek Yöneticisi Başlatıldı (Yer Tutucu)"); // Çekirdek içi print! gerekli
}

/// Görevler arası bağlam değiştirirken MMU bağlamını değiştirir.
/// satp register'ını yeni görevin sayfa tablosuna işaret edecek şekilde günceller.
pub fn switch_task_mmu(new_satp_value: usize) {
    unsafe {
        satp::write(
            satp::Mode::Sv39, // Aynı mod kullanılmalı
            (new_satp_value >> 44) as u16, // ASID (eğer kullanılıyorsa)
            new_satp_value & 0xFFFFFFFFFFF // PPN
        );
        // TLB'yi temizle (ya tamamen ya da sadece ASID'ye özel)
         tlbflush(); // Veya sfence.vma
    }
}


/// RISC-V mimarisinden gelen tüm trap'leri (sistem çağrıları, kesmeler, istisnalar) işleyen fonksiyondur.
/// Bu fonksiyon, set_trap_vector ile ayarlanan adreste bulunmalıdır.
/// Genellikle assembly dilinde bir giriş noktası burayı çağırır ve TrapFrame'i argüman olarak iletir.
#[no_mangle] // Assembly'den çağrılabilmesi için isim düzenlemesi yapılmaz
extern "C" fn trap_handler(trap_frame: &mut TrapFrame) {
    // Trap'in nedenini ve türünü oku
    let cause = mcause::read().cause();
    let epc = mepc::read();
    let tval = mtval::read(); // Trap ile ilgili adres veya değer (örn. hatalı bellek adresi)

    match cause {
        // Kesmeler (Interrupts)
        Trap::Interrupt(_) => {
            // TODO: Kesmeleri işlemek için uygun kesme denetleyicisine (PLIC/CLINT) yönlendir.
            // TODO: Zamanlayıcı kesmeleri burada işlenerek görev zamanlaması tetiklenebilir.
            println!("RISC-V Interrupt: {:?}", cause); // Hata ayıklama çıktısı
            // Kesme sonrası mepc'yi artırmaya gerek YOK. Kesme aynı yerden devam eder.
        }
        // İstisnalar (Exceptions)
        Trap::Exception(Exception::UserEnvCall) => { // Kullanıcı alanından gelen sistem çağrısı (ecall)
            // a7 register'ı sistem çağrısı numarasını tutar (RISC-V ABI kuralı)
            let syscall_number = trap_frame.a7;
            // a0-a5 register'ları sistem çağrısı argümanlarını tutar
            let arg1 = trap_frame.a0;
            let arg2 = trap_frame.a1;
            let arg3 = trap_frame.a2;
            let arg4 = trap_frame.a3;
            let arg5 = trap_frame.a4; // Karnal64 handle_syscall 5 argüman alıyor

            // Güvenlik Kontrolü: Kullanıcı tarafından sağlanan pointer argümanlarını doğrula!
            // Bu, kullanıcının adres alanında geçerli ve erişilebilir (okunabilir/yazılabilir)
            // olduklarını kontrol etmeyi içerir. MMU'nun ve mevcut görev sayfa tablosunun
            // kullanıldığı mimariye özgü bir işlemdir.
            // TODO: Pointer doğrulaması (örn. `kmemory::validate_user_ptr` gibi bir fonksiyon kullanarak)
             if syscall_number == SYSCALL_RESOURCE_ACQUIRE {
                 let id_ptr = arg1 as *const u8;
                 let id_len = arg2 as usize;
                 if karnal64::kmemory::validate_user_read_ptr(id_ptr, id_len).is_err() {
                      let k_err = karnal64::KError::BadAddress;
                      trap_frame.a0 = k_err as u64; // Hata kodunu a0'a yaz
                      trap_frame.mepc += 4; // ecall komutunu atla
                      return;
                 }
             }
            // ... diğer pointer argümanları için doğrulama ...


            // Karnal64 API'sının genel sistem çağrısı işleyicisini çağır.
            // Bu fonksiyon, kullanıcı alanından gelen argümanları (zaten doğrulandığı varsayılır)
            // alıp ilgili çekirdek yöneticisine yönlendirir.
            let syscall_result = karnal64::handle_syscall(
                syscall_number,
                arg1, arg2, arg3, arg4, arg5 // Karnal64 sadece 5 argüman kullanıyor gibi duruyor
            );

            // Karnal64'ten dönen sonucu (i64) kullanıcı alanına dönüş değeri register'ına (a0) yaz.
            // Hata durumunda negatif KError değeri, başarı durumunda pozitif/sıfır değer a0'a yazılır.
            trap_frame.a0 = syscall_result as u64;

            // Sistem çağrısını tetikleyen 'ecall' komutunu atlamak için mepc'yi ilerlet.
            // RISC-V'de ecall komutu genellikle 4 byte uzunluğundadır.
            trap_frame.mepc += 4; // İstisnadan sonra döneceği adresi ayarla

        }
        // Bellek Erişim Hataları (Page Faults)
        Exception::InstructionPageFault | Exception::LoadPageFault | Exception::StorePageFault => {
            // Geçersiz bellek erişimi olduğunda buraya düşeriz.
            // mtval register'ı, erişilmeye çalışılan hatalı sanal adresi tutar.
            let faulting_address = tval;
            let fault_cause = cause; // Hatanın türü (okuma, yazma, çalıştırma)

            println!("RISC-V Page Fault! Cause: {:?}, Address: {:#x}", fault_cause, faulting_address);

            // TODO: Bellek yöneticisine (kmemory) sayfa hatasını bildir.
            // Bellek yöneticisi bu hatanın geçerli bir durum (örn. yığın genişletme, Copy-on-Write)
            // olup olmadığını kontrol edip ilgili sayfayı mapleyebilir veya geçerli değilse görevi sonlandırabilir.
            let handle_result = karnal64::kmemory::handle_page_fault(faulting_address, fault_cause);

            match handle_result {
                Ok(_) => {
                    // Hata başarıyla ele alındı (örn. sayfa maplendi). Trap'in olduğu komut yeniden çalıştırılabilir.
                    // mepc'yi değiştirmeye gerek yok.
                }
                Err(k_err) => {
                    // Hata ele alınamadı (örn. geçersiz erişim). Görevi sonlandır.
                    println!("Karnal64 Bellek Yöneticisi Sayfa Hatasını Çözemedi: {:?}", k_err);
                    // TODO: Mevcut görevi güvenli bir şekilde sonlandır (karnal64::ktask::exit_current_task gibi).
                    loop {} // Geçici olarak takıl
                }
            }
        }
        // Diğer İstisnalar
        _ => {
            // Bilinmeyen veya beklenmeyen istisnalar. Genellikle bu bir hatadır.
            println!("Beklenmeyen RISC-V İstisnası! Cause: {:?}, EPC: {:#x}, TVAL: {:#x}", cause, epc, tval);
            // TODO: Sistemi güvenli bir duruma getir veya hata raporla.
            loop {} // Sistem takıldı
        }
    }

    // Trap işleyicisinden dönmeden önce kaydedilen registerları (TrapFrame'den) yükle
    // ve mepc'deki adrese sıçra. Bu, genellikle assembly dilinde yapılır.
    // Aşağıdaki satır sadece kavramsal olarak ne olduğunu anlatır:
     restore_registers_from_trap_frame(trap_frame);
     return_from_trap(); // mret komutu (Machine Return) gibi
}

// TODO: Karnal64 API'sının beklediği placeholder fonksiyonları burada doğrudan implemente edemeyiz
// çünkü onlar karnal64.rs içinde yer almalı. Ancak, yukarıdaki kodda çağırdığımız
// `karnal64::handle_syscall` ve `karnal64::kmemory::handle_page_fault` gibi fonksiyonların
// Karnal64 modülü içinde var olduğunu ve pub olduklarını varsayıyoruz.

// Karnal64 modülündeki kmemory'e eklenmesi gereken örnek placeholder fonksiyon tanımı:

// karnal64/kmemory.rs içinde (burada değil!)
mod kmemory {
    use super::*;
    // ... diğer kmemory fonksiyonları ...

    // Sayfa hatalarını işlemek için bu fonksiyonun Karnal64 kmemory modülünde olması gerekir.
    pub fn handle_page_fault(faulting_address: usize, cause: mcause::Trap) -> Result<(), KError> {
        println!("Kmemory: Sayfa Hatası İşleniyor (Yer Tutucu). Adres: {:#x}, Neden: {:?}", faulting_address, cause);
        // TODO: Sayfa tablosunu kontrol et, gerekli sayfayı maple, izinleri doğrula vb.
        Err(KError::BadAddress) // Geçici olarak her zaman hata döndür
    }

    // Kullanıcı pointerlarını doğrulamak için bu fonksiyonun Karnal64 kmemory modülünde olması gerekir.
    // Bu fonksiyon, mevcut görevin sayfa tablosunu kullanarak adresin geçerli ve erişilebilir olduğunu kontrol eder.
    pub fn validate_user_read_ptr(ptr: *const u8, len: usize) -> Result<(), KError> {
        println!("Kmemory: Kullanıcı Okuma Pointerı Doğrulanıyor (Yer Tutucu). Ptr: {:?}, Len: {}", ptr, len);
        // TODO: ptr + len aralığının kullanıcı adres alanında geçerli ve okunabilir olduğunu MMU üzerinden doğrula.
        // Eğer 0 uzunluklu bir pointer ise genellikle geçerlidir (null check hariç).
        if ptr.is_null() && len > 0 {
             return Err(KError::BadAddress);
        }
        // TODO: Gerçek MMU kontrolü burada yapılacak
        Ok(()) // Geçici olarak her zaman başarı döndür (GERÇEK KERNELDE BUNU YAPMAYIN!)
    }

     pub fn validate_user_write_ptr(ptr: *mut u8, len: usize) -> Result<(), KError> {
        println!("Kmemory: Kullanıcı Yazma Pointerı Doğrulanıyor (Yer Tutucu). Ptr: {:?}, Len: {}", ptr, len);
        // TODO: ptr + len aralığının kullanıcı adres alanında geçerli ve yazılabilir olduğunu MMU üzerinden doğrula.
         if ptr.is_null() && len > 0 {
             return Err(KError::BadAddress);
         }
        // TODO: Gerçek MMU kontrolü burada yapılacak
        Ok(()) // Geçici olarak her zaman başarı döndür (GERÇEK KERNELDE BUNU YAPMAYIN!)
    }
}


// --- Yardımcı/Düşük Seviye Fonksiyonlar (Assembly Gerekebilir) ---

// TrapHandler'a girerken registerları kaydeden ve çıkarken yükleyen assembly kodunu
// veya rust'ın inline assembly özelliklerini kullanmanız gerekecektir.
// trap_handler fonksiyonumuz 'extern "C"' ve #[no_mangle] olarak tanımlandı,
// bu da assembly giriş noktasından çağrılabileceği anlamına gelir.
// Örnek bir konsept (gerçek implementasyon assembly'de veya inline asm ile olur):

#[naked] // Fonksiyonun prologue/epilogue kodunu üretme (Rust'ın kendi stack yönetimini atla)
#[no_mangle]
unsafe extern "C" fn trap_entry() {
    // 1. TrapFrame için stack'te yer ayır.
    // 2. Tüm general purpose registerları bu TrapFrame'e kaydet (x1 - x31).
    // 3. mstatus, mepc gibi trap ile ilgili registerları TrapFrame'e kaydet.
    // 4. TrapFrame'in pointer'ını a0 register'ına koy (ilk argüman).
    // 5. trap_handler(frame: &mut TrapFrame) fonksiyonunu çağır (call trap_handler).
    // 6. trap_handler'dan döndükten sonra:
    // 7. mstatus, mepc gibi trap ile ilgili registerları TrapFrame'den geri yükle.
    // 8. General purpose registerları TrapFrame'den geri yükle (a0 hariç - dönüş değeri a0'dadır).
    // 9. Stack'i temizle (TrapFrame için ayrılan yeri geri al).
    // 10. mret komutu ile kullanıcı alanına (veya önceki ayrıcalık seviyesine) dön.

    // Örnek inline asm taslağı (detaylı implementasyon için RISC-V dökümantasyonu ve Rust book'un asm kısmı gereklidir):
    asm!(
        ".align 8", // Vektör tablosu için hizalama
        "trap_entry:",
        // Stack pointer'ı düşür (TrapFrame boyutu kadar)
        // fp (s0) registerını kaydet
        // ra registerını kaydet
        // gp registerını kaydet
        // tp registerını kaydet
        // t0-t6 registerlarını kaydet
        // a0-a7 registerlarını kaydet
        // s1-s11 registerlarını kaydet
        // mstatus, mepc registerlarını kaydet
        // TrapFrame pointer'ını a0'a yükle
        "call trap_handler", // Rust trap_handler fonksiyonunu çağır
        // a0 register'ını hariç tutarak diğer GPR'ları geri yükle
        // mstatus, mepc registerlarını geri yükle
        // Stack pointer'ı artır
        "mret", // Machine return komutu
        options(noreturn) // Bu fonksiyondan normal yolla dönülmez
    );
}
