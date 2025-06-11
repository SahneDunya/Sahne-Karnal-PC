use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use lazy_static::lazy_static;
use spin::Mutex;

// Karnal64 çekirdek API'sını ve diğer modülleri dışarıdan import ediyoruz.
// Bu modüllerin src/ dizininde veya başka bir yerde tanımlı olduğunu varsayıyoruz.
// extern crate karnal64; // Eğer ayrı bir crate ise
use crate::karnal64; // Genellikle aynı çekirdek projesi içindeyse

// Diğer çekirdek modülleri için yer tutucu importlar
use crate::ktask;
use crate::kmemory;

// --- Sabitler ---

// Programlanabilir Kesme Denetleyicisi (PIC) ofsetleri
// x86 sistemlerde donanım kesmeleri (IRQ'ler) genellikle 0-15 aralığındadır.
// Bu aralığın istisnalar (0-31) ile çakışmasını önlemek için PIC'leri yeniden haritalarız.
// Genellikle PIC1 (master) IRQ0-7 -> Vektör 32-39
// PIC2 (slave) IRQ8-15 -> Vektör 40-47
const PIC_1_OFFSET: u8 = 32;
const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

// Kullanacağımız sistem çağrısı kesme numarası (genel bir seçenektir)
const SYSCALL_INT_VECTOR: u8 = 0x80; // Vektör 128

// --- Kesme Tanımlayıcı Tablosu (IDT) ---

// IDT'yi statik olarak tanımlıyoruz ve erişimini Mutex ile koruyoruz.
// lazy_static, IDT'nin ilk kullanıldığında başlatılmasını sağlar.
lazy_static! {
    static ref IDT: Mutex<InterruptDescriptorTable> = Mutex::new(InterruptDescriptorTable::new());
}

// --- Kesme İşleyicileri (Handler) ---

// Bu fonksiyonlar, assembly stubs (yardımcı montaj kodları) tarafından çağrılır.
// Assembly stub'lar, kesme sırasında CPU'nun kaydettiği durumu (InterruptStackFrame) hazırlar,
// ek bilgileri (hata kodu gibi) yığına koyar ve bu Rust fonksiyonlarını çağırır.
// Daha sonra Rust fonksiyonundan döndükten sonra, assembly stub'lar kaydedilen durumu geri yükler ve `iret` ile kesmeden döner.

// Assembly stubs için extern tanımlamalar.
// Bu fonksiyonların gövdesi Rust'ta değil, assembly dilinde yazılacaktır.
extern "x86-interrupt" fn exception_handler(stack_frame: InterruptStackFrame, vector: u8, error_code: Option<u64>) {
    // Genel istisna işleyicisi
    println!("KERNEL PANIC: EXCEPTION: {}! Error Code: {:?} Stack Frame: {:#?}", vector, error_code, stack_frame);
    // Gerçek bir çekirdekte burada hata ayıklama bilgileri loglanır veya bir çökme ekranı gösterilir.
    // Sonsuz döngüye girerek sistemi durdurun.
    loop { x86_64::instructions::hlt(); }
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Zamanlayıcı kesme işleyicisi (IRQ0 / Vektör 32)
    // Bu kesme, işletim sisteminin zamanlayıcısı (scheduler) tarafından kullanılır.
    // Genellikle bir görev değişimi (context switch) burada tetiklenir.

    // TODO: Karnal64'ün görev yöneticisini (ktask) çağırarak zamanlayıcıyı çalıştır.
     ktask::schedule();

    // PIC'e (Programlanabilir Kesme Denetleyicisi) sinyal gönder: Kesmeyi işledik.
    // Bu, PIC'in aynı kesmeyi tekrar göndermesini engeller.
    // Bu PIC yönetimi src/pic.rs gibi ayrı bir modülde olabilir.
    unsafe {
         pic::notify_end_of_interrupt(PIC_1_OFFSET); // PIC1'den geldi
         // Yer tutucu PIC sinyali
        print!("."); // Zamanlayıcının çalıştığını göstermek için basit bir çıktı
    }
    // Zamanlayıcı kesmesi oldukça sık gerçekleşebilir, dikkatli loglama yapılmalı.
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Klavye kesme işleyicisi (IRQ1 / Vektör 33)
    // Klavye denetleyicisinden gelen tuş basma/bırakma olaylarını işler.

    // TODO: Klavye donanımından veriyi oku.
    // TODO: Okunan veriyi Karnal64'ün kaynak yöneticisi (kresource) üzerinden konsol kaynağına veya bir girdi kuyruğuna ilet.

    // PIC'e sinyal gönder: Kesmeyi işledik.
    unsafe {
        // pic::notify_end_of_interrupt(PIC_1_OFFSET + 1); // PIC1'den geldi
         // Yer tutucu PIC sinyali
        print!("K"); // Klavye kesmesinin geldiğini göstermek için basit bir çıktı
    }
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    // Sayfa Hatası işleyicisi (Exception 14)
    // Bellek yönetiminde (virtüel bellek) önemli bir istisnadır.
    // Erişilmek istenen sayfa bellekte değilse, sayfa koruma ihlali olursa tetiklenir.

    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Erişim Adresi: {:?}", Cr2::read()); // Hatanın oluştuğu sanal adres
    println!("Hata Kodu: {:?}", error_code);
    println!("Stack Frame: {:#?}", stack_frame);

    // TODO: Karnal64'ün bellek yöneticisini (kmemory) çağırarak sayfayı yüklemeye çalış,
    // izinleri kontrol et veya süreci sonlandır.
     kmemory::handle_page_fault(Cr2::read(), error_code);


    // Page Fault istisnası bir donanım kesmesi değil, CPU istisnasıdır.
    // Bu nedenle PIC'e EOI göndermeye GEREK YOKTUR.

    // Çözülemez bir sayfa hatası (örn. geçersiz adres) sistem çökmesine neden olur.
    loop { x86_64::instructions::hlt(); }
}

extern "x86-interrupt" fn general_protection_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    // Genel Koruma Hatası işleyicisi (Exception 13)
    // Segmentasyon ihlalleri, yetkisiz bellek erişimleri, geçersiz komutlar gibi birçok hatada tetiklenir.

    println!("EXCEPTION: GENERAL PROTECTION FAULT (GPF)");
    println!("Error Code: {}", error_code);
    println!("Stack Frame: {:#?}", stack_frame);

    // TODO: Hata koduna göre daha detaylı analiz yap.
    // Genellikle bu tür bir hata, kullanıcı programında ciddi bir hata veya çekirdekte bir bug olduğunu gösterir.
    // Kullanıcı sürecini sonlandır veya çekirdeği çökert.

    loop { x86_64::instructions::hlt(); }
}


// --- Sistem Çağrısı İşleyicisi ---

// `x86-interrupt` çağrı kuralı, CPU'nun kesme sırasındaki yığın düzenini ve
// kaydettiği durumu (InterruptStackFrame) otomatik olarak argüman olarak sağlar.
// Syscall kesmesi için hata kodu PUSH YAPMAZ, bu nedenle stack frame direkt gelir.
extern "x86-interrupt" fn syscall_handler(stack_frame: InterruptStackFrame) {
    // Sistem Çağrısı (Syscall) işleyicisi (Vektör 128 - 0x80)
    // Kullanıcı alanından çekirdek hizmetlerini talep etmek için kullanılır.

    // Kullanıcı alanı, sistem çağrısı numarasını ve argümanlarını belirli kayıtlara koyar.
    // x86_64 SysV ABI'sine göre (çoğu Unix benzeri sistemde yaygın):
    // Syscall numarası: RAX
    // Argümanlar: RDI, RSI, RDX, RCX, R8, R9
    // Dönüş değeri: RAX

    // stack_frame, kesme sırasında kullanıcı görevinin kaydettiği CPU durumunu içerir.
    let syscall_number = stack_frame.registers.rax;
    let arg1 = stack_frame.registers.rdi;
    let arg2 = stack_frame.registers.rsi;
    let arg3 = stack_frame.registers.rdx;
    let arg4 = stack_frame.registers.rcx; // RCX genellikle System V ABI'de 4. argüman olarak kullanılır
    let arg5 = stack_frame.registers.r8;
    let arg6 = stack_frame.registers.r9;

    // --- Karnal64 API Çağrısı ---
    // Karnal64'ün genel sistem çağrısı dağıtım fonksiyonunu çağır.
    // Bu fonksiyon, syscall numarasını kullanarak hangi Karnal64 fonksiyonunun
    // çalışacağını belirler ve gerekli validasyonları yapar.
    let syscall_result = karnal64::handle_syscall(
        syscall_number,
        arg1,
        arg2,
        arg3,
        arg4,
        arg5 // Karnal64 handle_syscall'da 5 argüman vardı, altıncıyı (arg6) şimdilik kullanmayalım.
    );

    // --- Sonucu Kullanıcı Alanına Döndürme ---
    // Sistem çağrısının dönüş değeri (syscall_result) kullanıcı alanına
    // genellikle RAX kaydı üzerinden döndürülür.
    // Bu nedenle, stack_frame üzerinde RAX kaydını güncelliyoruz.
    // `x86-interrupt` abi'si, handler döndüğünde bu stack frame'i kullanarak `iret` yapar.
    // Başarı (pozitif/sıfır) veya hata (negatif) değeri RAX'e yazılır.
    let mut_stack_frame = unsafe {
        // stack_frame'e yazmak için güvenli olmayan (unsafe) blok gerekli.
        // Dikkatli kullanılmalıdır!
        // stack_frame Immutable gelmiyor mu? InterruptStackFrame dokümantasyonuna bakmak lazım.
        // `x86_64` crate'inin `InterruptStackFrame`'i genellikle `&mut` olarak gelir.
        // Eğer gelmiyorsa, assembly stub'ın argümanı mutable yapması gerekebilir veya
        // buradaki `unsafe` cast yerine başka bir mekanizma (örneğin assembly'de doğrudan yazma) gerekebilir.
        // Genellikle `&mut` geldiği varsayılır.
         &mut *(core::ptr::addr_of!(stack_frame) as *mut InterruptStackFrame)
    };

    mut_stack_frame.registers.rax = syscall_result as u64; // i64'ü u64'e dönüştürürken taşma riskine dikkat

    // TODO: Eğer sistem çağrısı görev değişimi gerektiriyorsa (örn. sleep, yield),
    // zamanlayıcı (ktask) burada çağrılmalı ve dönüş değeri ayarlanmalıdır.
     task::yield_now(); // gibi
}


// --- IDT Kurulum Fonksiyonu ---

pub fn init_idt() {
    let mut idt = IDT.lock(); // IDT'ye Mutex kilidi ile güvenli erişim

    // İstisna işleyicilerini kur
    // x86-64 mimarisinde tanımlanmış istisnalar (0-31 arası)
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    // ... Diğer istisnalar için handler'lar (double_fault, general_protection_fault vb.) ...
    idt.page_fault.set_handler_fn(page_fault_handler);
    idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);

    // Donanım kesme işleyicilerini kur (PIC üzerinden gelenler)
    // Vektör 32: Zamanlayıcı (Timer - IRQ0)
    idt[PIC_1_OFFSET].set_handler_fn(timer_interrupt_handler);
    // Vektör 33: Klavye (Keyboard - IRQ1)
    idt[PIC_1_OFFSET + 1].set_handler_fn(keyboard_interrupt_handler);
    // ... Diğer donanım kesmeleri ...

    // Sistem Çağrısı işleyicisini kur
    // Vektör 128 (0x80) genellikle syscall için kullanılır
    idt[SYSCALL_INT_VECTOR]
        .set_handler_fn(syscall_handler)
        // DPL (Descriptor Privilege Level): Bu kapıya erişmek için gereken yetki seviyesi.
        // Kullanıcı alanının (Ring 3) sistem çağrısı yapabilmesi için DPL = 3 olmalıdır.
        .set_present(true)
        .set_privilege_level(x86_64:: PrivilegeLevel::Ring3);


    // ... Gerekirse diğer istisna ve kesmeleri de ekleyin ...

    // Hazırlanan IDT'yi donanıma yükle
    unsafe {
        idt.load();
    }

    println!("Karnal64: IDT yüklendi.");
}

// --- Kesmeleri Etkinleştirme ---

pub fn enable_interrupts() {
    unsafe {
        // PIC'leri başlat (IRQ'ları yönlendirmek için)
        // Bu fonksiyon pic modülünde implemente edilmelidir.
         pic::init_pics(PIC_1_OFFSET, PIC_2_OFFSET);

        // CPU'daki kesme bayrağını (IF - Interrupt Flag) ayarla (STI komutu).
        // Bu, CPU'nun kesmeleri dinlemeye başlamasını sağlar.
        x86_64::instructions::interrupts::enable();
    }
    println!("Karnal64: Kesmeler etkinleştirildi.");
}

// --- Basit Breakpoint İstisnası İşleyicisi (Hata Ayıklama İçin Kullanışlı) ---
// Bu handler, gdb gibi hata ayıklayıcılar için veya kodda kasıtlı durdurma noktaları için kullanışlıdır.
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}
