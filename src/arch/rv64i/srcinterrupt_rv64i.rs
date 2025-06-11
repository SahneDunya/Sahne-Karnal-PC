#![no_std]

// RISC-V CSR'lerine erişim için harici crate/modül
// Örnek olarak 'riscv' crate'i kullanılabilir.
// extern crate riscv; // Ya da kendi CSR makrolarınızı kullanın.
use riscv::register::{
    scause::{self, Exception, Interrupt, Trap},
    sepc, stval, sstatus,
};

// Karnal64 API'sını kullanacağız
#[allow(unused_imports)] // Henüz tam kullanılmıyor olabilir
use crate::karnal64::{
    self, KError, KHandle, KTaskId,
    // Kullanılacak Karnal64 API fonksiyonları
    handle_syscall,
    // Placeholder fonksiyonlar (kmemory, ktask vb. modüllerinizde olmalı)
    kmemory, ktask, ksync, kmessaging, kkernel,
};


/// Sistem çağrısı veya kesme/tuzak meydana geldiğinde kaydedilen işlemci durumu (bağlam).
/// Assembly giriş noktasında kaydedilir ve Rust handler'ına geçirilir.
/// Register'ların sırası ve dahil edilecekler, kullandığınız ABI'ya ve tasarımınıza bağlıdır.
/// Genellikle x1-x31 (x0 hardwired zero), sepc, sstatus kaydedilir.
#[repr(C)] // C ABI uyumluluğu, assembly ile etkileşim için önemli
#[derive(Debug)]
pub struct TrapFrame {
    // Genel Amaçlı Kayıtlar (x0 hariç)
    pub regs: [usize; 31], // x1 - x31

    // Kontrol ve Durum Kayıtları (CSRs)
    pub sepc: usize,    // Süpervizör İstisna Program Sayacı
    pub sstatus: usize, // Süpervizör Durum Kaydı
    pub stval: usize,   // Tuzak Değer Kaydı (sayfa hataları gibi)
    pub scause: usize,  // Tuzak Nedeni Kaydı
    // İhtiyaca göre başka CSR'ler eklenebilir (örn. sscratch, satp)
     pub sscratch: usize, // Süpervizör Scratch Register
     pub satp: usize,     // Süpervizör Adres Çeviri ve Koruma Kaydı
}

// RISC-V sistem çağrısı argümanları genellikle a0-a5 (x10-x15) kayıtlarındadır.
// Syscall numarası a7 (x17) kaydındadır.
// Bu, yukarıdaki `regs` dizisine erişirken indeksleri bilmeyi gerektirir.
 x10 -> regs[9]
 x11 -> regs[10]
 x12 -> regs[11]
 x13 -> regs[12]
 x14 -> regs[13]
 x15 -> regs[14]
  x17 -> regs[16]

/// Çekirdek tuzak/kesme işleyicisinin Rust tarafındaki ana giriş noktası.
/// Bu fonksiyon, düşük seviyeli assembly giriş noktasından çağrılır.
///
/// # Safety
/// Bu fonksiyon `unsafe`'dir çünkü doğrudan donanım bağlamı ve ham pointer'larla çalışır.
/// Çağıran assembly kodunun `trap_frame`'in geçerli bir bellek adresi olduğunu garanti etmesi gerekir.
#[no_mangle] // Assembly tarafından çağrılabilmesi için isim düzenlemesi yapılmaz
pub extern "C" fn trap_handler(trap_frame: *mut TrapFrame) {
    // trap_frame pointer'ının geçerli olduğunu varsayarak güvenli bir referans alıyoruz.
    // Gerçek bir çekirdekte, bu pointer'ın geçerliliği görev bağlamına göre daha sıkı kontrol edilmelidir.
    let trap_frame: &mut TrapFrame = unsafe { &mut *trap_frame };

    // scause CSR'ından tuzak nedenini oku
    let cause = scause::read();

    // sepc CSR'ından kesintiye uğrayan/tuzak oluşturan komutun adresini oku
    // Bu değer trap_frame'e zaten kaydedilmiş olmalı, ancak güncel değeri okumak da faydalı olabilir.
    let sepc_val = sepc::read();
    // stval CSR'ından tuzakla ilgili ek bilgi (örn. sayfa hatasında sanal adres) oku
    let stval_val = stval::read();

    // TrapFrame'deki CSR alanlarını güncelle (assembly kaydetmemişse veya emin olmak için)
    trap_frame.sepc = sepc_val;
    trap_frame.sstatus = sstatus::read().bits(); // sstatus'ı usize olarak kaydetmek için bitlerini al
    trap_frame.stval = stval_val;
    trap_frame.scause = cause.bits(); // scause'u usize olarak kaydetmek için bitlerini al

    match cause.cause() {
        // --- Eşzamanlı İstisnalar (Exceptions) ---
        Trap::Exception(Exception::UserEnvCall) | Trap::Exception(Exception::SupervisorEnvCall) => {
            // Sistem Çağrısı (ECALL)
            // RISC-V'de ecall komutu bir istisnaya neden olur.
            // Syscall numarası genellikle a7 (x17) kaydındadır.
            // Argümanlar a0-a5 (x10-x15) kayıtlarındadır.

            // Syscall numarasını al (x17 -> regs[16])
            let syscall_number = trap_frame.regs[16];

            // Argümanları al (a0-a5 -> regs[9] - regs[14])
            let arg1 = trap_frame.regs[9] as u64;
            let arg2 = trap_frame.regs[10] as u64;
            let arg3 = trap_frame.regs[11] as u64;
            let arg4 = trap_frame.regs[12] as u64;
            let arg5 = trap_frame.regs[13] as u64; // RISC-V ABI'de 6 argüman olabilir (a0-a5)

            // Karnal64 API'sındaki handle_syscall fonksiyonunu çağır
            // Bu fonksiyon KError'ları negatif i64 olarak döndürür.
            let syscall_result = karnal64::handle_syscall(
                syscall_number as u64, // Syscall numarası u64 olmalı
                arg1, arg2, arg3, arg4, arg5
            );

            // Sistem çağrısı sonucunu a0 (x10) kaydına yaz
            // Kullanıcı alanı buradaki değeri okuyacaktır.
            trap_frame.regs[9] = syscall_result as usize; // i64 sonucu usize'a çevir (dikkatli olunmalı)

            // Ecall komutu sadece bir komut sürer. Geri döndüğümüzde aynı komutu tekrar çalıştırmamak için
            // sepc'yi 4 (komut boyutu) artırarak bir sonraki komuta atlamalıyız.
            trap_frame.sepc += 4;

            // Başarılı sistem çağrısı işleme sonrası buradan devam eder.
            // Handle_syscall içinde görev değişimi (yield/sleep) olabilir, o durumda
            // scheduler yeni görevin trap_frame'ini ayarlar ve assembly o frame'i restore eder.
        }
        Trap::Exception(Exception::LoadPageFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::InstructionPageFault) =>
        {
            // Sayfa Hatası (Page Fault)
            // stval kaydında hata oluşturan sanal adres bulunur.
            let fault_address = stval_val;
            // sepc kaydında hata oluşturan komutun adresi bulunur.
            let fault_pc = sepc_val;

            // TODO: Bellek yöneticisini çağırarak sayfa hatasını çözmeye çalış.
            // Karnal64'ün kmemory modülünü kullanın.
            // Örnek: kmemory::handle_page_fault(fault_address, cause, trap_frame);
            let handled = kmemory::handle_page_fault(fault_address, cause.bits(), trap_frame);

            if !handled {
                // Sayfa hatası çözülemedi, bu kritik bir hata.
                // Görevi sonlandır veya panik yap.
                println!("Page Fault: addr={:x}, pc={:x}, cause={:?}", fault_address, fault_pc, cause.cause());
                // TODO: Karnal64'ün ktask modülünü kullanarak mevcut görevi sonlandır.
                ktask::exit_current_task(KError::BadAddress as i64); // Örnek hata kodu
            }

            // Sayfa hatası çözüldüyse (örneğin, sayfa eşlendi),
            // sepc değişmez, aynı komut tekrar çalıştırılır ve bu sefer hata oluşmaz.
        }
        // TODO: Diğer istisna türlerini ekleyin (örn. hizalama hataları, geçersiz komut vb.)
        // Bunların çoğu görev sonlandırmayla sonuçlanabilir.
        Trap::Exception(_) => {
             // Bilinmeyen veya işlenmeyen istisna
             println!("Unhandled Exception: cause={:?}, pc={:x}, stval={:x}", cause.cause(), sepc_val, stval_val);
             // TODO: Karnal64'ün ktask modülünü kullanarak mevcut görevi sonlandır.
             ktask::exit_current_task(KError::InternalError as i64); // Örnek hata kodu
        }

        // --- Zaman Uyumsuz Kesmeler (Interrupts) ---
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            // Süpervizör Zamanlayıcı Kesmesi
            // Bu kesme, zamanlayıcı (genellikle CLINT) tarafından periyodik olarak üretilir
            // ve görev zamanlama için kullanılır.

            // TODO: Zamanlayıcı kesmesini CLINT'te veya ilgili donanımda temizle.
            // Bu çok önemli, yoksa kesme tekrar tekrar tetiklenir.
             use riscv::register::mtimecmp; // CLINT için mtimecmp yazma
             let next_timer = get_current_time() + TIMER_INTERVAL;
             unsafe { mtimecmp::write(next_timer); }

            // TODO: Zamanlayıcı tikini işle ve görev zamanlayıcıyı (scheduler) çalıştır.
            // Karnal64'ün ktask modülünü kullanın.
            ktask::timer_tick(trap_frame); // Scheduler bu fonksiyonda görev değiştirebilir

            // Kesme işleyiciden geri döndüğümüzde sepc'nin ayarlanmasına gerek yok,
            // çünkü kesintiye uğrayan komutun kaldığı yerden devam etmesi gerekir.
        }
        Trap::Interrupt(Interrupt::SupervisorExternal) => {
            // Süpervizör Harici Kesme
            // Genellikle PLIC (Platform-Level Interrupt Controller) tarafından yönlendirilen
            // harici cihazlardan (UART, disk, ağ kartı vb.) gelen kesmeler.

            // TODO: PLIC'i sorgulayarak hangi cihazın kesme ürettiğini bul.
            // TODO: İlgili cihaz sürücüsünün kesme işleyicisini çağır.
            // Sürücüler Karnal64'ün ResourceProvider traitini implemente edebilir ve
            // kesmeleri işlemek için özel bir mekanizmaya sahip olabilirler.
            // Örnek: let interrupt_id = plic::claim();
                    resource_manager::dispatch_interrupt(interrupt_id); // ResourceProvider traitinin parçası olabilir
                    plic::complete(interrupt_id); // Kesmeyi PLIC'te tamamla

            println!("Unhandled Supervisor External Interrupt");
            // İşlenmeyen kesmeler genellikle bir hata değildir, sadece göz ardı edilebilir
            // veya bir debug mesajı verilebilir.
        }
        // TODO: Diğer kesme türlerini ekleyin (örn. yazılım kesmesi, makine kesmeleri)
        Trap::Interrupt(_) => {
             // Bilinmeyen veya işlenmeyen kesme
             println!("Unhandled Interrupt: cause={:?}", cause.cause());
             // Genellikle bir hata değildir, sadece bilgilendirmedir.
        }
    }

    // Buraya ulaşıldığında, tuzak/kesme işlenmiş demektir.
    // Assembly kodu, trap_frame'deki kayıtlı durumları yükleyerek geri dönecektir.
    // Eğer ktask::timer_tick veya handle_syscall içinde görev değişimi olduysa,
    // assembly yeni görevin trap_frame'ini yükleyecektir.
}

// --- Başlatma Fonksiyonu ---
// Çekirdek başlangıcında (boot) çağrılarak tuzak işleyiciyi ayarlar.
pub fn init() {
    // RISC-V'de tuzak vektör kaydı (mtvec veya stvec) ayarlanır.
    // Çekirdek, Süpervizör modunda (S-mode) çalıştığı için `stvec` kullanılır.
    // `stvec`'e assembly giriş noktasının adresi yazılır.
    // `stvec` formatı: | BASE [31:2] | MODE [1:0] |
    // MODE 0 (Direct): Tüm tuzaklar BASE adresine gider.
    // MODE 1 (Vectored): Kesmeler (Interrupts) BASE + cause * 4 adresine gider.
    // Genellikle Direct mode (0) kullanılır ve tek bir Rust/Assembly işleyiciye dallanılır.

    // `trap_entry` assembly etiketinin adresini al
    extern "C" {
        fn trap_entry(); // Assembly kodumuzdaki giriş noktası
    }

    let trap_entry_address = trap_entry as usize;

    // stvec'e adresi ve Direct mode'u yaz
    unsafe {
         riscv::register::stvec::write(trap_entry_address, stvec::TrapMode::Direct);
        // Alternatif olarak ham CSR yazma:
        core::arch::asm!("csrw stvec, {0}", in(reg) trap_entry_address, options(nostack));
    }

    // Süpervizör Yazılım Kesmeleri (SSIP) için bir kesme kurabiliriz.
    // Mesela ilk görevi başlatırken veya görevler arası iletişimde kullanılabilir.
    // Supervisor External Interrupts (SEIE) de harici kesmeler için aktif edilmelidir.
    // Supervisor Timer Interrupts (STIE) zamanlayıcı için aktif edilmelidir.

    // sstatus kaydındaki Süpervizör Kesme Etkin (SIE) bitini etkinleştirerek
    // genel kesmeleri açabiliriz. (Dikkat: Bunu çok erken yapmayın!)
     unsafe { sstatus::set_sie(); }

    println!("RISC-V Tuzak İşleyici Başlatıldı.");
}

// --- Yardımcı/Placeholder Fonksiyonlar (Kendi Karnal64 modüllerinizde bulunmalı) ---

// Karnal64 modülleri için dummy implementasyonlar (Bu dosyada bulunmaz, sadece referans amaçlıdır)
mod kmemory {
    use super::*;
    // Bu fonksiyon kmemory modülünüzde olacak
    #[allow(unused_variables)]
    pub fn handle_page_fault(fault_address: usize, cause_bits: usize, trap_frame: &mut TrapFrame) -> bool {
        // Gerçek sayfa hatası işleme mantığı (talep üzerine yükleme, copy-on-write vb.)
        // Çözülürse true, çözülemezse false döner.
        println!("KMemory: Sayfa hatası işlenmeye çalışılıyor (addr={:x})... (Yer Tutucu)", fault_address);
        // Örnek: Her hatayı çözülememiş say
        false // Gerçekte burada adres alanlarına bakıp sayfa ayırma/eşleme yapılır.
    }
     // kmemory diğer fonksiyonları...
}

mod ktask {
    use super::*;
     // Bu fonksiyonlar ktask modülünüzde olacak
    #[allow(unused_variables)]
    pub fn exit_current_task(exit_code: i64) -> ! {
        // Mevcut görevi sonlandırma mantığı
        // Bağlamı serbest bırak, kaynakları temizle, scheduler'a bildir.
        // Bu fonksiyon asla geri dönmez.
        println!("KTASK: Görev sonlandırılıyor (exit_code={})... (Yer Tutucu)", exit_code);
        loop {} // Gerçekte scheduler başka bir göreve geçer.
    }

    #[allow(unused_variables)]
    pub fn timer_tick(trap_frame: &mut TrapFrame) {
        // Zamanlayıcı tikini işle, scheduler'ı çalıştır.
        println!("KTASK: Zamanlayıcı tik işleniyor... (Yer Tutucu)");
        // Scheduler burada görev değiştirmeye karar verebilir.
        // Eğer görev değişirse, trap_frame pointer'ı scheduler tarafından
        // yeni görevin çekirdek stack'indeki trap frame'ine ayarlanır.
    }
    // ktask diğer fonksiyonları...
}
