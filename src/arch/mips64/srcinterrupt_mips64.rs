#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz

// Karnal64 çekirdek API'sından ihtiyacımız olan fonksiyonları ve tipleri içeri alalım.
// Özellikle sistem çağrısı işleyicisi handle_syscall bizim için önemli.
// Varsayım: 'karnal64' crate'i projenize eklenmiş ve public üyeleri mevcut.
use karnal64::{handle_syscall, KError, KHandle, KTaskId}; // Örnek için bazı tipleri de alalım

// MIPS mimarisine özgü CP0 registerlarına erişim için helper fonksiyonlar veya bir crate gerekebilir.
// Bu örnekte, sadece TrapFrame yapısı üzerinden registerlara erişimi simüle ediyoruz.
// Gerçekte, unsafe { read_cp0_register(...) } gibi çağrılar yapmanız gerekir.

// MIPS64 mimarisine özgü bir TrapFrame yapısı.
// Bir exception (istisna) veya interrupt (kesme) meydana geldiğinde, CPU'nun
// tüm kritik registerlarının durumunu kaydetmek için kullanılır.
// #[repr(C)] buranın C uyumlu olmasını ve düşük seviyeli montaj kodu tarafından
// doğru şekilde erişilebilir olmasını sağlar.
#[repr(C)]
#[derive(Debug, Clone, Copy)] // Debug ve Copy/Clone traitleri hata ayıklama ve kolay kullanım için faydalı
pub struct TrapFrame {
    // Genel Amaçlı Registerlar (GPRs) - MIPS64'te 32 adet 64-bit register (r0-r31).
    // r0 her zaman 0'dır, ancak tutarlılık için dizide yer alabilir.
    // Syscall argümanları: a0-a4 (regs[4] - regs[8])
    // Syscall dönüş değeri: v0 (regs[2])
    // Return address: ra (regs[31])
    // Stack pointer: sp (regs[29])
    // Diğer registerlar da kaydedilmelidir (s0-s7, t0-t9, gp, fp, vb.)
    // Tam bir TrapFrame tüm GPR'leri içermelidir:
    pub regs: [u64; 32], // r0-r31

    // MIPS CP0 Registerları
    pub status: u64, // Durum Registerı (SR)
    pub cause: u64,  // Neden Registerı
    pub epc: u64,    // Exception Program Counter (Exception'a neden olan instruction'ın adresi)
    pub bad_vaddr: u64, // Geçersiz sanal adres (TLB, Adres Hatası gibi exception'larda geçerli)
    // Diğer CP0 registerları (Config, Context, Wired vb.) duruma göre eklenebilir.
}

// MIPS Cause Register'ındaki ExcCode (Exception Code) bit alanının maskesi ve kaydırma miktarı
const CAUSE_EXCCODE_SHIFT: usize = 2;
const CAUSE_EXCCODE_MASK: u64 = 0x1F; // 5 bitlik alan (0-31)

// MIPS Exception Kodları (ExcCode) - Cause registerından okunur
// Bu değerler MIPS mimari kılavuzuna göre kontrol edilmelidir.
const EXCCODE_INT: u64 = 0b00000; // Interrupt (Kesme)
const EXCCODE_MOD: u64 = 0b00001; // TLB Modified (TLB Değiştirildi)
const EXCCODE_TLBL: u64 = 0b00010; // TLB Load/Instruction Fetch (TLB Yükleme/Komut Çekme)
const EXCCODE_TLBS: u64 = 0b00011; // TLB Store (TLB Saklama)
const EXCCODE_ADDR: u64 = 0b00100; // Address Error (Adres Hatası)
const EXCCODE_BUSI: u64 = 0b00101; // Bus Error (Instruction Fetch) (Veriyolu Hatası - Komut Çekme)
const EXCCODE_BUSD: u64 = 0b00110; // Bus Error (Data Load/Store) (Veriyolu Hatası - Veri Yükleme/Saklama)
const EXCCODE_SYSCALL: u64 = 0b01000; // Syscall (Sistem Çağrısı)
const EXCCODE_BP: u64 = 0b01001;    // Breakpoint
const EXCCODE_RI: u64 = 0b01010;    // Reserved Instruction (Ayrılmış Komut)
const EXCCODE_CPI: u64 = 0b01011;    // Coprocessor Unusable (Yardımcı İşlemci Kullanılamaz)
const EXCCODE_OV: u64 = 0b01100;     // Overflow (Taşma)
const EXCCODE_TR: u64 = 0b01101;     // Trap (Tuzak)
// ... Diğer kodlar ...

// MIPS Interrupt Pending (IP) bit alanının maskesi ve kaydırma miktarı (Cause registerı)
const CAUSE_IP_SHIFT: usize = 10;
const CAUSE_IP_MASK: u64 = 0xFF; // 8 bitlik alan (IP0-IP7)

// MIPS SR (Status Register) ERL/EXL/IE bitleri
// Exception Return Level (ERL), Exception Level (EXL), Interrupt Enable (IE)
const SR_IE: u64 = 0x1; // Interrupt Enable bit
const SR_EXL: u64 = 0x2; // Exception Level bit
const SR_ERL: u64 = 0x4; // Error Level bit

// --- Ana Exception/Interrupt İşleyici Fonksiyonu ---
// Bu fonksiyon, düşük seviyeli montaj dili kesme vektörü işleyicisi tarafından çağrılır.
// Montaj işleyicisi, exception oluştuğunda CPU durumunu TrapFrame yapısına kaydeder
// ve bu yapının bir pointer'ını bu fonksiyona iletir.
#[no_mangle] // Montaj kodu tarafından çağrılabilmesi için isim düzenlemesi yapılmaz
pub extern "C" fn handle_exception(frame: &mut TrapFrame) {
    // Exception nedenini belirle
    let exc_code = (frame.cause >> CAUSE_EXCCODE_SHIFT) & CAUSE_EXCCODE_MASK;

    // Interrupt pending bitlerini kontrol et (sadece EXCCODE_INT durumunda anlamlı)
    let interrupt_pending = (frame.cause >> CAUSE_IP_SHIFT) & CAUSE_IP_MASK;

    match exc_code {
        EXCCODE_SYSCALL => {
            // --- Sistem Çağrısı (Syscall) İşleme ---
            // Syscall numarası ve argümanları TrapFrame'den oku (MIPS64 ABI konvansiyonu)
            // Varsayılan MIPS64 N64 ABI'sinde:
            // Syscall Numarası: v0 (regs[2])
            // Argümanlar: a0-a7 (regs[4] - regs[11])
            // Karnal64 handle_syscall 5 arg (u64) alır, bu yüzden a0-a4'ü kullanacağız.

            let syscall_number = frame.regs[2]; // v0 registerı (r2)

            // Argümanlar a0-a4 registerlarında (r4-r8)
            let arg1 = frame.regs[4]; // a0 (r4)
            let arg2 = frame.regs[5]; // a1 (r5)
            let arg3 = frame.regs[6]; // a2 (r6)
            let arg4 = frame.regs[7]; // a3 (r7)
            let arg5 = frame.regs[8]; // a4 (r8) - 5. argüman için varsayım

            // !!! GÜVENLİK VE BELLEK YÖNETİMİ UYARISI:
            // Argümanlar arasında kullanıcı alanındaki pointerlar varsa (örn: resource_read için buffer pointerı),
            // Karnal64 fonksiyonlarına göndermeden ÖNCE, bu pointer'ların mevcut görevin
            // sanal adres alanında GEÇERLİ, SINIRLAR İÇİNDE ve uygun izinlere sahip (okunabilir/yazılabilir)
            // olduğunu bellek yönetim alt sistemi tarafından DOĞRULANMALIDIR.
            // Bu örnekte doğrulama adımları atlanmıştır, gerçek çekirdekte eklenmelidir.
            // Örneğin, `kmemory::validate_user_pointer(ptr, len, AccessMode::ReadWrite)` gibi bir çağrı yapılmalı.

            // Karnal64'ün genel sistem çağrısı işleyicisini çağır.
            // Bu fonksiyon çekirdek içinde ilgili Karnal64 API fonksiyonunu (resource_read, task_spawn vb.) çağıracaktır.
            let syscall_result = handle_syscall(syscall_number, arg1, arg2, arg3, arg4, arg5);

            // Karnal64'ten dönen sonucu (i64) kullanıcı alanına döndürmek için
            // TrapFrame'deki v0 registerına yaz (r2). MIPS'te dönüş değeri v0'a konur.
            frame.regs[2] = syscall_result as u64; // i64 -> u64 dönüşümü, negatif değerler (KError) için geçerlidir.

            // Syscall instruction'dan sonraki instruction'a atlamak için EPC'yi ilerlet.
            // MIPS'te syscall instruction 4 byte uzunluğundadır.
            // Bu, kullanıcının aynı syscall instruction'ı tekrar çalıştırmasını engeller.
            frame.epc += 4;

            // Syscall sonrası gerekirse durum registerı (Status) güncellenir.
            // Örneğin, EXL biti exception seviyesinden çıkıldığı için temizlenmelidir (montaj kodu tarafından yapılır).
            // SR registerındaki IE (Interrupt Enable) biti uygun şekilde yönetilmelidir.

             println!("Karnal64 MIPS: Syscall {} handled. Result: {}", syscall_number, syscall_result); // Debug amaçlı
        }
        EXCCODE_INT => {
            // --- Donanım Kesmesi (Interrupt) İşleme ---
            // Cause registerındaki IP (Interrupt Pending) bitlerini kontrol ederek hangi kesmenin geldiğini bul.
            let pending_interrupts = interrupt_pending;

            // Her bir IP biti için ilgili kesme işleyicisini çağır (veya bir kesme denetleyicisine devret)
            // Örnek: IP0 (Timer), IP1 (I/O), IP2 (IPC)...
            if (pending_interrupts >> 0) & 1 == 1 {
                 // IP0 - Zamanlayıcı Kesmesi olduğunu varsayalım
                  println!("Karnal64 MIPS: Timer Interrupt received."); // Debug amaçlı
                  ktask::handle_timer_interrupt(); // Zamanlayıcı modülünün kesme işleyicisini çağır
            }
            if (pending_interrupts >> 1) & 1 == 1 {
                 // IP1 - Örneğin UART veya diğer I/O kesmesi
                  println!("Karnal64 MIPS: I/O Interrupt received."); // Debug amaçlı
                  kresource::handle_io_interrupt(); // Kaynak yöneticisi veya ilgili sürücüyü çağır
            }
            // ... Diğer IP bitleri ve kesme kaynakları ...

            // Kesme işlendikten sonra, Status registerındaki IE bitini tekrar etkinleştirmek gerekebilir.
            // Ayrıca, MIPS'te kesmelerin neden olduğu EXL/ERL bitleri temizlenmelidir (montaj kodu).

             // Eğer zamanlayıcı kesmesi görev değiştirmeyi tetiklediyse, handle_timer_interrupt
             // fonksiyonu zamanlayıcıyı çağırabilir ve bu da yeni görevin TrapFrame'ine yükleme yapar.
        }
        EXCCODE_TLBL | EXCCODE_TLBS => {
            // --- TLB Miss (Page Fault benzeri) İşleme ---
            // BadVAddr registerı geçersiz adresi içerir.
            // Bellek yönetim birimi (MMU) ile etkileşim gerektiren karmaşık bir konudur.
            // Çekirdeğin bellek yöneticisi (kmemory), sayfa tablolarını kullanarak bu hatayı çözmeye çalışır.
            // Hata çözülürse (örn: lazy allocation, demand paging), handler geri döner ve hatayı tetikleyen
            // instruction yeniden denenir (EPC değeri arttırılmaz). Çözülemezse görev sonlandırılır.
            println!("Karnal64 MIPS: TLB Miss Exception! ExcCode: {}, BadVAddr: {:#x}, EPC: {:#x}",
                     exc_code, frame.bad_vaddr, frame.epc);
            // Bu hatayı çekirdeğin bellek yönetim modülüne devret:
             match kmemory::handle_tlb_exception(frame) {
                Ok(_) => { /* Başarılı, EPC yeniden deneme için zaten doğru */ }
                Err(kerr) => {
            //        // Hata çözülemedi, görevi sonlandır
                    println!("Karnal64 MIPS: Unhandled TLB Miss, terminating task.");
                     ktask::terminate_current_task(ExitReason::MemoryError);
            //        // terminate_current_task geri dönmeyecek bir çağrıdır
                    loop {} // Güvenlik için sonsuz döngü eğer terminate geri dönerse
                }
             }
            panic!("TLB Miss Exception - Memory management not fully implemented!"); // Geçici olarak panic
        }
         EXCCODE_ADDR => {
            // --- Adres Hatası İşleme ---
            // Genellikle hizalama hatası veya çekirdek/kullanıcı ayrımını ihlal eden erişim.
            println!("Karnal64 MIPS: Address Error Exception! BadVAddr: {:#x}, EPC: {:#x}",
                     frame.bad_vaddr, frame.epc);
            // Görevi sonlandır:
             ktask::terminate_current_task(ExitReason::AddressError);
            panic!("Address Error Exception!"); // Geçici olarak panic
         }
         EXCCODE_BUSI | EXCCODE_BUSD => {
            // --- Veriyolu Hatası İşleme ---
            // Erişim olmaya çalışılan fiziksel adresin geçersiz olması gibi donanımsal hatalar.
             println!("Karnal64 MIPS: Bus Error Exception! BadVAddr: {:#x}, EPC: {:#x}",
                      frame.bad_vaddr, frame.epc);
             // Görevi sonlandır veya çekirdek paniği:
              ktask::terminate_current_task(ExitReason::BusError);
             panic!("Bus Error Exception!"); // Geçici olarak panic
         }
         EXCCODE_RI | EXCCODE_CPI => {
             // --- Komut Hatası İşleme ---
             // Ayrılmış komut veya kullanılamayan yardımcı işlemci kullanma girişimi.
             println!("Karnal64 MIPS: Invalid Instruction Exception! ExcCode: {}, EPC: {:#x}",
                      exc_code, frame.epc);
             // Görevi sonlandır:
              ktask::terminate_current_task(ExitReason::InvalidInstruction);
             panic!("Invalid Instruction Exception!"); // Geçici olarak panic
         }
         EXCCODE_OV => {
             // --- Taşma Hatası İşleme ---
             // Aritmetik taşma.
             println!("Karnal64 MIPS: Overflow Exception! EPC: {:#x}", frame.epc);
             // Görevi sonlandır:
              ktask::terminate_current_task(ExitReason::Overflow);
             panic!("Overflow Exception!"); // Geçici olarak panic
         }
        // ... Diğer exception kodları buraya eklenir ve işlenir ...

        _ => {
            // --- Bilinmeyen veya Desteklenmeyen Exception ---
            // Beklenmedik bir durum, genellikle çekirdek hatası veya kurtarılamaz bir kullanıcı hatası.
            println!("Karnal64 MIPS: Unhandled Exception! ExcCode: {}, Cause: {:#x}, EPC: {:#x}",
                     exc_code, frame.cause, frame.epc);
            println!("Trap Frame: {:?}", frame); // Hata ayıklama için frame içeriğini yazdır
            // Bu durumda genellikle çekirdek panic yapar veya mevcut görevi sonlandırır.
             ktask::terminate_current_task(ExitReason::UnhandledException);
            panic!("Unhandled Exception!"); // Çekirdeğin durması sağlanır.
        }
    }
}
