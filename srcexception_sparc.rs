#![no_std] // Standart kütüphaneye ihtiyaç yok, çekirdek alanı
#![allow(unused_variables)] // Geliştirme aşamasında kullanılmayan değişkenlere izin ver

use core::arch::asm;
use core::ptr;

// Karnal64 API'sından ihtiyacımız olan çekirdek fonksiyonlarını ve tiplerini içe aktaralım.
// Bu dosya, karnal64.rs'teki API fonksiyonlarını kullanacaktır.
// Gerçek bir projede, karnal64 modülü `lib.rs` veya benzeri ana çekirdek kütüphanesinden erişilebilir olmalıdır.
// Şimdilik direkt path kullanıyoruz gibi düşünelim veya extern tanımları yapalım.

// Karnal64 API fonksiyonlarına dışarıdan erişim tanımları (eğer ayrı bir modüldeyse)
extern "C" {
    // karnal64.rs dosyasındaki handle_syscall fonksiyonu
    // Güvenlik Notu: Bu fonksiyon, kullanıcıdan gelen ham pointer'ları alır
    // ve çekirdek içinde doğruluğunu/güvenliğini kontrol etmekle yükümlüdür.
    fn handle_syscall(
        number: u64,
        arg1: u64,
        arg2: u64,
        arg3: u64,
        arg4: u64,
        arg5: u64,
    ) -> i64;

    // Karnal64'teki diğer dahili yöneticilerin fonksiyonları
    // (Örn: Page fault işleyicisi, zamanlayıcı vb.)
     fn kmemory_handle_page_fault(trap_frame: *mut TrapFrame, fault_addr: u64) -> Result<(), KError>;
     fn ktask_yield_from_trap() -> Result<(), KError>;
}

// SPARC V8 mimarisine özgü sabitler
// Sistem çağrısı tuzağının türü (Genellikle 0x8b veya 0x80+syscall_number olabilir, conventiona bağlı)
// Burada yaygın bir konvansiyonu kullanıyoruz, gerçek donanıma göre doğrulanmalı.
const SPARC_TRAP_TYPE_SYSCALL: u32 = 0x8B; // V8 SVC (SuperVisor Call) trap type
const SPARC_TRAP_TYPE_PAGE_FAULT_DATA: u32 = 0x0C; // Data Access Exception
const SPARC_TRAP_TYPE_PAGE_FAULT_INSTR: u32 = 0x0D; // Instruction Access Exception
const SPARC_TRAP_TYPE_ALIGNMENT: u32 = 0x07; // Alignment Error
const SPARC_TRAP_TYPE_ILLEGAL_INSTR: u32 = 0x02; // Illegal Instruction
const SPARC_TRAP_TYPE_DIVISION_BY_ZERO: u32 = 0x01; // Priveleged Instruction

// Tuzak çerçevesi boyutu ve register offsetleri (Save/Restore penceresi dikkate alınarak)
// SPARC V8'de trap olduğunda, caller'ın %o0-%o7 registerları, trap handler'ın %l0-%l7 registerları olur.
// Trap frame genellikle %g, %l, %i registerları, PSR, WIM, TBR, Y, PC, nPC'yi içerir.
// Buradaki yapı basitleştirilmiştir, tüm registerları içermelidir gerçekte.
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct TrapFrame {
    // Global registerlar (%g0-%g7)
    pub g_regs: [u32; 8],
    // Outgoing registerlar (%o0-%o7) - Tuzak olduğunda bunlar handler'ın %l registerları olur.
    // Kullanıcının sistem çağrısı argümanları burada bulunur (%o0-%o5).
    pub l_regs: [u32; 8], // Yerel registerlar (%l0-%l7)
    pub i_regs: [u32; 8], // Gelen registerlar (%i0-%i7)

    // İşlemci Durum Yazmacı (Processor State Register)
    pub psr: u32,
    // Pencere Yöneticisi Yazmacı (Window Invalid Mask) - SPARC V9'da daha önemli, V8'de genellikle 0
    pub wim: u32,
    // Tuzak Taban Yazmacı (Trap Base Register)
    pub tbr: u32, // Genellikle sadece pointer kısmı lazım olur
    // Y Yazmacı (Multiply/Divide)
    pub y: u32,

    // Program Counter ve Next Program Counter (Tuzak anındaki adresler)
    pub tpc: u32,
    pub tnpc: u32,

    // Ek durum bilgileri eklenebilir (örn. fault adresi)
    // SPARC'ta bu bilgiler genellikle TDS/TSNR yazmaçlarındadır ve trap frame'e kopyalanabilir.
}

// Basitleştirilmiş bir tuzak çerçevesi boyutu hesaplaması. Gerçekte tüm registerlar + ek bilgiler hesaplanmalı.
const TRAP_FRAME_SIZE: usize = core::mem::size_of::<TrapFrame>();
// Register offsetleri (örneğin g_regs'in frame başından offseti) gerçek assembly ile eşleşmeli.
// Bu offsetler, assembly'deki `st` (store) komutlarında kullanılacak adresleri belirler.
// Örnek offsetler (gerçek SPARC trap frame yapısına göre ayarlanmalı):
const G_REGS_OFFSET: usize = 0;
const L_REGS_OFFSET: usize = G_REGS_OFFSET + core::mem::size_of::<[u32; 8]>();
const I_REGS_OFFSET: usize = L_REGS_OFFSET + core::mem::size_of::<[u32; 8]>();
const PSR_OFFSET: usize = I_REGS_OFFSET + core::mem::size_of::<[u32; 8]>();
const WIM_OFFSET: usize = PSR_OFFSET + core::mem::size_of::<u32>();
const TBR_OFFSET: usize = WIM_OFFSET + core::mem::size_of::<u32>();
const Y_OFFSET: usize = TBR_OFFSET + core::mem::size_of::<u32>();
const TPC_OFFSET: usize = Y_OFFSET + core::mem::size_of::<u32>();
const TNPC_OFFSET: usize = TPC_OFFSET + core::mem::size_of::<u32>();


/// Düşük seviyeli assembly tuzak işleyicisi giriş noktası.
/// Donanımdan gelen tuzak (syscall, exception) buraya yönlendirilir.
/// Bu kodun `.text.trap_handler` gibi belirli bir bölüme yerleştirilmesi gerekebilir.
/// `#[no_mangle]` bu fonksiyon adının Rust derleyicisi tarafından değiştirilmesini engeller.
/// `global_asm!` veya `asm!` blockları bu assembly kodunu içerecektir.
#[no_mangle]
#[naked] // Rust'ın fonksiyon prolog/epilog eklemesini engeller, kontrol bizdedir.
pub unsafe extern "C" fn trap_handler_entry() {
    // Naked fonksiyon içinde manuel olarak assembly kodu yazılmalıdır.
    // Bu kod şunları yapmalıdır:
    // 1. Kullanıcının tüm kritik registerlarını (Global, In, Local/Out - pencereye bağlı) kaydet.
    // 2. Çekirdek stack'ine bir tuzak çerçevesi (TrapFrame) yapısı oluştur.
    // 3. PSR, WIM, TBR, Y, TPC, TNPC gibi durum yazmaçlarını kaydet.
    // 4. Tuzak tipini (TT alanından) al.
    // 5. Yüksek seviyeli Rust işleyicisine (handle_sparc_trap) tuzak çerçevesi pointer'ını ve tuzak tipini argüman olarak geçirerek çağrı yap.
    // 6. Rust işleyiciden döndükten sonra, tuzak çerçevesinden kayıtlı registerları geri yükle.
    // 7. RETT (Return from Trap) komutu ile kullanıcı moduna geri dön.

    asm!(
        // Stack'te tuzak çerçevesi için yer ayır. %sp (stack pointer) 16-byte hizalı olmalı.
        "sub %sp, {trap_frame_size}, %sp",

        // Global registerları kaydet
        "st %g0, [%sp + {g_regs_offset} + {reg_size}*0]", // %g0 always 0
        "st %g1, [%sp + {g_regs_offset} + {reg_size}*1]",
        "st %g2, [%sp + {g_regs_offset} + {reg_size}*2]",
        "st %g3, [%sp + {g_regs_offset} + {reg_size}*3]",
        "st %g4, [%sp + {g_regs_offset} + {reg_size}*4]",
        "st %g5, [%sp + {g_regs_offset} + {reg_size}*5]",
        "st %g6, [%sp + {g_regs_offset} + {reg_size}*6]",
        "st %g7, [%sp + {g_regs_offset} + {reg_size}*7]",

        // Local registerları kaydet (%l0-%l7)
        "st %l0, [%sp + {l_regs_offset} + {reg_size}*0]",
        "st %l1, [%sp + {l_regs_offset} + {reg_size}*1]",
        "st %l2, [%sp + {l_regs_offset} + {reg_size}*2]",
        "st %l3, [%sp + {l_regs_offset} + {reg_size}*3]",
        "st %l4, [%sp + {l_regs_offset} + {reg_size}*4]",
        "st %l5, [%sp + {l_regs_offset} + {reg_size}*5]",
        "st %l6, [%sp + {l_regs_offset} + {reg_size}*6]",
        "st %l7, [%sp + {l_regs_offset} + {reg_size}*7]",

        // Incoming registerları kaydet (%i0-%i7)
        "st %i0, [%sp + {i_regs_offset} + {reg_size}*0]",
        "st %i1, [%sp + {i_regs_offset} + {reg_size}*1]",
        "st %i2, [%sp + {i_regs_offset} + {reg_size}*2]",
        "st %i3, [%sp + {i_regs_offset} + {reg_size}*3]",
        "st %i4, [%sp + {i_regs_offset} + {reg_size}*4]",
        "st %i5, [%sp + {i_regs_offset} + {reg_size}*5]",
        "st %i6, [%sp + {i_regs_offset} + {reg_size}*6]", // %i6 = Frame pointer (%fp)
        "st %i7, [%sp + {i_regs_offset} + {reg_size}*7]", // %i7 = Return address (%ra)

        // Durum yazmaçlarını kaydet
        // MRS/MSR komutları SPARC V9'da yaygın, V8'de farklı yollar olabilir (STBAR + ST%PSR vb.)
        // Basitlik için MSR benzeri pseudo-komutlar veya platforma özgü intrinsics kullanılabilir.
        // V8'de genellikle özel registerları okumak için %g0 kullanılır.
        // Gerçek V8 için `rd %psr, %g1` gibi komutlar kullanılır, sonra %g1 kaydedilir.
        // Aşağıdaki assembly sadece kavramsal placeholder'dır:

         "rd %psr, %g1", // PSR oku
         "st %g1, [%sp + {psr_offset}]",
         "rd %wim, %g1", // WIM oku
         "st %g1, [%sp + {wim_offset}]",
         "rd %tbr, %g1", // TBR oku
         "st %g1, [%sp + {tbr_offset}]", // TBR'nin TT alanı, trap tipini içerir

        // Alternatif olarak, tuzak tipini TBR'den kendimiz okuyup bir registera koyalım
        "rd %tbr, %g1", // TBR oku
        "and %g1, 0xff, %g1", // TT (Trap Type) alanını izole et (en düşük 8 bit)
        // %g1 şimdi trap tipini tutuyor, bunu Rust fonksiyonuna argüman olarak geçireceğiz.
        // %g1 zaten kaydediliyor.

        // TPC ve TNPC'yi kaydet (bu registerlar otomatik olarak trap frame'e yazılmaz, RD komutları gerekir)
         "rd %tpc, %g2", // TPC oku
         "st %g2, [%sp + {tpc_offset}]",
         "rd %tnpc, %g2", // TNPC oku
         "st %g2, [%sp + {tnpc_offset}]",

        // Y yazmacını kaydet
         "rd %y, %g3", // Y oku
         "st %g3, [%sp + {y_offset}]",

        // Yüksek seviyeli Rust işleyicisini çağır: handle_sparc_trap(%sp, trap_type)
        // İlk argüman (%o0 veya %i0) tuzak çerçevesi pointer'ı (%sp)
        // İkinci argüman (%o1 veya %i1) tuzak tipi (%g1)
        "mov %sp, %o0", // Trap frame pointer'ını %o0'a taşı (1. arg)
        "mov %g1, %o1", // Trap tipini %o1'e taşı (2. arg)
        "call handle_sparc_trap", // Rust fonksiyonunu çağır. %o registerları kaydetmeden çağrıldığına dikkat! (naked fn)
        "nop", // Delay slot

        // Rust işleyicisinden döndükten sonra registerları geri yükle (ters sırada)
        // Durum yazmaçlarını geri yükle (PSR, WIM, Y vb.)
         "ld [%sp + {y_offset}], %g1",
         "wr %g1, %y",
         "ld [%sp + {wim_offset}], %g1",
         "wr %g1, %wim",
         "ld [%sp + {psr_offset}], %g1",
         "wr %g1, %psr", // PSR'yi geri yüklemek tuzakları tekrar etkinleştirebilir

        // Program counter'ları geri yüklemeye gerek yok, RETT halleder.

        // Incoming registerları geri yükle (%i0-%i7)
        "ld [%sp + {i_regs_offset} + {reg_size}*0], %i0",
        "ld [%sp + {i_regs_offset} + {reg_size}*1], %i1",
        "ld [%sp + {i_regs_offset} + {reg_size}*2], %i2",
        "ld [%sp + {i_regs_offset} + {reg_size}*3], %i3",
        "ld [%sp + {i_regs_offset} + {reg_size}*4], %i4",
        "ld [%sp + {i_regs_offset} + {reg_size}*5], %i5",
        "ld [%sp + {i_regs_offset} + {reg_size}*6], %i6",
        "ld [%sp + {i_regs_offset} + {reg_size}*7], %i7",

        // Local registerları geri yükle (%l0-%l7)
        "ld [%sp + {l_regs_offset} + {reg_size}*0], %l0",
        "ld [%sp + {l_regs_offset} + {reg_size}*1], %l1",
        "ld [%sp + {l_regs_offset} + {reg_size}*2], %l2",
        "ld [%sp + {l_regs_offset} + {reg_size}*3], %l3",
        "ld [%sp + {l_regs_offset} + {reg_size}*4], %l4",
        "ld [%sp + {l_regs_offset} + {reg_size}*5], %l5",
        "ld [%sp + {l_regs_offset} + {reg_size}*6], %l6",
        "ld [%sp + {l_regs_offset} + {reg_size}*7], %l7",

        // Global registerları geri yükle (%g1-%g7), %g0 her zaman 0
        "ld [%sp + {g_regs_offset} + {reg_size}*1], %g1",
        "ld [%sp + {g_regs_offset} + {reg_size}*2], %g2",
        "ld [%sp + {g_regs_offset} + {reg_size}*3], %g3",
        "ld [%sp + {g_regs_offset} + {reg_size}*4], %g4",
        "ld [%sp + {g_regs_offset} + {reg_size}*5], %g5",
        "ld [%sp + {g_regs_offset} + {reg_size}*6], %g6",
        "ld [%sp + {g_regs_offset} + {reg_size}*7], %g7",


        // Stack'teki tuzak çerçevesi alanını boşalt
        "add %sp, {trap_frame_size}, %sp",

        // Tuzaktan geri dön
        "rett %tnpc + 4", // TNPC'den 4 byte ilerisine dallan (delay slotu atla)
        "nop", // Delay slot

        // Assembly sabitlerini Rust sabitleriyle senkronize et
        trap_frame_size = const TRAP_FRAME_SIZE,
        reg_size = const 4, // u32 boyutu
        g_regs_offset = const G_REGS_OFFSET,
        l_regs_offset = const L_REGS_OFFSET,
        i_regs_offset = const I_REGS_OFFSET,
        psr_offset = const PSR_OFFSET,
        wim_offset = const WIM_OFFSET,
        tbr_offset = const TBR_OFFSET,
        y_offset = const Y_OFFSET,
        tpc_offset = const TPC_OFFSET,
        tnpc_offset = const TNPC_OFFSET,

        options(noreturn) // Bu assembly bloğu geri dönmez
    )
}


/// Yüksek seviyeli Rust tuzak işleyicisi dağıtıcısı.
/// Assembly giriş noktasından çağrılır.
/// Gelen tuzak çerçevesini ve tipini alarak uygun handler'a yönlendirir.
#[no_mangle]
unsafe extern "C" fn handle_sparc_trap(trap_frame_ptr: *mut TrapFrame, trap_type: u32) {
    // Güvenlik: trap_frame_ptr'nin geçerli bir çekirdek alanı adresi olduğu varsayılır.
    // Gerçek bir sistemde bu varsayımın doğruluğu assembly tarafından garanti edilmelidir.
    let trap_frame = &mut *trap_frame_ptr;

    // Tuzak tipine göre uygun handler'a dallan
    match trap_type {
        SPARC_TRAP_TYPE_SYSCALL => {
            // Sistem çağrısı
            handle_sparc_syscall(trap_frame);
        }
        SPARC_TRAP_TYPE_PAGE_FAULT_DATA | SPARC_TRAP_TYPE_PAGE_FAULT_INSTR => {
            // Sayfa hatası (Data veya Instruction)
            // Hata adresini bulmak için SFSR/SFAR yazmaçlarına bakmak gerekebilir.
            // Basitlik için şimdilik sadece yazdırıp hata verelim.
            // Gerçekte kmemory modülündeki bir fonksiyona yönlendirilmeli.
             let fault_addr = /* SFAR yazmacından oku */;
            println!("Karnal64: Sayfa Hatası! Tip: {}, PC: {:#x}", trap_type, trap_frame.tpc);
             kmemory_handle_page_fault(trap_frame_ptr, fault_addr).expect("Page fault handling failed");
            kernel_panic("Sayfa hatası işlenemedi!"); // Geçici olarak panic
        }
        SPARC_TRAP_TYPE_ALIGNMENT => {
            // Hizalama hatası
            println!("Karnal64: Hizalama Hatası! PC: {:#x}", trap_frame.tpc);
            kernel_panic("Hizalama hatası!");
        }
        SPARC_TRAP_TYPE_ILLEGAL_INSTR => {
            // Geçersiz komut
            println!("Karnal64: Geçersiz Komut! PC: {:#x}", trap_frame.tpc);
            kernel_panic("Geçersiz komut!");
        }
        SPARC_TRAP_TYPE_DIVISION_BY_ZERO => {
             // Sıfıra bölme
            println!("Karnal64: Sıfıra Bölme Hatası! PC: {:#x}", trap_frame.tpc);
            kernel_panic("Sıfıra bölme!");
        }
        // Diğer tuzak tipleri...
        _ => {
            // Bilinmeyen veya işlenmeyen tuzak tipi
            println!("Karnal64: Bilinmeyen Tuzak! Tip: {:#x}, PC: {:#x}", trap_type, trap_frame.tpc);
            kernel_panic("İşlenmeyen tuzak!");
        }
    }
}

/// Sistem çağrısı tuzağını işleyen fonksiyon.
/// Tuzak çerçevesinden sistem çağrısı numarasını ve argümanlarını alır,
/// Karnal64'teki handle_syscall fonksiyonunu çağırır ve sonucu kullanıcı registerlarına yazar.
unsafe fn handle_sparc_syscall(trap_frame: &mut TrapFrame) {
    // SPARC ABI konvansiyonuna göre sistem çağrısı numarası genellikle %g1 registerındadır.
    // Argümanlar ise %o0 - %o5 registerlarındadır, bunlar tuzak anında handler'ın %l0 - %l5'i olur.
    let syscall_number = trap_frame.g_regs[1] as u64; // %g1

    let arg1 = trap_frame.l_regs[0] as u64; // %o0 (şimdi %l0)
    let arg2 = trap_frame.l_regs[1] as u64; // %o1 (şimdi %l1)
    let arg3 = trap_frame.l_regs[2] as u64; // %o2 (şimdi %l2)
    let arg4 = trap_frame.l_regs[3] as u64; // %o3 (şimdi %l3)
    let arg5 = trap_frame.l_regs[4] as u64; // %o4 (şimdi %l4)
    // SPARC V8'de %o5 (şimdi %l5) 6. argüman olabilir.

    // Karnal64'teki ana sistem çağrısı işleyicisini çağır.
    // Bu fonksiyon, kullanıcı pointer'larını (eğer argümanlar pointer ise) doğrulamalıdır.
    let syscall_result = handle_syscall(
        syscall_number,
        arg1,
        arg2,
        arg3,
        arg4,
        arg5, // SPARC'ta 6 argüman yaygın
    );

    // Sistem çağrısı sonucunu kullanıcı alanındaki return register'ına (%o0) yaz.
    // Bu, trap handler'dan dönüldüğünde kullanıcının %o0 registerına yazılır.
    trap_frame.l_regs[0] = syscall_result as u32; // Sonucu %o0'a (şimdi %l0) koy
    // Bazı ABIs ikinci bir return değeri için %o1'i de kullanabilir, bu konvansiyona bağlıdır.

    // TPC ve TNPC'yi ilerlet. Sistem çağrısı komutu genellikle 4 byte'tır.
    // RETT %tnpc + 4 veya benzeri bir komut zaten bunu yapacağı için burada explicit ayarlama
    // gerekmeyebilir, ancak bazı karmaşık tuzaklarda gerekebilir.
    // Eğer tuzak komutu delay slot kullanıyorsa TPC+4'e değil, TNPC'ye dallanmak gerekir.
    // Bizim assembly RETT %tnpc + 4 kullandığı için PC otomatik ilerler.
    // Eğer handle_syscall gibi bir fonksiyon görev değiştirdiyse (schedule ettiyse),
    // geri dönüldüğünde farklı bir görev bağlamında ve TPC/TNPC değerleriyle dönülür.
    // Bu durumda trap_frame'deki TPC/TNPC zaten scheduler tarafından güncellenmiş olmalıdır
    // veya rett komutu yeni görevin TPC/TNPC'sini kullanmalıdır.
    // Basitlik için burada explicit bir PC ilerletme yapmıyoruz.
}

/// SPARC tuzak vektör tablosunu başlatır.
/// TBR (Trap Base Register) ayarlanır ve her tuzak tipi için handler giriş noktaları ayarlanır.
/// Bu fonksiyon boot sırasında çekirdek başlatılırken çağrılmalıdır.
pub fn init_trap_vector_table() {
    // TODO: Tuzak vektör tablosu için bellek ayır ve haritala (kernel alanında).
    // Bu genellikle `kmemory` modülü kullanılarak yapılır.
    // let trap_table_base = kmemory::allocate_kernel_page() as *mut u32;
    let trap_table_base = 0xFFFF_0000 as *mut u32; // Örnek sabit adres, gerçekte dinamik olmalı.
    let handler_entry_addr = trap_handler_entry as *const () as u32;

    // Tuzak vektör tablosunu doldur.
    // Her giriş (16 byte) genellikle bir `sethi` ve bir `jmp` komutundan oluşur.
    // Bu komutlar, ilgili tuzak tipinin handler'ına dallanır.
    // Bizim durumumuzda, tüm tuzaklar için ortak `trap_handler_entry`'ye dallanacağız.

    // Her vektör girişi için (0'dan 255'e kadar tuzak tipi):
     0x0000 -- sethi %hi(handler_entry), %g1
     0x0004 -- jmp %g1 + %lo(handler_entry)
     0x0008 -- nop (delay slot)
     0x000c -- nop

    for i in 0..256 {
        let vector_addr = unsafe { trap_table_base.add(i * 16 / 4) }; // Her giriş 16 byte = 4 kelime (u32)
        let handler_hi = (handler_entry_addr >> 10) & 0x3FFFFF;
        let handler_lo = handler_entry_addr & 0x3FF;

        unsafe {
            // sethi %hi(handler_entry), %g1
            ptr::write_volatile(vector_addr.add(0), 0x13000000 | handler_hi); // 0x13 = sethi opcode, %g1 = dest reg

            // jmp %g1 + %lo(handler_entry)
            ptr::write_volatile(vector_addr.add(1), 0x81C06000 | handler_lo); // 0x81C06000 = jmp %g1 + %lo(0)

            // NOPs for delay slot
            ptr::write_volatile(vector_addr.add(2), 0x01000000); // nop
            ptr::write_volatile(vector_addr.add(3), 0x01000000); // nop
        }
    }

    // TBR (Trap Base Register) yazmacını ayarla.
    // Bu, CPU'ya tuzak tablosunun nerede olduğunu söyler.
    let tbr_value = trap_table_base as u32;
    unsafe {
        // MSR/WR komutları platforma özgü olabilir.
        // V8'de WR %tbr, %g0, value kullanılır.
         asm!(
             "wr {tbr_value}, 0, %tbr",
             tbr_value = in(reg) tbr_value,
             options(nostack, nomem)
         );
    }

    // PSR (Processor State Register) içindeki ET (Enable Traps) bitini set et.
    // Bu, tuzakların oluşmasına izin verir. Çekirdek tamamen hazır olmadan bu bit set edilmemelidir!
    unsafe {
        let mut psr: u32;
        asm!("rd %psr, {psr}", psr = out(reg) psr);
        psr |= 0x00000004; // ET bitini set et (genellikle 2. bit)
        asm!("wr {psr}, 0, %psr", psr = in(reg) psr);
    }

    println!("Karnal64: SPARC Tuzak Vektör Tablosu Başlatıldı."); // Kernel print! fonksiyonu gerektirir
}

/// Basit bir çekirdek panik fonksiyonu.
/// Ciddi bir hata durumunda sistemi durdurur.
fn kernel_panic(message: &str) -> ! {
    println!("Karnal64 PANIC: {}", message);
    // TODO: Daha gelişmiş panik handler (stack trace, debug info, halt CPU)
    loop {
        // CPU'yu durdurmak için assembly komutu (örneğin SPARC'ta `halt` veya sonsuz döngü)
        unsafe { asm!("ta 0", options(noreturn)) }; // `ta 0` (Trap Always) bazen debugger/emulator tarafından yakalanabilir.
    }
}

// TODO: Başka helper fonksiyonlar (örneğin kullanıcı pointerlarını doğrulamak için)
// Bu doğrulama mantığı handle_syscall içinde veya syscall argümanları işlenmeden önce yapılmalıdır.
 unsafe fn validate_user_pointer<T>(ptr: *const T, len: usize, writable: bool) -> Result<(), KError> {
//    // Görevin sanal adres alanında ptr[0..len] aralığının geçerli ve erişilebilir (okuma/yazma)
//    // olduğunu MMU/Bellek Yöneticisi üzerinden kontrol et.
     let task = ktask::current_task();
     task.validate_address_range(...)
    Ok(()) // Placeholder
 }
