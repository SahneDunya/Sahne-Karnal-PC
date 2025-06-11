#![no_std] // Standart kütüphaneye ihtiyaç yok
#![allow(dead_code)] // Geliştirme sırasında kullanılmayan kodlara izin ver
#![allow(unused_variables)] // Kullanılmayan argümanlara izin ver

// Karnal64 modülünü import et. Çekirdek kök dizininde veya üst seviyede olduğunu varsayalım.
use super::karnal64;
// Çekirdek içi yazdırma için bir makro olduğunu varsayalım (debugging için)
 use super::println;

// OpenRISC Kayıtlarını Saklamak İçin Tuzak Çerçevesi (Trap Frame) Yapısı
// Bu yapı, bir kesme veya tuzak oluştuğunda donanım veya giriş assembly kodu tarafından
// kaydedilen CPU durumunu temsil eder. OpenRISC'in gerçek register setine göre ayarlanmalıdır.
#[repr(C)] // C uyumluluğu için
#[derive(Debug, Default)] // Debug yazdırma ve varsayılan değerler için
pub struct TrapFrame {
    // Genel Amaçlı Kayıtlar (GPRs) - R0-R31
    // Tamamını kaydetmek en güvenlisidir, ancak syscall ABI'sine göre sadece kullanılanları
    // veya değiştirilenleri kaydetmek performans için optimize edilebilir.
    // Basitlik adına, syscall ABI'sinde kullanılan R3-R8 ve SP (R1) gibi kritik registerları ekleyelim.
    // Gerçek bir implementasyonda R0'dan R31'e kadar tüm GPR'lar olmalıdır.
    pub r0: u32, // R0 genellikle 0'dır, kaydedilmesi zorunlu olmayabilir
    pub r1: u32, // R1: Stack Pointer (SP)
    pub r2: u32, // R2: Frame Pointer (FP) veya diğer kullanım
    pub r3: u32, // R3: Genellikle Fonksiyon Çağrısı Dönüş Değeri veya Syscall Numarası
    pub r4: u32, // R4-R8: Fonksiyon Argümanları veya Syscall Argümanları
    pub r5: u32,
    pub r6: u32,
    pub r7: u32,
    pub r8: u32,
    // ... Diğer GPR'lar (R9-R31) gerçek implementasyonda buraya eklenecek ...
    pub r9: u32,
    pub r10: u32,
    pub r11: u32,
    pub r12: u32,
    pub r13: u32,
    pub r14: u32,
    pub r15: u32,
    pub r16: u32,
    pub r17: u32,
    pub r18: u32,
    pub r19: u32,
    pub r20: u32,
    pub r21: u32,
    pub r22: u32,
    pub r23: u32,
    pub r24: u32,
    pub r25: u32,
    pub r26: u32,
    pub r27: u32,
    pub r28: u32,
    pub r29: u32,
    pub r30: u32,
    pub r31: u32,

    // Özel Amaçlı Kayıtlar (SPRs)
    // Exception Program Counter (EPCR): Tuzak/Kesme oluştuğunda çalışmakta olan instruction'ın adresi (Syscall instruction'ı).
    // Supervisor Register (SR): Çekirdek/kullanıcı modu, kesme etkinleştirme vb. bilgileri içerir.
    // Exception Cause Register (ECR): Tuzak/Kesme sebebini içerir.
    pub epcr: u32, // Exception Program Counter
    pub sr: u32,   // Supervisor Register
    pub ecr: u32,  // Exception Cause Register (veya benzeri bir kayıt)
    // ... Diğer önemli SPR'lar (esr, ever, ppr vb.) gerçek implementasyonda buraya eklenecek ...
    pub esr: u32, // Exception Syndrome Register
    // ...
}

// Tuzak Sebepleri İçin Sabitler (OpenRISC ECR register değerlerine göre)
// Bu değerler OpenRISC mimari kılavuzundan alınmalıdır. Örnek değerler kullanıyoruz.
const TRAP_CAUSE_SYSCALL: u32 = 0x800; // OpenRISC sistem çağrısı tuzağı sebebi (örnek değer)
const TRAP_CAUSE_TIMER: u32 = 0x200;   // Zamanlayıcı kesmesi sebebi (örnek değer)
// ... Diğer tuzak sebepleri (page fault, alignment, illegal instruction vb.) buraya eklenecek ...
const TRAP_CAUSE_PAGE_FAULT_LOAD: u32 = 0x100; // Örnek Load Page Fault
const TRAP_CAUSE_PAGE_FAULT_STORE: u32 = 0x080; // Örnek Store Page Fault
const TRAP_CAUSE_ILLEGAL_INSTR: u32 = 0x040; // Örnek Illegal Instruction
const TRAP_CAUSE_BUS_ERROR: u32 = 0x020;   // Örnek Bus Error


/// Kesme ve Tuzak İşleyici Giriş Noktası.
/// Assembly dilindeki düşük seviyeli tuzak/kesme giriş kodu tarafından çağrılır.
/// CPU durumunu temsil eden TrapFrame pointer'ını alır.
///
/// Güvenlik Notu: Bu fonksiyon, kullanıcı modundan gelen bir kesme/tuzak sonrası
/// çekirdek modunda çalışır. `frame` pointer'ının geçerli bir çekirdek alanı
/// adresini gösterdiğinden emin olunmalıdır.
#[no_mangle] // Assembly'den çağrılabilmesi için ismini değiştirmesini engelle
pub extern "C" fn exception_handler(frame: *mut TrapFrame) {
    // 'frame' pointer'ının null olmadığını ve geçerli olduğunu varsayarak unsafe kullanıyoruz.
    // Gerçek bir çekirdekte, bu noktada temel geçerlilik kontrolleri yapılabilir,
    // ancak genellikle assembly giriş kodu bu pointer'ı güvenli bir şekilde hazırlar.
    let frame = unsafe { &mut *frame };

    // Tuzak/Kesme sebebini ECR register'ından oku (frame yapısına kaydedildiğini varsayarak)
    let trap_cause = frame.ecr;

    // Sebebe göre ilgili handler'a dallan
    match trap_cause {
        TRAP_CAUSE_SYSCALL => {
            // Sistem Çağrısı Tuzağı
            println!("Çekirdek: Sistem Çağrısı Tuzağı Yakalandı!"); // Debug çıktısı

            // Syscall ABI'sine göre argümanları TrapFrame'den çıkar
            let syscall_number = frame.r3 as u64; // R3'te syscall numarası varsayımı
            let arg1 = frame.r4 as u64;       // R4 ilk argüman
            let arg2 = frame.r5 as u64;       // R5 ikinci argüman
            let arg3 = frame.r6 as u64;       // R6 üçüncü argüman
            let arg4 = frame.r7 as u64;       // R7 dördüncü argüman
            let arg5 = frame.r8 as u64;       // R8 beşinci argüman

            // Karnal64 API'sının sistem çağrısı işleyicisini çağır
            // Bu fonksiyon, Karnal64'ün iç mantığını çalıştırır ve sonucu i64 olarak döndürür.
            let result = karnal64::handle_syscall(
                syscall_number,
                arg1,
                arg2,
                arg3,
                arg4,
                arg5,
            );

            // Karnal64'ten dönen sonucu kullanıcı alanının beklediği register'a (R3) yaz
            // Başarı durumunda pozitif/sıfır, hata durumunda negatif KError kodu döner.
            frame.r3 = result as u32; // R3'e dönüş değeri yazma varsayımı (i64 -> u32 dönüşümüne dikkat)

            // Kullanıcı programının sistem çağrısı instruction'ından sonraki instruction'dan
            // devam edebilmesi için EPCR'yi (kaydedilmiş PC) güncelle.
            // OpenRISC 'l.syscall' instruction'ı 4 byte'tır.
            frame.epcr += 4;

            println!("Çekirdek: Sistem Çağrısı Tamamlandı, Kullanıcıya Dönülüyor."); // Debug çıktısı
        }
        TRAP_CAUSE_TIMER => {
            // Zamanlayıcı Kesmesi
            println!("Çekirdek: Zamanlayıcı Kesmesi Yakalandı!");
            // TODO: Zamanlayıcı kesmesini işle (zamanlayıcı sayacını güncelle, görev zamanlayıcıyı çalıştır vb.)
            // Örneğin: ktask::timer_tick();
            // Gerekiyorsa EPCR'yi güncelleme (kesmeler genellikle instruction'ı tekrar çalıştırmaz)
        }
        TRAP_CAUSE_PAGE_FAULT_LOAD | TRAP_CAUSE_PAGE_FAULT_STORE => {
            // Bellek Yönetimi Hatası (Page Fault)
            //println!("Çekirdek: Sayfa Hatası Yakalandı! ECR: {:#x}, ESR: {:#x}", frame.ecr, frame.esr);
            // TODO: Sayfa hatasını işle (sayfayı belleğe yükle, haritalama hatası ise sinyal gönder vb.)
            // kmemory::handle_page_fault(frame.epcr, frame.ecr, frame.esr, frame);
            // Şimdilik panikliyoruz (çekirdek hata durumuna düşüyor)
            panic!("Unhandled Page Fault! ECR: {:#x}, ESR: {:#x}, EPCR: {:#x}", frame.ecr, frame.esr, frame.epcr);
        }
         TRAP_CAUSE_ILLEGAL_INSTR => {
            // Geçersiz Instruction Hatası
            println!("Çekirdek: Geçersiz Instruction! EPCR: {:#x}", frame.epcr);
            // TODO: Geçersiz instruction'ı işle (göreve sinyal gönder, sonlandır vb.)
            // Şimdilik panikliyoruz
             panic!("Illegal Instruction at EPCR: {:#x}", frame.epcr);
         }
         TRAP_CAUSE_BUS_ERROR => {
            // Bus Hatası
             println!("Çekirdek: Bus Hatası! EPCR: {:#x}", frame.epcr);
             // TODO: Bus hatasını işle
             // Şimdilik panikliyoruz
             panic!("Bus Error at EPCR: {:#x}", frame.epcr);
         }
        _ => {
            // Bilinmeyen veya İşlenmeyen Tuzak Sebebi
            println!("Çekirdek: İşlenmeyen Tuzak/Kesme Yakalandı! Sebebi (ECR): {:#x}, EPCR: {:#x}", trap_cause, frame.epcr);
            // TODO: Bu tür hataları daha güvenli bir şekilde işle (görevi sonlandır, logla vb.)
            // Şimdilik çekirdeği durduruyoruz (panic)
            panic!("Unhandled trap/interrupt! Cause (ECR): {:#x}, EPCR: {:#x}", trap_cause, frame.epcr);
        }
    }

    // İşleyici fonksiyonundan dönüş yapıldığında, düşük seviyeli assembly kodu
    // TrapFrame'den registerları geri yükleyerek kullanıcı programına (EPCR'den başlayarak) dönecektir.
}
