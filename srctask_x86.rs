#![no_std]
use core::ptr::NonNull;
use core::panic::PanicInfo;

// Basit görev yapısı (Simple task structure)
pub struct Task {
    stack: [u8; 1024], // Her görev için 1KB yığın (1KB stack for each task)
    context: TaskContext,
}

#[derive(Default, Copy, Clone)]
#[repr(C)]
struct TaskContext {
    // x86-64 genel amaçlı registerler (x86-64 general purpose registers)
    // Burada sadece örnek olarak bazı temel registerler dahil edilmiştir.
    // (Only some basic registers are included here as an example.)
    rsp: u64, // Yığın işaretçisi (Stack Pointer)
    rip: u64, // Komut işaretçisi (Instruction Pointer)
    rbx: u64, // Örnek genel amaçlı register (Example general purpose register)
    rflags: u64, // Bayrak registeri (Flags register) - Zorunlu olmayabilir basit örnek için (Might not be necessary for a simple example)
    rbp: u64, // Base Pointer
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
}

static mut CURRENT_TASK: Option<NonNull<Task>> = None;
static mut TASKS: [Option<Task>; 2] = [None, None]; // Görevleri tutmak için statik dizi, bu örnek için sabit boyut 2 (Static array to hold tasks, fixed size of 2 for this example)

pub fn init() {
    unsafe {
        // İlk görevi başlat (görev 0) (Initialize the first task (task 0))
        TASKS[0] = Some(Task { stack: [0; 1024], context: TaskContext::default() });
        // Mevcut görevi ilk göreve ayarla (Set the current task to the first task)
        CURRENT_TASK = NonNull::new(TASKS[0].as_mut().expect("TASKS[0] için mutable referans alınamadı (Failed to get mutable reference to TASKS[0])"));
    }
}

pub fn create_task(entry_point: fn()) {
    static mut TASK_ID: usize = 1; // Statik görev ID, görev 0 zaten başlatıldığı için 1'den başlar (Static task ID, starts from 1 as task 0 is already initialized)
    unsafe {
        // TASK_ID'ye göre görev slotuna mutable referans al (Get a mutable reference to the task slot based on TASK_ID)
        let task_slot = TASKS.get_mut(TASK_ID).expect("TASK_ID TASKS dizisi sınırları dışında (TASK_ID out of bounds for TASKS array)");
        // Görev slotunda yeni görev oluştur (Create a new task in the task slot)
        *task_slot = Some(Task {
            stack: [0; 1024],
            context: TaskContext {
                rip: entry_point as u64, // Görevin giriş noktasını ayarla (Set the entry point of the task)
                ..Default::default() // Diğer context alanlarını varsayılandan devral (Inherit other context fields from default)
            }
        });
        TASK_ID += 1; // Sonraki görev oluşturma için görev ID'sini artır (Increment task ID for the next task creation)
    }
}

#[naked] // Yığın çerçevesi oluşturmayı önler (Prevents stack frame creation)
#[no_caller_safety] // Çağıranın güvenliğini garanti etmez (Does not guarantee caller safety)
pub unsafe extern "C" fn switch_task() {
    // `naked` fonksiyon kullanıldığı için tüm koruma/geri yükleme işlemleri elle yapılmalıdır.
    // (Since `naked` function is used, all save/restore operations must be done manually.)

    // Mevcut görev context'ini kaydet (Save current task's context)
    // CURRENT_TASK'dan mevcut görevi al (Get current task from CURRENT_TASK)
    let current_task_ptr = CURRENT_TASK.expect("CURRENT_TASK is None").as_mut();

    // `rsp`, `rip`, `rbx`, `rflags` ve diğer registerler context'e kaydedilir.
    // (`rsp`, `rip`, `rbx`, `rflags` and other registers are saved to context.)
    asm!(
        "mov %rsp, {rsp}", // Yığın işaretçisini kaydet (Save stack pointer)
        "mov %rip, {rip}", // Komut işaretçisini kaydet (Save instruction pointer) -  Gerekli olmayabilir, fonksiyon dönüş adresi otomatik kaydediliyor olabilir. (Might not be necessary, function return address might be automatically saved.)
        "mov %rbx, {rbx}", // RBX kaydet (Save RBX)
        "pushfq",          // RFLAGS'ı yığına kaydet (Push RFLAGS onto the stack)
        "pop {rflags}",   // RFLAGS'ı context'e taşı (Pop RFLAGS to context)
        "mov %rbp, {rbp}",
        "mov %r12, {r12}",
        "mov %r13, {r13}",
        "mov %r14, {r14}",
        "mov %r15, {r15}",


        rsp = inout(reg) current_task_ptr.context.rsp => _,
        rip = inout(reg) current_task_ptr.context.rip => _, // RIP'i kaydetmek için doğru yol emin değil (Not sure if this is the correct way to save RIP)
        rbx = inout(reg) current_task_ptr.context.rbx => _,
        rflags = inout(reg) current_task_ptr.context.rflags => _,
        rbp = inout(reg) current_task_ptr.context.rbp => _,
        r12 = inout(reg) current_task_ptr.context.r12 => _,
        r13 = inout(reg) current_task_ptr.context.r13 => _,
        r14 = inout(reg) current_task_ptr.context.r14 => _,
        r15 = inout(reg) current_task_ptr.context.r15 => _,

        options(noreturn) // Fonksiyonun geri dönmeyeceğini belirt (Indicate function will not return normally)
    );

    // Sonraki görev context'ine geç (Switch to the next task's context)
    // Basitlik için sonraki görev görev 1 olarak varsayılır (For simplicity, next task is assumed to be task 1)
    let next_task_ptr = TASKS.get_mut(1).expect("TASKS[1] is None").as_mut().expect("TASKS[1] için mutable referans alınamadı (Failed to get mutable reference to TASKS[1])");

    // CURRENT_TASK'ı sonraki göreve güncelle (Update CURRENT_TASK to the next task)
    CURRENT_TASK = NonNull::new(next_task_ptr as *mut Task);

    // Sonraki görev context'ini yükle (Load next task's context)
    asm!(
        "mov {rsp}, %rsp", // Yığın işaretçisini yükle (Load stack pointer)
        "mov {rip}, %rip", // Komut işaretçisini yükle (Load instruction pointer) - Gerekli olmayabilir, `ret` komutu RIP'i ayarlıyor olabilir. (Might not be necessary, `ret` instruction might set RIP.)
        "mov {rbx}, %rbx", // RBX yükle (Load RBX)
        "push {rflags}",    // RFLAGS'ı yığına yükle (Push RFLAGS onto the stack)
        "popfq",            // RFLAGS'ı registere taşı (Pop RFLAGS to register)
        "mov {rbp}, %rbp",
        "mov {r12}, %r12",
        "mov {r13}, %r13",
        "mov {r14}, %r14",
        "mov {r15}, %r15",
        "ret",              // Görev giriş noktasına geri dön (Return to task entry point - RIP daha önce context'ten yüklendi varsayımıyla (assuming RIP was loaded from context))

        rsp = in(reg) next_task_ptr.context.rsp,
        rip = in(reg) next_task_ptr.context.rip, // RIP'i yüklemek için doğru yol emin değil (Not sure if this is the correct way to load RIP)
        rbx = in(reg) next_task_ptr.context.rbx,
        rflags = in(reg) next_task_ptr.context.rflags,
        rbp = in(reg) next_task_ptr.context.rbp,
        r12 = in(reg) next_task_ptr.context.r12,
        r13 = in(reg) next_task_ptr.context.r13,
        r14 = in(reg) next_task_ptr.context.r14,
        r15 = in(reg) next_task_ptr.context.r15,

        options(noreturn) // Fonksiyonun geri dönmeyeceğini belirt (Indicate function will not return normally)
    );
}


// Örnek görev giriş noktası (sadece gösteri amaçlı) (Example task entry point (just for demonstration))
fn task1_entry() {
    let mut count = 0;
    loop {
        count += 1;
        // Gerçek bir no_std ortamında, gecikmeler için donanım zamanlayıcısı veya benzeri kullanabilirsiniz.
        // (In a real no_std environment, you might use a hardware timer or similar for delays.)
        // Bu örnek için basit bir döngü yer tutucu olarak hizmet eder. (For this example, a simple loop serves as a placeholder.)
        for _ in 0..100000 {
            // zaman harca (waste time)
        }
        unsafe {
            // Görev geçişini belirtmek için temel çıktı (seri port gibi bir çıktı biçiminin başka bir yerde başlatıldığını varsayarak)
            // (Basic output to indicate task switching (assuming some form of output like serial is initialized elsewhere))
            let ptr = 0x80000000 as *mut u32; // Örnek bellek adresi çıktı için (Example memory address for output)
            ptr.write_volatile(count);
        }
    }
}

// Örnek kullanım (kavramsal main fonksiyon gösteri için) (Example usage (conceptual main function for demonstration))
fn main() {
    init(); // Görev yönetimini başlat (Initialize task management)

    create_task(task1_entry); // Görev 1'i oluştur ve giriş noktasını ayarla (Create task 1 and set its entry point)

    // İlk görevi çalıştırmaya başla (görev 0 init'te başlatılır).
    // (Start running the first task (task 0 is initialized in init).)
    // Gerçek bir işletim sisteminde, görev 0 başlangıç kurulumunu yapabilir ve ardından görev geçişini başlatabilir.
    // (In a real OS, task 0 might do initial setup and then start task switching.)

    loop {
        unsafe {
            // Gösteri için, görevleri bir döngüde değiştir.
            // (For demonstration, switch tasks in a loop.)
            // Gerçek bir sistemde, görev geçişi olaylar tarafından tetiklenir (örneğin, zamanlayıcı kesmesi).
            // (In a real system, task switching would be triggered by events (e.g., timer interrupt).)
            switch_task();
            for _ in 0..200000 { // görev 0'da zaman harca (waste time in task 0)
                // zaman harca (waste time)
            }
            let ptr = 0x80000004 as *mut u32; // Görev 0 için çıktı için örnek bellek adresi (Example memory address for output for task 0)
            ptr.write_volatile(0); // Görev 0'ın çalıştığını belirt (Indicate task 0 is running)
        }
    }
}

// no_std ortamı için gerekli, panic handler tanımla (Required for no_std environment, define panic handler)
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}