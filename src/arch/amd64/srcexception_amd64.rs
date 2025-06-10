#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz
 #![allow(dead_code)]
 #![allow(unused_variables)]

// x86_64 mimarisine özel yapılar ve özellikler için crate'leri kullanabiliriz.
// Gerçek bir projede bu crate'ler çekirdeğe dahil edilmelidir.
// Bu örnekte, bu crate'lerin sağladığı yapıları kavramsal olarak kullanıyoruz
// veya kendi basit implementasyonlarımızı yapıyoruz.
 use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
 use x86_64::PrivilegeLevel;

// Karnal64 API'sından gerekli fonksiyonları kullanacağımızı varsayalım.
// Örneğin, handle_syscall fonksiyonu ve KError gibi tipler.
// Gerçek implementasyonda bu 'extern crate' veya 'use' ifadeleri çekirdek yapısına göre değişir.
 use crate::karnal64::{handle_syscall, KError, KTaskId};
 use crate::ktask; // Görev yönetimi için
 use crate::kmemory; // Bellek yönetimi için
 use crate::kkernel; // Çekirdek bilgisi/durdurma için
 use crate::klog; // Geçici loglama/debug çıktısı için

// --- Sabitler ve Tanımlar ---

// İstisna (Exception) Vektör Numaraları (Intel/AMD manuelinden)
const DIVIDE_ERROR_VECTOR: u8 = 0x0;
const DEBUG_VECTOR: u8 = 0x1;
const NON_MASKABLE_INTERRUPT_VECTOR: u8 = 0x2;
const BREAKPOINT_VECTOR: u8 = 0x3;
const OVERFLOW_VECTOR: u8 = 0x4;
const BOUND_RANGE_EXCEEDED_VECTOR: u8 = 0x5;
const INVALID_OPCODE_VECTOR: u8 = 0x6;
const DEVICE_NOT_AVAILABLE_VECTOR: u8 = 0x7;
const DOUBLE_FAULT_VECTOR: u8 = 0x8;
const INVALID_TSS_VECTOR: u8 = 0xA;
const SEGMENT_NOT_PRESENT_VECTOR: u8 = 0xB;
const STACK_SEGMENT_FAULT_VECTOR: u8 = 0xC;
const GENERAL_PROTECTION_FAULT_VECTOR: u8 = 0xD;
const PAGE_FAULT_VECTOR: u8 = 0xE;
const X87_FLOATING_POINT_VECTOR: u8 = 0x10;
const ALIGNMENT_CHECK_VECTOR: u8 = 0x11;
const MACHINE_CHECK_VECTOR: u8 = 0x12;
const SIMD_FLOATING_POINT_VECTOR: u8 = 0x13;
const VIRTUALIZATION_VECTOR: u8 = 0x14;
const CONTROL_PROTECTION_VECTOR: u8 = 0x15;

// Donanım Kesme (IRQ) Vektör Numaraları
// Genellikle PIC (Programmable Interrupt Controller) veya APIC (Advanced PIC) tarafından yönetilir.
// Yaygın olarak IRQ'lar 0x20'den başlayarak vektörlere haritalanır (örn. PIC offset 0x20).
const PIC_1_OFFSET: u8 = 0x20;
const TIMER_INTERRUPT_VECTOR: u8 = PIC_1_OFFSET + 0; // IRQ0
const KEYBOARD_INTERRUPT_VECTOR: u8 = PIC_1_OFFSET + 1; // IRQ1
// ... diğer IRQ'lar ...

// Sistem Çağrısı (Syscall) Vektör Numarası (yaygın olarak kullanılır)
const SYSCALL_VECTOR: u8 = 0x80;

// --- Çekirdek Bellek Düzeni ve Stack Yapısı (Kavramsal) ---
// İstisna/Kesme meydana geldiğinde, CPU mevcut görev/iş parçacığının stack'ine
// belirli bilgileri (RIP, CS, RFLAGS, RSP, SS ve bazı istisnalar için hata kodu) push eder.
// Düşük seviyeli assembly işleyicilerimiz (stubs), bu bilgileri ve diğer kaydedilmiş
// yazmaçları (register) alıp, Rust handler fonksiyonlarımıza uygun bir yapıya (stack frame) dönüştürür.

/// CPU tarafından veya ara assembly kodu tarafından stack'e push edilen bilgilerin temsili.
/// x86-interrupt çağırma kuralı, bu yapıyı otomatik olarak yönetir.
/// Ancak, hangi istisnanın hata kodu push ettiğini bilmek önemlidir.
#[repr(C)] // C uyumlu bellek düzeni
#[derive(Debug)]
pub struct InterruptStackFrame {
    /// Hata kodu, yalnızca bazı istisnalar tarafından push edilir.
    /// Assembly stub'ların, hata kodu push etmeyen istisnalar için 0 veya
    /// başka bir belirteç push ettiği varsayılır.
    pub error_code: Option<u64>, // Daha robust bir sistemde bu field burada olmaz,
                                 // assembly stub'lar error code durumuna göre farklı
                                 // Rust fn imzalarını çağırır veya dummy bir değer koyar.
                                 // Bu örnekte, Option kullanarak esneklik simüle ediliyor.

    /// Instruction Pointer: İstisnanın/kesmenin meydana geldiği anki komutun adresi.
    pub instruction_pointer: u64,
    /// Code Segment: Komutun ait olduğu kod segmenti seçicisi.
    pub code_segment: u64,
    /// CPU Flags: İstisnanın meydana geldiği anki RFLAGS yazmacının değeri.
    pub cpu_flags: u64,
    /// Stack Pointer: İstisnanın meydana geldiği anki stack pointer (RSP) değeri.
    pub stack_pointer: u64,
    /// Stack Segment: İstisnanın meydana geldiği anki stack segmenti seçicisi.
    pub stack_segment: u64,

    // Çoğu sistemde assembly stub, ayrıca tüm genel amaçlı yazmaçları (registers)
    // bu yapının *önüne* veya *arkasına* kaydeder.
    // Bu örnekte basitlik adına yazmaçlar burada gösterilmemiştir, ancak gerçekte
    // RegisterState gibi bir struct burada veya ayrı bir argüman olarak olmalıdır.
}

// --- İstisna/Kesme İşleyicileri (Handlers) ---

// x86-interrupt çağırma kuralı, bu fonksiyonların CPU tarafından doğru şekilde
// çağrılmasını ve stack'in yönetilmesini sağlar.
// 'naked' fonksiyonlar assembly stub'lar için kullanılabilir ancak Rust'ta
// 'x86-interrupt' ile genellikle gerek kalmaz.

/// Tüm istisna ve kesmeler için ortak giriş noktası (Assembly stub'lar tarafından çağrılır).
/// Assembly stub'un, vector numarasını ve hata kodunu (varsa) stack frame'den
/// ayıklayıp bu fonksiyona argüman olarak verdiğini varsayıyoruz.
/// Daha yaygın bir yöntem, her vector için ayrı bir `extern "x86-interrupt" fn` tanımlayıp,
/// bunların kendi içinde veya ortak bir helper fonksiyonu çağırarak işi yapmasıdır.
/// Bu örnekte ikinci yöntemi (`extern "x86-interrupt" fn` + helper) takip edelim.

// Farklı istisna tipleri için özel `extern "x86-interrupt" fn` imzaları:
// Hata kodu PUSH edenler: Double Fault, Invalid TSS, Segment Not Present, Stack Segment Fault,
// General Protection Fault, Page Fault, Alignment Check, Control Protection.
// Hata kodu PUSH etmeyenler: Diğerleri ve tüm IRQ'lar.

// Hata kodu PUSH etmeyen istisnalar ve IRQ'lar için imza
extern "x86-interrupt" fn basic_interrupt_handler(stack_frame: InterruptStackFrame) {
    // Vektör numarasını belirlemek için biraz daha karmaşık bir yapı gerekir
    // (örn. assembly stub'un push ettiği bir değer veya IDT lookup).
    // Basitlik adına, her handler'ı ayrı bir vektöre atayacağız.
    // Bu fonksiyon aslında *kullanılmayacak*, sadece imza örneği.
    // Gerçek handler'lar aşağıda `fn handle_...` şeklinde.
}

// Hata kodu PUSH eden istisnalar için imza
extern "x86-interrupt" fn error_code_interrupt_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    // Bu fonksiyon da aslında *kullanılmayacak*, sadece imza örneği.
}


/// Tüm istisna/kesme işleyicilerinin çağırdığı genel yönlendirici (dispatcher).
/// Assembly stub'ların, CPU'nun push ettiği `InterruptStackFrame` üzerine
/// bir `vector` numarası ve (varsa) bir `error_code` push ettiğini varsayıyoruz.
#[no_mangle] // Assembly stub tarafından çağrılacaksa no_mangle gerekir
pub extern "C" fn common_interrupt_handler(stack_frame: InterruptStackFrame, vector: u8) {
     // Eğer assembly stub error code push etmiyorsa, buraya sadece vector gelir.
     // Eğer assembly stub error code push ediyorsa, o zaman fonksiyon imzası değişir
     // veya assembly stub, error code'u stack frame'e dahil eder (yukarıdaki InterruptStackFrame tanımına bak).
     // En robust yol, assembly stub'ların farklı Rust fonksiyonlarını çağırması veya
     // tutarlı bir stack frame oluşturmasıdır.
     // Bu örnekte, assembly stub'un vector numarasını ve hata kodunu (bazıları için)
     // stack frame üzerine koyduğunu ve common_interrupt_handler'ın bu ek bilgileri
     // alacak şekilde tasarlandığını VEYA ayrı handler'ların doğrudan çağrıldığını varsayalım.
     // Daha temiz olması için, her vector için ayrı `extern "x86-interrupt" fn` tanımlayıp,
     // onların bu `handle_...` fonksiyonlarını çağırmasını sağlayalım.


    // CPU state (yazmaçlar) stack frame içinde veya ayrı bir yapıda olmalıdır.
    // Şu anki InterruptStackFrame tanımımız bunu içermiyor, gerçekte içermesi gerekir.
     let saved_registers = ...; // Stack frame'den veya ayrı bir yapıdan alınacak

    // Çekirdek durumunu (örneğin şu anki görev) kaydet/geçici olarak değiştir.
     ktask::save_current_task_state(&saved_registers); // Kavramsal

    let maybe_error_code: Option<u64> = match vector {
        // Hata kodu PUSH eden istisna vektörleri
        DOUBLE_FAULT_VECTOR | INVALID_TSS_VECTOR | SEGMENT_NOT_PRESENT_VECTOR |
        STACK_SEGMENT_FAULT_VECTOR | GENERAL_PROTECTION_FAULT_VECTOR | PAGE_FAULT_VECTOR |
        ALIGNMENT_CHECK_VECTOR | CONTROL_PROTECTION_VECTOR => {
            // Bu handler çağrıldığında stack frame'in ilk elemanı hata kodu OLMALIDIR.
            // Assembly stub'un bunu düzgünce ayarladığını varsayıyoruz.
            // `x86-interrupt` kuralı bunu otomatik halleder, `InterruptStackFrame` struct'ın
            // ilk alanı olarak `error_code` koymak doğru değildir.
            // Assembly stub'un `error_code`'u ayrı bir argüman olarak pass ettiğini varsayalım.
            // Bu durumda `common_interrupt_handler` imzası `(stack_frame: InterruptStackFrame, vector: u8, error_code: u64)` olurdu.
            // Ya da hata kodu olmayanlar için dummy 0 push edildiğini varsayalım.
            // En yaygın Rust kernel deseni: `extern "x86-interrupt" fn` stubs -> common handler `(stack_frame: InterruptStackFrame)` + vector numarasını lookup
            // VEYA: Hata kodlu ve kodsuz signature'lara sahip farklı `extern "x86-interrupt" fn`'ler.
            // İkinci yaklaşım daha doğru. Öyle yapalım ve common_interrupt_handler'ı ayıralım.

            // Bu `common_interrupt_handler` artık kullanılmayacak.
            // Her vektör için ayrı bir `extern "x86-interrupt" fn` ve bu fn'lerin içinden
            // uygun `handle_...` fonksiyonlarını çağırmak daha Rust-idiomatic.
            None // Placeholder, bu handler kaldırılacak
        },
        _ => None, // Diğer istisnalar ve IRQ'lar hata kodu PUSH etmez.
    };


    // CPU state'i stack frame'den veya ayrı bir yapıdan alıp,
    // Karnal64 görev yönetimi bağlamına (context) kaydetmek veya taşımak gerekir.
     let mut task_context = ktask::TaskContext::from_stack_frame(&stack_frame, &saved_registers); // Kavramsal

    match vector {
        PAGE_FAULT_VECTOR => {
            // Hata kodu ve Faulting Virtual Address (CR2 yazmacında) gereklidir.
            // let cr2: u64 = unsafe { core::arch::x86_64::_read_cr2() }; // CR2 okuma
            let error_code = maybe_error_code.expect("Page Fault should have an error code");
            // handle_page_fault(stack_frame, error_code, cr2);
            klog::error!("PAGE FAULT: addr={:#x}, code={:#x}, RIP={:#x}",
                        unsafe { core::arch::x86_64::_read_cr2() }, error_code, stack_frame.instruction_pointer);
            // Bellek yöneticisine danış, sayfayı eşle veya hatayı işle.
            // match kmemory::handle_page_fault(unsafe { core::arch::x86_64::_read_cr2() }, error_code) {
                 Ok(_) => { /* Başarılı şekilde sayfa eşlendi, devam et */ }
                 Err(_) => { /* Kritik hata, görevi sonlandır veya panik yap */
                            // ktask::terminate_current_task(KError::BadAddress); // Kavramsal
                             kkernel::panic("Page Fault: Cannot handle"); // Kavramsal
                         }
             }
             kkernel::panic("PAGE FAULT - UNHANDLED"); // Yer Tutucu Panik
        }
        GENERAL_PROTECTION_FAULT_VECTOR => {
             let error_code = maybe_error_code.expect("General Protection Fault should have an error code");
             klog::error!("GENERAL PROTECTION FAULT: code={:#x}, RIP={:#x}", error_code, stack_frame.instruction_pointer);
             // Genellikle ölümcül bir hata. Görevi sonlandır veya panik yap.
             // ktask::terminate_current_task(KError::PermissionDenied); // Kavramsal
             kkernel::panic("GENERAL PROTECTION FAULT"); // Yer Tutucu Panik
        }
        SYSCALL_VECTOR => {
             // Sistem çağrısı işleme. Yazmaçlardaki argümanlar alınmalı.
             // `handle_syscall` Karnal64 API'sından gelir.
             // Syscall numarasını ve argümanları stack frame içindeki yazmaçlardan almalıyız.
              let syscall_num = stack_frame.registers.rax; // Kavramsal yazmaç erişimi
              let arg1 = stack_frame.registers.rdi;
             // ... diğer argümanlar ...
             //
              let result = handle_syscall(syscall_num, arg1, arg2, arg3, arg4, arg5); // Karnal64 API çağrısı
             //
             // Sonucu kullanıcı alanının beklediği yazmaca (genellikle RAX) koy.
             // stack_frame.registers.rax = result as u64; // Kavramsal
             klog::debug!("SYSCALL received: vector={}", vector);
              handle_syscall_entry(stack_frame); // Ayrı bir helper'a yönlendirilebilir
             kkernel::panic("SYSCALL - UNIMPLEMENTED"); // Yer Tutucu Panik
        }
        TIMER_INTERRUPT_VECTOR => {
             klog::trace!("TIMER INTERRUPT");
             // Zamanlayıcı kesmesi, zamanlama ve görev değişimi için kullanılır.
             // Zamanlayıcı sayacını güncelle.
              ktimer::tick(); // Kavramsal
             // Zamanlayıcıyı sıfırla (eğer periyodik ise).
              ktimer::reset_timer(); // Kavramsal
             // Görev zamanlayıcıyı çalıştır (gerekiyorsa görev değişimi yapar).
              ktask::schedule(); // Kavramsal
             // Donanım kesmesini onayla (PIC/APIC'e EOI gönder).
              src_interrupts::acknowledge_irq(0); // Kavramsal (başka bir modülde)
        }
        BREAKPOINT_VECTOR => {
            klog::debug!("BREAKPOINT at RIP: {:#x}", stack_frame.instruction_pointer);
            // Hata ayıklama amacıyla kullanılabilir. Görevi duraklat veya bilgi yazdır.
            // Bu istisnanın hata kodu yoktur.
            // İzin verilirse görevi devam ettir.
        }
        DOUBLE_FAULT_VECTOR => {
            let error_code = maybe_error_code.expect("Double Fault should have an error code");
            klog::error!("DOUBLE FAULT: code={:#x}, RIP={:#x}", error_code, stack_frame.instruction_pointer);
            // Çift hata kurtarılamaz. Panik yap.
            kkernel::panic("DOUBLE FAULT"); // Yer Tutucu Panik
        }
        // Diğer istisnalar ve IRQ'lar için işleyiciler eklenecek...
        // Örneğin klavye kesmesi:
         KEYBOARD_INTERRUPT_VECTOR => {
             klog::debug!("KEYBOARD INTERRUPT");
        //     // Klavye sürücüsüne danış.
              src_drivers::keyboard::handle_interrupt(); // Kavramsal
        //     // Donanım kesmesini onayla.
              src_interrupts::acknowledge_irq(1); // Kavramsal
         }

        _ => {
            // Bilinmeyen veya unimplemented istisnalar/kesmeler.
            // Stack frame içeriğini yazdırıp panik yapmak genellikle en güvenlisidir.
            klog::error!("UNHANDLED INTERRUPT/EXCEPTION: Vector {}", vector);
            klog::error!("Stack Frame: {:?}", stack_frame);
            // Görev bağlamını da yazdırılabilir.
             klog::error!("Current Task State: {:?}", ktask::get_current_task_state()); // Kavramsal
            kkernel::panic("UNHANDLED INTERRUPT/EXCEPTION"); // Yer Tutucu Panik
        }
    }

    // Kesme/İstisna işlendikten sonra CPU state'i geri yükle
    // (Assembly stub bunu `iret`/`iretq` ile yapar).
     ktask::restore_task_state(&mut stack_frame, &mut saved_registers); // Kavramsal
}

/// Page Fault işleyicisinin yüksek seviye mantığı.
#[allow(unused_variables)]
fn handle_page_fault(stack_frame: &InterruptStackFrame, error_code: u64, faulting_address: u64) {
    klog::error!("PAGE FAULT [{:#x}] at {:#x} during access from {:#x}",
                error_code, faulting_address, stack_frame.instruction_pointer);

    use PageFaultErrorCode; // Tanımlanacak bir enum varsayımı

    let pf_error = PageFaultErrorCode(error_code);

    klog::error!("Error Code Flags: Present={}, Write={}, User={}, Reserved={}",
                pf_error.present(), pf_error.write(), pf_error.user(), pf_error.reserved());
    // Daha fazla bayrak olabilir (Instruction Fetch, Protection Key, SGX)

    // Örnek Mantık:
    // 1. Hatanın türünü belirle (sayfa yok, izin hatası vb.)
    // 2. Faulting Address (CR2) hangi bellek bölgesinde? (Kernel, User, Heap, Stack, Mapped File vb.)
    // 3. Eğer sayfa yoksa ve bu adres geçerli bir sanal bellek bölgesindeyse:
    //    - kmemory::handle_page_fault(faulting_address, error_code) çağrısı yap.
    //    - Bellek yöneticisi, fiziksel bir sayfa bulup eşlemeyi deneyebilir.
    //    - Başarılı olursa fonksiyondan normal dönülür, CPU eşlenen sayfaya erişmeyi tekrar dener.
    // 4. Eğer sayfa yoksa ve adres geçersizse veya izin hatasıysa:
    //    - Bu genellikle ölümcül bir hata. İlgili görevi sonlandır.
        ktask::terminate_current_task(KError::BadAddress); // Kavramsal

    // Yer Tutucu: Şimdilik sadece panik yapıyoruz.
    kkernel::panic("UNHANDLED PAGE FAULT");
}

/// General Protection Fault işleyicisinin yüksek seviye mantığı.
#[allow(unused_variables)]
fn handle_gp_fault(stack_frame: &InterruptStackFrame, error_code: u64) {
    klog::error!("GENERAL PROTECTION FAULT [{:#x}] at {:#x}",
                error_code, stack_frame.instruction_pointer);

    // Bu genellikle kurtarılamaz bir hatadır (segmentasyon hatası, izin ihlali vb.).
    // İlgili görevi sonlandır.
     ktask::terminate_current_task(KError::PermissionDenied); // Kavramsal

    // Yer Tutucu: Şimdilik sadece panik yapıyoruz.
    kkernel::panic("UNHANDLED GENERAL PROTECTION FAULT");
}

/// Sistem Çağrısı işleyicisinin yüksek seviye mantığı.
/// Syscall Convention: Yaygın olarak syscall numarası RAX'ta, argümanlar RDI, RSI, RDX, RCX, R8, R9'dadır.
/// Sonuç RAX'a konur.
#[no_mangle] // Assembly stub tarafından doğrudan çağrılacaksa no_mangle gerekir.
             // Veya common_interrupt_handler tarafından çağrılabilir.
             // Rust'ta `extern "x86-interrupt"` ile bu fn'leri tanımlayıp,
             // onların içinden Karnal64::handle_syscall'ı çağırmak daha iyi.
pub extern "x86-interrupt" fn syscall_entry(stack_frame: InterruptStackFrame) {
    // Saved registers are usually part of or pointed to by the stack_frame
    // in the x86-interrupt calling convention, or pushed by the assembly stub.
    // Assuming stack_frame somehow gives access to registers (e.g. via a `registers: Registers` field).
     let syscall_num = stack_frame.registers.rax; // Kavramsal
     let arg1 = stack_frame.registers.rdi;
     let arg2 = stack_frame.registers.rsi;
     let arg3 = stack_frame.registers.rdx;
     let arg4 = stack_frame.registers.r10; // veya RCX depends on convention
     let arg5 = stack_frame.registers.r8;
     let arg6 = stack_frame.registers.r9;

    // Sistem çağrısı numarasını ve argümanları al.
    // Gerçekte bu değerler stack_frame içindeki yazmaçlardan okunmalıdır.
    // Örnek olarak, stack_frame'de `rax`, `rdi`, ... alanları olduğunu varsayalım:
     let syscall_num = stack_frame.rax;
     let arg1 = stack_frame.rdi;
     let arg2 = stack_frame.rsi;
     let arg3 = stack_frame.rdx;
     let arg4 = stack_frame.r10; // rcx yerine r10 kullanılıyor bazı konvansiyonlarda
     let arg5 = stack_frame.r8;

    // Güvenlik Notu: user_buffer_ptr gibi argümanlar kullanıcı alanı pointer'larıdır.
    // Karnal64::handle_syscall'a geçirilmeden önce mutlaka kullanıcı alanında
    // geçerli ve izinli oldukları DOĞRULANMALIDIR! (Bellek Yönetimi Birimi yardımıyla).
    // Bu doğrulama `handle_syscall` içinde veya daha düşük seviyede yapılmalıdır.
    // Şimdilik doğrudan Karnal64 API'sına çağrıyı simüle edelim.

    // let result = handle_syscall(syscall_num, arg1, arg2, arg3, arg4, arg5); // Karnal64 API çağrısı

    klog::debug!("SYSCALL entry, calling handle_syscall...");

    // Sonucu kullanıcı alanına dönecek şekilde stack_frame'deki uygun yazmaca yaz (genellikle RAX).
    // stack_frame.rax = result as u64; // Kavramsal

    // Gerçekte Karnal64::handle_syscall bir i64 döner.
     stack_frame.rax = result as u64; // Başarı
     stack_frame.rax = (result as i64) as u64; // Hata kodu (negatif)

    // Yer Tutucu: Simüle edilmiş bir dönüş değeri (örn. 0 başarı)
     stack_frame.rax = 0; // Kavramsal

    // İşlem bitti, CPU normal şekilde geri dönecek (assembly stub'daki iretq ile).
}

/// Zamanlayıcı (Timer) kesmesi işleyicisinin yüksek seviye mantığı.
#[no_mangle] // Eğer assembly stub tarafından doğrudan çağrılıyorsa
pub extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    klog::trace!("Timer interrupt!");

    // Zamanlayıcı sayacını artır.
     kkernel::increment_timer(); // Kavramsal

    // Görev zamanlayıcıyı çalıştır, görev değişimi gerekebilir.
     ktask::schedule(); // Kavramsal

    // PIC (Programmable Interrupt Controller) veya APIC'e End Of Interrupt (EOI) sinyali gönder.
    // Bu, aynı kesmenin tekrar gelmesini sağlar. Eğer bunu yapmazsanız, timer kesmesi durur!
    // Eğer 8259 PIC kullanılıyorsa:
     const PIC_1_COMMAND: u16 = 0x20;
     const PIC_2_COMMAND: u16 = 0xA0;
     unsafe { x86_64::instructions::port::Port::new(PIC_1_COMMAND).write(0x20u8); } // EOI komutu
    // Eğer APIC kullanılıyorsa, APIC'e EOI yazılır.
     src_interrupts::send_eoi(TIMER_INTERRUPT_VECTOR - PIC_1_OFFSET); // Kavramsal helper

    // Yer Tutucu EOI simülasyonu:
      src_interrupts::send_eoi(0); // IRQ0 için
}

/// Bilinmeyen veya henüz özel işleyicisi olmayan kesme/istisna.
#[no_mangle]
pub extern "x86-interrupt" fn unknown_interrupt_handler(stack_frame: InterruptStackFrame) {
    // Vektör numarasını buradan doğrudan almak zor.
    // Assembly stub'un vektörü `common_interrupt_handler`'a geçirmesi en iyi yöntemdi.
    // Varsayım: Eğer bu handler çağrılıyorsa, vektör bilinmiyordur.
    klog::error!("UNKNOWN INTERRUPT!");
    klog::error!("Stack Frame: {:?}", stack_frame);
    // Panik yap veya hata ayıklama ekranına gir.
    kkernel::panic("UNKNOWN INTERRUPT");
}


// Diğer istisna/kesme handler'ları burada tanımlanacak (Debugger, NMI, Invalid Opcode vb.)
// Örnek: Invalid Opcode handler
 #[no_mangle]
 pub extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
     klog::error!("INVALID OPCODE at {:#x}", stack_frame.instruction_pointer);
//     // Görevi sonlandır.
//     // ktask::terminate_current_task(KError::InvalidArgument); // Kavramsal
     kkernel::panic("INVALID OPCODE");
 }


// --- IDT (Interrupt Descriptor Table) Tanımı ve Yükleme ---

// IDT, CPU'ya her istisna/kesme vektörü için hangi handler fonksiyonunun
// çağrılacağını ve hangi segment seçicisini kullanacağını (kernel kodu segmenti)
// ve hangi stack'e geçileceğini (IST - Interrupt Stack Table) söyleyen bir tablodur.

// Gerçek x86_64 IDT tanımı oldukça detaylıdır. Basit bir temsili:
struct IdtEntry {
    handler_low: u16,       // Handler adresinin düşük 16 biti
    gdt_selector: u16,      // Global Descriptor Table'daki kod segmenti seçicisi (kernel segmenti)
    options: u16,           // Bayraklar (P, DPL, D) ve IST (Interrupt Stack Table) indeksi
    handler_middle: u16,    // Handler adresinin orta 16 biti
    handler_high: u32,      // Handler adresinin yüksek 32 biti
    reserved: u32,          // Ayrılmış (0 olmalı)
}

impl IdtEntry {
    const NULL: Self = IdtEntry {
        handler_low: 0, handler_middle: 0, handler_high: 0,
        gdt_selector: 0, options: 0, reserved: 0,
    };

    // Bir handler fonksiyonu için IDT girişini ayarlar.
    // `handler_fn`: `extern "x86-interrupt" fn` veya uygun imza ile bir fonksiyon pointer'ı.
    // `kernel_code_selector`: GDT'deki kernel kod segmenti seçicisi.
    // `ist_index`: Kullanılacak Interrupt Stack Table indeksi (0-7), 0 ise kullanılmaz.
    // Gerçekte `options` bayrakları (Present, DPL, Trap/Interrupt Gate) da ayarlanmalıdır.
    fn set_handler_fn(&mut self, handler_fn: u64, kernel_code_selector: u16, ist_index: Option<u16>) {
        self.handler_low = handler_fn as u16;
        self.handler_middle = (handler_fn >> 16) as u16;
        self.handler_high = (handler_fn >> 32) as u32;
        self.gdt_selector = kernel_code_selector;

        // Options bayrakları (basit temsili):
        // P=1 (Present), DPL=0 (Kernel Privilege Level), D=1 (32-bit veya 64-bit Gate - 64-bit için 1)
        // Trap Gate (istisnayı devre dışı bırakmaz) veya Interrupt Gate (istisnayı devre dışı bırakır).
        // Genellikle kesmeler için Interrupt Gate, istisnalar için Trap Gate kullanılır.
        // Bu örnekte sabit bir değer kullanalım (Örn: Present, Kernel DPL, 64-bit Interrupt Gate).
        self.options = 0b1000_1110_0000_0000; // P=1, DPL=0, 0, Gate Type (1110 = 64-bit Interrupt Gate)

        // IST indeksi (Interrupt Stack Table):
        if let Some(index) = ist_index {
             // IST indisi options'ın düşük 3 bitinde tutulur (1-7 arası, 0 özel anlamı var).
             // Index 1-7 için geçerlidir. 0 ise IST kullanılmaz.
             if index >= 1 && index <= 7 {
                 self.options |= index;
             }
        }
    }
}

// IDT'yi statik olarak tanımla (256 giriş).
// 'static mut' kullanmak güvenli değildir, 'static' ve bir InitCell gibi yapılarla
// veya 'lazy_static' gibi bir crate ile güvenli init edilmelidir.
// Basitlik adına burada 'static mut' kullanılıyor, dikkatli olunmalı!
static mut IDT: [IdtEntry; 256] = [IdtEntry::NULL; 256];

/// Kernel kod segmenti seçicisi (GDT'den alınacak).
/// Gerçek bir kernelde GDT modülünden erişilir.
static KERNEL_CODE_SELECTOR: u16 = 8; // Örnek değer (genellikle GDT'nin 1. girdisi 0'dır, 2. girdisi code segmenti olabilir, 1*8=8)

/// IDT'yi yüklemek için kullanılan pointer yapısı (lidt komutu için).
#[repr(C, packed)]
struct IdtPointer {
    limit: u16, // IDT'nin boyutu - 1
    base: u64,  // IDT'nin başlangıç adresi
}

/// IDT'yi başlatır ve istisna/kesme işleyicilerini tabloya yerleştirir.
pub fn init() {
    klog::info!("Initializing IDT...");

    // IDT'ye erişim için unsafe blok gerekli.
    unsafe {
        // Tüm girişleri başlangıç değeriyle sıfırla (zaten yapıldı, ama tekrar etmek zarar vermez)
        for entry in IDT.iter_mut() {
            *entry = IdtEntry::NULL;
        }

        // Her istisna ve kesme vektörü için uygun handler'ı ayarla.
        // Her handler fonksiyonu için Assembly stub'lara giden pointerlar kullanılmalıdır.
        // Rust'ta 'extern "x86-interrupt" fn' ve bu fonksiyonların adreslerini almak
        // otomatik olarak doğru stub'ı veya doğrudan handler'ı işaret eder.

        // İstisna İşleyicileri (Genellikle hata kodu push edenler IST kullanır)
        // IST (Interrupt Stack Table) kullanarak çift hata gibi kritik istisnalarda
        // her zaman çalışan bir stack'e geçiş yapılır.
        IDT[DIVIDE_ERROR_VECTOR as usize].set_handler_fn(divide_error_handler as u64, KERNEL_CODE_SELECTOR, None);
        IDT[BREAKPOINT_VECTOR as usize].set_handler_fn(breakpoint_handler as u64, KERNEL_CODE_SELECTOR, None);
        IDT[INVALID_OPCODE_VECTOR as usize].set_handler_fn(invalid_opcode_handler as u64, KERNEL_CODE_SELECTOR, None);
        IDT[GENERAL_PROTECTION_FAULT_VECTOR as usize].set_handler_fn(gp_fault_handler as u64, KERNEL_CODE_SELECTOR, None);
        IDT[PAGE_FAULT_VECTOR as usize].set_handler_fn(page_fault_handler as u64, KERNEL_CODE_SELECTOR, None);

        // Double Fault için özel IST (Interrupt Stack Table) kullanılır.
        // IST indeksi (örneğin 1) IDT girişinde belirtilir.
        IDT[DOUBLE_FAULT_VECTOR as usize].set_handler_fn(double_fault_handler as u64, KERNEL_CODE_SELECTOR, Some(1)); // IST #1 kullan

        // Sistem Çağrısı İşleyicisi
        // Syscall genellikle Trap Gate olarak kurulur (RFLAGS'taki interrupt bayrağını temizlemez).
        // DPL=3 (Kullanıcı alanından çağrılabilir) olarak ayarlanmalıdır.
        // Bu entry için options bayrakları farklı olacaktır!
        IDT[SYSCALL_VECTOR as usize].set_handler_fn(syscall_entry as u64, KERNEL_CODE_SELECTOR, None); // DPL ve Gate tipi ayarlanmalı!
         IDT[SYSCALL_VECTOR as usize].options = 0b1110_1110_0000_0000 | (3 << 13); // Present, DPL=3, 64-bit Trap Gate


        // Donanım Kesme İşleyicileri (IRQ'lar)
        // PIC/APIC offset'ine dikkat edilmeli.
        IDT[TIMER_INTERRUPT_VECTOR as usize].set_handler_fn(timer_interrupt_handler as u64, KERNEL_CODE_SELECTOR, None);
         IDT[KEYBOARD_INTERRUPT_VECTOR as usize].set_handler_fn(keyboard_interrupt_handler as u64, KERNEL_CODE_SELECTOR, None);
        // ... diğer IRQ'lar ...

        // Henüz özel handler'ı olmayan tüm diğer vektörler için varsayılan handler'ı ayarla.
        // Bu, unhandled istisna/kesmelerde panik yapmamızı sağlar.
         for i in 0..256 {
             if IDT[i].handler_low == 0 && IDT[i].handler_middle == 0 && IDT[i].handler_high == 0 {
                 IDT[i].set_handler_fn(unknown_interrupt_handler as u64, KERNEL_CODE_SELECTOR, None);
             }
         }


        // IDT pointer yapısını doldur.
        let idt_ptr = IdtPointer {
            limit: (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: IDT.as_ptr() as u64,
        };

        // LIDT komutu ile IDT'yi CPU'ya yükle.
        core::arch::asm!("lidt ({})", in(reg) &idt_ptr, options(nostack));
    }

    klog::info!("IDT loaded successfully.");
}

// --- `extern "x86-interrupt" fn` ile Tanımlanmış Özel Handler'lar ---
// Bu fonksiyonlar, x86-interrupt çağırma kuralına göre çalışır ve CPU
// tarafından doğrudan veya assembly stub aracılığıyla çağrılabilir.
// Hata kodu push eden istisnalar için `error_code` argümanı otomatik eklenir.

extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    klog::error!("EXCEPTION: Divide Error at RIP: {:#x}", stack_frame.instruction_pointer);
    kkernel::panic("Divide Error");
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    klog::debug!("EXCEPTION: Breakpoint at RIP: {:#x}", stack_frame.instruction_pointer);
    // Hata ayıklama için kullanılır, normalde devam edilebilir.
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
     klog::error!("EXCEPTION: Invalid Opcode at RIP: {:#x}", stack_frame.instruction_pointer);
     kkernel::panic("Invalid Opcode");
}

extern "x86-interrupt" fn gp_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    handle_gp_fault(&stack_frame, error_code); // Yüksek seviye handler'ı çağır
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    // CR2 yazmacı faulting adresi içerir.
    let faulting_address: u64 = unsafe { core::arch::x86_64::_read_cr2() };
    handle_page_fault(&stack_frame, error_code, faulting_address); // Yüksek seviye handler'ı çağır
}

// Double Fault kurtarılamaz, özel bir stack (IST) kullanır ve panik yapmalıdır.
// Hata kodu PUSH eder.
extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) -> ! {
    klog::error!("EXCEPTION: DOUBLE FAULT! code={:#x}", error_code);
    klog::error!("Stack frame: {:?}", stack_frame);
    // Bu noktada kurtarma genellikle imkansızdır. Panik yap ve sistemi durdur.
    kkernel::panic_halt("DOUBLE FAULT"); // Varsayımsal bir panik ve durdurma fonksiyonu
}

// Diğer tüm istisna/kesme handler'ları benzer şekilde tanımlanabilir...
// Eğer bir vektör için özel bir handler tanımlanmadıysa, IDT init kısmında
// `unknown_interrupt_handler` atanır.

// --- Yardımcı Yapılar ve Fonksiyonlar ---

/// Page Fault Hata Kodu bayraklarını temsil eden yapı.
/// Bu yapı Page Fault handler'ında hata kodunu yorumlamak için kullanılır.
#[derive(Debug, Copy, Clone)]
#[repr(transparent)] // Sadece u64 olarak temsil edilir.
struct PageFaultErrorCode(u64);

impl PageFaultErrorCode {
    /// Page Present (P): 0 ise sayfa yoktu, 1 ise izin hatasıydı.
    fn present(&self) -> bool { (self.0 & (1 << 0)) != 0 }
    /// Write (W): 0 ise okuma, 1 ise yazma erişimi sırasında hata oldu.
    fn write(&self) -> bool { (self.0 & (1 << 1)) != 0 }
    /// User (U): 0 ise süpervizör, 1 ise kullanıcı modunda hata oldu.
    fn user(&self) -> bool { (self.0 & (1 << 2)) != 0 }
    /// Reserved (RSVD): 0 ise ayrılmış bit 0'dı, 1 ise 1'di (geçersiz sayfa tablosu girdisi).
    fn reserved(&self) -> bool { (self.0 & (1 << 3)) != 0 }
    /// Instruction Fetch (I/F): 0 ise veri erişimi, 1 ise komut getirme sırasında hata oldu. (Intel CPU'larda)
    fn instruction_fetch(&self) -> bool { (self.0 & (1 << 4)) != 0 }
    // Diğer bayraklar (Protection Key, SGX vb.) eklenebilir.
}


// --- PLACEHOLDER KERNEL BİLEŞENLERİ (Kavramsal Çağrılar) ---
// Bu fonksiyonlar veya modüller gerçek çekirdekte implemente edilmiş olmalıdır.
// `srcexception_x86.rs` bu bileşenleri çağırır.

mod klog {
    // Basit bir loglama makrosu simülasyonu
    macro_rules! println {
        ($($arg:tt)*) => {{
            // Gerçekte seri port, VGA veya başka bir hata ayıklama çıktısına yazılır.
            // core::fmt::Arguments::new_v1(...) ile formatlanır ve donanım yazıcısına gönderilir.
            // Şimdilik hiçbir şey yapmıyoruz veya stdout'a (varsa) yazıyoruz.
              #[cfg(feature = "debug_output")] { /* Yaz */ }
        }};
    }
    pub fn info(msg: &str) { println!("K_INFO: {}", msg); }
    pub fn debug(msg: &str) { println!("K_DEBUG: {}", msg); msg.len(); /* Kullanılmayan değişken uyarısını sustur */ } // msg kullanıldı
    pub fn trace(msg: &str) { println!("K_TRACE: {}", msg); msg.len(); /* Kullanılmayan değişken uyarısını sustur */ } // msg kullanıldı
    pub fn error(msg: &str) { println!("K_ERROR: {}", msg); msg.len(); /* Kullanılmayan değişken uyarısını sustur */ } // msg kullanıldı

    // Hata kodları ve argümanlarla formatlama için
     #[macro_export] // Dışarıdan kullanılabilir
     macro_rules! klog_error {
        ($fmt:expr, $($arg:tt)*) => {{
            $crate::srcexception_x86::klog::_error_fmt(core::format_args!($fmt, $($arg)*));
        }};
     }

     pub fn _error_fmt(args: core::fmt::Arguments) {
         // Formatlanmış argümanları alıp donanıma yazma mantığı
          #[cfg(feature = "debug_output")] { /* Yaz args */ }
          println!("K_ERROR: {}", args); // Basit simülasyon
         let _ = args; // Kullanılmayan değişken uyarısını sustur
     }
}

mod kkernel {
     // Çekirdek durdurma/panik fonksiyonları (kurtarılamaz hatalarda çağrılır)
     pub fn panic(msg: &str) -> ! {
         klog::error!("KERNEL PANIC: {}", msg);
         // Donanımı durduracak düşük seviye kod (örn. hlt döngüsü)
         loop { core::arch::asm!("hlt", options(nomem, nostack)); }
     }
     pub fn panic_halt(msg: &str) -> ! {
         klog::error!("KERNEL PANIC AND HALT: {}", msg);
         loop { core::arch::asm!("hlt", options(nomem, nostack)); }
     }
}

// Bellek yönetimi placeholder
#[allow(dead_code)]
mod kmemory {
     use super::*; // karnal64.rs scope'undaki tipleri kullan
     // Kavramsal sayfa hatası işleyici
     pub fn handle_page_fault(address: u64, error_code: u64) -> Result<(), KError> {
         klog::debug!("kmemory::handle_page_fault(addr={:#x}, code={:#x}) - Placeholder", address, error_code);
         // Gerçek implementasyon: sayfa tablosunu kontrol et, yeni sayfa ayır/eşle vb.
         Err(KError::BadAddress) // Şimdilik başarısız olduğunu varsayalım
     }
}

// Görev yönetimi placeholder
#[allow(dead_code)]
mod ktask {
      use super::*; // Karnal64 tipleri
     // Kavramsal görev zamanlayıcı ve sonlandırma
     pub fn schedule() { klog::trace!("ktask::schedule() - Placeholder"); }
      pub fn terminate_current_task(error: KError) {
          klog::error!("ktask::terminate_current_task with error: {:?}", error);
     //     // Gerçek implementasyon: Görevi sonlandır, kaynakları temizle, zamanlayıcıyı çalıştır.
          loop { core::arch::asm!("hlt"); } // Basit durdurma
      }
}

// Interrupts placeholder (EOI göndermek gibi)
#[allow(dead_code)]
mod src_interrupts {
     pub fn send_eoi(_irq: u8) {
         klog::trace!("src_interrupts::send_eoi({}) - Placeholder", _irq);
         // Gerçek implementasyon: PIC veya APIC'e EOI komutu gönderme.
     }
     // PIC offset'lerini ve port adreslerini burada tanımlayabilirsiniz.
      const PIC_1_COMMAND_PORT: u16 = 0x20;
      const PIC_2_COMMAND_PORT: u16 = 0xA0;
      const PIC_EOI_COMMAND: u8 = 0x20;
}


// Karnal64 API'sından handle_syscall fonksiyonu (prototip)
// Bu dosya bu fonksiyonu çağırır, bu fonksiyon Karnal64 API dosyasında implemente edilmiştir.
 extern "C" fn handle_syscall(
     number: u64,
     arg1: u64,
     arg2: u64,
     arg3: u64,
     arg4: u64,
     arg5: u64
 ) -> i64; // Başarıda pozitif/sıfır, hatada negatif KError değeri

// Varsayılan handler'ın kullanılabilmesi için `unknown_interrupt_handler` fonksiyon pointer'ını
// sağlamamız gerekiyor. Bu fonksiyon IDT'ye atanacak.
// `fn` olarak referans verilmesi, fonksiyon pointer'ı olarak kullanılmasını sağlar.
const UNKNOWN_HANDLER_PTR: u64 = unknown_interrupt_handler as u64;

// Diğer handler pointer'ları (IDT init'te kullanmak üzere)
const DIVIDE_ERROR_HANDLER_PTR: u64 = divide_error_handler as u64;
const BREAKPOINT_HANDLER_PTR: u64 = breakpoint_handler as u64;
const INVALID_OPCODE_HANDLER_PTR: u64 = invalid_opcode_handler as u64;
const GP_FAULT_HANDLER_PTR: u64 = gp_fault_handler as u64;
const PAGE_FAULT_HANDLER_PTR: u64 = page_fault_handler as u64;
const DOUBLE_FAULT_HANDLER_PTR: u64 = double_fault_handler as u64;
const SYSCALL_ENTRY_PTR: u64 = syscall_entry as u64;
const TIMER_INTERRUPT_HANDLER_PTR: u64 = timer_interrupt_handler as u64;

// Gerçek IDT init fonksiyonunu güncelle
pub fn init() {
    klog::info!("Initializing IDT...");

    // GDT'deki kernel kod segmenti seçicisini doğru bir şekilde almalıyız.
    // Bu değer, GDT'nin nerede tanımlandığına bağlıdır. Varsayımsal olarak 8 kullandık.
     const KERNEL_CODE_SELECTOR: u16 = src_gdt::KERNEL_CODE_SELECTOR.0; // Kavramsal GDT modülü

    unsafe {
        // Tüm girişleri sıfırla
        for entry in IDT.iter_mut() {
            *entry = IdtEntry::NULL;
        }

        // İstisna İşleyicilerini ayarla
        IDT[DIVIDE_ERROR_VECTOR as usize].set_handler_fn(DIVIDE_ERROR_HANDLER_PTR, KERNEL_CODE_SELECTOR, None);
        IDT[DEBUG_VECTOR as usize].set_handler_fn(UNKNOWN_HANDLER_PTR, KERNEL_CODE_SELECTOR, None); // Placeholder
        IDT[NON_MASKABLE_INTERRUPT_VECTOR as usize].set_handler_fn(UNKNOWN_HANDLER_PTR, KERNEL_CODE_SELECTOR, None); // Placeholder
        IDT[BREAKPOINT_VECTOR as usize].set_handler_fn(BREAKPOINT_HANDLER_PTR, KERNEL_CODE_SELECTOR, None);
        IDT[OVERFLOW_VECTOR as usize].set_handler_fn(UNKNOWN_HANDLER_PTR, KERNEL_CODE_SELECTOR, None); // Placeholder
        IDT[BOUND_RANGE_EXCEEDED_VECTOR as usize].set_handler_fn(UNKNOWN_HANDLER_PTR, KERNEL_CODE_SELECTOR, None); // Placeholder
        IDT[INVALID_OPCODE_VECTOR as usize].set_handler_fn(INVALID_OPCODE_HANDLER_PTR, KERNEL_CODE_SELECTOR, None);
        IDT[DEVICE_NOT_AVAILABLE_VECTOR as usize].set_handler_fn(UNKNOWN_HANDLER_PTR, KERNEL_CODE_SELECTOR, None); // Placeholder
        IDT[DOUBLE_FAULT_VECTOR as usize].set_handler_fn(DOUBLE_FAULT_HANDLER_PTR, KERNEL_CODE_SELECTOR, Some(1)); // IST #1 kullan
        IDT[INVALID_TSS_VECTOR as usize].set_handler_fn(UNKNOWN_HANDLER_PTR, KERNEL_CODE_SELECTOR, Some(2)); // Placeholder, IST #2
        IDT[SEGMENT_NOT_PRESENT_VECTOR as usize].set_handler_fn(UNKNOWN_HANDLER_PTR, KERNEL_CODE_SELECTOR, Some(3)); // Placeholder, IST #3
        IDT[STACK_SEGMENT_FAULT_VECTOR as usize].set_handler_fn(UNKNOWN_HANDLER_HANDLER_PTR, KERNEL_CODE_SELECTOR, Some(4)); // Placeholder, IST #4
        IDT[GENERAL_PROTECTION_FAULT_VECTOR as usize].set_handler_fn(GP_FAULT_HANDLER_PTR, KERNEL_CODE_SELECTOR, None); // GPF genellikle IST kullanmaz ama gerekirse kullanabilir.
        IDT[PAGE_FAULT_VECTOR as usize].set_handler_fn(PAGE_FAULT_HANDLER_PTR, KERNEL_CODE_SELECTOR, None); // Page Fault genellikle IST kullanmaz.
        IDT[X87_FLOATING_POINT_VECTOR as usize].set_handler_fn(UNKNOWN_HANDLER_PTR, KERNEL_CODE_SELECTOR, None); // Placeholder
        IDT[ALIGNMENT_CHECK_VECTOR as usize].set_handler_fn(UNKNOWN_HANDLER_PTR, KERNEL_CODE_SELECTOR, None); // Placeholder
        IDT[MACHINE_CHECK_VECTOR as usize].set_handler_fn(UNKNOWN_HANDLER_PTR, KERNEL_CODE_SELECTOR, None); // Placeholder
        IDT[SIMD_FLOATING_POINT_VECTOR as usize].set_handler_fn(UNKNOWN_HANDLER_PTR, KERNEL_CODE_SELECTOR, None); // Placeholder
        IDT[VIRTUALIZATION_VECTOR as usize].set_handler_fn(UNKNOWN_HANDLER_PTR, KERNEL_CODE_SELECTOR, None); // Placeholder
        IDT[CONTROL_PROTECTION_VECTOR as usize].set_handler_fn(UNKNOWN_HANDLER_PTR, KERNEL_CODE_SELECTOR, None); // Placeholder

        // Sistem Çağrısı İşleyicisini ayarla (DPL=3 ve Trap Gate olarak)
        // set_handler_fn bu bayrakları ayarlayabilmeli veya manuel set edilmeli.
        // Örnek: Trap Gate, DPL=3, Present=1, 64-bit Gate
        IDT[SYSCALL_VECTOR as usize].set_handler_fn(SYSCALL_ENTRY_PTR, KERNEL_CODE_SELECTOR, None);
        IDT[SYSCALL_VECTOR as usize].options = (IDT[SYSCALL_VECTOR as usize].options & !0xE00) // Gate Type'ı temizle
                                              | 0b1110_0000_0000 // 64-bit Trap Gate (1110)
                                              | (3 << 13); // DPL=3

        // Donanım Kesme İşleyicilerini ayarla
        IDT[TIMER_INTERRUPT_VECTOR as usize].set_handler_fn(TIMER_INTERRUPT_HANDLER_PTR, KERNEL_CODE_SELECTOR, None);
         IDT[KEYBOARD_INTERRUPT_VECTOR as usize].set_handler_fn(KEYBOARD_INTERRUPT_HANDLER_PTR, KERNEL_CODE_SELECTOR, None);
        // ... diğer IRQ'lar ...

        // Varsayılan handler'ı henüz atanmamış tüm girişlere ata.
         for i in 0..256 {
             // Basit kontrol: Eğer handler adresi hala NULL (0) ise
             if IDT[i].handler_low == 0 && IDT[i].handler_middle == 0 && IDT[i].handler_high == 0 {
                  // IDT[i].set_handler_fn(UNKNOWN_HANDLER_PTR, KERNEL_CODE_SELECTOR, None); // Zaten atanmış
                  // Ancak Syscall gibi DPL'si veya tipi değişenleri de kapsamak gerekebilir.
                  // Daha iyi bir kontrol: Eğer bir giriş boşsa (NULL), varsayılanı ata.
                  // Veya her şeyi baştan `unknown_interrupt_handler` ile doldurup,
                  // sonra bilinenleri overwrite etmek daha güvenli olabilir.
             }
         }
         // Daha güvenli yaklaşım: Önce tümünü varsayılan ile doldur, sonra özel olanları üzerine yaz.
         for i in 0..256 {
              IDT[i].set_handler_fn(UNKNOWN_HANDLER_PTR, KERNEL_CODE_SELECTOR, None);
         }
         // Şimdi özel handler'ları tekrar set et (bu kısım önceki ayarlamaları tekrarlıyor ama üzerine yazıyor).
         IDT[DIVIDE_ERROR_VECTOR as usize].set_handler_fn(DIVIDE_ERROR_HANDLER_PTR, KERNEL_CODE_SELECTOR, None);
         IDT[BREAKPOINT_VECTOR as usize].set_handler_fn(BREAKPOINT_HANDLER_PTR, KERNEL_CODE_SELECTOR, None);
         IDT[INVALID_OPCODE_VECTOR as usize].set_handler_fn(INVALID_OPCODE_HANDLER_PTR, KERNEL_CODE_SELECTOR, None);
         IDT[GENERAL_PROTECTION_FAULT_VECTOR as usize].set_handler_fn(GP_FAULT_HANDLER_PTR, KERNEL_CODE_SELECTOR, None);
         IDT[PAGE_FAULT_VECTOR as usize].set_handler_fn(PAGE_FAULT_HANDLER_PTR, KERNEL_CODE_SELECTOR, None);
         IDT[DOUBLE_FAULT_VECTOR as usize].set_handler_fn(DOUBLE_FAULT_HANDLER_PTR, KERNEL_CODE_SELECTOR, Some(1));
         IDT[SYSCALL_VECTOR as usize].set_handler_fn(SYSCALL_ENTRY_PTR, KERNEL_CODE_SELECTOR, None);
         IDT[SYSCALL_VECTOR as usize].options = (IDT[SYSCALL_VECTOR as usize].options & !0xE00) | 0b1110_0000_0000 | (3 << 13);
         IDT[TIMER_INTERRUPT_VECTOR as usize].set_handler_fn(TIMER_INTERRUPT_HANDLER_PTR, KERNEL_CODE_SELECTOR, None);


        // IDT pointer yapısını doldur.
        let idt_ptr = IdtPointer {
            limit: (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: IDT.as_ptr() as u64,
        };

        // LIDT komutu ile IDT'yi CPU'ya yükle.
        core::arch::asm!("lidt ({})", in(reg) &idt_ptr, options(nostack));
    }

    klog::info!("IDT loaded successfully.");

    // Donanım kesmelerini (IRQ'ları) etkinleştirmek için PIC/APIC'i yapılandırmak gerekir.
     src_interrupts::init_pic(); // Kavramsal PIC başlatma
    // CPU'da kesmeleri etkinleştir.
     unsafe { x86_64::instructions::interrupts::enable(); } // Kesmeleri etkinleştir
    klog::info!("Interrupts might need enabling via PIC/APIC setup and `sti`.");
}

// --- Assembly Stubs (Kavramsal) ---
// Bu Rust dosyası, genellikle ayrı bir assembly (.s) dosyasından veya
// Rust'ın `global_asm!` makrosu ile derlenen assembly kodundan çağrılır.
// Her istisna/kesme vektörü için bir stub gerekir.
// Stub'ların görevi:
// 1. CPU tarafından push edilen Stack Frame'i al.
// 2. Diğer yazmaçları stack'e kaydet.
// 3. İlgili istisna/kesme vektör numarasını ve (gerekiyorsa) hata kodunu stack'e push et.
// 4. Rust'taki `common_interrupt_handler` gibi bir fonksiyona veya doğrudan
//    `extern "x86-interrupt" fn` handler'ına zıpla (call).
// 5. Rust handler'ından döndükten sonra yazmaçları stack'ten geri yükle.
// 6. `iret` veya `iretq` komutu ile istisnadan/kesmeden geri dön.


// Assembly Stub Örneği (Konsept, tam kod değil):

.macro handler vector, has_error_code
.global handler_\vector
handler_\vector:
    // Kaydedilecek yazmaçları push et (r15-r8, rbp, rsi, rdi, rdx, rcx, rbx, rax)
    push r15
    push r14
    ...
    push rax

    // Vektör numarasını push et
    push \vector

    // Ortak Rust handler'ını çağır
    call common_interrupt_handler

    // Vektör numarasını pop et
    pop rdi // veya başka bir geçici yazmaç

    // Kaydedilmiş yazmaçları pop et (rax, rbx, ..., r15)
    pop rax
    ...
    pop r15

    // İstisnadan/kesmeden geri dön
    iretq
.endm
