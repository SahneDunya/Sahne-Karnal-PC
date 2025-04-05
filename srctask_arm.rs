#![no_std]

use core::ptr::NonNull;
use core::panic::PanicInfo;

// ARM için Basit Task Yapısı
pub struct Task {
    stack: [u8; 1024], // Her task için 1KB stack
    context: TaskContext,
}

#[derive(Default, Copy, Clone)]
#[repr(C)]
struct TaskContext {
    // ARM mimarisi için context (basit örnek için genel amaçlı registerlar yeterli)
    r: [usize; 16], // R0-R15 Genel amaçlı registerlar. R13(SP), R14(LR), R15(PC)
    sp: usize,      // Stack Pointer (R13)
    lr: usize,      // Link Register (R14) - Fonksiyon dönüş adresleri için
    pc: usize,      // Program Counter (R15) - Bir sonraki çalıştırılacak komutun adresi
    cpsr: usize,    // Current Program Status Register - Mevcut işlemci durumunu tutar
}

static mut CURRENT_TASK: Option<NonNull<Task>> = None;
static mut TASKS: [Option<Task>; 2] = [None, None]; // Statik task dizisi, bu örnek için sabit boyut 2

pub fn init() {
    unsafe {
        // İlk task'ı (task 0) başlat
        TASKS[0] = Some(Task { stack: [0; 1024], context: TaskContext::default() });
        // Mevcut task'ı ilk task olarak ayarla
        CURRENT_TASK = NonNull::new(TASKS[0].as_mut().expect("TASKS[0] için mutable referans alınamadı"));
    }
}

pub fn create_task(entry_point: fn()) {
    static mut TASK_ID: usize = 1; // Statik task ID, task 0 zaten başlatıldığı için 1'den başlar
    unsafe {
        // TASK_ID'ye göre task slotuna mutable referans al
        let task_slot = TASKS.get_mut(TASK_ID).expect("TASK_ID, TASKS dizisi sınırlarının dışında");
        // Task slotunda yeni bir task oluştur
        *task_slot = Some(Task {
            stack: [0; 1024],
            context: TaskContext {
                pc: entry_point as usize, // Task'ın giriş noktasını ayarla (Program Counter)
                sp: task_slot.as_mut().unwrap().stack.as_ptr_range().end as usize, // Stack'in sonunu stack pointer olarak ayarla
                cpsr: 0x53, // Örnek CPSR değeri (Thumb mode, Supervisor mode, IRQ/FIQ aktif değil) - **ARM mimarisine göre ayarlanmalı!**
                ..Default::default() // Diğer context alanlarını default değerlerden al
            }
        });
        TASK_ID += 1; // Bir sonraki task oluşturma için task ID'yi arttır
    }
}

pub fn switch_task() {
    unsafe {
        // Mevcut ve sonraki task'lara mutable referanslar al (task 1 sonraki task olarak varsayılır)
        let current_task = CURRENT_TASK.expect("CURRENT_TASK None").as_mut();
        let next_task = TASKS.get_mut(1).expect("TASKS[1] None").as_mut().expect("TASKS[1] için mutable referans alınamadı");

        // Mevcut task'ın context'ini kaydet
        // CPSR register'ını kaydet
        asm!(" mrs {}, CPSR", out(reg) current_task.context.cpsr :::: "volatile");
        // PC register'ını kaydet (Temel olarak LR'yi kaydetmek yeterli olabilir, örnek basit tutuluyor)
        asm!(" mov {}, pc", out(reg) current_task.context.pc :::: "volatile"); // **Bu satır PC'yi kaydetmek için doğru olmayabilir, ARM mimarisine göre düzeltilmeli!**
        // Stack pointer'ı (SP/R13) kaydet
        asm!(" mov {}, sp", out(reg) current_task.context.sp :::: "volatile");
        // Link Register'ı (LR/R14) kaydet
        asm!(" mov {}, lr", out(reg) current_task.context.lr :::: "volatile");

        // Genel amaçlı registerları kaydet (R0-R12 - Örnek basitleştirilmiş, tüm registerlar kaydedilmiyor)
        for i in 0..13 { // R0-R12 (R13-R15 özel registerlar)
            asm!(" mov {}, r{}", out(reg) current_task.context.r[i] , i :::: "volatile");
        }


        // Sonraki task'ın context'ine geç
        // Sonraki task'ın CPSR register'ını yükle
        asm!(" msr CPSR, {}", in(reg) next_task.context.cpsr :::: "volatile");
        // Sonraki task'ın PC register'ını yükle (Aslında başlangıç adresi SEPC gibi bir alanda tutulmalıydı RISC-V örneğinde olduğu gibi, örnek basit)
        asm!(" mov pc, {}", in(reg) next_task.context.pc :::: "volatile"); // **Bu satır PC'yi yüklemek için doğru olmayabilir, ARM mimarisine göre düzeltilmeli! Genellikle context restore'da PC doğrudan set edilmez, exception dönüş mekanizmaları veya benzeri kullanılır.**
        // Sonraki task'ın stack pointer'ını yükle
        asm!(" mov sp, {}", in(reg) next_task.context.sp :::: "volatile");
        // Sonraki task'ın Link Register'ını yükle
        asm!(" mov lr, {}", in(reg) next_task.context.lr :::: "volatile");

        // Genel amaçlı registerları yükle (R0-R12)
        for i in 0..13 {
            asm!(" mov r{}, {}, ", i, in(reg) next_task.context.r[i] :::: "volatile");
        }


        // CURRENT_TASK'ı sonraki task olarak güncelle
        CURRENT_TASK = NonNull::new(next_task as *mut Task);
    }
}

// Örnek task giriş noktası (sadece gösteri amaçlı)
fn task1_entry() {
    let mut count = 0;
    loop {
        count += 1;
        // Gerçek bir no_std ortamında, gecikmeler için donanım timer'ı vb. kullanılabilir.
        // Bu örnek için basit bir döngü yer tutucu olarak hizmet eder.
        for _ in 0..100000 {
            // zaman harca
        }
        unsafe {
            // Task switching'i belirtmek için temel çıktı (seri port gibi bir çıkışın başka yerde başlatıldığı varsayılır)
            let ptr = 0x80000000 as *mut u32; // Örnek bellek adresi
            ptr.write_volatile(count);
        }
    }
}

// Örnek kullanım (kavramsal main fonksiyonu, gerçek main fonksiyonu farklı olabilir)
fn main() {
    init(); // Task yönetimini başlat

    create_task(task1_entry); // Task 1'i oluştur ve giriş noktasını ayarla

    // İlk task'ı çalıştırmaya başla (task 0 init() içinde başlatılır).
    // Gerçek bir işletim sisteminde, task 0 ilk kurulumu yapar ve sonra task switching'i başlatır.

    loop {
        unsafe {
            // Gösteri için, döngü içinde task'ları değiştir.
            // Gerçek bir sistemde, task switching olaylar tarafından tetiklenir (örn. timer interrupt).
            switch_task();
            for _ in 0..200000 { // task 0'da zaman harca
                // zaman harca
            }
            let ptr = 0x80000004 as *mut u32; // Task 0 için örnek bellek adresi
            ptr.write_volatile(0); // Task 0'ın çalıştığını belirt
        }
    }
}

// no_std ortamı için gerekli, panic handler tanımla
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}