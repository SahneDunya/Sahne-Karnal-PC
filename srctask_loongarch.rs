#![no_std]
use core::ptr::NonNull;
use core::panic::PanicInfo;

// LoongArch için basit görev yapısı
pub struct Task {
    stack: [u8; 1024], // Her görev için 1KB yığın
    context: TaskContext,
}

#[derive(Default, Copy, Clone)]
#[repr(C)]
struct TaskContext{
    r:[usize; 32], // Genel amaçlı yazmaçlar (LoongArch r0-r31)
    csr_statu: usize, // Yönetici durum yazmacı (Tahmini isim, LoongArch dokümantasyonuna göre doğrulanmalı)
    csr_epc: usize,  // Yönetici istisna program sayacı (Tahmini isim, LoongArch dokümantasyonuna göre doğrulanmalı)
}

static mut CURRENT_TASK: Option<NonNull<Task>> = None;
static mut TASKS: [Option<Task>; 2] = [None, None]; // Görevleri tutmak için statik dizi, bu örnek için sabit boyut 2

pub fn init(){
    unsafe{
        // İlk görevi (görev 0) başlat
        TASKS[0] = Some(Task{stack: [0; 1024], context: TaskContext::default()});
        // Mevcut görevi ilk göreve ayarla
        CURRENT_TASK = NonNull::new(TASKS[0].as_mut().expect("TASKS[0] için mutable referans alınamadı"));
    }
}

pub fn create_task(entry_point: fn()){
    static mut TASK_ID: usize = 1; // Statik görev ID, görev 0 zaten başlatıldığı için 1'den başlar
    unsafe{
        // TASK_ID'ye göre görev yuvasına mutable referans al
        let task_slot = TASKS.get_mut(TASK_ID).expect("TASK_ID, TASKS dizisi sınırlarının dışında");
        // Görev yuvasında yeni görev oluştur
        *task_slot = Some(Task{
            stack: [0; 1024],
            context: TaskContext{
                csr_epc: entry_point as usize, // Görevin giriş noktasını ayarla
                ..Default::default() // Diğer context alanlarını varsayılandan devral
            }
        });
        TASK_ID += 1; // Sonraki görev oluşturma için görev ID'sini artır
    }
}

pub fn switch_task(){
    unsafe{
        // Mevcut ve sonraki görevlere mutable referans al (görev 1, bu örnekte basitlik için sonraki görev olarak varsayılır)
        let current_task = CURRENT_TASK.expect("CURRENT_TASK None").as_mut();
        let next_task = TASKS.get_mut(1).expect("TASKS[1] None").as_mut().expect("TASKS[1] için mutable referans alınamadı");

        // Mevcut görevin context'ini kaydet
        // csr_statu yazmacını kaydet
        asm!("csrrd {}, csr_statu", out(reg) current_task.context.csr_statu); // LoongArch için csrrd (CSR Read) talimatı kullanılıyor (doğrulanmalı)
        // csr_epc yazmacını kaydet
        asm!("csrrd {}, csr_epc", out(reg) current_task.context.csr_epc); // LoongArch için csrrd (CSR Read) talimatı kullanılıyor (doğrulanmalı)
        // Yığın işaretçisini kaydet (LoongArch'ta yığın işaretçisi için 'sp' veya 'r2' kullanılır, burada 'r2' olarak varsayıyoruz)
        asm!("move {}, $r2", out(reg) current_task.context.r[2]); // LoongArch'ta yığın işaretçisi genellikle r2/sp'dir. 'move' talimatı register kopyalamak için kullanılır.

        // Sonraki görevin context'ine geç
        // Sonraki görevin csr_statu yazmacını yükle
        asm!("csrwr csr_statu, {}", in(reg) next_task.context.csr_statu); // LoongArch için csrwr (CSR Write) talimatı kullanılıyor (doğrulanmalı)
        // Sonraki görevin csr_epc yazmacını yükle
        asm!("csrwr csr_epc, {}", in(reg) next_task.context.csr_epc); // LoongArch için csrwr (CSR Write) talimatı kullanılıyor (doğrulanmalı)
        // Sonraki görevin yığın işaretçisini yükle
        asm!("move $r2, {}", in(reg) next_task.context.r[2]); // LoongArch'ta yığın işaretçisi genellikle r2/sp'dir. 'move' talimatı register kopyalamak için kullanılır.


        // CURRENT_TASK'ı sonraki göreve güncelle
        CURRENT_TASK = NonNull::new(next_task as *mut Task);
    }
}

// Örnek görev giriş noktası (sadece gösteri amaçlı)
fn task1_entry() {
    let mut count = 0;
    loop {
        count += 1;
        // Gerçek bir no_std ortamında, gecikmeler için bir donanım zamanlayıcısı veya benzeri kullanabilirsiniz.
        // Bu örnekte, basit bir döngü yer tutucu olarak hizmet eder.
        for _ in 0..100000 {
            // zaman harca
        }
        unsafe {
            // Görev değiştirmeyi belirtmek için temel çıktı (seri port gibi bir çıktı biçiminin başka bir yerde başlatıldığı varsayılır)
            let ptr = 0x90000000 as *mut u32; // LoongArch için örnek bellek adresi (doğrulanmalı)
            ptr.write_volatile(count);
        }
    }
}

// Örnek kullanım (gösteri için kavramsal main fonksiyonu)
fn main() {
    init(); // Görev yönetimini başlat

    create_task(task1_entry); // Görev 1'i oluştur ve giriş noktasını ayarla

    // İlk görevi çalıştırmaya başla (görev 0 init'te başlatılır).
    // Gerçek bir işletim sisteminde, görev 0 başlangıç kurulumunu yapabilir ve ardından görev değiştirmeyi başlatabilir.

    loop {
        unsafe {
            // Gösteri için, görevleri bir döngü içinde değiştir.
            // Gerçek bir sistemde, görev değiştirme olaylarla tetiklenir (örneğin, zamanlayıcı kesmesi).
            switch_task();
            for _ in 0..200000 { // görev 0'da zaman harca
                // zaman harca
            }
            let ptr = 0x90000004 as *mut u32; // LoongArch için örnek bellek adresi (doğrulanmalı)
            ptr.write_volatile(0); // Görev 0'ın çalıştığını belirt
        }
    }
}

// no_std ortamı için gerekli, panik işleyicisini tanımla
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}