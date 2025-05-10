#![no_std]

use core::arch::asm;
use core::ptr;
use core::slice;

// Karnal64 API ve tiplerini içeri aktar
// Bu import yolunu kendi projenizin yapısına göre ayarlayın
use crate::karnal64::{
    KError, KHandle, KTaskId,
    // Karnal64 API fonksiyonları (handle_syscall ve çekirdek modül fns)
    handle_syscall,
    // Çekirdek modüllerinden ihtiyaç duyulacak fonksiyonlar (örneğin, task sonlandırma, bellek yönetimi)
    // Bunlar genellikle handle_syscall içinde kullanılır, ancak istisna işleyici de doğrudan çağırabilir.
    // Örnek: Sayfa hatası durumunda bellek yöneticisi çağrısı veya task sonlandırma.
    kmemory, ktask, // Diğer modüller de gerekebilir
};

// Çekirdek içi pointer doğrulama fonksiyonu placeholder'ı.
// Gerçek implementasyon, MMU ve sayfa tablolarını kullanarak
// pointer'ın geçerli kullanıcı alanı belleğinde olup olmadığını ve
// istenen erişim iznine (okuma/yazma) sahip olup olmadığını kontrol eder.
#[inline]
fn validate_user_pointer<T>(ptr: *mut T, count: usize, writeable: bool) -> Result<*mut T, KError> {
    // TODO: Gerçek pointer ve sınır doğrulama mantığını buraya ekleyin.
    // MMU durumunu, mevcut görevin adres alanını ve talep edilen boyutu kontrol edin.
    // writeable bayrağını kullanın.

    // Şimdilik basit bir Null pointer kontrolü (gerçek güvenlik sağlamaz!)
    if ptr.is_null() && count > 0 {
        return Err(KError::BadAddress);
    }

    // Güvenlik Notu: Bu *gerçek* bir doğrulama değildir. Gerçek bir çekirdekte,
    // kullanıcının adres alanında ptr..ptr+size aralığının tamamen eşlenmiş ve
    // doğru izinlere sahip olduğunu *kesin* olarak kontrol etmeniz gerekir.
    // Başarılı olursa ptr'yi döndürün.
    Ok(ptr)
}

// Kullanıcı alanından gelen sabit pointer'lar için doğrulama
#[inline]
fn validate_user_pointer_read_only<T>(ptr: *const T, count: usize) -> Result<*const T, KError> {
    // read-only pointer için validate_user_pointer'ı kullan
    validate_user_pointer(ptr as *mut T, count, false).map(|p| p as *const T)
}


/// MIPS CPU bağlamını (register'lar) kaydetmek için kullanılan yapı.
/// Assembly glu kodu tarafından doldurulur ve bu Rust fonksiyonuna geçirilir.
/// MIPS64 için 64-bit register'lar varsayılmıştır.
#[repr(C)] // C ile uyumlu bellek düzeni
#[derive(Debug)]
pub struct TrapFrame {
    // Genel Amaçlı Registerlar (GPRs)
    // Genellikle $0 (zero) hariç tümü kaydedilir.
    // Kaydetme sırası MIPS ABI'sine veya çekirdeğin convention'ına bağlıdır.
    // Örnek bir sıra (ABI'den sapmalar olabilir):
    // $1 (at), $2-$3 (v0-v1), $4-$7 (a0-a3), $8-$15 (t0-t7), $16-$23 (s0-s7),
    // $24-$25 (t8-t9), $26-$27 (k0-k1 - çekirdek için kullanılır, normalde kurtarılmaz),
    // $28 (gp), $29 (sp), $30 (fp), $31 (ra)
    pub regs: [u64; 32], // $0 - $31

    // CP0 Registerları
    pub status: u64,   // CP0_Status
    pub cause: u64,    // CP0_Cause
    pub epc: u64,      // CP0_EPC - Exception Program Counter
    pub bad_vaddr: u64, // CP0_BadVAddr (TLB hataları için)

    // Ek registerlar (eğer gerekirse, örn: FPU, DSP)
     pub fpu_regs: [u62; 32], // FPU Registerları (eğer kullanılıyorsa ve kaydediliyorsa)
}

// MIPS Cause registerındaki önemli bit alanları/maskeleri
const CAUSE_EXCCODE_SHIFT: usize = 2;
const CAUSE_EXCCODE_MASK: u64 = 0x1F; // 5 bit exception code
const CAUSE_INTERRUPT_PENDING_MASK: u64 = 0xFF00; // IP0-IP7

// Önemli MIPS Exception Kodları (Cause Register)
const EXCCODE_INTERRUPT: u64 = 0; // Interrupt
const EXCCODE_TLB_MODIFIED: u64 = 1; // TLB Modified
const EXCCODE_TLB_LOAD_FETCH: u64 = 2; // TLB (Load/Instruction Fetch)
const EXCCODE_TLB_STORE: u64 = 3; // TLB (Store)
const EXCCODE_ADDRESS_ERROR_LOAD_FETCH: u64 = 4; // Address Error (Load/Instruction Fetch)
const EXCCODE_ADDRESS_ERROR_STORE: u64 = 5; // Address Error (Store)
const EXCCODE_BUS_ERROR_FETCH: u64 = 6; // Bus Error (Instruction Fetch)
const EXCCODE_BUS_ERROR_DATA: u64 = 7; // Bus Error (Data Load/Store)
const EXCCODE_SYSCALL: u64 = 8; // Syscall
const EXCCODE_BREAKPOINT: u64 = 9; // Breakpoint
const EXCCODE_RESERVED_INSTRUCTION: u64 = 10; // Reserved Instruction
const EXCCODE_COPROCESSOR_UNUSABLE: u64 = 11; // Coprocessor Unusable
const EXCCODE_OVERFLOW: u64 = 12; // Overflow
const EXCCODE_TRAP: u64 = 13; // Trap (MIPS IV)
const EXCCODE_VIRTUAL_COHERENCY_INSTRUCTION: u64 = 14; // Virtual Coherency on Instruction fetch
const EXCCODE_FLOATING_POINT: u64 = 15; // Floating Point (MIPS III)
const EXCCODE_WATCH: u64 = 23; // Watch
const EXCCODE_VIRTUAL_COHERENCY_DATA: u64 = 30; // Virtual Coherency on data access

/// MIPS Exception Vektöründen çağrılan ana C/Rust fonksiyonu.
/// Assembly glu kodu CPU durumunu `frame` yapısına kaydeder ve buraya bir işaretçi geçirir.
#[no_mangle] // Assembly kodu tarafından çağrılabilmesi için ismini değiştirmeyi engelle
pub extern "C" fn handle_exception(frame: *mut TrapFrame) {
    // Güvenlik Kontrolü: frame pointer'ının geçerli ve çekirdek alanında olduğunu varsayıyoruz.
    // Bu, assembly glu kodunun sorumluluğundadır.
    let tf = unsafe { &mut *frame };

    // Cause registerından exception kodunu oku
    let exception_code = (tf.cause >> CAUSE_EXCCODE_SHIFT) & CAUSE_EXCCODE_MASK;

    // Exception/Interrupt türüne göre işlem yap
    match exception_code {
        EXCCODE_INTERRUPT => {
            // Kesme (Interrupt) İşleyici
            let pending_interrupts = (tf.cause & CAUSE_INTERRUPT_PENDING_MASK) >> 8;

            // TODO: Donanım platformuna özel kesme denetleyicisi (interrupt controller)
            // ile etkileşime girerek kesmenin kaynağını (timer, IO, vb.) belirle.
            // Kesmeyi acknowledge (onayla) ve/veya clear et.

            if pending_interrupts & (1 << 7) != 0 {
                // IP7 genellikle zamanlayıcı kesmesi için kullanılır (platforma bağlı)
                 println!("Timer interrupt!"); // Çekirdek içi print! gerektirir

                // TODO: Zamanlayıcı donanımını tekrar ayarla (next tick)

                // Karnal64 zamanlayıcısını çağır. Bu bağlam değiştirmeye neden olabilir.
                // schedule fonksiyonu, dönecek TrapFrame'i güncelleyebilir.
                // Örneğin, mevcut task'ın frame'ini kaydedip, bir sonraki task'ın frame'ini yükleyebilir.
                ktask::schedule(tf); // tf'yi schedule'a geçirerek bağlam değişimini yönetmesini sağla
            }

            // TODO: Diğer kesme seviyeleri (IP0-IP6) için işleyicileri çağır

            // EPC'yi artırmaya gerek yok, kesme aynı instruksiyonu tekrar çalıştırır.
        }
        EXCCODE_SYSCALL => {
            // Sistem Çağrısı (Syscall) İşleyici
            // Sistem çağrısı numarası ve argümanları genellikle MIPS registerlarında bulunur.
            // MIPS convention'ına göre:
            // syscall numarası: $v0 (regs[2])
            // argümanlar: $a0-$a3 (regs[4]-regs[7])
            // Diğer argümanlar stack üzerinde olabilir.
            // Dönüş değeri: $v0 (regs[2]), $v1 (regs[3])
            // Hata durumunda $a3 (regs[7]) genellikle bir hata bayrağı olur, $v0 (regs[2]) ise hata kodunu tutar.

            let syscall_number = tf.regs[2] as u64; // $v0
            let arg1 = tf.regs[4]; // $a0
            let arg2 = tf.regs[5]; // $a1
            let arg3 = tf.regs[6]; // $a2
            let arg4 = tf.regs[7]; // $a3 (İlk 4. argüman veya hata bayrağı)
            // Eğer daha fazla argüman varsa, kullanıcı stack'inden okunmaları gerekir.
            // Bu karmaşıklık, syscall ABI'sine ve argüman sayısına bağlıdır.
            // Şimdilik sadece ilk 4 register argümanını alalım.

            // NOT: Karnal64'ün handle_syscall'ı 5 argüman alıyor, bizim MIPS ABI'mız
            // $a0-$a3 ve stack kullanabilir. ABI uyumluluğu burada önemlidir.
            // Karnal64'ün 5. argümanı (arg5) MIPS'te stack'ten gelmelidir veya
            // Karnal64 API'sı MIPS'e özgü argüman alma şekline uyarlanmalıdır.
            // Örnek olarak, 5. argümanı stack'ten alalım (ABI varsayımı):
            let arg5: u64 = 0; // Stack argümanı okuma mantığı buraya eklenecek
                               let user_stack_ptr = tf.regs[29] as *const u64; // $sp
                               if syscall_needs_arg5(syscall_number) {
                                   arg5 = unsafe { *user_stack_ptr }; // Dikkat! Güvenlik: Stack pointer'ı doğrulanmalı!
                               }

            // Pointer argümanlarını handle_syscall'a geçirmeden önce DOĞRULA!
            // Hangi argümanların pointer olduğunu syscall numarasına göre bilmelisiniz.
            // Örneğin, RESOURCE_ACQUIRE için arg1=id_ptr, arg2=id_len
            // RESOURCE_READ için arg2=user_buffer_ptr, arg3=user_buffer_len
            // RESOURCE_WRITE için arg2=user_buffer_ptr, arg3=user_buffer_len
            // MESSAGE_SEND için arg2=message_ptr, arg3=message_len
            // MESSAGE_RECEIVE için arg1=buffer_ptr, arg2=buffer_len

            let result = match syscall_number {
                // SYSCALL_RESOURCE_ACQUIRE örneği (syscall numarası 5 olduğunu varsayalım)
                5 => { // SYSCALL_RESOURCE_ACQUIRE
                    let id_ptr = arg1 as *const u8;
                    let id_len = arg2 as usize;
                    let mode = arg3 as u32;
                    // Kaynak ID pointer'ını oku/doğrula
                    let id_slice_res = validate_user_pointer_read_only(id_ptr, id_len);

                    match id_slice_res {
                        Ok(ptr) => {
                             // Doğrulama başarılı, Karnal64 API'yı çağır
                             handle_syscall(syscall_number, ptr as u64, id_len as u64, mode as u64, 0, 0) // Diğer arglar kullanılmıyor varsayım
                        },
                        Err(e) => e as i64, // Pointer hatası Karnal64 hatasına dönüşür
                    }
                }
                // SYSCALL_RESOURCE_READ örneği (syscall numarası 6 olduğunu varsayalım)
                6 => { // SYSCALL_RESOURCE_READ
                     let handle_value = arg1;
                     let user_buffer_ptr = arg2 as *mut u8;
                     let user_buffer_len = arg3 as usize;
                     // Kullanıcı tamponu pointer'ını yazma için doğrula
                     let buffer_ptr_res = validate_user_pointer(user_buffer_ptr, user_buffer_len, true); // writeable=true

                     match buffer_ptr_res {
                         Ok(ptr) => {
                             // Doğrulama başarılı, Karnal64 API'yı çağır
                             handle_syscall(syscall_number, handle_value, ptr as u64, user_buffer_len as u64, 0, 0) // Diğer arglar kullanılmıyor varsayım
                         },
                         Err(e) => e as i64, // Pointer hatası Karnal64 hatasına dönüşür
                     }
                }
                // SYSCALL_RESOURCE_WRITE örneği (syscall numarası 7 olduğunu varsayalım)
                7 => { // SYSCALL_RESOURCE_WRITE
                    let handle_value = arg1;
                    let user_buffer_ptr = arg2 as *const u8;
                    let user_buffer_len = arg3 as usize;
                    // Kullanıcı tamponu pointer'ını okuma için doğrula
                    let buffer_ptr_res = validate_user_pointer_read_only(user_buffer_ptr, user_buffer_len); // read-only

                    match buffer_ptr_res {
                        Ok(ptr) => {
                            // Doğrulama başarılı, Karnal64 API'yı çağır
                            handle_syscall(syscall_number, handle_value, ptr as u64, user_buffer_len as u64, 0, 0) // Diğer arglar kullanılmıyor varsayım
                        },
                        Err(e) => e as i64, // Pointer hatası Karnal64 hatasına dönüşür
                    }
                }
                // Diğer syscall'lar (pointer argümanı olmayanlar veya validate edilenler)
                _ => {
                    // Pointer argümanı olmayan veya yukarıda özel olarak ele alınmayan syscall'lar
                    // Güvenlik Notu: Eğer syscall numarası bilinmiyorsa veya argümanları
                    // doğru şekilde doğrulanmamışsa güvenlik açığı oluşabilir.
                    // Tüm beklenen syscall numaraları ve argüman türleri burada ele alınmalıdır.
                    handle_syscall(syscall_number, arg1, arg2, arg3, arg4, arg5)
                }
            };


            // handle_syscall'dan dönen sonucu kullanıcı alanına aktar
            if result < 0 {
                // Hata durumu (KError kodları negatif)
                tf.regs[2] = result as u64; // $v0'a hata kodu (-1, -2, vb.) yaz
                tf.regs[7] = 1;             // $a3'e hata olduğunu belirten bayrak yaz (MIPS convention)
            } else {
                // Başarı durumu (pozitif veya sıfır)
                tf.regs[2] = result as u64; // $v0'a başarı değerini yaz
                tf.regs[3] = (result >> 32) as u64; // $v1'e sonucun yüksek 32 bitini yaz (64-bit sonuç için)
                tf.regs[7] = 0;             // $a3'e başarıyı belirten bayrak yaz
            }

            // Sistem çağrısı instruksiyonundan sonra devam etmek için EPC'yi artır
            tf.epc += 4; // MIPS instruksiyon boyutu 4 byte
        }
        EXCCODE_TLB_MODIFIED | EXCCODE_TLB_LOAD_FETCH | EXCCODE_TLB_STORE => {
            // TLB (Page Fault) Hatası İşleyici
            // CPU, BadVAddr registerına geçersiz/olmayan adresi yazar.
            let bad_vaddr = tf.bad_vaddr;
            // Hatanın türünü (okuma/yazma, kullanıcı/çekirdek) Cause ve Status'tan belirle
            let is_write = exception_code == EXCCODE_TLB_MODIFIED || exception_code == EXCCODE_TLB_STORE;
            let is_user = (tf.status & (1 << 3)) != 0; // Status Register PLV (Previous Level bit) veya KSU (Kernel/Supervisor/User) bitlerine bağlı

            // Karnal64 bellek yöneticisine page fault'u bildir.
            // Bellek yöneticisi ilgili sayfayı haritalamaya veya cow (copy-on-write) yapmaya çalışabilir.
            // tf pointer'ını da geçirmek gerekebilir, çünkü bellek yöneticisi bloklanan görevi uyandırabilir
            // veya hata düzeltilemezse görevi sonlandırabilir.
            let fault_result = kmemory::handle_page_fault(bad_vaddr, is_write, is_user, frame);

            match fault_result {
                Ok(_) => {
                    // Hata başarıyla düzeltildi (sayfa haritalandı vb.).
                    // Exception handler'dan dönüldüğünde, faulted instruksiyon tekrar denenir.
                }
                Err(e) => {
                    // Hata düzeltilemedi (geçersiz adres, izin yok vb.).
                    // Bu genellikle mevcut görevin sonlandırılması anlamına gelir.
                    // Karnal64 görev yöneticisini çağırarak görevi sonlandır.
                    // terminate_current_task fonksiyonu normalde geri dönmez.
                    ktask::terminate_current_task(e);
                    // Eğer terminate_current_task geri dönerse (ki dönmemeli),
                    // burada bir panic veya kurtarılamaz hata işleme yapılmalıdır.
                     loop {} // Beklenmedik dönüş durumunda sonsuz döngü/kernel panic
                }
            }
            // EPC artırılmaz, faulted instruksiyon tekrar denenecek.
        }
        EXCCODE_ADDRESS_ERROR_LOAD_FETCH | EXCCODE_ADDRESS_ERROR_STORE |
        EXCCODE_BUS_ERROR_FETCH | EXCCODE_BUS_ERROR_DATA => {
            // Adres Hatası veya Bus Hatası İşleyici
            // Genellikle kurtarılamaz hatalardır.
            let bad_vaddr = tf.bad_vaddr; // Hatalı adres
            // Hatanın türünü logla
             println!("Address/Bus Error at EPC: {:#x}, BadVAddr: {:#x}, Cause: {:#x}",
                      tf.epc, tf.bad_vaddr, tf.cause); // Çekirdek içi print!

            // Hata genellikle mevcut görevin sonlandırılmasıyla sonuçlanır.
            // Karnal64 görev yöneticisini çağırarak görevi sonlandır.
            // Uygun bir hata kodu kullanın.
            ktask::terminate_current_task(KError::BadAddress);
            // terminate_current_task geri dönmez.
             loop {} // Beklenmedik dönüş durumunda sonsuz döngü/kernel panic
        }
        EXCCODE_BREAKPOINT | EXCCODE_RESERVED_INSTRUCTION |
        EXCCODE_COPROCESSOR_UNUSABLE | EXCCODE_OVERFLOW | EXCCODE_TRAP |
        EXCCODE_WATCH | EXCCODE_FLOATING_POINT |
        EXCCODE_VIRTUAL_COHERENCY_INSTRUCTION | EXCCODE_VIRTUAL_COHERENCY_DATA => {
            // Diğer İstisnalar
            // Hatanın türünü logla ve genellikle mevcut görevi sonlandır.
             println!("Unhandled Exception Code {} at EPC: {:#x}, Cause: {:#x}",
                      exception_code, tf.epc, tf.cause); // Çekirdek içi print!

            // Bu istisnalar genellikle programlama hatası veya donanım sorunudur.
            // Mevcut görevi sonlandır.
            ktask::terminate_current_task(KError::InternalError); // Veya daha spesifik bir hata kodu
            // terminate_current_task geri dönmez.
             loop {} // Beklenmedik dönüş durumunda sonsuz döngü/kernel panic
        }
        _ => {
            // Bilinmeyen Exception Kodu
             println!("Unknown Exception Code {} at EPC: {:#x}, Cause: {:#x}",
                      exception_code, tf.epc, tf.cause); // Çekirdek içi print!

            // Kurtarılamaz hata. Çekirdeğin kendisinde bir sorun olabilir veya
            // donanımdan beklenmedik bir exception gelmiş olabilir.
            // Çekirdek panik yap (sistemi durdur).
            // TODO: Kendi çekirdek panic implementasyonunuzu çağırın.
             panic!("KERNEL PANIC: Unknown Exception"); // Rust panic! sadece debug amaçlıdır
        }
    }

    // Assembly glu koduna geri dönülecek.
    // Assembly kodu, TrapFrame'deki register durumunu restore edecek
    // ve EPC'ye dallanarak istisna öncesi akışa (veya schedule sonrası yeni akışa) dönecektir.
    // Eğer bir görev sonlandırıldıysa, buraya asla ulaşılmaz.
}

// --- Placeholder Çekirdek Modülü Fonksiyonları (Çağrılacak olanlar) ---
// Bu fonksiyonların Karnal64 modülleri içinde tanımlı ve public olması gerekir.

// kmemory module:
mod kmemory {
    use super::*; // srcexception_mips.rs scope'undaki tipleri kullan

    // Sayfa hatasını işleyen fonksiyon.
    // Başarılı olursa Ok, hata olursa Err(KError) döner.
    pub fn handle_page_fault(vaddr: u64, is_write: bool, is_user: bool, frame: *mut TrapFrame) -> Result<(), KError> {
        // TODO: Gerçek TLB/Page Table yönetim mantığı.
        // vaddr için geçerli bir harita var mı kontrol et.
        // COW sayfaları için kopyala.
        // İzinleri (okuma/yazma, kullanıcı/çekirdek) kontrol et.
        // Gerekirse TLB girdisini güncelle.
        // Başarılı olursa Ok(()), aksi halde uygun KError ile Err() döndür.
         println!("Kmemory: Handling page fault for vaddr {:#x}, write: {}, user: {}", vaddr, is_write, is_user); // Yer Tutucu
        // Şimdilik her zaman hata döndürsün ki hata işleme yolu test edilsin
        Err(KError::BadAddress)
    }

    // Diğer kmemory fonksiyonları (allocate, map vb.) burada kullanılmayabilir
    // ama Karnal64 API'sı içinde tanımlı olmalılar.
}

// ktask module:
mod ktask {
    use super::*; // srcexception_mips.rs scope'undaki tipleri kullan

    // Görev zamanlayıcısını çalıştırır.
    // Gerekirse bağlam değişimini yönetir.
    // frame pointer'ını alması, mevcut task'ın durumunu kaydetmesi ve
    // bir sonraki task'ın durumunu yüklemesi için gereklidir.
    pub fn schedule(frame: *mut TrapFrame) {
        // TODO: Görev zamanlayıcı algoritmasını çalıştır (round-robin, priority vb.).
        // Mevcut task'ın durumunu frame'den kendi task_control_block'una kaydet.
        // Çalıştırılacak bir sonraki task'ı seç.
        // Seçilen task'ın durumunu (kayıtlı TrapFrame'ini) frame'e yükle.
        // Bu fonksiyon döndüğünde, assembly kodu frame'deki yeni durumu restore edecek.
         println!("Ktask: Scheduling..."); // Yer Tutucu
        // Bağlam değişimi simülasyonu (gerçekte burada task'lar değişir ve frame güncellenir)
    }

    // Mevcut görevi sonlandırır.
    // Normalde geri dönmez.
    pub fn terminate_current_task(error_code: KError) -> ! {
        // TODO: Mevcut task'ı sonlandırma mantığı.
        // Görevi zamanlayıcı kuyruklarından kaldır.
        // Sahip olduğu kaynakları (handle'lar, bellek) serbest bırak.
        // Çıkış kodunu (error_code) ata.
        // Bir sonraki task'ı zamanlayıcıdan seç ve ona geçiş yap.
         println!("Ktask: Terminating current task with error: {:?}", error_code); // Yer Tutucu
        // Normalde burada bir bağlam değişimi olur ve bu fonksiyon asla dönmez.
        // Simülasyon için bir panik veya sonsuz döngü koyabiliriz.
         loop {} // Görev sonlandığında bu noktaya gelinmemeli, sonsuz döngüye girilmeli
    }

    // Diğer ktask fonksiyonları (spawn, exit - bu syscall handler'da kullanılır, get_id vb.)
    // burada kullanılmayabilir ama Karnal64 API'sı içinde tanımlı olmalılar.
}

// kkernel module:
mod kkernel {
     use super::*; // srcexception_mips.rs scope'undaki tipleri kullan
     // Placeholder for kernel information functions if needed by exception handler
      pub fn get_time() -> u64 { /* ... */ }
      pub fn log(msg: &str) { /* ... */ } // Çekirdek içi loglama
     pub fn init_manager() { /* */ } // dummy init
}

// Diğer Karnal64 modülleri için dummy/placeholder init fonksiyonları
// Karnal64 init fonksiyonu bu modülleri başlatır.
mod kresource { pub fn init_manager() { /* */ } }
mod ksync { pub fn init_manager() { /* */ } }
mod kmessaging { pub fn init_manager() { /* */ } }


// --- Örnek Kullanım (Gerçek Çekirdek Kodu Değildir) ---
// Bu kısım sadece örnek olarak nasıl çağrılabileceğini göstermek içindir ve
// çekirdeğin kendisi tarafından doğrudan çağrılmaz (assembly tarafından çağrılır).
#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{size_of, MaybeUninit};

    // Dummy Karnal64 handle_syscall fonksiyonu (Test amaçlı)
    #[no_mangle]
    extern "C" fn handle_syscall(
        number: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
        arg4: u64,
        arg5: u64
    ) -> i64 {
        println!("Dummy handle_syscall called: num={} arg1={} arg2={} arg3={} arg4={} arg5={}",
                 number, arg1, arg2, arg3, arg4, arg5);
        // Basit bir syscall simülasyonu: Eğer num 5 ise başarı (123) döndür, değilse Unsupported hata döndür.
        if number == 5 { 123 } else { KError::NotSupported as i64 }
    }


    #[test]
    fn test_syscall_handling() {
        // Sistem çağrısı için bir TrapFrame simüle et
        let mut frame = MaybeUninit::<TrapFrame>::uninit();
        let tf_ptr = frame.as_mut_ptr();
        let tf = unsafe { &mut *tf_ptr };

        // Kayıtları sıfırla
        tf.regs.iter_mut().for_each(|r| *r = 0);

        // Bir syscall simüle et (örn: SYSCALL_RESOURCE_ACQUIRE = 5)
        tf.cause = EXCCODE_SYSCALL << CAUSE_EXCCODE_SHIFT; // Cause registerına syscall kodunu yaz
        tf.epc = 0x1000; // Syscall'ın olduğu adres (rastgele)

        // Syscall argümanlarını registerlara koy (MIPS Convention)
        let syscall_num: u64 = 5; // SYSCALL_RESOURCE_ACQUIRE
        let resource_name_ptr: u64 = 0x2000; // Kullanıcı alanı string pointer'ı (simüle)
        let resource_name_len: u64 = 5; // "cons0" gibi
        let mode: u64 = 1; // READ mode

        tf.regs[2] = syscall_num; // $v0 = syscall numarası
        tf.regs[4] = resource_name_ptr; // $a0 = id_ptr
        tf.regs[5] = resource_name_len; // $a1 = id_len
        tf.regs[6] = mode;             // $a2 = mode

        // Kullanıcı pointer'ı için dummy data oluştur (validate_user_pointer'ın başarılı olmasını simüle etmek için)
        // Gerçekte bu kullanıcı adres alanında olurdu.
        let dummy_user_data = b"cons0";
        let user_data_ptr = dummy_user_data.as_ptr() as *mut u8;

        // validate_user_pointer'ı dummy veri adresini döndürecek şekilde geçici olarak değiştirin
        // Normalde bu testte validate_user_pointer mock'lanmalı veya gerçek MMU simülasyonu olmalı.
        // Bu basit test için, validate_user_pointer'ın her zaman Ok dönmesini sağlayalım.
        // Gerçek çekirdek testleri çok daha karmaşıktır.
         unsafe { // HACK: validate_user_pointer global static olursa mock yapılabilir, fonksiyon mock'lama zor.
        //    // Burada validate_user_pointer'ın davranışını geçici olarak değiştirmenin bir yolu olmalı
         }

        // handle_exception fonksiyonunu çağır
        unsafe { handle_exception(tf_ptr) };

        // Sonuçları kontrol et (başarı bekleniyor)
        let result = tf.regs[2] as i64; // $v0
        let error_flag = tf.regs[7];   // $a3

        println!("Syscall Result: {} (i64), Error Flag: {}", result, error_flag);

        // Başarı durumunda $a3 = 0, $v0 = 123 (dummy syscall sonucu) ve EPC + 4 beklenir.
        assert_eq!(error_flag, 0, "Syscall should indicate success ($a3 == 0)");
        assert_eq!(result, 123, "Syscall result ($v0) should be 123");
        assert_eq!(tf.epc, 0x1004, "EPC should be incremented by 4");
    }

     #[test]
     fn test_page_fault_handling() {
         // Page fault için bir TrapFrame simüle et
         let mut frame = MaybeUninit::<TrapFrame>::uninit();
         let tf_ptr = frame.as_mut_ptr();
         let tf = unsafe { &mut *tf_ptr };

         // Kayıtları sıfırla
         tf.regs.iter_mut().for_each(|r| *r = 0);

         // Bir TLB Load/Fetch hatası simüle et (kurtarılamaz hata)
         tf.cause = EXCCODE_TLB_LOAD_FETCH << CAUSE_EXCCODE_SHIFT;
         tf.epc = 0x2000; // Hatanın olduğu adres
         tf.bad_vaddr = 0xBAD_ADDR; // Hatalı sanal adres (simüle)
         tf.status = 0x10; // User mode (PLV=1 veya KSU bitleri uygun şekilde ayarlanmış)

         // ktask::terminate_current_task panik yapacağı için, testin panik yapmasını beklemeliyiz.
         // Bu tür testler için genellikle Rust'ın `should_panic` özelliği kullanılır veya
         // bir test framework'ü panik yakalama özelliği sunar.
         // Burada basitçe panic olacağını belirtelim.
         println!("Testing page fault leading to task termination (expected panic):");
         // Bu çağrı panic!e yol açar (çünkü kmemory::handle_page_fault hata döner ve ktask::terminate_current_task çağrılır).
         unsafe { handle_exception(tf_ptr) };

         // Bu noktaya asla ulaşılmamalıdır eğer terminate_current_task doğru çalışıyorsa.
         // Eğer ulaşılırsa test başarısız demektir.
         panic!("Test did not panic as expected");
     }

    // TODO: Diğer exception türleri için testler (interrupt, address error vb.)
}
