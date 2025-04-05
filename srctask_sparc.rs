#![no_std]

 use core::ptr::NonNull;
 use core::panic::PanicInfo;

 // SPARC için basit görev yapısı
 pub struct Task {
     stack: [u8; 1024], // Görev başına 1KB yığın
     context: TaskContext,
 }

 #[derive(Default, Copy, Clone)]
 #[repr(C)]
 struct TaskContext {
     // SPARC mimarisi için bağlam.
     // Basitlik adına, burada sadece temel kayıtları saklıyoruz.
     // Gerçek bir uygulamada, daha fazla kayıt ve durum bilgisi gerekebilir.
     g_regs: [usize; 8], // Genel amaçlı global kayıtlar (%g1-%g7 - %g0 her zaman sıfırdır)
     o_regs: [usize; 8], // Genel amaçlı çıktı kayıtları (%o0-%o7)
     l_regs: [usize; 8], // Genel amaçlı yerel kayıtlar (%l0-%l7)
     i_regs: [usize; 8], // Genel amaçlı girdi kayıtları (%i0-%i7)
     sp: usize,        // Yığın işaretçisi (%sp veya %r14)
     pc: usize,        // Program sayacı (%pc) - genellikle yığında saklanır/geri yüklenir.
     psr: usize,       // İşlemci durum kaydı (%psr) - önemli durum bitleri
     wim: usize,       // Pencere giriş maskesi (%wim) - pencere kayıtları yönetimi için kritik
     // cwp: usize,       // Geçerli pencere göstergesi (%cwp) - pencere yönetimi için (daha gelişmiş senaryolar için)
 }

 static mut CURRENT_TASK: Option<NonNull<Task>> = None;
 static mut TASKS: [Option<Task>; 2] = [None, None]; // Görevleri tutmak için statik dizi (bu örnek için sabit boyut 2)

 pub fn init() {
     unsafe {
         // İlk görevi (görev 0) başlat
         TASKS[0] = Some(Task { stack: [0; 1024], context: TaskContext::default() });
         // Mevcut görevi ilk görev olarak ayarla
         CURRENT_TASK = NonNull::new(TASKS[0].as_mut().expect("TASKS[0] için mutable referans alınamadı"));
     }
 }

 pub fn create_task(entry_point: fn()) {
     static mut TASK_ID: usize = 1; // Statik görev ID, görev 0 zaten başlatıldığı için 1'den başlar
     unsafe {
         // TASK_ID'ye göre görev slotuna mutable referans al
         let task_slot = TASKS.get_mut(TASK_ID).expect("TASK_ID, TASKS dizisi sınırlarının dışında");
         // Görev slotunda yeni bir görev oluştur
         *task_slot = Some(Task {
             stack: [0; 1024],
             context: TaskContext {
                 pc: entry_point as usize, // Görevin giriş noktasını ayarla
                 ..Default::default() // Diğer bağlam alanlarını varsayılandan devral
             }
         });
         TASK_ID += 1; // Bir sonraki görev oluşturma için görev ID'sini artır
     }
 }

 pub fn switch_task() {
     unsafe {
         // Mevcut ve sonraki görevlere mutable referanslar al (görev 1, bu örnekte basitlik için sonraki görev olarak varsayılır)
         let current_task = CURRENT_TASK.expect("CURRENT_TASK None değerinde").as_mut();
         let next_task = TASKS.get_mut(1).expect("TASKS[1] None değerinde").as_mut().expect("TASKS[1] için mutable referans alınamadı");

         // Mevcut görevin bağlamını kaydet
         // Yığın işaretçisini kaydet
         asm!("mov %sp, {}", out(reg) current_task.context.sp);
         // Program sayacını kaydet (basit örnek için, bu genellikle yığın aracılığıyla yönetilir)
         // PSR'yi kaydet (İşlemci Durum Kaydı)
         asm!("rdpsr {}", out(reg) current_task.context.psr);
         // WIM'i kaydet (Pencere Giriş Maskesi) - Pencere kayıtları kullanılıyorsa kritik
         asm!("rdwim {}", out(reg) current_task.context.wim);

         // *** ÖNEMLİ NOT ***
         // SPARC'ta pencere kayıtları yönetimi çok önemlidir. Basit bir geçiş için,
         // pencere taşmasını (window overflow) veya yetersiz akışı (window underflow) tetiklememeye
         // dikkat etmek gerekebilir. Daha sağlam bir uygulama, pencere taşması ve yetersiz akış
         // işleyicilerini (window overflow/underflow traps) ve register window set'i (RWS)
         // doğru şekilde yönetmeyi içermelidir. Aşağıdaki örnek çok basittir ve
         // pencere kayıtlarını tam olarak kaydetmez/geri yüklemez.
         // Tam bir bağlam geçişi için, tüm register window set'inin kaydedilmesi ve
         // geri yüklenmesi gerekebilir.

         // Sonraki görevin bağlamına geç
         // Sonraki görevin yığın işaretçisini yükle
         asm!("mov {}, %sp", in(reg) next_task.context.sp);
         // Program sayacını yükle (yine, basit örnekte yığın üzerinden yönetim varsayılır)
         // Sonraki görevin PSR'sini yükle
         asm!("wrpsr {}", in(reg) next_task.context.psr);
         // Sonraki görevin WIM'ini yükle
         asm!("wrwim {}", in(reg) next_task.context.wim);


         // CURRENT_TASK'ı sonraki görev olarak güncelle
         CURRENT_TASK = NonNull::new(next_task as *mut Task);
     }
 }

 // Örnek görev giriş noktası (sadece gösteri amaçlı)
 fn task1_entry() {
     let mut count = 0;
     loop {
         count += 1;
         // Gerçek bir no_std ortamında, gecikmeler için donanım zamanlayıcısı veya benzeri kullanılabilir.
         // Bu örnek için basit bir döngü yer tutucu olarak hizmet eder.
         for _ in 0..100000 {
             // zaman harca
         }
         unsafe {
             // Görev geçişini belirtmek için temel çıktı (seri port gibi bir çıktı biçiminin başka yerde başlatıldığını varsayarak)
             let ptr = 0x80000000 as *mut u32; // Örnek bellek adresi çıktı için
             ptr.write_volatile(count);
         }
     }
 }

 // Örnek kullanım (gösteri için kavramsal main fonksiyonu)
 fn main() {
     init(); // Görev yönetimini başlat

     create_task(task1_entry); // Görev 1'i oluştur ve giriş noktasını ayarla

     // İlk görevi çalıştırmaya başla (görev 0 init fonksiyonunda başlatılır).
     // Gerçek bir işletim sisteminde, görev 0 başlangıç kurulumunu yapabilir ve ardından görev geçişini başlatabilir.

     loop {
         unsafe {
             // Gösteri için, görevleri bir döngü içinde değiştir.
             // Gerçek bir sistemde, görev geçişi olaylar tarafından tetiklenir (örneğin, zamanlayıcı kesmesi).
             switch_task();
             for _ in 0..200000 { // görev 0'da zaman harca
                 // zaman harca
             }
             let ptr = 0x80000004 as *mut u32; // Görev 0 için örnek bellek adresi çıktı
             ptr.write_volatile(0); // Görev 0'ın çalıştığını belirt
         }
     }
 }

 // no_std ortamı için gerekli, panik işleyicisini tanımla
 #[panic_handler]
 fn panic(_info: &PanicInfo) -> ! {
     loop {}
 }