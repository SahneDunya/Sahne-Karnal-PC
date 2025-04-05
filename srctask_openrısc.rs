#![no_std]
use core::ptr::NonNull;
use core::panic::PanicInfo;

// Görev durumlarını temsil etmek için Enum
#[derive(PartialEq, Copy, Clone)]
pub enum TaskState {
    Ready,    // Görev çalışmaya hazır
    Running,  // Görev şu anda çalışıyor
}

// Görev yapısı, görev durumu eklendi
pub struct Task {
    stack: [u8; 1024], // Her görev için 1KB yığın
    context: TaskContext,
    state: TaskState,   // Görev durumu
}

#[derive(Default, Copy, Clone)]
#[repr(C)]
struct TaskContext{
    x:[usize; 32], // Genel amaçlı yazmaçlar
    sstatus: usize, // Süpervizör durum yazmacı
    sepc: usize,  // Süpervizör istisna program sayacı
}

static mut CURRENT_TASK: Option<NonNull<Task>> = None;
static mut TASKS: [Option<Task>; 2] = [None, None]; // Görevleri tutmak için statik dizi, bu örnek için sabit boyut 2
static mut TASK_COUNT: usize = 0; // Toplam görev sayısını takip etmek için
static mut CURRENT_TASK_ID: usize = 0; // Mevcut görevin ID'sini takip etmek için

pub fn init(){
    unsafe{
        // İlk görevi (görev 0) başlat
        TASKS[0] = Some(Task{
            stack: [0; 1024],
            context: TaskContext::default(),
            state: TaskState::Running, // İlk görev başlangıçta çalışıyor
        });
        TASK_COUNT += 1;
        // Mevcut görevi ilk göreve ayarla
        CURRENT_TASK = NonNull::new(TASKS[0].as_mut().expect("TASKS[0] için mutable referans alınamadı"));
        CURRENT_TASK_ID = 0;
    }
}

pub fn create_task(entry_point: fn()){
    unsafe{
        if TASK_COUNT >= TASKS.len() {
            // Maksimum görev sayısına ulaşıldı, daha fazla görev oluşturulamaz
            return; // Veya uygun bir hata işleme mekanizması
        }
        let task_id = TASK_COUNT; // Yeni görev için görev ID'si olarak TASK_COUNT'u kullan
        // Görev yuvasına mutable referans al
        let task_slot = TASKS.get_mut(task_id).expect("TASK_ID, TASKS dizisi sınırlarının dışında");
        // Görev yuvasında yeni görev oluştur
        *task_slot = Some(Task{
            stack: [0; 1024],
            context: TaskContext{
                sepc: entry_point as usize, // Görevin giriş noktasını ayarla
                ..Default::default() // Diğer bağlam alanlarını varsayılandan devral
            },
            state: TaskState::Ready, // Yeni görev başlangıçta hazır durumda
        });
        TASK_COUNT += 1; // Bir sonraki görev oluşturma için görev ID'sini artır
    }
}

pub fn switch_task(){
    unsafe{
        let current_task_id = CURRENT_TASK_ID;
        let next_task_id = (CURRENT_TASK_ID + 1) % TASK_COUNT; // Basit round-robin, bir sonraki göreve geç

        // Mevcut ve bir sonraki görevlere mutable referanslar al
        let current_task = CURRENT_TASK.expect("CURRENT_TASK is None").as_mut();
        let next_task = TASKS.get_mut(next_task_id)
            .expect("TASKS[next_task_id] is None")
            .as_mut()
            .expect("TASKS[next_task_id] için mutable referans alınamadı");

        // Eğer mevcut görev hala çalışıyorsa durumunu Hazır'a ayarla (round-robin için)
        if current_task.state == TaskState::Running {
            current_task.state = TaskState::Ready;
        }
        // Bir sonraki görevin durumunu Çalışıyor'a ayarla
        next_task.state = TaskState::Running;

        // Mevcut görevin bağlamını kaydet
        // sstatus yazmacını kaydet
        asm!("csrr {}, sstatus", out(reg) current_task.context.sstatus);
        // sepc yazmacını kaydet
        asm!("csrr {}, sepc", out(reg) current_task.context.sepc);
        // Yığın işaretçisini kaydet (x[2], RISC-V kuralında tipik olarak yığın işaretçisi olarak kullanılır)
        asm!("mv {}, sp", out(reg) current_task.context.x[2]);

        // Bir sonraki görevin bağlamına geç
        // Bir sonraki görevin sstatus yazmacını yükle
        asm!("csrw sstatus, {}", in(reg) next_task.context.sstatus);
        // Bir sonraki görevin sepc yazmacını yükle
        asm!("csrw sepc, {}", in(reg) next_task.context.sepc);
        // Bir sonraki görevin yığın işaretçisini yükle
        asm!("mv sp, {}", in(reg) next_task.context.x[2]);

        // CURRENT_TASK'ı bir sonraki göreve güncelle
        CURRENT_TASK = NonNull::new(next_task as *mut Task);
        CURRENT_TASK_ID = next_task_id; // Mevcut görev ID'sini güncelle
    }
}

// Örnek görev giriş noktası (sadece gösteri amaçlı)
fn task1_entry() {
    let mut count = 0;
    loop {
        count += 1;
        // Gerçek bir no_std ortamında, gecikmeler için bir donanım zamanlayıcısı veya benzeri kullanabilirsiniz.
        // Bu örnek için basit bir döngü yer tutucu olarak hizmet vermektedir.
        for _ in 0..100000 {
            // zaman harca
        }
        unsafe {
            // Görev geçişini belirtmek için temel çıktı (seri port gibi bir çıktı biçiminin başka bir yerde başlatıldığını varsayarak)
            let ptr = 0x80000000 as *mut u32; // Çıktı için örnek bellek adresi
            ptr.write_volatile(count);
        }
    }
}

// Örnek kullanım (gösteri için kavramsal main fonksiyonu)
fn main() {
    init(); // Görev yönetimini başlat

    create_task(task1_entry); // Görev 1'i oluştur ve giriş noktasını ayarla

    // İlk görevi çalıştırmaya başla (görev 0 init içinde başlatılır).
    // Gerçek bir işletim sisteminde, görev 0 başlangıç kurulumunu yapabilir ve ardından görev geçişini başlatabilir.

    loop {
        unsafe {
            // Gösteri için, bir döngüde görevleri değiştir.
            // Gerçek bir sistemde, görev geçişi olaylar tarafından tetiklenir (örneğin, zamanlayıcı kesmesi).
            switch_task();
            for _ in 0..200000 { // Görev 0'da zaman harca
                // zaman harca
            }
            let ptr = 0x80000004 as *mut u32; // Görev 0 için örnek bellek adresi çıktı için
            ptr.write_volatile(0); // Görev 0'ın çalıştığını belirt
        }
    }
}

// no_std ortamı için gerekli, panik işleyicisini tanımla
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}