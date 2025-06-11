#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz, çekirdek alanında çalışırız

use super::karnal64; // Karnal64 API'sını kullanmak için

// LoongArch Genel Amaçlı Register Sayısı
const NUM_GPRS: usize = 32;
// LoongArch özelinde syscall numarasının ve argümanlarının hangi registerlarda geldiği
// ABI'ye bağlıdır. Genel olarak a0-a7 argümanlar için kullanılır.
// Karnal64 handle_syscall (number, arg1, ..., arg5) 6 argüman bekler.
// Varsayım: Syscall numarası a0'da (r4), arg1-arg5 a1-a5'te (r5-r9) geliyor.
// Dönüş değeri a0'a (r4) yazılacak.
const RA: usize = 22; // r22: Return Address
const SP: usize = 3;  // r3: Stack Pointer (LoongArch specific, some ABIs use r1)
const A0: usize = 4;  // r4: Argument 0 / Return Value
const A1: usize = 5;  // r5: Argument 1
const A2: usize = 6;  // r6: Argument 2
const A3: usize = 7;  // r7: Argument 3
const A4: usize = 8;  // r8: Argument 4
const A5: usize = 9;  // r9: Argument 5
// Syscall numarasının geldiği register indexi (Yukarıdaki varsayıma göre A0)
const SYSCALL_NUM_REG: usize = A0;
// Syscall argümanlarının başladığı register indexi
const SYSCALL_ARG_START_REG: usize = A1;
// Syscall dönüş değerinin yazılacağı register indexi
const SYSCALL_RET_REG: usize = A0;


// LoongArch Özel CSR'ler (Kontrol ve Durum Registerları) - Örnekler ve yer tutucular
// Gerçek isimler ve bit alanları LoongArch ISA Spesifikasyonuna göre kontrol edilmelidir.
const CSR_ERA: u16 = 0x14; // Exception Return Address
const CSR_CRMD: u16 = 0x0; // Current Mode
const CSR_ESTAT: u16 = 0x10; // Exception Status Register (Cause code here)
const CSR_PRMD: u16 = 0x1; // Previous Mode

// LoongArch İstisna (Exception) Neden Kodları (ESTAT registerından okunur) - Örnekler
// Gerçek kodlar LoongArch ISA Spesifikasyonuna göre kontrol edilmelidir.
const LOONGARCH_EXCODE_SYSCALL: u16 = 0x0B; // Sistem çağrısı (genel bir varsayım, MIPS/RISC-V benzeri)
const LOONGARCH_EXCODE_TLB_MISS_LOAD: u16 = 0x02; // TLB/Page Fault Load
const LOONGARCH_EXCODE_TLB_MISS_STORE: u16 = 0x03; // TLB/Page Fault Store
const LOONGARCH_EXCODE_ILLEGAL_INSN: u16 = 0x01; // Illegal Instruction
const LOONGARCH_EXCODE_BREAKPOINT: u16 = 0x09; // Breakpoint
// TODO: Diğer exception kodları eklenecek

// LoongArch Kesme (Interrupt) Neden Kodları (ESTAT registerından okunur veya ayrı bir mekanizma ile)
// Genellikle Timer, External I/O vb. kesmeler için kullanılır.
const LOONGARCH_INTCODE_TIMER: u16 = 0x8000 | 0; // Varsayımsal: Interrupt bit + Interrupt ID
const LOONGARCH_INTCODE_EXTERNAL: u16 = 0x8000 | 1; // Varsayımsal: Interrupt bit + External Interrupt ID
// TODO: Diğer interrupt kodları eklenecek


/// Kesme/Tuzak (Interrupt/Trap) anında CPU registerlarının kaydedildiği yapı.
/// Bu yapı, assembly giriş noktasında doldurulur ve Rust handler'ına geçirilir.
/// Kullanıcı veya çekirdek görevlerinin bağlamını temsil eder.
#[repr(C)] // C uyumluluğu, assembly'den erişilebilmesi için
#[derive(Debug, Clone, Copy)]
pub struct TrapFrame {
    /// Genel Amaçlı Registerlar (r0 - r31)
    pub regs: [u64; NUM_GPRS],
    /// İstisna/Kesme Dönüş Adresi Registerı (ERA)
    pub csr_era: u64,
    /// Mevcut Mod Registerı (CRMD)
    pub csr_crmd: u64,
    /// İstisna Durum Registerı (ESTAT) - Neden kodu burada bulunur
    pub csr_estat: u64,
    /// Önceki Mod Registerı (PRMD)
    pub csr_prmd: u64,
    // TODO: Bağlam değiştirme ve hata ayıklama için gerekli olabilecek diğer CSR'ler
}

// Assembly giriş noktası (`_trap_entry`) bu fonksiyonu çağırır.
// `tf` pointer'ı, assembly tarafından kernel stack'ine kaydedilmiş registerları gösterir.
// `exception_code`, ESTAT registerından okunan neden kodudur.
#[no_mangle] // Bu fonksiyonun isminin derleme sonrası değişmemesi için
pub extern "C" fn handle_trap(tf: &mut TrapFrame, exception_code: u64) -> *mut TrapFrame {
    // TODO: Çekirdek loglama veya hata ayıklama için trapframe'i yazdır.
     println!("--- TRAP! code: {:#x}, ERA: {:#x} ---", exception_code, tf.csr_era);
    // TODO: Şu anki görevin/iş parçacığının bağlamını kaydet (Scheduler/Task Manager'a bildir).

    // İstisna/Kesme nedenine göre uygun handler'a yönlendir
    match exception_code as u16 {
        LOONGARCH_EXCODE_SYSCALL => {
            // Sistem Çağrısı İşleme
            // Syscall numarasını ve argümanlarını trap frame'den al
            let syscall_number = tf.regs[SYSCALL_NUM_REG];
            let arg1 = tf.regs[SYSCALL_ARG_START_REG];
            let arg2 = tf.regs[SYSCALL_ARG_START_REG + 1];
            let arg3 = tf.regs[SYSCALL_ARG_START_REG + 2];
            let arg4 = tf.regs[SYSCALL_ARG_START_REG + 3];
            let arg5 = tf.regs[SYSCALL_ARG_START_REG + 4];

            // TODO: Kullanıcı alanı pointer'ı olan argümanlar için güvenlik doğrulaması YAPILMALIDIR.
            // Bu doğrulama handle_syscall içinde veya çağırmadan önce yapılabilir.
            // Şu an için doğrudan Karnal64 API'sını çağırıyoruz.

            let result = karnal64::handle_syscall(
                syscall_number,
                arg1, arg2, arg3, arg4, arg5
            );

            // Sistem çağrısı sonucunu trap frame'in dönüş değeri registerına yaz
            tf.regs[SYSCALL_RET_REG] = result as u64; // Karnal64 i64 döner, u64'e çevir (negatifler hata kodu)

            // Syscall komutundan sonra devam etmek için ERA'yı güncelle.
            // LoongArch genellikle ERA'yı doğru ayarlar ama kontrol etmek/ayarlamak gerekebilir.
             tf.csr_era += 4; // Örnek: 4 byte'lık bir syscall komutu sonrası

            // TODO: Eğer syscall schedule'a neden olduysa (örn: task_exit, task_sleep, yield)
            // yeni bir trapframe pointer'ı döndürülmeli.
        }
        LOONGARCH_INTCODE_TIMER => {
            // Zamanlayıcı Kesmesi İşleme
            // TODO: Zamanlayıcı kesmesi sayacını veya ilgili donanımı resetle/yapılandır.
            // TODO: Görev zamanlayıcısına (Scheduler) zamanlayıcı kesmesi olduğunu bildir.
             ktask::handle_timer_interrupt();
            // TODO: Eğer zamanlayıcı kesmesi bağlam değişimini tetiklediyse,
            // Scheduler'dan bir sonraki görevin trapframe pointer'ını al.
        }
        LOONGARCH_INTCODE_EXTERNAL => {
            // Harici Donanım Kesmesi İşleme
            // TODO: Hangi donanımın kesme ürettiğini belirle (IRQ kontrolcüsünden oku).
            // TODO: İlgili sürücünün kesme işleyicisini çağır.
             driver_manager::handle_irq(irq_number);
            // TODO: IRQ kontrolcüsünde kesmeyi onayla (ACK).
        }
        LOONGARCH_EXCODE_TLB_MISS_LOAD | LOONGARCH_EXCODE_TLB_MISS_STORE => {
            // Sayfa Hatası (Page Fault) İşleme
            // TODO: Hata adresini (BadVAddr veya benzeri CSR'den) oku.
            // TODO: Sayfa hatası nedenini (okuma/yazma, kullanıcı/çekirdek modu) belirle.
            // TODO: Bellek yöneticisine (Memory Manager) sayfa hatasını bildir.
             kmemory::handle_page_fault(fault_address, fault_reason);
            // TODO: Eğer sayfa yöneticisi hatayı çözebilirse (örn: demand paging), fonksiyondan dön.
            // TODO: Çözülemezse, hata mesajı yazdır ve görevi sonlandır.
             println!("PAGE FAULT! Addr: {:#x}, Reason: {:?}", fault_address, fault_reason);
             ktask::exit_current(karnal64::KError::BadAddress as i32); // Görevi sonlandırır, geri dönmez
        }
        LOONGARCH_EXCODE_ILLEGAL_INSN => {
            // Geçersiz Komut İşleme
            // TODO: Hata mesajı yazdır ve görevi sonlandır.
             println!("ILLEGAL INSTRUCTION!");
             ktask::exit_current(karnal64::KError::InvalidArgument as i32); // Görevi sonlandırır, geri dönmez
        }
        LOONGARCH_EXCODE_BREAKPOINT => {
             // Breakpoint/Hata Ayıklama Tuzak İşleme
             // TODO: Hata ayıklayıcıya kontrolü ver (varsa).
             // TODO: Hata mesajı yazdır veya devam et.
              println!("BREAKPOINT!");
              tf.csr_era += 4; // Genellikle breakpoint komutundan sonra devam etmek için ERA'yı ilerlet
        }
        // TODO: Diğer istisna/kesme nedenleri için case'ler eklenecek.

        _ => {
            // Bilinmeyen İstisna/Kesme
            // TODO: Hata mesajı yazdır, sistem durumunu kaydet (panik için) ve çekirdeği durdur.
             println!("!!! UNHANDLED TRAP! code: {:#x}, ERA: {:#x} !!!", exception_code, tf.csr_era);
             kernel_panic("Unhandled trap"); // Çekirdek panik fonksiyonu
             loop {} // Sistem durduruldu
        }
    }

    // TODO: Zamanlayıcının (Scheduler) bir bağlam değişimine karar verip vermediğini kontrol et.
    // Eğer karar verdiyse, bir sonraki görevin trapframe pointer'ını döndür.
    // Yoksa, mevcut trapframe pointer'ını döndür (aynı göreve geri dönecek).
    // Örneğin:
     let next_task_tf = ktask::schedule();
     if let Some(next_tf_ptr) = next_task_tf {
         next_tf_ptr
     } else {
         tf // Aynı göreve geri dön
     }
}

/// Kernel başlatılırken kesme/tuzak vektörünü kuracak fonksiyon (Kavramsal).
/// Bu fonksiyon, çekirdek boot sürecinde bir kez çağrılmalıdır.
#[no_mangle]
pub extern "C" fn init_trap() {
    // TODO: LoongArch mimarisine özel CSR'leri yapılandır:
    // - Tuzak giriş noktası adresini (örneğin `_trap_entry` assembly label'ı) ilgili CSR'e yaz (EENTRY?).
    // - Kesmeleri ve istisnaları etkinleştir (CRMD, ECFG veya ilgili CSR'ler).
    // - Çekirdek yığınını ayarla (assembly veya ayrı bir fonksiyonda yapılır).
    // - TLB/MMU ile ilgili başlangıç ayarlarını yap (eğer gerekiyorsa).

    // Örnek (placeholder):
    // unsafe {
    //     // Set EENTRY to the address of our assembly trap handler
          write_csr!(CSR_EENTRY, _trap_entry as u64); // write_csr! bir makro/inline assembly gerektirir
    //
    //     // Enable interrupts/exceptions (simplified example)
          let mut crmd = read_csr!(CSR_CRMD);
          crmd |= (1 << INTERRUPT_ENABLE_BIT); // INTERRUPT_ENABLE_BIT LoongArch spec'ten alınmalı
          write_csr!(CSR_CRMD, crmd);
     }

     println!("LoongArch Trap/Interrupt handler initialized (Placeholder)");
}


Aşağıdaki assembly kodu (veya Rust inline assembly), çekirdeğin giriş noktasında
veya ayrı bir assembly dosyasında yer alacak ve `handle_trap` fonksiyonunu çağıracaktır.
Rust tarafından doğrudan yazılmaz ama Rust kodu bu assembly'nin beklediği arayüzü sağlar.

global _trap_entry
_trap_entry:
    // 1. Kritik Bölgeye Gir: Kesmeleri devre dışı bırak.
    // LoongArch CRMD registerı kullanılır

    // 2. Registerları Kaydet:
    // Tüm genel amaçlı registerları (r0-r31) mevcut stack üzerine kaydet.
    // Yeni bir TrapFrame yapısı oluşturuluyor gibi düşünülebilir.
    // Örneğin:
     addi.d sp, sp, -TRAP_FRAME_SIZE // stack pointer'ı indir
     st.d r0, sp, TF_R0_OFFSET      // r0'ı kaydet
     st.d r1, sp, TF_R1_OFFSET      // r1'ı kaydet (sp'nin kendisini de kaydediyoruz)
    // ... r31'e kadar ...

    // 3. CSR'leri Kaydet:
    // ERA, ESTAT, CRMD, PRMD gibi ilgili CSR'leri stack üzerindeki TrapFrame içine kaydet.
     mfcsr t0, CSR_ERA          // ERA'yı oku
     st.d t0, sp, TF_ERA_OFFSET // TrapFrame'e yaz
    // ... diğer CSR'ler ...

    // 4. Çekirdek Stack'ine Geç:
    // Eğer tuzak kullanıcı modunda oluştuysa, çekirdek moduna geçilmeli
    // ve göreve özel çekirdek stack'ine geçiş yapılmalıdır.
    // Bu genellikle PRMD ve CRMD registerları ile yönetilir.
     t0 = current_task.kernel_stack_pointer
     move sp, t0

    // 5. Rust Handler'ı Çağır:
    // handle_trap fonksiyonunu çağır.
    // Argümanlar: TrapFrame pointer'ı (yeni sp'nin adresi), exception_code.
    // LARCH ABI'ye göre argümanlar a0 (r4) ve a1'e (r5) konulur.
     move a0, sp // TrapFrame pointer'ı
     ld.d a1, sp, TF_ESTAT_OFFSET // ESTAT'ı (exception_code) yükle
     jal handle_trap // Rust fonksiyonunu çağır

    // 6. Dönüş Değerini Kontrol Et:
    // handle_trap fonksiyonu bir sonraki çalışacak TrapFrame'in pointer'ını döndürür (a0 registerında).
    // Eğer döndürülen pointer mevcut TrapFrame ile aynıysa, aynı göreve döneceğiz.
    // Eğer farklıysa, bağlam değişimi olacak.

    // 7. Yeni Bağlamı Yükle (veya Mevcut Bağlamı Geri Yükle):
    // a0 (dönüş değeri) şimdiki TrapFrame pointer'ı.
     ld.d r0, a0, TF_R0_OFFSET // TrapFrame'den r0'ı yükle
    // ... r31'e kadar ...
     ld.d t0, a0, TF_ERA_OFFSET // TrapFrame'den ERA'yı yükle
     mtcsr t0, CSR_ERA         // ERA'yı CSR'e yaz
    // ... diğer CSR'ler ...

    // 8. Kritik Bölgeden Çık: Kesmeleri yeniden etkinleştir.
    // LoongArch CRMD registerı kullanılır.

    // 9. Tuzaktan Dön:
    // İstisna dönüş komutunu kullan (LoongArch'ta ERET veya benzeri?).
    // ERET komutu, ERA'daki adrese atlar ve CRMD/PRMD'yi güncelleyerek uygun moda döner.
     ERET
