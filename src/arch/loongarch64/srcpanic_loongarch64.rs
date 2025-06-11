#![no_std] // Standart kütüphaneye ihtiyacımız yok, çekirdek alanındayız.
#![panic_handler] // Bu fonksiyonun panic handler olduğunu işaretler.

use core::panic::PanicInfo;
use core::arch::asm; // Mimariye özel assembly için
use core::fmt::Write; // Mesaj formatlama için

// --- Varsayılan Panik-Güvenli Çıktı Fonksiyonları (Karnal64 Temel Hizmeti) ---
// Bu fonksiyonların çekirdeğin başka bir yerinde (örn. temel konsol sürücüsü)
// panik anında güvenli bir şekilde çalışacak şekilde implemente edildiği varsayılır.
// Bunlar, Karnal64'ün normal ResourceProvider akışını atlayarak panik durumunda
// çıktı almayı sağlar.
extern "C" {
    /// Panik konsoluna tek bir byte yazar.
    fn kpanic_print_byte(byte: u8);
    /// Panik konsoluna bir string yazar.
    fn kpanic_print_str(s: &str);
    /// Panik konsoluna yeni satır karakteri yazar.
    fn kpanic_println();
}

// --- Yardımcı Fonksiyonlar ---

// Sayıları hex formatında yazdırmak için basit, panik-güvenli yardımcı.
// Karnal64'ün ResourceProvider.write metodunu KULLANMAZ, doğrudan kpanic_print_byte kullanır.
fn kpanic_print_hex(value: u64) {
    let mut buffer = [0u8; 16]; // u64 için max 16 hex hane
    let mut i = buffer.len();
    let mut current_value = value;

    if current_value == 0 {
        i -= 1;
        buffer[i] = b'0';
    } else {
        while current_value > 0 && i > 0 {
            i -= 1;
            let digit = (current_value & 0xF) as u8;
            buffer[i] = if digit < 10 {
                b'0' + digit
            } else {
                b'a' + digit - 10 // Küçük harf hex
            };
            current_value >>= 4;
        }
    }

    kpanic_print_str("0x");

    // Tamponun kullanılan kısmını yazdır
    for j in i..buffer.len() {
        kpanic_print_byte(buffer[j]);
    }
}

// `core::fmt::Arguments`'ı panik-güvenli çıktı fonksiyonlarımıza yazmak için writer.
// Bu, `info.message()`'ı yazdırmamızı sağlar.
struct PanicWriter;

impl Write for PanicWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        kpanic_print_str(s);
        Ok(())
    }

    fn write_char(&mut self, c: char) -> core::fmt::Result {
        // Basit ASCII veya benzeri için
        kpanic_print_byte(c as u8);
        Ok(())
    }
}


// --- Mimariye Özel Register Okuma Fonksiyonları (LoongArch) ---
// Bunlar, LoongArch mimarisine özel assembly veya intrinsics kullanır.
// Tam implementasyon ISA'ya bağlıdır.

/// Program Counter (PC) değerini okur (Genellikle EPC - Exception Program Counter).
unsafe fn read_pc() -> u64 {
     let mut pc: u64;
     // LoongArch CSRrd instruction (CSR 0x07 is usually EPC)
     asm!("csrrd {0}, 0x07", out(reg) pc, options(nomem, nostack, preserves_flags));
     pc
}

/// Genel Amaçlı Register (GPR) değerini okur.
/// Reg_num 0-31 arasındadır.
unsafe fn read_gpr(reg_num: usize) -> u64 {
    // NOTE: LoongArch assembly'de GPR'ler $r0..$r31 veya $zero, $ra, $sp vb. ile referans alınır.
    // Dinamik olarak register numarasına göre okuma yapmak için assembly macro'ları veya
    // register'ları listeleyen daha karmaşık bir yapı gerekebilir.
    // Basitlik adına, burada sadece yaygın olan R1 (ra) ve R3 (sp) için örnek verelim.
    // Diğer GPR'ler için read_gpr'ın içi doldurulmalıdır.
     let value: u64;
     match reg_num {
         0 => value = 0, // $zero register
         1 => asm!("move {0}, $ra", out(reg) value, options(nomem, nostack)), // $ra (R1)
         3 => asm!("move {0}, $sp", out(reg) value, options(nomem, nostack)), // $sp (R3)
         // TODO: Diğer GPR'ler için asm! ekle ($r4, $r5, ... $r31)
         _ => value = 0xDEADBEEF, // Bilinmeyen veya okunmayan register için yer tutucu
     }
     value
}

/// Cause Register değerini okur (CSR 0x0A).
unsafe fn read_cause() -> u64 {
    let mut cause: u64;
    asm!("csrrd {0}, 0x0A", out(reg) cause, options(nomem, nostack, preserves_flags));
    cause
}

/// Status Register değerini okur (CSR 0x01).
unsafe fn read_status() -> u64 {
    let mut status: u64;
    asm!("csrrd {0}, 0x01", out(reg) status, options(nomem, nostack, preserves_flags));
    cause
}

/// BadVAddr Register değerini okur (CSR 0x0E).
unsafe fn read_badvaddr() -> u64 {
    let mut badvaddr: u64;
    asm!("csrrd {0}, 0x0E", out(reg) badvaddr, options(nomem, nostack, preserves_flags));
    badvaddr
}


// --- PANIC HANDLER ---

/// Karnal64 çekirdeği için LoongArch mimarisine özel panic handler.
/// Sistem geri dönülemeyen bir hatayla karşılaştığında bu fonksiyon çağrılır.
fn panic(info: &PanicInfo) -> ! {
    // 1. Tüm kesmeleri (interrupts) hemen devre dışı bırak.
    // Bu, panik anında başka kesmelerin durumu daha da kötüleştirmesini önler.
    unsafe {
        // Bu assembly kodu LoongArch interrupt disable mekanizmasına bağlıdır.
        // Örnek: CSR.ST registerına yazarak interruptları kapatmak.
        // Gerçek implementasyon ISA ve platforma özel olacaktır.
        asm!(
            "li.d $a0, 0", // Load immediate 0 into $a0
            "csrwr $a0, 0x01", // Write $a0 to CSR.ST (Status Register) - This might disable interrupts depending on flags
            options(nomem, nostack, preserves_flags)
        );
         // Daha sağlam bir yaklaşım, mevcut durumu okuyup sadece ilgili bitleri maskelemek olabilir.
    }

    // 2. Panik başlığını yazdır.
    kpanic_print_str("!!! KERNEL PANIC !!!");
    kpanic_println();

    // 3. Panik konumunu yazdır (dosya adı ve satır numarası).
    if let Some(location) = info.location() {
        kpanic_print_str("at ");
        kpanic_print_str(location.file());
        kpanic_print_str(":");
        // itoa gibi bir crate kullanarak u32/u64 satır numarasını stringe çevirip yazdır
        // Eğer itoa veya benzeri yoksa, sayı yerine bir yer tutucu kullanabilirsiniz.
        // Örnek (itoa crate kullanıldığı varsayılır):
         let mut line_buffer = [0u8; 20]; // Max satır no string uzunluğu
         if let Ok(line_str) = itoa::write(&mut line_buffer[..], location.line() as u64) {
             kpanic_print_str(line_str);
         } else {
             kpanic_print_str("???"); // itoa yoksa veya hata verirse
         }
        kpanic_println();
    }

    // 4. Panik mesajını yazdır.
    kpanic_print_str("Reason: ");
    // PanicInfo'daki formatlanmış mesajı PanicWriter kullanarak yazdır.
    // info.message() Option<&Arguments> döndürür.
    if let Some(message) = info.message() {
        // PanicWriter, kpanic_print_str/byte kullanarak yazdığı için panik-güvenlidir.
        let _ = core::fmt::write(&mut PanicWriter, *message);
    } else {
        // Eğer message yoksa, payload'a bakabiliriz.
        if let Some(payload) = info.payload().downcast_ref::<&'static str>() {
            kpanic_print_str("Payload: ");
            kpanic_print_str(payload);
        } else {
            kpanic_print_str("No specific panic message.");
        }
    }
    kpanic_println();


    // 5. Mimariye Özel Durumu Yazdır (LoongArch Registerları)
    kpanic_println();
    kpanic_print_str("LoongArch Register State:");
    kpanic_println();

    unsafe {
        kpanic_print_str("  PC: "); kpanic_print_hex(read_pc()); kpanic_println();
        kpanic_print_str("  SP (R3): "); kpanic_print_hex(read_gpr(3)); kpanic_println(); // R3 Stack Pointer
        kpanic_print_str("  RA (R1): "); kpanic_print_hex(read_gpr(1)); kpanic_println(); // R1 Return Address

        // Yaygın kullanılan diğer bazı CSR'ları yazdır
        kpanic_print_str("  Cause: "); kpanic_print_hex(read_cause()); kpanic_println();
        kpanic_print_str("  Status: "); kpanic_print_hex(read_status()); kpanic_println();
        kpanic_print_str("  BadVAddr: "); kpanic_print_hex(read_badvaddr()); kpanic_println();

        // TODO: İhtiyaca göre veya debug için diğer GPR'ları (R0-R31) yazdırma eklenebilir.
        // Bu, read_gpr fonksiyonunun tüm GPR'ları desteklemesini gerektirir.
        kpanic_print_str("  (Other GPRs not shown)");
        kpanic_println();
    }

    // 6. Stack Trace (Gelişmiş bir özelliktir, implementasyonu karmaşıktır ve mimariye bağlıdır)
    // Stack unwinding desteği veya frame pointer tabanlı yürüme mekanizması gerektirir.
    kpanic_println();
    kpanic_print_str("Stack trace not available.");
    kpanic_println();


    // 7. Sistemi durdur - Sonsuz döngü veya mimariye özel durma talimatı.
    // Bu döngüden çıkılmamalıdır.
    loop {
        unsafe {
            // LoongArch için bir "Software Debug Breakpoint" veya benzeri bir durma talimatı.
            // `sdbb` yaygın bir seçenektir, veya sadece bir boş döngü.
             asm!("sdbb 0", options(nomem, nostack));
            // Veya basit boş döngü:
             asm!("", options(nomem, nostack));
        }
    }
}

// --- Gerekli Bağlantılar ---
// Bu dosyanın derlenebilmesi için, başka bir yerdeki LoongArch'a özel kodunuzda şu fonksiyonları
// 'no_mangle' ve 'extern "C"' olarak tanımlayıp implemente etmeniz ve bağlamanız gerekir:
//
 #[no_mangle]
 pub extern "C" fn kpanic_print_byte(byte: u8) { /* Donanıma yazma */ }
 #[no_mangle]
 pub extern "C" fn kpanic_print_str(s: &str) { /* Byte döngüsü */ }
 #[no_mangle]
 pub extern "C" fn kpanic_println() { /* Yeni satır karakterleri */ }
