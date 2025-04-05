#![no_std]

use core::ptr::NonNull;
use core::panic::PanicInfo;

// Basit bir görev yapısı (Simple task structure)
pub struct Task {
    stack: [u8; 1024], // Her görev için 1KB yığın (1KB stack for each task)
    context: TaskContext,
}

#[derive(Default, Copy, Clone)]
#[repr(C)]
struct TaskContext {
    gpr: [u32; 32], // Genel amaçlı yazmaçlar (General Purpose Registers - r0-r31) - PowerPC'de 32-bit varsayılmıştır (assumed 32-bit in PowerPC for this example)
    sp: u32,        // Yığın işaretçisi (Stack Pointer - r1/sp)
    pc: u32,        // Program Sayacı (Program Counter)
    msr: u32,       // Makine Durum Yazmacı (Machine State Register) - Temel durum için (for basic state)
    cr: u32,        // Durum Yazmacı (Condition Register)
    lr: u32,        // Bağlantı Yazmacı (Link Register) - Geri dönüş adresleri için (for return addresses)
    ctr: u32,       // Sayım Yazmacı (Count Register) - Döngüler vb. için (for loops etc.)
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
        // Görev slotuna TASK_ID'ye göre mutable referans al (Get a mutable reference to the task slot based on TASK_ID)
        let task_slot = TASKS.get_mut(TASK_ID).expect("TASK_ID, TASKS dizisi sınırlarının dışında (TASK_ID out of bounds for TASKS array)");
        // Görev slotunda yeni bir görev oluştur (Create a new task in the task slot)
        *task_slot = Some(Task {
            stack: [0; 1024],
            context: TaskContext {
                pc: entry_point as u32, // Görevin başlangıç noktasını ayarla (Set the entry point of the task)
                ..Default::default()     // Diğer context alanlarını varsayılan değerlerden al (Inherit other context fields from default)
            }
        });
        TASK_ID += 1; // Sonraki görev oluşturma için görev ID'sini arttır (Increment task ID for the next task creation)
    }
}

pub fn switch_task() {
    unsafe {
        // Mevcut ve sonraki görevlere mutable referanslar al (bu örnekte sonraki görev görev 1 varsayılıyor) (Get mutable references to current and next tasks (task 1 is assumed to be the next task for simplicity in this example))
        let current_task = CURRENT_TASK.expect("CURRENT_TASK None").as_mut();
        let next_task = TASKS.get_mut(1).expect("TASKS[1] None").as_mut().expect("TASKS[1] için mutable referans alınamadı (Failed to get mutable reference to TASKS[1])");

        // Mevcut görevin context'ini kaydet (Save current task's context)

        // Stack pointer'ı kaydet (r1 genellikle yığın işaretçisidir) (Save Stack pointer (r1 is typically stack pointer))
        asm!("mr {}, r1", out(reg) current_task.context.sp); // PowerPC'de r1 genellikle SP'dir (r1 is often SP in PowerPC)

        // Program sayacını (PC) kaydetmek - Fonksiyon çağrısı yaptığımız için PC otomatik kaydedilir, ancak görev devamlılığı için 'context.pc'yi kullanacağız.
        // (Saving Program Counter (PC) - PC is automatically saved due to function call, but we will use 'context.pc' for task continuity.)
        // Bu örnekte, PC'yi açıkça kaydetmeyeceğiz, ancak görev başlangıç noktasını 'context.pc' içinde tutuyoruz.
        // (In this example, we won't explicitly save PC here, but we keep the task's entry point in 'context.pc'.)

        // Makine Durum Yazmacını (MSR) kaydet (Save Machine State Register (MSR))
        asm!("mfmsr {},", out(reg) current_task.context.msr);

        // Durum Yazmacını (CR) kaydet (Save Condition Register (CR))
        asm!("mfcr {},", out(reg) current_task.context.cr);

        // Bağlantı Yazmacını (LR) kaydet (Save Link Register (LR))
        asm!("mflr {},", out(reg) current_task.context.lr);

        // Sayım Yazmacını (CTR) kaydet (Save Count Register (CTR))
        asm!("mfctr {},", out(reg) current_task.context.ctr);


        // Genel amaçlı yazmaçları (r2-r31) yığına kaydet (r0 ve r1 genellikle özel amaçlıdır) (Save general purpose registers (r2-r31) to stack (r0 and r1 are usually special purpose))
        // r0, r1 ve SP'yi (r1) kaydetmiyoruz, çünkü bunlar geçici veya özel amaçlıdır (Not saving r0, r1, and SP (r1) as they are temporary or special purpose)
        asm!("
            stw r2,  0(r1)
            stw r3,  4(r1)
            stw r4,  8(r1)
            stw r5,  12(r1)
            stw r6,  16(r1)
            stw r7,  20(r1)
            stw r8,  24(r1)
            stw r9,  28(r1)
            stw r10, 32(r1)
            stw r11, 36(r1)
            stw r12, 40(r1)
            stw r13, 44(r1)
            stw r14, 48(r1)
            stw r15, 52(r1)
            stw r16, 56(r1)
            stw r17, 60(r1)
            stw r18, 64(r1)
            stw r19, 68(r1)
            stw r20, 72(r1)
            stw r21, 76(r1)
            stw r22, 80(r1)
            stw r23, 84(r1)
            stw r24, 88(r1)
            stw r25, 92(r1)
            stw r26, 96(r1)
            stw r27, 100(r1)
            stw r28, 104(r1)
            stw r29, 108(r1)
            stw r30, 112(r1)
            stw r31, 116(r1)
        ");


        // Sonraki görevin context'ine geç (Switch to the next task's context)

        // Sonraki görevin stack pointer'ını yükle (Load next task's stack pointer)
        asm!("mr r1, {}", in(reg) next_task.context.sp); // PowerPC'de r1 genellikle SP'dir (r1 is often SP in PowerPC)

        // Makine Durum Yazmacını (MSR) yükle (Load Machine State Register (MSR))
        asm!("mtmsr {},", in(reg) next_task.context.msr);

        // Durum Yazmacını (CR) yükle (Load Condition Register (CR))
        asm!("mtcr {},", in(reg) next_task.context.cr);

        // Bağlantı Yazmacını (LR) yükle (Load Link Register (LR))
        asm!("mtlr {},", in(reg) next_task.context.lr);

        // Sayım Yazmacını (CTR) yükle (Load Count Register (CTR))
        asm!("mtctr {},", in(reg) next_task.context.ctr);


        // Genel amaçlı yazmaçları (r2-r31) yığından yükle (Load general purpose registers (r2-r31) from stack)
        asm!("
            lwz r2,  0(r1)
            lwz r3,  4(r1)
            lwz r4,  8(r1)
            lwz r5,  12(r1)
            lwz r6,  16(r1)
            lwz r7,  20(r1)
            lwz r8,  24(r1)
            lwz r9,  28(r1)
            lwz r10, 32(r1)
            lwz r11, 36(r1)
            lwz r12, 40(r1)
            lwz r13, 44(r1)
            lwz r14, 48(r1)
            lwz r15, 52(r1)
            lwz r16, 56(r1)
            lwz r17, 60(r1)
            lwz r18, 64(r1)
            lwz r19, 68(r1)
            lwz r20, 72(r1)
            lwz r21, 76(r1)
            lwz r22, 80(r1)
            lwz r23, 84(r1)
            lwz r24, 88(r1)
            lwz r25, 92(r1)
            lwz r26, 96(r1)
            lwz r27, 100(r1)
            lwz r28, 104(r1)
            lwz r29, 108(r1)
            lwz r30, 112(r1)
            lwz r31, 116(r1)
        ");

        // CURRENT_TASK'ı sonraki göreve güncelle (Update CURRENT_TASK to the next task)
        CURRENT_TASK = NonNull::new(next_task as *mut Task);

        // Sonraki görevin başlangıç noktasına dallan (Branch to the next task's entry point)
        asm!("blr"); // Link Register'dan dön (Return from Link Register) - LR'nin doğru ayarlandığını varsayar (assumes LR is set correctly). Bu örnekte, LR'yi doğrudan ayarlamadık.
                       // Gerçek bir uygulamada, görev ilk kez çalıştırıldığında veya bir görevden dönüldüğünde LR'nin ayarlanması gerekebilir.
                       // (In a real application, LR may need to be set when a task is first started or when returning from a task.)
                       // Şimdilik basitlik için 'blr' kullanıyoruz, fakat daha detaylı context yükleme senaryolarında PC'yi doğrudan ayarlamak gerekebilir.
                       // (For simplicity, we are using 'blr' for now, but in more detailed context loading scenarios, setting PC directly might be necessary.)

        // ÖNEMLİ NOT: Bu örnekte 'blr' komutu kullanılarak bir fonksiyon geri dönüşü simüle edilmektedir.
        // Gerçek bir görev geçişinde, program sayacını (PC) ve bağlantı yazmacını (LR) daha doğru bir şekilde yönetmek gerekebilir.
        // Örneğin, görev başlangıç noktasını 'context.pc'ye kaydedip, bir sonraki göreve geçişte bu adresi PC'ye yüklemek daha tipik bir yöntem olabilir.
        // Ancak, bu basit örnekte, 'blr' komutu ile temel görev geçişi işlevselliğini göstermeyi amaçlıyoruz.
        // (IMPORTANT NOTE: In this example, a function return is simulated using the 'blr' instruction.
        // In a real task switch, managing the program counter (PC) and link register (LR) more accurately may be necessary.
        // For example, saving the task's entry point to 'context.pc' and loading this address to PC when switching to the next task might be a more typical approach.
        // However, in this simple example, we aim to demonstrate basic task switching functionality with the 'blr' instruction.)
    }
}


// Örnek görev başlangıç noktası (sadece gösteri amaçlı) (Example task entry point (just for demonstration))
fn task1_entry() {
    let mut count = 0;
    loop {
        count += 1;
        // Gerçek bir no_std ortamında, gecikmeler için bir donanım zamanlayıcısı veya benzeri kullanabilirsiniz. (In a real no_std environment, you might use a hardware timer or similar for delays.)
        // Bu örnek için, basit bir döngü yer tutucu olarak hizmet vermektedir. (For this example, a simple loop serves as a placeholder.)
        for _ in 0..100000 {
            // zaman harca (waste time)
        }
        unsafe {
            // Görev geçişini belirtmek için temel çıktı (seri port gibi bir çıktı biçiminin başka bir yerde başlatıldığını varsayarak)
            // (Basic output to indicate task switching (assuming some form of output like serial is initialized elsewhere))
            let ptr = 0x80000000 as *mut u32; // Örnek çıktı bellek adresi (Example memory address for output)
            ptr.write_volatile(count);
        }
    }
}

// Örnek kullanım (gösteri için kavramsal main fonksiyonu) (Example usage (conceptual main function for demonstration))
fn main() {
    init(); // Görev yönetimini başlat (Initialize task management)

    create_task(task1_entry); // Görev 1'i oluştur ve başlangıç noktasını ayarla (Create task 1 and set its entry point)

    // İlk görevi çalıştırmaya başla (görev 0, init içinde başlatıldı). (Start running the first task (task 0 is initialized in init).)
    // Gerçek bir işletim sisteminde, görev 0 ilk kurulumu yapabilir ve sonra görev geçişini başlatabilir. (In a real OS, task 0 might do initial setup and then start task switching.)

    loop {
        unsafe {
            // Gösteri için, görevleri bir döngü içinde değiştir. (For demonstration, switch tasks in a loop.)
            // Gerçek bir sistemde, görev geçişi olaylar tarafından tetiklenir (örn. zamanlayıcı kesmesi). (In a real system, task switching would be triggered by events (e.g., timer interrupt).)
            switch_task();
            for _ in 0..200000 { // görev 0'da zaman harca (waste time in task 0)
                // zaman harca (waste time)
            }
            let ptr = 0x80000004 as *mut u32; // Görev 0 için örnek çıktı bellek adresi (Example memory address for output for task 0)
            ptr.write_volatile(0); // Görev 0'ın çalıştığını belirt (Indicate task 0 is running)
        }
    }
}

// no_std ortamı için gerekli, panik işleyicisini tanımla (Required for no_std environment, define panic handler)
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}