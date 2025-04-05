#![no_std]
use core::ptr::NonNull;
use core::panic::PanicInfo;

// Elbrus mimarisi için basit bir görev yapısı (örnek amaçlı)
pub struct Task {
    stack: [u8; 1024], // Her görev için 1KB yığın
    context: TaskContext,
}

#[derive(Default, Copy, Clone)]
#[repr(C)]
struct TaskContext {
    // Elbrus mimarisi genel amaçlı yazmaçları (örnek olarak x0-x31 kullanılıyor,
    // gerçek Elbrus yazmaç isimleri farklı olabilir)
    x: [usize; 32],
    // Elbrus mimarisi durum yazmaçları (örnek olarak PSR - Program Status Register kullanılıyor,
    // gerçek isim ve yapı farklı olabilir)
    psr: usize,
    // Elbrus mimarisi yönerge işaretçisi (örnek olarak IP - Instruction Pointer kullanılıyor,
    // gerçek isim farklı olabilir)
    ip: usize,
}

static mut CURRENT_TASK: Option<NonNull<Task>> = None;
static mut TASKS: [Option<Task>; 2] = [None, None]; // Görevleri tutmak için statik dizi (örnek için sabit boyut 2)

pub fn init() {
    unsafe {
        // İlk görevi başlat (görev 0)
        TASKS[0] = Some(Task { stack: [0; 1024], context: TaskContext::default() });
        // Mevcut görevi ilk göreve ayarla
        CURRENT_TASK = NonNull::new(TASKS[0].as_mut().expect("TASKS[0] için mutable referans alınamadı"));
    }
}

pub fn create_task(entry_point: fn()) {
    static mut TASK_ID: usize = 1; // Statik görev ID, görev 0 zaten başlatıldığı için 1'den başlar
    unsafe {
        // Görev ID'sine göre görev yuvasına mutable referans al
        let task_slot = TASKS.get_mut(TASK_ID).expect("TASK_ID, TASKS dizisi sınırlarının dışında");
        // Görev yuvasında yeni bir görev oluştur
        *task_slot = Some(Task {
            stack: [0; 1024],
            context: TaskContext {
                ip: entry_point as usize, // Görevin giriş noktasını ayarla (Instruction Pointer)
                ..Default::default() // Diğer bağlam alanlarını varsayılandan al
            }
        });
        TASK_ID += 1; // Bir sonraki görev oluşturma için görev ID'sini artır
    }
}

pub fn switch_task() {
    unsafe {
        // Mevcut ve bir sonraki görevlere mutable referanslar al (örnekte bir sonraki görev görev 1 olarak varsayılıyor)
        let current_task = CURRENT_TASK.expect("CURRENT_TASK None değerinde").as_mut();
        let next_task = TASKS.get_mut(1).expect("TASKS[1] None değerinde").as_mut().expect("TASKS[1] için mutable referans alınamadı");

        // Mevcut görevin bağlamını kaydet
        // PSR (Program Status Register) yazmacını kaydet
        // **ÖNEMLİ**: Elbrus için gerçek PSR yazmacı ve erişim yönergeleri farklı olabilir.
        asm!("/* Elbrus PSR kaydetme yönergesi (ÖRNEK, GERÇEK DEĞİL) */ MOV {}, PSR", out(reg) current_task.context.psr);

        // IP (Instruction Pointer) yazmacını kaydet
        // **ÖNEMLİ**: Elbrus için gerçek IP yazmacı ve erişim yönergeleri farklı olabilir.
        asm!("/* Elbrus IP kaydetme yönergesi (ÖRNEK, GERÇEK DEĞİL) */ MOV {}, IP", out(reg) current_task.context.ip);

        // Yığın işaretçisini (SP - Stack Pointer) kaydet
        // **ÖNEMLİ**: Elbrus'ta yığın işaretçisi ve ilgili yazmaç ismi farklı olabilir.
        asm!("/* Elbrus SP kaydetme yönergesi (ÖRNEK, GERÇEK DEĞİL) */ MOV {}, SP", out(reg) current_task.context.x[2]); // x[2] örnek olarak kullanılıyor


        // Bir sonraki görevin bağlamına geç
        // Bir sonraki görevin PSR yazmacını yükle
        // **ÖNEMLİ**: Elbrus için gerçek PSR yazmacı ve yükleme yönergeleri farklı olabilir.
        asm!("/* Elbrus PSR yükleme yönergesi (ÖRNEK, GERÇEK DEĞİL) */ MOV PSR, {}", in(reg) next_task.context.psr);

        // Bir sonraki görevin IP yazmacını yükle
        // **ÖNEMLİ**: Elbrus için gerçek IP yazmacı ve yükleme yönergeleri farklı olabilir.
        asm!("/* Elbrus IP yükleme yönergesi (ÖRNEK, GERÇEK DEĞİL) */ MOV IP, {}", in(reg) next_task.context.ip);

        // Bir sonraki görevin yığın işaretçisini yükle
        // **ÖNEMLİ**: Elbrus'ta yığın işaretçisi ve ilgili yükleme yönergesi farklı olabilir.
        asm!("/* Elbrus SP yükleme yönergesi (ÖRNEK, GERÇEK DEĞİL) */ MOV SP, {}", in(reg) next_task.context.x[2]); // x[2] örnek olarak kullanılıyor

        // CURRENT_TASK'ı bir sonraki göreve güncelle
        CURRENT_TASK = NonNull::new(next_task as *mut Task);
    }
}

// Örnek görev giriş noktası (sadece gösteri amaçlı)
fn task1_entry() {
    let mut count = 0;
    loop {
        count += 1;
        // Gerçek bir no_std ortamında, gecikmeler için donanım zamanlayıcısı veya benzeri kullanılabilir.
        // Bu örnek için basit bir döngü yer tutucu olarak hizmet vermektedir.
        for _ in 0..100000 {
            // zaman harca
        }
        unsafe {
            // Görev geçişini belirtmek için temel çıktı (seri port gibi bir çıktı biçiminin başka bir yerde başlatıldığı varsayılıyor)
            let ptr = 0x80000000 as *mut u32; // Çıktı için örnek bellek adresi
            ptr.write_volatile(count);
        }
    }
}

// Örnek kullanım (kavramsal ana fonksiyon gösteri için)
fn main() {
    init(); // Görev yönetimini başlat

    create_task(task1_entry); // Görev 1'i oluştur ve giriş noktasını ayarla

    // İlk görevi çalıştırmaya başla (görev 0 init içinde başlatılır).
    // Gerçek bir işletim sisteminde, görev 0 başlangıç kurulumunu yapabilir ve ardından görev geçişini başlatabilir.

    loop {
        unsafe {
            // Gösteri için, döngü içinde görevleri değiştir.
            // Gerçek bir sistemde, görev geçişi olaylarla (örneğin, zamanlayıcı kesmesi) tetiklenir.
            switch_task();
            for _ in 0..200000 { // görev 0'da zaman harca
                // zaman harca
            }
            let ptr = 0x80000004 as *mut u32; // Görev 0 için çıktı için örnek bellek adresi
            ptr.write_volatile(0); // Görev 0'ın çalıştığını belirt
        }
    }
}

// no_std ortamı için gerekli, panik işleyicisini tanımla
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}