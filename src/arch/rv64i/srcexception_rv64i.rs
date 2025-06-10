#![no_std] // Standart kütüphane yok
#[allow(dead_code)]
#[allow(unused_variables)]
#[allow(unused_imports)] 

// CSR'lara erişim için gerekli crate
use riscv::register::*;
use riscv::register::scause::{Exception, Interrupt, Trap};

// Kernel genel tipleri ve API'sine erişim
// Bu path, projenizin Karnal64 API'sını nerede tanımladığına bağlı olacaktır.
// Örneğin, Karnal64 API'sı src/karnal64/api.rs içinde ise burası 'crate::karnal64::api' olabilir.
// Şimdilik varsayımsal bir 'super' veya 'crate::karnal64' kullanıyoruz.
use crate::karnal64::{self, KError}; // Karnal64'ün temel tipleri ve handle_syscall fonksiyonu için

// Kaydedilmiş kullanıcı bağlamını (registerları) temsil eden yapı.
// Assembly kodu, trap anında tüm general-purpose registerları, sepc ve sstatus'ı bu yapıya kaydetmelidir.
// Register sıralaması ve adlandırması RISC-V ABI'sine uygun olmalıdır.
#[repr(C)] // C ABI'sine uygun bellek düzeni
pub struct TrapContext {
    // x0 (zero) kaydedilmez, her zaman 0'dır.
    // x1 (ra - return address)
    // x2 (sp - stack pointer)
    // x3 (gp - global pointer)
    // x4 (tp - thread pointer)
    // x5-x7 (t0-t2 - temporaries)
    // x8 (s0/fp - saved register/frame pointer)
    // x9 (s1 - saved register)
    // x10-x17 (a0-a7 - function arguments/return values)
    // x18-x27 (s2-s11 - saved registers)
    // x28-x31 (t3-t6 - temporaries)
    pub x: [usize; 32], // RISC-V'nin 32 genel amaçlı registerı için (x0-x31) - x0 kullanılmasa da ABI hizalaması için diziye dahil edilebilir

    pub sstatus: sstatus::Sstatus, // sstatus registerı (SPP, SPIE vb. içerir)
    pub sepc: usize,              // sepc registerı (trap'a neden olan/sonraki komutun adresi)

    // Eğer FPU kullanılıyorsa:
     pub f: [usize; 32], // f0-f31 floating point registerları
     pub fcsr: usize,    // fcsr registerı
}

impl TrapContext {
    // Trap bağlamından sistem çağrısı numarasını ve argümanlarını alır.
    // Varsayılan RISC-V RV64G ABI'sine göre:
    // syscall numarası a7'de (x17) bulunur.
    // argümanlar a0-a5'te (x10-x15) bulunur.
    pub fn syscall_args(&self) -> (u64, u64, u64, u64, u64, u64) {
        let syscall_num = self.x[17] as u64; // a7
        let arg1 = self.x[10] as u64; // a0
        let arg2 = self.x[11] as u64; // a1
        let arg3 = self.x[12] as u64; // a2
        let arg4 = self.x[13] as u64; // a3
        let arg5 = self.x[14] as u64; // a4
        // a5 (x15) de bazı ABI'lerde 6. argüman olarak kullanılabilir,
        // Karnal64 API'sı 5 argüman tanımlamış, buna göre 5'e kadar alalım.
        (syscall_num, arg1, arg2, arg3, arg4, arg5)
    }

    // Sistem çağrısı sonucunu a0 registerına yazar.
    // Karnal64 API'si i64 döndürür (pozitif/sıfır başarı, negatif hata).
    pub fn set_syscall_return_value(&mut self, value: i64) {
        self.x[10] = value as usize; // a0
    }
}


/// Çekirdek Trap (Kesme/İstisna) İşleyicisi.
/// Bu fonksiyon, Assembly'de yazılmış alt seviye trap giriş noktasından çağrılır.
/// Kaydedilmiş kullanıcı bağlamını (registerları) argüman olarak alır.
/// #[no_mangle] özniteliği, Rust derleyicisinin fonksiyon adını değiştirmesini engeller,
/// böylece Assembly kodu bu fonksiyonu kolayca çağırabilir.
/// extern "C" özniteliği C çağrı kuralını kullanmasını sağlar, bu da Assembly ile uyumluluk için yaygındır.
#[no_mangle]
pub extern "C" fn riscv_trap_handler(trap_cx: &mut TrapContext) {
    // Trap'ın nedenini ve detaylarını oku
    let cause = scause::read();
    let stval = stval::read();

    // scause registerının en üst bitine bakarak interrupt mı exception mı kontrol et
    match cause.cause() {
        Trap::Exception(exception) => {
            // Bu bir istisna (synchronous trap)
            // sepc, istisnanın oluştuğu komutun adresini tutar.
            match exception {
                Exception::EnvironmentCallU => { // Kullanıcı alanından ecall (sistem çağrısı)
                    // 1. Sistem çağrısı numarasını ve argümanlarını TrapContext'ten al
                    let (syscall_num, arg1, arg2, arg3, arg4, arg5) = trap_cx.syscall_args();

                    // 2. Karnal64 API'sındaki sistem çağrısı işleyiciyi çağır
                    // Karnal64 handle_syscall fonksiyonu i64 döndürmek üzere tasarlanmıştı.
                    let result: i64 = karnal64::handle_syscall(syscall_num, arg1, arg2, arg3, arg4, arg5);

                    // 3. Sistem çağrısı sonucunu kullanıcının a0 registerına (TrapContext'e) yaz
                    trap_cx.set_syscall_return_value(result);

                    // 4. ecall komutu 4 byte'tır. Sistem çağrısı tamamlandıktan sonra
                    // program akışının ecall komutundan sonraki komuttan devam etmesi için
                    // sepc'yi 4 artırmalıyız.
                    trap_cx.sepc += 4;
                }
                Exception::LoadPageFault | Exception::StorePageFault | Exception::InstructionPageFault => {
                    // Bellek sayfa hatası (page fault)
                    println!("Load/Store/Instruction Page Fault: sepc = {:#x}, stval = {:#x}", trap_cx.sepc, stval);
                    // TODO: Bellek yöneticisine (kmemory) page fault'u handle etmesi için çağrı yap
                    // Örneğin: match kmemory::handle_page_fault(trap_cx.sepc, stval, exception) { ... }
                    // Eğer bellek yöneticisi hatayı gideremezse (örn. geçersiz adres),
                    // görevi sonlandırmamız veya paniklememiz gerekir.
                    panic!("TODO: Implement Page Fault Handler!");
                }
                Exception::IllegalInstruction => {
                    // Geçersiz komut hatası
                    println!("Illegal Instruction: sepc = {:#x}, stval = {:#x}", trap_cx.sepc, stval);
                    // TODO: Görevi sonlandır (ktask::kill_task(current_task_id()))
                    panic!("TODO: Implement Illegal Instruction Handler!");
                }
                // TODO: Diğer istisna türlerini burada ele al (breakpoint, alignment fault vb.)
                _ => {
                    // Desteklenmeyen veya bilinmeyen istisna türü
                    println!("Unknown Exception: {:?}, sepc = {:#x}, stval = {:#x}", exception, trap_cx.sepc, stval);
                    panic!("Unhandled exception!");
                }
            }
        }
        Trap::Interrupt(interrupt) => {
            // Bu bir kesme (asynchronous trap)
            // sepc, kesmenin meydana geldiği sırada çalışan komutun adresini tutar.
            match interrupt {
                Interrupt::SupervisorTimer => {
                    // Zamanlayıcı kesmesi
                    // TODO: Zamanlayıcı cihazını sıfırla veya bir sonraki kesme zamanını ayarla.
                    // Örneğin, CLINT'in mtimecmp registerına yeni bir değer yaz.
                    // Aksi takdirde aynı kesme tekrar tekrar tetiklenir.
                     clint::set_next_timer(1_000_000); // 1 milyon döngü sonra

                    // TODO: Görev zamanlayıcısını (scheduler) uyar.
                    // Zaman dilimi (time slice) dolmuş olabilir, bağlam değiştirme gerekebilir.
                     ktask::timer_tick();

                    // Kesmelerde sepc'yi manuel artırmaya gerek yoktur,
                    // mret/sret komutu sepc'den yürütmeye devam eder.
                }
                // TODO: Diğer kesme türlerini burada ele al (external interrupt vb.)
                 Interrupt::SupervisorExternal => {
                     // Harici (örneğin cihaz) kesmesi
                     // TODO: Kesme kaynağını (PLIC vb.) sorgula ve ilgili sürücünün kesme işleyicisini çağır.
                      plic::handle_external_interrupt();
                 }
                _ => {
                    // Desteklenmeyen veya bilinmeyen kesme türü
                    println!("Unknown Interrupt: {:?}, sepc = {:#x}", interrupt, trap_cx.sepc);
                    panic!("Unhandled interrupt!");
                }
            }
        }
    }

    // Trap handler'dan çıkış, Assembly stub tarafından TrapContext kullanılarak yapılır.
    // sret komutu sepc'deki adrese döner ve sstatus'taki SPP bitine göre doğru moda geçer.
}

/// Trap sistemini başlatmak için fonksiyon.
/// Çekirdek başlatma sırasında çağrılır.
pub fn init() {
    // TODO: Assembly'de yazılmış trap giriş noktasının adresini stvec registerına yaz.
    // Assembly giriş noktasının sembol adı '_trap_entry' veya benzeri bir şey olabilir.
    // Bu sembolün adresini alıp stvec'e yazmalısınız.
    // Assembly kodu, trap olduğunda _trap_entry adresine atlar.
    // Orada registerları kaydeder, trap_cx yapısını oluşturur ve riscv_trap_handler fonksiyonunu çağırır.
    // Örneğin (varsayımsal bir assembly sembolü varsayımıyla):
     extern "C" { fn _trap_entry(); }
     stvec::write(_trap_entry as usize, TrapMode::Direct); // Veya VECTORED eğer kullanılıyorsa

    // Örnek bir yer tutucu:
    let trap_entry_address = 0x80200000; // Bu gerçek Assembly giriş noktasının adresi OLMALIDIR!
    unsafe { stvec::write(trap_entry_address, stvec::TrapMode::Direct) };

    println!("RISC-V Trap Sistemi Başlatıldı (stvec = {:#x})", trap_entry_address);

    // TODO: Supervisor External Interrupt (SEIE) ve Supervisor Timer Interrupt (STIE)
    // gibi temel kesmeleri SSTATUS registerında etkinleştirmeniz gerekebilir,
    // eğer bunlar çekirdek tarafından kullanılacaksa. MIE registerı machine mode içindir.
     unsafe { sstatus::set_sie() }; // Supervisor Interrupt Enable
}
