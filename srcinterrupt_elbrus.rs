#![no_std] // Standart kütüphaneye ihtiyaç duymuyoruz

// Gerekli core modüllerini içeri aktar
use core::arch::asm; // Inline assembly (CSR erişimi için)
use core::ptr;      // Pointer işlemleri için
use core::mem;      // Gerekirse bellek işlemleri için

// `volatile` crate'inden Volatile sarmalayıcıyı içeri aktar (cargo.toml'da tanımlı olmalı)
use volatile::Volatile;

// Konsol çıktı makrolarını kullanabilmek için (hata durumlarında loglama veya debug çıktısı için)
// Bu makrolar Sahne64 crate'i tarafından sağlanır ve resource API'sini kullanır.
// Bu crate'te kullanılabilir olmaları için uygun kurulum (örn. #[macro_use]) gereklidir.
// Bu örnekte, #[cfg] ile std/no_std çıktısını ayarlayarak makroların
// uygun ortamda kullanılabilir olduğunu varsayıyoruz.

// Elbrus/RISC-V çipine özgü ve donanıma bağımlı sabitler
// ** GERÇEK DEĞERLERİ ELBRUS REFERANS KILAVUZUNDAN ALIN! **
// Bu değerler çip modeline, çevre birimi instancelarına ve kesme kontrolcüsüne göre değişir.

// USB Kesme ile ilgili Sabitler
const USB_IRQ_BIT: usize = 1 << 10; // ÖRNEK DEĞER: mie CSR'sinde USB kesmesini temsil eden bit/maske
const USB_STATUS_REGISTER: usize = 0x4000_1000; // ÖRNEK DEĞER: USB Denetleyicisi Durum Kaydı (Memory-mapped I/O adresi)
const USB_DATA_REGISTER: usize = 0x4000_1004; // ÖRNEK DEĞER: USB Denetleyicisi Veri Kaydı
const USB_STATUS_DATA_RECEIVED: u32 = 0x01; // ÖRNEK DEĞER: Durum kaydındaki Veri Alındı biti
const USB_STATUS_DATA_SENT: u32 = 0x02;    // ÖRNEK DEĞER: Durum kaydındaki Veri Gönderildi biti
const USB_STATUS_CLEAR_MASK: u32 = USB_STATUS_DATA_RECEIVED | USB_STATUS_DATA_SENT; // Temizlenecek durum bit maskesi

// Diğer Elbrus/RISC-V mimarisine özgü donanım/CSR adresleri ve sabitleri
const MTVEC_ADDRESS: usize = 0x300; // MTVEC CSR adresi
const MIE_ADDRESS: usize = 0x304; // MIE CSR adresi
const MSTATUS_ADDRESS: usize = 0x300; // MSTATUS CSR adresi (MIE bit burada)
const MSTATUS_MIE_BIT: usize = 1 << 3; // MSTATUS kaydındaki Machine Interrupt Enable (MIE) biti

// Kesme Vektör Tablosu (IVT) - exception.rs dosyasında tanımlanmalı
// MTVEC CSR'si bu adresin başlangıcını işaret eder.
// RISC-V'de vektörlü modda (MODE=1), her kesme nedeni (mcause) için ayrı bir ofset bulunur.
extern "C" {
    // Bu, linker script veya exception.rs içinde tanımlanan semboldür.
    // Bu sembolün adresini MTVEC'e yazacağız.
   pub static __exception_vector_table: u8; // Linker script veya exception.rs'den gelen sembol adı
}

// Basit bir exception/trap giriş noktası modülü
// Gerçek bir implementasyon mcause CSR'sini okuyup uygun handler'a dallanmalıdır.
pub mod exception {
    use core::arch::asm;
    use core::ptr;

    // Trap giriş noktası. Linker script tarafından ayarlanır.
    #[link_section = ".trap.entry"] // Linker script bölümü
    #[naked] // Fonksiyonun prolog ve epilog içermemesi için
    #[export_name = "_start"] // Programın ana giriş noktası
    pub unsafe extern "C" fn _start() -> ! {
         // TODO: İlk stack pointer'ı ayarla! Bu çok kritik bir adımdır.
         // Genellikle linker script'ten stack'in bitiş adresi alınır.
          let stack_top = ptr::read_volatile(__stack_top as *const usize); // Varsayımsal sembol
          asm!("mv sp, {}", in(reg) stack_top);

         // TODO: Global pointer (gp) ve Thread pointer (tp) gibi diğer ABI kayıtlarını ayarla (Gerekirse).

         // TODO: .data ve .bss bölümlerini başlat (eğer bunlar bootloader tarafından yapılmıyorsa)
         // Bunlar genellikle bir başlatma rutini (crt0) tarafından yapılır.
          extern "C" { fn __init_data(); fn __zero_bss(); }
          __init_data();
          __zero_bss();

         // TODO: Global allocator'ı başlat (eğer 'alloc' kullanılıyorsa ve global allocator crate'i seçildiyse)
          #[cfg(feature = "alloc")] my_allocator::init();

         // TODO: Rust çalışma zamanını başlat (varsa)
         // extern "Rust" { fn main(argc: isize, argv: *const *const u8) -> isize; } // Eğer main'e geçilecekse
          init(); // Sistem/Platform init fonksiyonunu çağır
          let exit_code = main(0, ptr::null()); // Main fonksiyonunu çağır (eğer entry point main ise)

         // En basit durumda, sadece doğrudan platform init'e atla veya çağır
          platform::init(); // main veya init fonksiyonunu çağırmadan önce platform init'ini çağır

         // TODO: Sistem init'i tamamlandıktan sonra çekirdek ana döngüsüne gir veya ilk görevi başlat
         // Bu noktada kontrol asla geri dönmemelidir.
          loop {}; // Veya çekirdek idle döngüsüne atla

         // Şimdilik sadece exception entry'ye atla simülasyonu yapıyoruz.
         // Gerçek _start çok daha karmaşıktır.
         asm!("j exception_entry", options(noreturn)); // Doğrudan istisna girişine atla
    }


    // Genel İstisna/Kesme giriş noktası.
    // MTVEC tarafından işaret edilir. mcause CSR'sini okuyarak trap tipini belirler
    // ve uygun işleyiciye dallanır.
    #[link_section = ".trap.exceptions"] // Linker script bölümü
    #[export_name = "exception_entry"] // Linker script tarafından başvurulabilir
    pub unsafe extern "C" fn exception_entry() {
         // ** Güvenlik: Bu fonksiyon kritik bir bağlamda çalışır. **
         // Stack pointer'ı geçici olarak Machine-mode stack'ine kaydet/değiştir (Gerekirse)
         // Çağıranın (kesilen kodun) durumunu (kayıtlar) stack'e kaydet!
         // Bu manuel veya donanım tarafından otomatik yapılabilir (tcfg/tdata).

         // mcause CSR'sini oku (Trap nedeni ve tipi)
         let mcause: usize;
         asm!("csrr {0}, mcause", out(reg) mcause);

         // mepc CSR'sini oku (Kesme/İstisna oluştuğunda program sayacı)
         let mepc: usize;
         asm!("csrr {0}, mepc", out(reg) mepc);

         // mstatus CSR'sini oku (Önceki privilege seviyesi vb.)
         let mstatus: usize;
         asm!("csrr {0}, mstatus", out(reg) mstatus);

         // Trap tipini belirle: Interrupt (mcause'un üst biti set) veya Exception (üst bit 0)
         let is_interrupt = (mcause >> (mem::size_of::<usize>() * 8 - 1)) & 1;
         let trap_cause = mcause & !(1 << (mem::size_of::<usize>() * 8 - 1)); // Neden kodu

         if is_interrupt == 1 {
             // Bu bir kesme (Interrupt)
             // Kesme nedenine (trap_cause) göre IVT'deki ilgili işleyiciye dallan
             // Vektörlü modda (MTVEC MODE=1), handler adresi = MTVEC + 4 * trap_cause
             // Scaler modda (MTVEC MODE=0), tüm trap'ler MTVEC'deki tek bir handler'a gider.
             // Elbrus'un hangi modu kullandığına göre burası farklı olacaktır.
             // Varsayılan olarak Vektörlü Mod (Mode=1) varsayalım.

             let ivt_base = ptr::read_volatile(MTVEC_ADDRESS as *const usize) & !0b11; // MTVEC'in base adresi (MODE bitleri temizlenir)
             let handler_address = ivt_base.wrapping_add(trap_cause.wrapping_mul(mem::size_of::<usize>())); // trap_cause * pointer boyutu (4 veya 8)

             // Handler fonksiyon işaretçisini al
             let handler_fn = handler_address as *const () as *const unsafe extern "C" fn();

             // Handler'ı çağır!
             // Bu çağrı panik yapmamalı veya geri dönmemelidir, işleyici işini bitirmeli.
             if !handler_fn.is_null() {
                 // TODO: Handler çağrısı sırasında kaydedilen durum geri yüklenmeli (Gerekirse)
                  ptr::read_volatile(handler_fn)(); // Handler fonksiyonunu çağır
                 // TODO: Handler çağrısından sonra kaydedilen durum geri yüklenmeli (Gerekirse)

                 // TODO: GIC gibi harici bir kesme kontrolcüsü varsa EOI (End of Interrupt) işlemi yapılmalı.
                  write_gic_eoi_register(trap_cause); // Donanıma özel

             } else {
                 // Beklenmeyen/tanımsız kesme
                 // Bu durum default_interrupt_handler tarafından da yakalanabilir,
                 // ama burada mcause biliniyor.
                 #[cfg(feature = "std")] std::eprintln!("UYARI: Tanımsız kesme nedeni: {}", trap_cause);
                 #[cfg(not(feature = "std"))] eprintln!("UYARI: Tanımsız kesme nedeni: {}", trap_cause); // Sahne64 macro varsayımı
                 // Kritik hata veya panik
                  loop {}; // Veya halt_system();
             }

         } else {
             // Bu bir istisna (Exception)
             // İstisna nedenine (trap_cause) göre işlem yap.
             // Örnek: Load/Store Access Fault, Illegal Instruction, Syscall vb.
             // Sahne64 API syscall'ları da burada Machine mode'da yakalanır
             // ve çekirdek syscall handler'ına yönlendirilir.

             match trap_cause {
                 8 => { // Environment Call from U-mode (System Call)
                     // U-mode'dan gelen syscall
                     // TODO: Syscall numarasını ve argümanları oku (a0-a5 kayıtları)
                     // TODO: Çekirdek syscall dispatcher'ına yönlendir
                     // TODO: Sonucu a0 kaydına yaz ve mepc + 4 ile dön.
                     #[cfg(feature = "std")] std::println!("U-mode Syscall yakalandı.");
                     #[cfg(not(feature = "std"))] println!("U-mode Syscall yakalandı."); // Sahne64 macro varsayımı

                     // Simülasyon: mepc + 4 yap ve dön
                     let new_mepc = mepc.wrapping_add(4); // Syscall instruction length is usually 4 bytes
                     asm!("csrw mepc, {0}", in(reg) new_mepc);
                 }
                 // TODO: Diğer istisnalar (Page Fault, Illegal Instruction vb.)
                 // Bunlar için uygun hata işleme veya görev sonlandırma yapılmalıdır.
                 _ => {
                     // Beklenmeyen istisna
                     #[cfg(feature = "std")] std::eprintln!("KRİTİK HATA: Beklenmeyen istisna nedeni: {} mepc: 0x{:x}", trap_cause, mepc);
                     #[cfg(not(feature = "std"))] eprintln!("KRİTİK HATA: Beklenmeyen istisna nedeni: {} mepc: 0x{:x}", trap_cause, mepc); // Sahne64 macro varsayımı
                     // Bu durum genellikle bir görevin sonlandırılması veya sistemin durdurulması anlamına gelir.
                      loop {}; // Veya halt_system();
                 }
             }
         }

         // Durumu geri yükle (kaydedilen kayıtlar, mstatus) ve önceki moda MRET ile dön.
         // Bu, exception_entry'nin sonunda yapılmalıdır.
         // Kaydedilen kayıtları yükle (Gerekirse)
         // mstatus'ı geri yükle (PIP, PPIE bitleri)
         asm!("mret"); // Machine Return from Trap
    }
     // NOTE: exception_entry fonksiyonu `mret` ile döner, normal bir Rust fonksiyonu gibi geri dönmez.
     // İmzasındaki `unsafe extern "C" fn()` dönüş değeri belirtmez ama asm!("mret") ile bitmelidir.
}

// exception::init() fonksiyonu şu anda boş veya başka bir yerde yapılıyor olabilir.
// Eğer mtvec ve mie/mstatus ayarları burada init() fonksiyonunda yapılıyorsa,
// exception::init()'in başka bir rolü olmalı veya çağrılmasına gerek yoktur.
// Önceki koda göre çağrısı korunuyor ama yorumlandı.
 pub fn init() {
     // Bu fonksiyon kernel privilege seviyesinde çalışmalıdır.
     unsafe {
         // 1. MTVEC'i ayarla (Kesme Vektör Tablosu Adresi)
         // exception.rs'deki __exception_vector_table sembolünün adresini kullan.
         let ivt_address = &__exception_vector_table as *const u8 as usize;
         // MTVEC'e yazma (CSR yazma)
         asm!("csrw mtvec, {0}", in(reg) ivt_address, options(nostack)); // options(nostack) burada da olabilir

         // 2. İlgili Çevre Birimi Kesmesini Etkinleştirme (mie register - machine interrupt enable)
         // USB kesmesini MIE CSR'sinde etkinleştir.
         // csrrs instr: CSR Register Set. Okur, ORlar, yazar. mie = mie | USB_IRQ_BIT
         asm!("csrrs {0}, mie, {1}", out(reg) _, in(reg) USB_IRQ_BIT, options(nostack)); // options(nostack)


         // 3. Genel Kesmeleri Etkinleştirme (mstatus register - machine status, MIE biti - Machine Interrupt Enable)
         // MSTATUS CSR'sindeki MIE bitini (3. bit) set et.
         // csrrs instr: mstatus = mstatus | (1 << 3)
         asm!("csrrs {0}, mstatus, {1}", out(reg) _, in(reg) MSTATUS_MIE_BIT, options(nostack)); // options(nostack)

          exception::init(); // Eğer exception::init'in farklı bir rolü yoksa bu çağrıya gerek yok.
                            // Önceki kodda vardı, şimdilik yoruma alındı.
     }
     // init fonksiyonu başarıyla tamamlanırsa geri döner.
 }

// Interrupt handler fonksiyonları
// Bu fonksiyonlar exception_entry'den çağrılır.
// Linker scriptte .trap.interrupt_handlers bölümüne yerleştirilirler.

#[no_mangle] // Linker script veya exception_entry tarafından çağrılabilir
#[link_section = ".trap.interrupt_handlers"] // Kesme işleyicileri için ayrı bir bölüm
pub unsafe extern "C" fn usb_interrupt_handler() {
    // ** Güvenlik: Bu işleyici unsafe'dir çünkü kesme bağlamında çalışır **
    // ve donanımla doğrudan etkileşime girer.

    // ** DİKKİR: Bu bölüm DONANIMA ÖZGÜDÜR ve ELBRUS REFERANS KILAVUZUNA GÖRE KODLANMALIDIR! **
    // Bu işleyici çekirdek içinde çalışır, kullanıcı alanındaki Sahne64 API'sini doğrudan çağırmaz.
    // Gelen USB verisini alır ve çekirdekteki USB sürücüsü koduna iletir.
    // Ardından, bu veriyi bekleyen kullanıcı görevini Sahne64 çekirdek zamanlama mekanizması
    // aracılığıyla uyandırır (örn. resource::read için bekleyen görev).

    // 1. Kesme kaynağını belirle (Elbrus'a özgü STATUS REGISTER'ı oku)
    // Hangi kesme bitlerinin set olduğuna bakılır.
    let status = ptr::read_volatile(USB_STATUS_REGISTER as *const u32);

    // 2. İşlenen kesme bitlerini (flag'larını) temizle (ÇOK ÖNEMLİ!)
    // Bu, kesmenin tekrar tetiklenmesini önler. Temizleme yöntemi donanıma özgüdür.
    // Bu örnekte, işlenen bitleri 0 yaparak statüs kaydına yazıyoruz.
    // SADECE işlenen bitleri değiştirdiğimizden emin olmak için dikkatli olunmalıdır.
    let status_to_clear = status & USB_STATUS_CLEAR_MASK; // Sadece temizlenecek bitleri ayıkla
    if status_to_clear != 0 {
         // Sadece temizlenecek bitler varsa yazma yap
         let mut status_reg = Volatile::new(USB_STATUS_REGISTER as *mut u32);
         // Okunan değerdeki sadece temizlenecek bitleri 0 yapıp geri yaz
         // Alternatif olarak, bazı donanımlar temizlemek için 1 yazılmasını bekler. Kılavuza bakın!
         status_reg.write(status & !status_to_clear); // Örnek: Bitleri temizle
    }


    // 3. Kesme nedenine göre işlem yap (Veri geldi, Veri gönderildi vb.)
    // Bu kısım, USB sürücüsünün temel logic'idir ve çekirdek içinde yer alır.
    if (status & USB_STATUS_DATA_RECEIVED) != 0 {
        // Veri geldi kesmesi oluştu
        // Veriyi USB kontrolcüsünden oku (hardware-specific)
         let data = ptr::read_volatile(USB_DATA_REGISTER as *const u32); // Örnek okuma
        // TODO: Okunan veriyi çekirdekteki USB sürücüsü tamponuna yaz.
        // TODO: Resource'u (USB console kaynağı) bekleyen görevleri uyandır (örn. resource::read yapan görev).
         #[cfg(feature = "std")] std::println!("USB Veri Alındı: 0x{:x}", data); // Debug çıktı
         #[cfg(not(feature = "std"))] println!("USB Veri Alındı: 0x{:x}", data); // Debug çıktı
    }

    if (status & USB_STATUS_DATA_SENT) != 0 {
         // Veri gönderildi kesmesi oluştu
         // TODO: Çekirdek tamponundan bir sonraki veriyi USB kontrolcüsüne yaz (eğer gönderilecek veri varsa).
         // TODO: Resource'u (USB console kaynağı) bekleyen görevleri uyandır (örn. resource::write'ın tamamlanmasını bekleyen görev).
         #[cfg(feature = "std")] std::println!("USB Veri Gönderildi (işleyici içinde)."); // Debug çıktı
         #[cfg(not(feature = "std"))] println!("USB Veri Gönderildi (işleyici içinde)."); // Debug çıktı
    }

    // TODO: Diğer kesme nedenleri (hata kesmeleri, bağlantı durum değişiklikleri vb.)


    // NOTE: Kesme işleyiciden çıkış exception_entry'de MRET ile yapılır.
    // İşleyici fonksiyonunun kendisi normal bir fonksiyon gibi geri döner.
    // exception_entry, handler'ı çağırdıktan sonra durumu geri yükleyip MRET yapar.
}
