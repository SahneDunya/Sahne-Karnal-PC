#![no_std] // Standart kütüphane yok
#![allow(dead_code)] // Geliştirme aşamasında kullanılmayan kodlara izin ver
#![allow(unused_variables)] // Geliştirme aşamasında kullanılmayan değişkenlere izin ver
#![feature(asm_inline)] // Inline assembly kullanmak için (mimariye özgü kodlar için gerekli olabilir)
#![feature(naked_functions)] // Giriş noktası gibi çok düşük seviye fonksiyonlar için (optimizasyonları kapatır)

// Karnal64 çekirdek API'sını ve tiplerini içe aktar
// Bu, karnal64.rs dosyasındaki pub ilan edilmiş öğelere erişimi sağlar.
#[path = "../karnal64.rs"] // karnal64.rs dosyasının yolu (örneğin src/karnal64.rs ise ..)
mod karnal64;

// LoongArch mimarisine özgü register, trap frame, MMU yapıları vb.
// Bu modüllerin içeriği tamamen LoongArch'ın teknik özelliklerine göre doldurulmalıdır.
mod arch {
    // TODO: LoongArch işlemcisinin özel registerlarına erişim fonksiyonları
    // TODO: Trap frame yapısı (Sistem çağrısı ve kesmeler sırasında CPU state'ini kaydeder)
    #[repr(C)] // Genellikle C uyumlu düzen kullanılır
    pub struct TrapFrame {
        // Örneğin, genel amaçlı registerlar (r0-r31 gibi LoongArch registerları)
        // fp registerı (frame pointer)
        // sp registerı (stack pointer)
        // ra registerı (return address)
        // pc registerı (program counter)
        // PRMD registerı (privilege mode)
        // BADVADDR registerı
        // CRMD registerı (control/status register)
        // vb. mimariye özgü tüm önemli registerlar buraya tanımlanmalı
        pub registers: [u64; 32], // Yer tutucu
        pub pc: u64,
        pub crmd: u64, // Örnek kontrol registerı
        pub prmd: u64, // Örnek privilege registerı
        // ... diğer mimariye özgü state bilgileri
        pub syscall_nr: u64, // Sistem çağrısı numarası genellikle belirli bir registerda gelir, burada kolaylık için ayrı tutalım
        pub syscall_args: [u64; 5], // Sistem çağrısı argümanları belirli registerlarda gelir, burada kolaylık için ayrı tutalım
        pub syscall_return: i64, // Sistem çağrısı dönüş değeri belirli bir registera yazılır
        // ... diğer trap/exception bilgileri (cause, status vb.)
    }

    // TODO: MMU kontrol fonksiyonları (sayfa tablosu oluşturma, haritalama, koruma ayarlama vb.)
    pub mod mmu {
        pub fn init() {
            // TODO: LoongArch MMU'sunu başlat
            // TODO: Çekirdek bellek haritasını ayarla
        }
        // TODO: map, unmap, set_permissions fonksiyonları
    }

    // TODO: Kesme denetleyicisi (PIC/APIC benzeri) ve timer yönetimi
    pub mod irq {
        pub fn init() {
            // TODO: LoongArch kesme denetleyicisini başlat
            // TODO: Timer'ı ayarla (çekirdek zamanlayıcısı için)
        }
        // TODO: enable_irq, disable_irq, handle_irq fonksiyonları
    }

    // TODO: Konsol veya hata ayıklama çıkışı için temel fonksiyonlar (henüz ResourceProvider yokken)
    pub mod console {
        pub fn putc(c: u8) {
            // TODO: LoongArch donanımına doğrudan yazarak karakteri ekrana gönder
        }
    }

    // TODO: Bağlam değiştirme (context switch) fonksiyonları (görev/iş parçacığı zamanlama için)
    // Bu genellikle kritik derecede mimariye özgü ve assembly gerektiren bir kısımdır.
    pub mod context {
        // TODO: save_context, restore_context fonksiyonları
    }
}


/// Çekirdeğin ana giriş noktası. Bootloader tarafından çağrılır.
/// Burası çok düşük seviyedir, C ABI'sine uygun olmalıdır.
#[no_mangle]
#[naked_functions] // Minimal stack frame ve setup için naked
pub extern "C" fn _start() -> ! {
    // TODO: LoongArch mimarisine özgü başlangıç setup'ı
    // - CPU'yu bilinen bir duruma getir
    // - Çekirdek stack'ini ayarla
    // - Temel MMU'yu başlat (çekirdek kodunun çalışması için gereken en az seviye)
    // - Trap/Exception vektörlerini ayarla (handle_trap fonksiyonuna işaret edecek şekilde)
    // - Kesmeleri/istisnaları etkinleştir (güvenli moda geçtikten sonra)

    // Örnek placeholder setup adımları (gerçek kod LoongArch assembly veya inline asm olacaktır)
    unsafe {
        // Fake: Enable FPU, set privilege level etc.
         asm!("...") // Gerçek LoongArch assembly kodları

        // Fake: Setup a basic stack
         extern "C" { static __KERNEL_STACK_START: u64; } // Linker script'ten stack adresi
         asm!("li sp, {}", in(reg) __KERNEL_STACK_START); // Stack pointer'ı ayarla
    }


    // Temel mimari alt sistemlerini başlat
    arch::mmu::init();
    arch::irq::init();
    // TODO: Diğer mimari alt sistemlerini başlat

    // Karnal64 API'sını başlat
    // Bu fonksiyon, Karnal64'ün iç veri yapılarını (kaynak yöneticisi, görev yöneticisi yer tutucuları vb.) başlatır.
    karnal64::init();

    // İlk kullanıcı alanı görevini başlat (veya boşta döngüsüne gir)
    // TODO: İlk kullanıcı alanını (init process gibi) yükle ve çalıştır
    // Bu genellikle resource_acquire kullanarak kod kaynağını açmayı,
    // memory_allocate ile adres alanı oluşturmayı, memory_map ile kodu yüklemeyi ve
    // task_spawn ile yeni bir görev başlatmayı içerir.

    // İlk görev başlatılana kadar veya sistem durana kadar boşta döngüsü
    loop {
        // TODO: Düşük güç moduna geçme veya bekleme talimatı (LoongArch specific)
        unsafe { core::arch::asm!("nop") } // Yer tutucu: Bir şey yapma
    }
}

/// LoongArch trap/exception/sistem çağrısı işleyicisi.
/// Çekirdek boot sırasında trap vektör tablosuna bu fonksiyonun adresi yazılır.
/// Donanımdan gelen kesme, istisna veya sistem çağrıları buraya yönlendirilir.
#[no_mangle] // Trap vektör tablosundan çağrılabilmesi için
pub extern "C" fn handle_trap(trap_frame: *mut arch::TrapFrame) {
    // Güvenlik Notu: Bu fonksiyon, kesme/trap geldiğinde mevcut iş parçacığının stack'inde çalışır.
    // Trap frame pointer'ı (trap_frame) donanım tarafından sağlanan CPU state'ini gösterir.
    // Bu noktada kullanıcı alanı/çekirdek ayrımı korunmaktadır, ancak trap frame içindeki
    // kullanıcı alanı pointer'ları doğrudan DEREFERENCE edilmemelidir.

    let frame = unsafe { &mut *trap_frame };

    // TODO: Trap'in nedenini belirle (LoongArch mimarisine göre)
    // Örneğin, hangi register veya özel bellek alanı trap nedenini belirtiyor?
    let trap_cause = 0; // Yer tutucu

    match trap_cause {
        // TODO: LoongArch'taki sistem çağrısı trap nedeni kodu nedir?
        // Örneğin 8 (Syscall exception in MIPS/LoongArch legacy)
        8 => { // Varsayımsal SYSCALL trap nedeni
            // Sistem çağrısı argümanlarını trap frame'den (registerlardan) al
            // Varsayımsal olarak, argümanlar a0-a5 registerlarında, syscall numarası v0'da olabilir (MIPS/RISC-V benzeri)
            let syscall_number = frame.syscall_nr; // Veya frame.registers[..]
            let arg1 = frame.syscall_args[0];
            let arg2 = frame.syscall_args[1];
            let arg3 = frame.syscall_args[2];
            let arg4 = frame.syscall_args[3];
            let arg5 = frame.syscall_args[4];

            // --- Güvenlik Açısından KRİTİK Adım ---
            // Kullanıcı alanından gelen pointer argümanlarını (örneğin, read/write için buffer pointer'ları)
            // çekirdek tarafından kullanılmadan önce MUTLAKA doğrula.
            // Bu doğrulama, pointer'ın:
            // 1. Mevcut görevin kullanıcı alanı bellek haritasında geçerli bir adresi gösterdiğini,
            // 2. İstenen işlem için (okuma/yazma) ilgili bellek alanının izinlerinin uygun olduğunu,
            // 3. Sağlanan uzunluğun adres sınırları içinde kaldığını kontrol eder.
            // Bu kontrol, arch::mmu modülü ve görev yöneticisinin bellek haritası bilgisi kullanılarak yapılır.
            // TODO: pointer_is_valid_and_accessible(ptr, len, permissions) gibi bir fonksiyon çağrılmalı

            // Karnal64 API'sındaki sistem çağrısı işleyiciyi çağır
            // Bu fonksiyon, argümanları yorumlar ve ilgili Karnal64 hizmetini çağırır.
            let syscall_result: i64;
            // TODO: Kullanıcı pointer'ları doğrulaması başarılı olursa bu çağrıyı yap
             // Kullanıcı pointer'larının doğrulanmış, çekirdek tarafından erişilebilir versiyonlarını veya ham hallerini ilet.
             // Karnal64 API fonksiyonları (handle_syscall gibi) genellikle ham pointer'ı alır ve doğrulamayı kendileri yapar.
             // Bu taslakta handle_syscall'ın ham pointer'ları aldığını varsayalım.
             // karnal64::handle_syscall fonksiyonu i64 döner (başarı ise pozitif/sıfır, hata ise negatif KError).
            syscall_result = karnal64::handle_syscall(syscall_number, arg1, arg2, arg3, arg4, arg5);


            // Sistem çağrısı sonucunu kullanıcı alanına dönecek registera yaz
            frame.syscall_return = syscall_result; // Veya frame.registers[..]

            // TODO: Program Counter'ı (PC) sistem çağrısından sonraki talimata ilerlet
            // (Genellikle sistem çağrısı talimatının uzunluğu kadar ilerletilir)
            frame.pc += 4; // Varsayımsal talimat uzunluğu
        }
        // TODO: Diğer trap nedenlerini işle (page fault, genel koruma hatası, undefined instruction vb.)
        // Bunlar genellikle görevin sonlandırılmasına veya bir sinyal gönderilmesine neden olur.
        _ => {
            // Bilinmeyen veya işlenmeyen trap
            // TODO: Hata mesajı bas (arch::console::putc kullanarak)
            // TODO: Görevi sonlandır (ktask::task_exit kullanarak)
            // Veya daha basitçe, çekirdeği durdur.
             unsafe { core::arch::asm!("break") } // Hata ayıklama için breakpoint
            loop {
                 // TODO: Hata durumunda dur veya resetle (LoongArch specific)
                 unsafe { core::arch::asm!("nop") }
            }
        }
    }
}
