#![no_std]

use core::ptr::NonNull;
use core::panic::PanicInfo;

// Basit bir görev yapısı (Simple Task Structure)
pub struct Task {
    stack: [u8; 1024], // Her görev için 1KB yığın (1KB stack for each task)
    context: TaskContext,
}

#[derive(Default, Copy, Clone)]
#[repr(C)]
struct TaskContext {
    regs: [u32; 32], // Genel amaçlı yazmaçlar (General purpose registers - MIPS'te 32 adet)
    sp: u32,        // Yığın işaretçisi (Stack Pointer)
    ra: u32,        // Dönüş adresi (Return Address)
    fp: u32,        // Çerçeve işaretçisi (Frame Pointer - s8/fp olarak da bilinir)
    gp: u32,        // Global işaretçi (Global Pointer)
}

static mut CURRENT_TASK: Option<NonNull<Task>> = None;
static mut TASKS: [Option<Task>; 2] = [None, None]; // Görevleri tutmak için statik dizi, bu örnek için sabit boyut 2 (Static array to hold tasks, fixed size of 2 for this example)

pub fn init() {
    unsafe {
        // İlk görevi başlat (görev 0) (Initialize the first task (task 0))
        TASKS[0] = Some(Task { stack: [0; 1024], context: TaskContext::default() });
        // Mevcut görevi ilk göreve ayarla (Set the current task to the first task)
        CURRENT_TASK = NonNull::new(TASKS[0].as_mut().expect("TASKS[0] için mutable referans alınamadı (Failed to get mutable reference to TASKS[0])"));
        // Yığın işaretçisini ayarla (Initialize stack pointer for task 0 - önemli!)
        if let Some(task_ref) = &mut TASKS[0] {
            let task_ptr = task_ref as *mut Task;
            let stack_top = task_ptr as usize + 1024; // Yığının tepesi (top of the stack)
            task_ref.context.sp = stack_top as u32; // Yığın işaretçisini yığının tepesine ayarla (Set stack pointer to top of stack)
        }
    }
}

pub fn create_task(entry_point: fn()) {
    static mut TASK_ID: usize = 1; // Statik görev ID, görev 0 zaten başlatıldığı için 1'den başlar (Static task ID, starts from 1 as task 0 is already initialized)
    unsafe {
        // TASK_ID'ye göre görev yuvasına mutable referans al (Get a mutable reference to the task slot based on TASK_ID)
        let task_slot = TASKS.get_mut(TASK_ID).expect("TASK_ID, TASKS dizisi sınırlarının dışında (TASK_ID out of bounds for TASKS array)");
        // Görev yuvasında yeni bir görev oluştur (Create a new task in the task slot)
        *task_slot = Some(Task {
            stack: [0; 1024],
            context: TaskContext {
                ra: entry_point as u32, // Görevin giriş noktasını ayarla (Set the entry point of the task - dönüş adresi olarak ayarla)
                ..Default::default() // Diğer bağlam alanlarını varsayılandan devral (Inherit other context fields from default)
            }
        });
        // Yığın işaretçisini ayarla (Initialize stack pointer for the new task - önemli!)
        if let Some(task_ref) = task_slot {
            let task_ptr = task_ref as *mut Task;
            let stack_top = task_ptr as usize + 1024; // Yığının tepesi (top of the stack)
            task_ref.context.sp = stack_top as u32; // Yığın işaretçisini yığının tepesine ayarla (Set stack pointer to top of stack)
        }

        TASK_ID += 1; // Sonraki görev oluşturma için görev ID'sini artır (Increment task ID for the next task creation)
    }
}

pub fn switch_task() {
    unsafe {
        // Mevcut ve sonraki görevlere mutable referanslar al (görev 1, bu örnekte basitlik için sonraki görev olarak varsayılır) (Get mutable references to current and next tasks (task 1 is assumed to be the next task for simplicity in this example))
        let current_task = CURRENT_TASK.expect("CURRENT_TASK None değerinde (CURRENT_TASK is None)").as_mut();
        let next_task = TASKS.get_mut(1).expect("TASKS[1] None değerinde (TASKS[1] is None)").as_mut().expect("TASKS[1] için mutable referans alınamadı (Failed to get mutable reference to TASKS[1])");

        // Mevcut görevin bağlamını kaydet (Save current task's context)

        // Yığın işaretçisini kaydet ($sp) (Save Stack Pointer ($sp))
        asm!("move {}, $sp", out(reg) current_task.context.sp);
        // Dönüş adresini kaydet ($ra) (Save Return Address ($ra))
        asm!("move {}, $ra", out(reg) current_task.context.ra);
        // Çerçeve işaretçisini kaydet ($fp - s8) (Save Frame Pointer ($fp - s8))
        asm!("move {}, $fp", out(reg) current_task.context.fp);
        // Global işaretçiyi kaydet ($gp) (Save Global Pointer ($gp))
        asm!("move {}, $gp", out(reg) current_task.context.gp);
        // s0-s7 yazmaçlarını kaydet (Save s0-s7 registers)
        asm!("move {}, $s0", out(reg) current_task.context.regs[16]); // s0 - regs[16]
        asm!("move {}, $s1", out(reg) current_task.context.regs[17]); // s1 - regs[17]
        asm!("move {}, $s2", out(reg) current_task.context.regs[18]); // s2 - regs[18]
        asm!("move {}, $s3", out(reg) current_task.context.regs[19]); // s3 - regs[19]
        asm!("move {}, $s4", out(reg) current_task.context.regs[20]); // s4 - regs[20]
        asm!("move {}, $s5", out(reg) current_task.context.regs[21]); // s5 - regs[21]
        asm!("move {}, $s6", out(reg) current_task.context.regs[22]); // s6 - regs[22]
        asm!("move {}, $s7", out(reg) current_task.context.regs[23]); // s7 - regs[23]


        // Sonraki görevin bağlamına geç (Switch to the next task's context)

        // Sonraki görevin yığın işaretçisini yükle ($sp) (Load next task's Stack Pointer ($sp))
        asm!("move $sp, {}", in(reg) next_task.context.sp);
        // Sonraki görevin dönüş adresini yükle ($ra) (Load next task's Return Address ($ra))
        asm!("move $ra, {}", in(reg) next_task.context.ra);
        // Sonraki görevin çerçeve işaretçisini yükle ($fp - s8) (Load next task's Frame Pointer ($fp - s8))
        asm!("move $fp, {}", in(reg) next_task.context.fp);
        // Sonraki görevin global işaretçisini yükle ($gp) (Load next task's Global Pointer ($gp))
        asm!("move $gp, {}", in(reg) next_task.context.gp);
        // s0-s7 yazmaçlarını yükle (Load s0-s7 registers)
        asm!("move $s0, {}", in(reg) next_task.context.regs[16]); // s0 - regs[16]
        asm!("move $s1, {}", in(reg) next_task.context.regs[17]); // s1 - regs[17]
        asm!("move $s2, {}", in(reg) next_task.context.regs[18]); // s2 - regs[18]
        asm!("move $s3, {}", in(reg) next_task.context.regs[19]); // s3 - regs[19]
        asm!("move $s4, {}", in(reg) next_task.context.regs[20]); // s4 - regs[20]
        asm!("move $s5, {}", in(reg) next_task.context.regs[21]); // s5 - regs[21]
        asm!("move $s6, {}", in(reg) next_task.context.regs[22]); // s6 - regs[22]
        asm!("move $s7, {}", in(reg) next_task.context.regs[23]); // s7 - regs[23]


        // CURRENT_TASK'ı sonraki göreve güncelle (Update CURRENT_TASK to the next task)
        CURRENT_TASK = NonNull::new(next_task as *mut Task);
    }
}

// Örnek görev giriş noktası (sadece gösteri amaçlı) (Example task entry point (just for demonstration))
fn task1_entry() {
    let mut count = 0;
    loop {
        count += 1;
        // Gerçek bir no_std ortamında, gecikmeler için donanım zamanlayıcısı veya benzeri kullanabilirsiniz.
        // Bu örnek için, basit bir döngü yer tutucu olarak hizmet eder.
        for _ in 0..100000 {
            // zaman harca (waste time)
        }
        unsafe {
            // Görev değiştirmenin temel çıktısı (seri gibi bir çıktı biçiminin başka bir yerde başlatıldığını varsayarak)
            let ptr = 0xbfc00000 as *mut u32; // Çıktı için örnek bellek adresi (Example memory address for output - MIPS MMIO alanı genellikle bu adreste başlar)
            ptr.write_volatile(count);
        }
    }
}

// Örnek kullanım (gösteri için kavramsal ana fonksiyon) (Example usage (conceptual main function for demonstration))
fn main() {
    init(); // Görev yönetimini başlat (Initialize task management)

    create_task(task1_entry); // Görev 1'i oluştur ve giriş noktasını ayarla (Create task 1 and set its entry point)

    // İlk görevi çalıştırmaya başla (görev 0 init'te başlatılır).
    // Gerçek bir işletim sisteminde, görev 0 ilk kurulumu yapabilir ve ardından görev değiştirmeyi başlatabilir.
    // (Start running the first task (task 0 is initialized in init).
    // In a real OS, task 0 might do initial setup and then start task switching.)

    loop {
        unsafe {
            // Gösteri için, görevleri bir döngü içinde değiştir.
            // Gerçek bir sistemde, görev değiştirme olaylarla tetiklenir (örn. zamanlayıcı kesmesi).
            // (For demonstration, switch tasks in a loop.
            // In a real system, task switching would be triggered by events (e.g., timer interrupt).)
            switch_task();
            for _ in 0..200000 { // Görev 0'da zaman harca (waste time in task 0)
                // zaman harca (waste time)
            }
            let ptr = 0xbfc00004 as *mut u32; // Görev 0 için çıktı için örnek bellek adresi (Example memory address for output for task 0 - MIPS MMIO alanı genellikle bu adreste başlar)
            ptr.write_volatile(0); // Görev 0'ın çalıştığını göster (Indicate task 0 is running)
        }
    }
}

// no_std ortamı için gerekli, panik işleyicisini tanımla (Required for no_std environment, define panic handler)
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}