#![no_std]
use core::arch::asm;
use core::fmt;
use core::panic::PanicInfo;

// İstisna Sebepleri (x86 mimarisi için yaygın sebepler)
#[repr(u32)]
enum ExceptionCause {
    DivideError = 0,          // Bölme Hatası
    DebugException = 1,       // Hata Ayıklama İstisnası
    Breakpoint = 3,           // Kesme Noktası
    InvalidOpcode = 6,        // Geçersiz İşlem Kodu
    PageFault = 14,           // Sayfa Hatası
    GeneralProtectionFault = 13, // Genel Koruma Hatası
    DoubleFault = 8,          // Çift Hata
    StackSegmentFault = 12,     // Yığın Segmenti Hatası
    Unknown = 0xFFFF,         // Bilinmeyen sebepler için
}

impl fmt::Display for ExceptionCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExceptionCause::DivideError => write!(f, "Bölme hatası"),
            ExceptionCause::DebugException => write!(f, "Hata ayıklama istisnası"),
            ExceptionCause::Breakpoint => write!(f, "Kesme noktası"),
            ExceptionCause::InvalidOpcode => write!(f, "Geçersiz işlem kodu"),
            ExceptionCause::PageFault => write!(f, "Sayfa hatası"),
            ExceptionCause::GeneralProtectionFault => write!(f, "Genel koruma hatası"),
            ExceptionCause::DoubleFault => write!(f, "Çift hata"),
            ExceptionCause::StackSegmentFault => write!(f, "Yığın segmenti hatası"),
            ExceptionCause::Unknown => write!(f, "Bilinmeyen sebep"),
        }
    }
}

// ExceptionContext yapısı (x86'ya özgü bağlam bilgileri)
#[derive(Debug)]
#[repr(C)]
pub struct ExceptionContext {
    pub rip: u64,        // Instruction Pointer (İstisna anındaki komut adresi)
    pub rsp: u64,        // Stack Pointer
    pub rflags: u64,     // RFLAGS kayıt değeri
    pub error_code: u64, // Hata kodu (bazı istisnalar için geçerli)
    // ... diğer kayıtlar ve bağlam bilgileri eklenebilir
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub cs: u64,
    pub ds: u64,
    pub es: u64,
    pub fs: u64,
    pub gs: u64,
    pub ss: u64,
}

// println! makrosu için basit bir uygulama (MMIO tabanlı UART simülasyonu)
mod io {
    use core::fmt::Write;

    // MMIO adresleri (örnek olarak - x86 sisteminize göre değişebilir!)
    const UART_DATA: u32 = 0x3F8;   // Genellikle COM1 için kullanılır
    const UART_STATUS: u32 = 0x3FD; // FIFO kontrol/durum kaydı (örnek)

    struct Stdout;

    impl Write for Stdout {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            for byte in s.bytes() {
                unsafe {
                    // Basit MMIO simülasyonu - x86 port G/Ç kullanılarak
                    core::arch::asm!(
                        "out dx, al",
                        in("dx") UART_DATA as u16,
                        in("al") byte,
                    );
                }
            }
            Ok(())
        }
    }

    pub fn println(s: &str) {
        let mut stdout = Stdout;
        core::fmt::write!(&mut stdout, "{}\n", s).unwrap();
    }
}


#[no_mangle]
extern "C" fn exception_handler(context: &ExceptionContext) {
    let cause = match context.error_code { // Hata kodunu kullanarak sebebi belirlemeye çalışın (basit örnek)
        0 => match context.rip { // Daha detaylı sebep analizi için RIP veya başka bağlam bilgileri kullanılabilir
            _ => ExceptionCause::Unknown, // Şu an için basit bir Unknown ataması
        },
        _ => ExceptionCause::Unknown, // Genel Unknown durumu (geliştirilebilir)
    };

    io::println!("İSTİSNA OLUŞTU (x86)!");
    io::println!("RIP: {:#x}", context.rip);
    io::println!("RSP: {:#x}", context.rsp);
    io::println!("RFLAGS: {:#x}", context.rflags);
    io::println!("Hata Kodu (varsa): {:#x}", context.error_code);
    io::println!("Sebep: {}", cause); // Sebep şu an için çok basitçe Unknown olarak işaretleniyor

    io::println!("Kayıtlar:");
    io::println!("  RAX: {:#x}", context.rax);
    io::println!("  RBX: {:#x}", context.rbx);
    io::println!("  RCX: {:#x}", context.rcx);
    io::println!("  RDX: {:#x}", context.rdx);
    io::println!("  RSI: {:#x}", context.rsi);
    io::println!("  RDI: {:#x}", context.rdi);
    io::println!("  RBP: {:#x}", context.rbp);
    io::println!("  RSP (exception context): {:#x}", context.rsp); // Tekrar, bağlamdaki RSP
    io::println!("  R8: {:#x}", context.r8);
    io::println!("  R9: {:#x}", context.r9);
    io::println!("  R10: {:#x}", context.r10);
    io::println!("  R11: {:#x}", context.r11);
    io::println!("  R12: {:#x}", context.r12);
    io::println!("  R13: {:#x}", context.r13);
    io::println!("  R14: {:#x}", context.r14);
    io::println!("  R15: {:#x}", context.r15);
    io::println!("  CS: {:#x}", context.cs);
    io::println!("  DS: {:#x}", context.ds);
    io::println!("  ES: {:#x}", context.es);
    io::println!("  FS: {:#x}", context.fs);
    io::println!("  GS: {:#x}", context.gs);
    io::println!("  SS: {:#x}", context.ss);


    panic!("İstisna İşlenemedi (x86): {}", cause);
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    io::println!("PANIC! (x86)");
    if let Some(location) = info.location() {
        io::println!("Dosya: {}, Satır: {}", location.file(), location.line());
    }
    if let Some(message) = info.message() {
        io::println!("Mesaj: {}", message);
    }

    loop {} // Sonsuz döngüde kal
}

// Basit IDT girişi yapısı (64-bit için)
#[repr(C, packed)]
struct IDTEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,       // Interrupt Stack Table index (genellikle 0)
    type_attributes: u8, // Tip ve özellik bitleri
    offset_middle: u16,
    offset_high: u32,
    reserved: u32,
}

static mut IDT: [IDTEntry; 256] = [IDTEntry { // 256 girişlik IDT
    offset_low: 0,
    selector: 0,
    ist: 0,
    type_attributes: 0,
    offset_middle: 0,
    offset_high: 0,
    reserved: 0,
}; 256];

// IDT işaretçi yapısı
#[repr(C, packed)]
struct IDTPointer {
    limit: u16,
    base: u64,
}

static IDT_POINTER: IDTPointer = IDTPointer {
    limit: (core::mem::size_of::<[IDTEntry; 256]>() - 1) as u16,
    base: unsafe { &IDT as *const _ as u64 },
};


// Basit bir IDT girişi oluşturma fonksiyonu
fn create_idt_entry(handler_addr: u64) -> IDTEntry {
    IDTEntry {
        offset_low: (handler_addr & 0xFFFF) as u16,
        selector: 0x08, // Kod segmenti seçicisi (tipik olarak 0x08 - kernel kodu segmenti)
        ist: 0,
        type_attributes: 0x8E, // P=1, DPL=00, S=0, Type=1110 (Interrupt Gate)
        offset_middle: ((handler_addr >> 16) & 0xFFFF) as u16,
        offset_high: (handler_addr >> 32) as u32,
        reserved: 0,
    }
}

// Herhangi bir istisna için basit assembly handler stub (tüm kayıtları stack'e iter ve Rust handler'ı çağırır)
#[naked] // Derleyicinin ekstra kod eklemesini engeller
#[no_mangle]
unsafe extern "C" fn exception_handler_stub() {
    asm!(
        "push rax",
        "push rbx",
        "push rcx",
        "push rdx",
        "push rsi",
        "push rdi",
        "push rbp",
        "push r8",
        "push r9",
        "push r10",
        "push r11",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "push ds", // Data segment kayıtlarını da iter
        "push es",
        "push fs",
        "push gs",
        "mov rdi, rsp", // Bağlam bilgisinin adresini ilk argüman olarak ayarla (ExceptionContext*)
        "call exception_handler", // Rust exception_handler fonksiyonunu çağır
        "pop gs",      // Segment kayıtlarını geri yükle
        "pop fs",
        "pop es",
        "pop ds",
        "pop r15",     // Kayıtları stack'ten geri al
        "pop r14",
        "pop r13",
        "pop r12",
        "pop r11",
        "pop r10",
        "pop r9",
        "pop r8",
        "pop rbp",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",
        "pop rbx",
        "pop rax",
        "add rsp, 8",   // Hata kodunu (eğer varsa) stack'ten temizle (bu örnekte hep temizliyoruz, gerçekte kontrol edilmeli)
        "iretq",        // İstisnadan geri dön
        options(noreturn) // Fonksiyonun geri dönmeyeceğini belirt
    )
}


pub fn init() {
    // IDT'yi ayarla
    unsafe {
        // Örnek olarak sadece Divide-by-Zero (vector 0) ve Breakpoint (vector 3) için handler ayarlıyoruz.
        IDT[0] = create_idt_entry(exception_handler_stub as u64); // Divide-by-Zero
        IDT[3] = create_idt_entry(exception_handler_stub as u64); // Breakpoint
        IDT[6] = create_idt_entry(exception_handler_stub as u64); // Invalid Opcode
        IDT[8] = create_idt_entry(exception_handler_stub as u64); // Double Fault
        IDT[13] = create_idt_entry(exception_handler_stub as u64); // General Protection Fault
        IDT[14] = create_idt_entry(exception_handler_stub as u64); // Page Fault

        // IDTR kaydını IDT adres ve limiti ile yükle
        asm!("lidt [{}]", in(reg) &IDT_POINTER, options(att_syntax));
    }

    io::println!("IDT ayarlandı ve istisna işleyicisi kuruldu. (x86)");
}