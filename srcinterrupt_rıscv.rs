#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz

// Gerekli core modüllerini içeri aktar
use core::arch::asm; // Inline assembly (CSR erişimi için)
use core::ptr;      // Pointer işlemleri için
use core::mem;      // Gerekirse bellek işlemleri için (Örn: size_of, align_of) - şu an kullanılmıyor

// `volatile` crate'inden Volatile sarmalayıcıyı içeri aktar (cargo.toml'da tanımlı olmalı)
use volatile::Volatile; // <-- Added import

// Konsol çıktı makrolarını kullanabilmek için (hata durumlarında loglama veya debug çıktısı için)
// Bu makrolar Sahne64 crate'i tarafından sağlanır ve resource API'sini kullanır.
// Bu crate'te kullanılabilir olmaları için uygun kurulum (örn. #[macro_use]) gereklidir.
// Bu örnekte, #[cfg] ile std/no_std çıktısını ayarlayarak makroların
// uygun ortamda kullanılabilir olduğunu varsayıyoruz.
 use sahne64::eprintln; // Örnek import eğer macro publicse


// RISC-V mimarisine özgü ve donanıma bağımlı sabitler
// ** GERÇEK DEĞERLERİ İŞLEMCİ VE ÇEVRE BİRİMİ REFERANS KILAVUZLARINDAN ALIN! **
// Bu değerler çip modeline, kesme kontrolcüsüne (CLINT, PLIC) ve konfigürasyona göre değişir.

// RISC-V Makine Modu CSR Adresleri
const CSR_MTVEC: usize = 0x305; // Trap Vector Base Address
const CSR_MIE: usize = 0x304;   // Machine Interrupt Enable
const CSR_MSTATUS: usize = 0x300; // Machine Status
const CSR_MCAUSE: usize = 0x342; // Machine Trap Cause
const CSR_MEPC: usize = 0x341;   // Machine Exception Program Counter

// MSTATUS Register'ındaki Bitler
const MSTATUS_MIE_BIT: usize = 1 << 3;     // Machine Interrupt Enable (Genel kesme etkinleştirme)
const MSTATUS_MPIE_BIT: usize = 1 << 7;    // Machine Previous Interrupt Enable (MRET sonrası MIE değeri)
const MSTATUS_MPP_SHIFT: usize = 11;   // Machine Previous Privilege (Önceki privilege seviyesi)
const MSTATUS_MPP_MASK: usize = 0b11 << MSTATUS_MPP_SHIFT;

// MIE Register'ındaki Bitler (Hangi interrupt kaynaklarının trap oluşturacağını maskeler)
const MIE_MSIE_BIT: usize = 1 << 3;     // Machine Software Interrupt Enable
const MIE_MTIE_BIT: usize = 1 << 7;    // Machine Timer Interrupt Enable (RTC/CLINT timer için)
const MIE_MEIE_BIT: usize = 1 << 11;   // Machine External Interrupt Enable (PLIC gibi harici kaynaklar için)


// Kesme Vektör Tablosu (IVT) başlangıç adresi
// MTVEC CSR'si bu adresin başlangıcını işaret eder.
// Bu sembol, linker script veya trap/exception handling kodunda tanımlanmalıdır.
// RISC-V'de MTVEC Mode (Direct=0, Vectored=1) önemlidir. Vectored mode'da
// her interrupt cause için MTVEC + 4*cause adresindeki handler çağrılır.
extern "C" {
    // Bu, linker script veya trap handling'den gelen semboldür.
    // Bu sembolün adresini MTVEC'e yazacağız.
   pub static __exception_vector_table: u8; // Linker script veya trap handling'den gelen sembol adı
}

// Standart RISC-V Makin Zamanlayıcısı (CLINT) Adresleri ve Registerları (Memory-mapped I/O)
// Bu adresler platforma bağlıdır, CLINT dokümantasyonuna bakın.
const CLINT_BASE_ADDRESS: usize = 0x2000000; // ÖRNEK CLINT temel adresi (Kılavuza bakın!)
const CLINT_MTIMECMP_OFFSET: usize = 0x4000; // MTIMECMP register offset (Machine Timer Compare value)
const CLINT_MTIME_OFFSET: usize = 0xBFF8; // MTIME register offset (Machine Timer value - 64 bit)


/// RISC-V mimarisi (Makine Modu) için kesme ve trap altyapısını başlatır.
/// MTVEC, MIE ve MSTATUS gibi ilgili CSR'ları ayarlar.
/// Bu fonksiyon, sistem başlangıcında (kernel init sürecinde) M-mode'da çağrılmalıdır.
pub fn init() {
    // Bu fonksiyon Machine mode (M-mode) privilege seviyesinde çalışmalıdır.
    unsafe {
        // 1. MTVEC'i ayarla (Kesme Vektör Tablosu Adresi)
        // __exception_vector_table sembolünün adresini kullan.
        // MTVEC'e yazma (CSR yazma)
        // Trap Vector Mode'u da ayarlanır (genellikle Vektörlü mod için adresin son biti 1 yapılır).
        // Örneğin, adresin son 2 bitini temizleyip sonra OR ile 1 ekleyerek Mode=1 (Vectored) ayarlanabilir.
        let ivt_address = &__exception_vector_table as *const u8 as usize;
        // Vektörlü mod (Mode = 1) için adresin son 2 bitini 00 yap, sonra OR ile 1 yap.
        let mtvec_value = (ivt_address & !0b11) | 0b01; // Varsayım: Vectored mode

        // csrwr instr: CSR yazma (riscv-asm syntax: csr rw, value)
        asm!("csrw mtvec, {0}", in(reg) mtvec_value, options(nostack));
        // ** AÇIKLAMA: MTVEC adresini ve Modunu sisteme bildirme işlemi DONANIMA ÖZGÜDÜR! **
        // Doğru CSR adresi ve yazma yönergesi için RISC-V ISA ve çip kılavuzuna bakın.


        // 2. İlgili Çevre Birimi Kesmelerini Etkinleştirme (MIE register - Machine Interrupt Enable)
        // Zamanlayıcı (MTIE) ve Harici (MEIE - PLIC varsa) kesmelerini MIE CSR'sinde etkinleştir.
        // csrrs instr: CSR Register Set. CSR'yi okur, rs ile ORlar, yazar. csr = csr | rs
        // rs=zero (x0) ile kullanıldığında, sadece rd'ye CSR'nin eski değerini okur.
        // Buradaki kullanım: MIE'ye istenen bit maskesini set et.
        let irqs_to_enable_mask = MIE_MTIE_BIT | MIE_MEIE_BIT; // Zamanlayıcı ve Harici kesmeleri etkinleştir

        asm!("csrrs {0}, {1}, {2}", // csrrs rd, csr, rs
            out(reg) _,          // eski MIE değeri (kullanmıyoruz, _ ile ignore et)
            const CSR_MIE,       // MIE CSR adresi
            in(reg) irqs_to_enable_mask, // Set edilecek bit maskesi (kaynak register)
            options(nostack)     // Stack manipulation olmadığını belirtir
        );
        // **DİKKAT:** MIE register'ı ve bit pozisyonları için RISC-V ISA ve çip kılavuzuna bakın.


        // 3. Genel Kesmeleri Etkinleştirme (MSTATUS register - Machine Status, MIE biti)
        // MSTATUS CSR'sindeki MIE bitini (Machine Interrupt Enable) set et.
        // Bu bit olmadan, bireysel kesme etkinleştirme bitleri (MIE CSR içindekiler) tek başına yeterli DEĞİLDİR.
        // csrrs instr: mstatus = mstatus | rs
        asm!("csrrs {0}, {1}, {2}", // csrrs rd, csr, rs
            out(reg) _,          // eski MSTATUS değeri (kullanmıyoruz)
            const CSR_MSTATUS,   // MSTATUS CSR adresi
            in(reg) MSTATUS_MIE_BIT, // Set edilecek MIE biti maskesi (kaynak register)
            options(nostack)     // Stack manipulation olmadığını belirtir
        );
        // **DİKKAT:** MSTATUS register'ı ve MIE bit pozisyonu için RISC-V ISA kılavuzuna bakın.

        // 4. RISC-V Zamanlayıcısını Başlatma (CLINT MTIMECMP)
        // Periyodik zamanlayıcı kesmeleri almak için MTIMECMP register'ını ayarlayın.
        // MTIME değeri MTIMECMP'ye ulaştığında zamanlayıcı kesmesi (MTI) oluşur.
        let mtime_address = CLINT_BASE_ADDRESS.wrapping_add(CLINT_MTIME_OFFSET); // Güvenli adres hesaplama
        let mtimecmp_address = CLINT_BASE_ADDRESS.wrapping_add(CLINT_MTIMECMP_OFFSET); // Güvenli adres hesaplama

        // MTIME değeri 64 bitir, 2 x u32 volatile okuma/yazma gerekebilir 32 bit sistemde.
        // Burada 64 bit sistem varsayıyoruz, usize = 64 bit.
        let current_mtime = ptr::read_volatile(mtime_address as *const usize); // MTIME oku
        let interval = 100000; // Örnek kesme periyodu (cycles)
        let next_mtimecmp = current_mtime.wrapping_add(interval); // Bir sonraki kesme zamanı

        // MTIMECMP register'ını yaz.
        ptr::write_volatile(mtimecmp_address as *mut usize, next_mtimecmp); // MTIMECMP set et
        // **UYARI:** CLINT adresleri ve MTIMECMP/MTIME registerları platforma özgüdür!

        // Diğer platforma özgü başlatma adımları buraya eklenebilir (örn. PLIC, diğer çevre birimleri init)
    }
    // init fonksiyonu başarıyla tamamlanırsa geri döner.
}

// Örnek Makine Modu Zamanlayıcı Kesme İşleyicisi
// Bu fonksiyon, trap entry noktasından (MTVEC tarafından işaret edilen) çağrılır.
// RISC-V vektörlü modda, cause=7 (Machine Timer Interrupt) için handler adresi = MTVEC + 4 * 7
#[no_mangle] // Linker script veya trap entry tarafından çağrılabilir
// Kesme işleyici fonksiyonları genellikle 'unsafe extern "C"' olarak tanımlanır.
pub unsafe extern "C" fn MachineTimer_interrupt_handler() {
    // ** Güvenlik: Bu işleyici unsafe'dir çünkü kesme bağlamında çalışır **
    // ve potansiyel olarak donanım veya paylaşılan bellekle etkileşime girer.
    // TODO: Kesme bağlamında registerları kaydet!

    // ** DİKKAT: Bu bölüm DONANIMA ÖZGÜDÜR ve ZAMANLAYICI (CLINT) KILAVUZUNA GÖRE KODLANMALIDIR! **
    // Bu işleyici çekirdek içinde çalışır, kullanıcı alanındaki Sahne64 API'sini doğrudan çağırmaz.
    // Periyodik zamanlayıcı olayını çekirdek zamanlayıcıya bildirir.

    // 1. Zamanlayıcı kesme bayrağını temizleyin (RISC-V'de genellikle MTIMECMP'yi güncelleyerek yapılır).
    // MTIME >= MTIMECMP olduğu sürece kesme pending kalır.
    // Bir sonraki kesmeyi planlayarak bayrağı implicit olarak temizleriz.
    let mtime_address = CLINT_BASE_ADDRESS.wrapping_add(CLINT_MTIME_OFFSET);
    let mtimecmp_address = CLINT_BASE_ADDRESS.wrapping_add(CLINT_MTIMECMP_OFFSET);

    let current_mtime = ptr::read_volatile(mtime_address as *const usize);
    let interval = 100000; // init() fonksiyonundaki interval ile aynı olmalı
    let next_mtimecmp = current_mtime.wrapping_add(interval);

    ptr::write_volatile(mtimecmp_address as *mut usize, next_mtimecmp); // MTIMECMP'yi güncelleyerek bayrağı temizle

    // 2. Zamanlayıcı tick sayısını artır veya çekirdek zamanlayıcıyı güncelle.
    static mut TIMER_COUNT: usize = 0; // unsafe static mut usage

    unsafe {
         TIMER_COUNT = TIMER_COUNT.wrapping_add(1); // Güvenli artırma
         // İsteğe bağlı: TIMER_COUNT değerini bir yere yazdırabilir veya loglayabilirsiniz.
         // Sahne64 konsol makrolarını kullanarak çıktı
         #[cfg(feature = "std")] std::println!("Zamanlayıcı kesmesi (tick {}).", TIMER_COUNT);
         #[cfg(not(feature = "std"))] println!("Zamanlayıcı kesmesi (tick {}).", TIMER_COUNT); // Sahne64 macro varsayımı
    }

    // TODO: Sahne64 çekirdek zamanlayıcısını güncelle (çekirdek içindeki bir fonksiyona çağrı)
     kernel_timer_tick(); // Varsayımsal çekirdek fonksiyonu


    // TODO: Kesme işleyiciden çıkış. Kaydedilen registerları geri yükle.
    // Trap entry noktasından çağrıldığı için, bu handler geri döndüğünde
    // trap entry kodu durumu geri yüklemeli ve MRET yapmalıdır.
}

// no_std ortamında temel çıktı için Sahne64'ün konsol makrolarını kullanıyoruz.
// Lokal stdio modülü veya println! makrosu bu dosyada tanımlı değil.
// Sahne64 crate'i tarafından sağlandığı varsayılır.

// Örnek Harici Kesme İşleyicisi (PLIC/Device IRQ için)
// Eğer harici kesmeler (örneğin USB) kullanılıyorsa, MIE'de MEIE biti etkinleştirilir
// ve genellikle PLIC (Platform Level Interrupt Controller) kullanılır.
// PLIC, hangi harici kesmenin olduğunu belirler ve ilgili işleyiciye yönlendirir.

#[no_mangle]
pub unsafe extern "C" fn MachineExternal_interrupt_handler() {
     // TODO: Kesme bağlamında registerları kaydet!

     // 1. PLIC'ten kesme ID'sini talep et (claim). Bu, PLIC'te bayrağı pending'den active'e geçirir.
      PLIC_BASE_ADDRESS + PLIC_CLAIM_COMPLETE_OFFSET + hart_id * stride
      let claim_address = PLIC_BASE_ADDRESS.wrapping_add(PLIC_CLAIM_COMPLETE_OFFSET).wrapping_add(hart_id * stride);
      let irq_id = ptr::read_volatile(claim_address as *const u32);

     // 2. Kesme ID'sine göre ilgili aygıt sürücüsü işleyicisine dallan.
     // Bu, çekirdek içindeki bir dispatch tablosu veya sürücü framework'ü tarafından yapılır.
      match irq_id {
          USB_PLIC_IRQ_ID => usb_device_driver_interrupt_handler(), // Örnek: USB sürücü işleyicisini çağır
          UART_PLIC_IRQ_ID => uart_device_driver_interrupt_handler(), // Örnek: UART sürücü işleyicisini çağır
          _ => { // Beklenmeyen/tanımsız harici kesme
               #[cfg(feature = "std")] std::eprintln!("UYARI: Beklenmeyen PLIC kesme ID: {}", irq_id);
               #[cfg(not(feature = "std"))] eprintln!("UYARI: Beklenmeyen PLIC kesme ID: {}", irq_id);
     //          // Kritik hata veya panik
                loop {}; // Veya halt_system();
          }
      }

     // 3. PLIC'te kesmeyi tamamla (complete). Bu, PLIC'te bayrağı active'den inactive'e geçirir.
     // Aynı claim_complete register'ına kesme ID'sini yazılır.
      ptr::write_volatile(claim_address as *mut u32, irq_id);

     // TODO: Kaydedilen registerları geri yükle.
     // TODO: Kesme işleyiciden çıkış (trap entry'ye dönülür).
}
