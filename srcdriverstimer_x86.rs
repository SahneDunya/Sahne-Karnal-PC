#![no_std]
use core::sync::atomic::{AtomicU64, Ordering};
// x86 mimarisine özgü I/O port erişimi ve kesme yönetimi için gerekli yapılar (basit bir örnek için doğrudan inline assembly kullanılacak)

static TICKS: AtomicU64 = AtomicU64::new(0);

// Gecikme fonksiyonunda kullanılacak saat frekansı (örnek olarak PIT frekansı kullanılacak)
// Gerçek değer platforma ve PIT yapılandırmasına bağlıdır. 1.193182 MHz (yaklaşık) PIT'in temel frekansıdır.
// Bu örnek için yaklaşık bir değer kullanıyoruz. Doğru değer için platform dökümantasyonuna bakılmalıdır.
pub mod platform {
    pub const CLOCK_FREQ: u64 = 1_193_182; // Yaklaşık PIT frekansı (Hz)
}


pub fn ticks() -> u64 {
    TICKS.load(Ordering::SeqCst)
}

pub fn delay_ms(ms: u64) {
    let target_ticks = ticks() + (ms * platform::CLOCK_FREQ / 1000);
    while ticks() < target_ticks {
        core::hint::spin_loop_hint();
    }
}

// x86 için zamanlayıcı kesmesi işleyicisi (IRQ0 - PIT için varsayılan)
// #[naked] özniteliği ve inline assembly ile kesme işleyici
#[naked]
#[no_mangle]
pub extern "C" fn timer_interrupt_handler() {
    // #![naked] kullanıldığı için tüm fonksiyon assembly olarak yazılmalı
    // stack'i ayarlayın ve Rust fonksiyonlarını çağırın
    unsafe {
        asm!(
            "push rax",
            "push rcx",
            "push rdx",
            "push rbx",
            "push rsp", // Mevcut rsp'yi kaydet
            "push rbp",
            "push rsi",
            "push rdi",
            "push r8",
            "push r9",
            "push r10",
            "push r11",
            "push r12",
            "push r13",
            "push r14",
            "push r15",
            "mov rdi, rsp", // Kaydedilmiş rsp'yi argüman olarak geçir (stack pointer)
            "call _timer_interrupt_handler_inner", // İç Rust işleyici fonksiyonunu çağır
            "pop r15",
            "pop r14",
            "pop r13",
            "pop r12",
            "pop r11",
            "pop r10",
            "pop r9",
            "pop r8",
            "pop rdi",
            "pop rsi",
            "pop rbp",
            "pop rsp", // Kaydedilmiş rsp'yi geri yükle
            "pop rbx",
            "pop rdx",
            "pop rcx",
            "pop rax",
            "iretq", // Kesmeden dönüş
            options(noreturn)
        );
    }
}

// İç Rust zamanlayıcı kesmesi işleyicisi (naked fonksiyondan çağrılır)
#[no_mangle]
pub extern "C" fn _timer_interrupt_handler_inner() {
    // Kesme sayacını artır
    TICKS.fetch_add(1, Ordering::SeqCst);

    // EOI (End of Interrupt) gönder - PIC'e (Programlanabilir Kesme Denetleyicisi) kesmenin işlendiğini bildir
    // Standart 8259 PIC için EOI adresi 0x20 (Ana PIC) ve 0xA0 (Bağlı PIC), kesme numarasına göre ayarlanabilir.
    // IRQ0 için ana PIC'e EOI göndermek yeterli olabilir.
    unsafe {
        // Basit örnek için Ana PIC'e (master PIC) EOI gönderiyoruz (0x20 portu).
        // Gerçek sistemde, kullanılan PIC yapısına ve kesme numarasına göre EOI adresi ve yöntemi değişebilir.
        // Ayrıca APIC (Gelişmiş Programlanabilir Kesme Denetleyicisi) kullanılıyorsa EOI yöntemi farklıdır.
        // Bu sadece basit bir örnek olduğu için 8259 PIC ve IRQ0 varsayımı ile ilerliyoruz.
        // Doğru EOI gönderme yöntemi donanım ve kesme denetleyiciye özgüdür ve platform dökümantasyonuna bakılmalıdır.
        out8(0x20, 0x20); // Ana PIC'e EOI sinyali gönder
    }

    // Bir sonraki kesmeyi ayarlamak için PIT'i yeniden programlama (gerekirse).
    // Rate Generator modunda PIT otomatik olarak yeniden başlayabilir, bu durumda yeniden programlama gerekmeyebilir.
    // Eğer frekansı değiştirmek veya farklı bir mod kullanmak istersek burada PIT'i yeniden programlamamız gerekir.
    // Bu basit örnek için PIT'in Rate Generator modunda ve sabit frekansta çalıştığını varsayıyoruz, bu nedenle
    // PIT'i her kesmede yeniden programlamıyoruz.
}


// I/O portlarına erişim için basit inline fonksiyonlar (x86 mimarisine özgü)
// **Dikkat**: Doğrudan I/O port erişimi unsafe'dir ve dikkatli kullanılmalıdır.
unsafe fn out8(port: u16, val: u8) {
    asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
}

#[allow(dead_code)] // Şimdilik kullanılmayan fonksiyon uyarısını gizle
unsafe fn in8(port: u16) -> u8 {
    let val: u8;
    asm!("in al, dx", out("dx") port, out("al") val, options(nomem, nostack, preserves_flags));
    val
}


pub fn init() {
    // PIT (Programlanabilir Aralık Zamanlayıcısı) başlatma
    // PIT'i Rate Generator moduna (mod 2) ve istenilen frekansa ayarlayın.
    // Kanal 0, Mod 2 (Rate Generator), erişim modu LSB sonra MSB, binary mod
    unsafe {
        out8(0x43, 0b00110100); // Kontrol word'ü yaz
        // Frekansı ayarlayın (bölücü değerini ayarlayarak).
        // Frekans = PIT Temel Frekansı / Bölücü.  Bölücü = PIT Temel Frekansı / İstenilen Kesme Frekansı.
        // Örneğin, 1ms periyot için, kesme frekansı 1000 Hz olmalı.
        // Bölücü = 1193182 / 1000 = 1193.182. Tamsayı bölücü kullanacağız, yaklaşık 1193 veya 1194.
        let divisor: u16 = (platform::CLOCK_FREQ / 1000) as u16; // 1ms periyot için bölücü
        let lsb = (divisor & 0xFF) as u8;
        let msb = ((divisor >> 8) & 0xFF) as u8;
        out8(0x40, lsb); // Kanal 0 veri portu (LSB)
        out8(0x40, msb); // Kanal 0 veri portu (MSB)

        // Kesmeleri etkinleştir - PIC (Programlanabilir Kesme Denetleyicisi) üzerinde IRQ0 hattını etkinleştir
        // 8259 PIC için, kesmeleri etkinleştirmek için OCW1 (Operation Control Word 1) kullanılır.
        // IRQ0, PIC'in IM (Interrupt Mask) kaydında 0. bit ile temsil edilir. Bu biti 0 yaparak IRQ0'ı unmask ediyoruz (etkinleştiriyoruz).
        let current_pic_mask = in8(0x21); // Mevcut maskeyi oku (OCW1 - PIC1 Mask register portu)
        out8(0x21, current_pic_mask & !0b00000001); // IRQ0'ı etkinleştir (0. biti temizle) ve maskeyi geri yaz

        // Global kesmeleri etkinleştir (x86 için sti komutu)
        enable_interrupts();
    }

    // Kesme işleyicisini ayarlayın - **Bu kısım platforma özgüdür ve Rust içinde doğrudan IDT ayarlamak karmaşık olabilir.**
    // Genellikle bootloader veya işletim sistemi çekirdeği başlangıcı tarafından IDT ayarlanır.
    // Bu örnek kodda, IDT'nin doğru şekilde ayarlandığı ve 8. kesme vektörünün (IRQ0 + 32 = Kesme Vektör No. 8)
    // `timer_interrupt_handler` fonksiyonuna yönlendirildiği varsayılır.
    // **Gerçek bir sistemde, IDT kurulumu ve kesme vektörlerinin doğru ayarlanması kritik öneme sahiptir.**
    // Bu kod, sadece zamanlayıcı sürücüsünün mantığını göstermek için basitleştirilmiştir.

    // Kesme işleyicisini ayarlamak için varsayımsal bir fonksiyon (gerçek uygulamada platforma özgü IDT yönetimi gereklidir)
    // interrupt::set_interrupt_handler(8, timer_interrupt_handler); // RISC-V örneğindeki gibi varsayımsal bir fonksiyon - x86'da IDT yönetimi farklıdır.
    // **x86'da genellikle IDT (Interrupt Descriptor Table) bootloader veya işletim sistemi çekirdeği tarafından kurulur ve yönetilir.**
    // Bu basit örnekte, IDT'nin zaten doğru ayarlandığını ve 8. vektörün `timer_interrupt_handler` adresine yönlendirildiğini **varsayıyoruz**.
    // Gerçek bir durumda, IDT'nin doğru kurulumu ve yönetimi bu sürücünün dışında ele alınmalıdır.
}


// Global kesmeleri etkinleştirmek için basit fonksiyon (x86 mimarisine özgü)
// **Dikkat**: Global kesmeleri etkinleştirmek unsafe'dir ve dikkatli kullanılmalıdır.
unsafe fn enable_interrupts() {
    asm!("sti", options(nomem, nostack, preserves_flags)); // Set Interrupt Flag (STI) - global kesmeleri etkinleştir
}

// Global kesmeleri devre dışı bırakmak için basit fonksiyon (x86 mimarisine özgü)
#[allow(dead_code)] // Şimdilik kullanılmayan fonksiyon uyarısını gizle
unsafe fn disable_interrupts() {
    asm!("cli", options(nomem, nostack, preserves_flags)); // Clear Interrupt Flag (CLI) - global kesmeleri devre dışı bırak
}