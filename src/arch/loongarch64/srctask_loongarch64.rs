#![no_std] // Bu da bir çekirdek bileşeni olduğu için standart kütüphaneye ihtiyaç duymaz.
#![allow(dead_code)] // Geliştirme sırasında kullanılmayan kodlar için izin
#![allow(unused_variables)] // Geliştirme sırasında kullanılmayan değişkenler için izin

// Karnal64 API'sından temel tipleri içe aktar.
// Varsayım: Karnal64.rs'teki ilgili tipler 'pub' olarak işaretlenmiştir.
// 'super::' kullanarak aynı crate içindeki üst modüldeki (varsayımsal olarak karnal64.rs) öğelere erişiyoruz.
use super::KError;
use super::KTaskId;
use super::KThreadId;
use super::KHandle;

// TODO: LoongArch 64-bit mimarisine özgü register setini tanımla.
// Bu yapı, bir iş parçacığının veya görevin kaydedilmesi/geri yüklenmesi gereken
// CPU registerlarının tamamını veya bir kısmını (genellikle kaydedici-çağıran anlaşmasına göre) saklar.
// Tam register seti ve bunların bellekteki düzeni, LoongArch 64-bit ABI (Application Binary Interface) dokümantasyonuna göre doğru şekilde doldurulmalıdır.
#[repr(C)] // C uyumluluğu, düşük seviye assembly kodları ile etkileşimde önemlidir.
#[derive(Debug, Copy, Clone)]
pub struct LoongArchContext {
    // --- Genel Amaçlı Registerlar (GPRs) ---
    // LoongArch'ın 32 adet 64-bit genel amaçlı registerı (r0-r31).
    // Genellikle r0 sabittir (sıfır), r1 stack pointer (sp) vb.
    // Bağlam değiştirme sırasında hangilerinin kaydedilip hangilerinin kaydedilmeyeceği ABI'ye bağlıdır.
    // Çoğu OS çekirdeği, istisna/sistem çağrısı sırasında CPU tarafından kaydedilenlere ek olarak,
    // zamanlama kesmelerinde tüm GPR'ları kaydetmeyi tercih eder.
    gprs: [u64; 32], // r0-r31 için yer tutucu

    // --- Kontrol ve Durum Registerları (CSRs) ---
    // Görev/iş parçacığına özgü olması gereken kritik CSR'lar.
    csr_era: u64,  // Exception Return Address - İstisna dönüş adresi (sistem çağrısı sonrası veya bağlam değiştirme sonrası devam edilecek adres)
    csr_crmd: u64, // Current Mode - Mevcut işlem modu (Kernel/User) ve ilgili bayraklar
    // TODO: LoongArch'a özgü kaydedilmesi gereken diğer kritik CSR'lar (örneğin, MMU ile ilgili, kesmelerle ilgili durumlar)

    // --- FPU/SIMD Registerları (Eğer kullanılıyorsa) ---
    // Eğer görevler kayan nokta veya SIMD talimatları kullanıyorsa, bu registerlar da kaydedilmelidir.
    // LoongArch'ın VPU (Vector Processing Unit) registerları.
     vregs: [u128; 32], // vr0-vr31 için yer tutucu (tam tip ve boyut LoongArch'a göre belirlenmeli)
     fcsr: u32,        // Floating-point Control and Status Register

    // TODO: Diğer mimariye özgü durumlar veya registerlar
}

impl LoongArchContext {
    /// Yeni bir görev/iş parçacığı için başlangıç yürütme bağlamını (context) oluşturur.
    /// Bu bağlam, görev ilk kez zamanlandığında CPU'ya yüklenecek register değerlerini içerir.
    ///
    /// # Arguments
    /// * `entry_point`: Görevin çalışmaya başlayacağı sanal adres.
    /// * `stack_top`: Görevin kullanacağı kullanıcı stack'inin en üst adresi (stack genellikle yukarıdan aşağı doğru büyür).
    /// * `arg`: Göreve (genellikle ilk argüman olarak) geçirilecek 64-bitlik değer.
    /// * `is_user`: Bu bağlamın kullanıcı modunda mı (true) yoksa çekirdek modunda mı (false) çalışacağını belirtir.
    pub fn create_initial_context(
        entry_point: u64, // LoongArch'ta adres büyüklüğü neyse o kullanılmalı (u64 varsayımı)
        stack_top: u64,
        arg: u64,
        is_user: bool,
    ) -> Self {
        // TODO: LoongArch ABI'sine göre başlangıç register değerlerini ayarla.
        let mut context = Self {
            gprs: [0; 32], // Tüm GPR'ları sıfırla (veya ABI'ye göre uygun başlangıç değerlerini ver)
            csr_era: entry_point,
            csr_crmd: 0, // Varsayılan olarak sıfırla, sonra mod bayraklarını ayarla
            // Diğer CSR'ları sıfırla veya başlangıç değerlerini ver
        };

        // Stack Pointer (sp) genellikle r3 veya r4 olarak kullanılır, ABI'ye bakılmalı. Varsayımsal olarak r3.
        // LoongArch ABI'ye göre sp'nin ilk değeri ayarlanmalı. Stack top'ı kullanıyoruz.
        if let Some(sp_reg_idx) = LoongArchContext::get_sp_register_index() {
             context.gprs[sp_reg_idx] = stack_top;
        } else {
             // TODO: SP register indeksi bulunamadı, hata durumu veya panic yönetimi
             panic!("LoongArch SP register index not defined!");
        }


        // Göreve geçirilecek argüman genellikle ilk argüman registerına konur (örn. r4 veya r5, ABI'ye bakılmalı). Varsayımsal olarak r4.
        if let Some(arg0_reg_idx) = LoongArchContext::get_arg0_register_index() {
             context.gprs[arg0_reg_idx] = arg;
        } else {
             // TODO: Arg0 register indeksi bulunamadı, hata durumu veya panic yönetimi
        }


        // Dönüş adresi (ra) genellikle r1 olarak kullanılır, ABI'ye bakılmalı.
        // Görev ana fonksiyonundan dönüldüğünde çalışacak bir çekirdek fonksiyonunun adresi buraya konulmalıdır
        // (örneğin görev sonlandırma işleyicisi).
        if let Some(ra_reg_idx) = LoongArchContext::get_ra_register_index() {
            // TODO: Gerçek görev sonlandırma çekirdek fonksiyonunun adresini al
            let task_exit_kernel_handler_address: u64 = 0xTODO_GET_TASK_EXIT_HANDLER_ADDRESS;
            context.gprs[ra_reg_idx] = task_exit_kernel_handler_address;
        } else {
             // TODO: RA register indeksi bulunamadı
        }


        // Ayrıcalık seviyesini ayarla (CSR_CRMD veya benzeri register).
        // LoongArch'ta kullanıcı modu ve çekirdek modu için uygun bitleri ayarla.
        // TODO: LoongArch CSR_CRMD (veya ilgili) registerındaki kullanıcı/çekirdek mod bayraklarını ayarla
        if is_user {
            // Kullanıcı modu bayraklarını ayarla
             context.csr_crmd |= LOONGARCH_CRMD_USER_MODE_BITS; // Yer tutucu
        } else {
            // Çekirdek modu bayraklarını ayarla (genellikle zaten çekirdek modunda başlarız)
             context.csr_crmd |= LOONGARCH_CRMD_KERNEL_MODE_BITS; // Yer tutucu
        }

        // TODO: Kesme durumunu ayarla (genellikle başlangıçta kesmeler kapalı olmalı)
         context.csr_crmd &= !LOONGARCH_CRMD_INTERRUPT_ENABLE_BIT; // Yer tutucu

        // TODO: Eğer FPU/VPU kullanılıyorsa, ilgili durum registerlarını ayarla

        context
    }

    /// LoongArch ABI'ye göre SP register indeksini döndürür (Yer Tutucu).
    #[inline(always)]
    fn get_sp_register_index() -> Option<usize> {
        // TODO: LoongArch ABI'ye göre SP register indeksini (0-31 arası) döndür.
        // LoongArch Dokümantasyonuna bakın (genellikle r3 veya r4).
         Some(3) // Örnek olarak 3 diyelim, doğru indeksi kontrol edin.
    }

     /// LoongArch ABI'ye göre RA register indeksini döndürür (Yer Tutucu).
     #[inline(always)]
     fn get_ra_register_index() -> Option<usize> {
         // TODO: LoongArch ABI'ye göre RA register indeksini (0-31 arası) döndür.
         // LoongArch Dokümantasyonuna bakın (genellikle r1).
          Some(1) // Örnek olarak 1 diyelim, doğru indeksi kontrol edin.
     }

     /// LoongArch ABI'ye göre ilk argüman register indeksini döndürür (Yer Tutucu).
     #[inline(always)]
     fn get_arg0_register_index() -> Option<usize> {
         // TODO: LoongArch ABI'ye göre ilk argüman register indeksini (0-31 arası) döndür.
         // LoongArch Dokümantasyonuna bakın (genellikle r4 veya r5).
          Some(4) // Örnek olarak 4 diyelim, doğru indeksi kontrol edin.
     }


    // TODO: Gerçek bağlam değiştirme assembly fonksiyonunu dışarıdan tanımla.
    // Bu fonksiyon, bu dosyanın yanında veya ayrı bir assembly dosyasında (örn. `loongarch_asm.S`) implemente edilir.
    // Parametreleri, mevcut bağlam yapısının pointer'ı ve geçiş yapılacak bağlam yapısının pointer'ı olmalıdır.
    // Fonksiyon, mevcut bağlamı ilk parametredeki adrese kaydetmeli, ikinci parametredeki adresten yeni bağlamı yüklemeli
    // ve yeni bağlamın `csr_era` adresine atlamalıdır.
    extern "C" {
        /// LoongArch'a özgü düşük seviye bağlam değiştirme fonksiyonu.
        /// Mevcut bağlamı `current` pointer'ına kaydeder, `next` pointer'ından yeni bağlamı yükler
        /// ve yeni bağlama atlar.
        ///
        /// # Safety
        /// Bu fonksiyonun çağrılması **güvenli değildir** ve sadece çekirdek zamanlayıcısı gibi
        /// düşük seviye kodlardan, sağlanan pointer'ların geçerli `LoongArchContext` yapılarına
        /// işaret ettiği garanti edildiğinde çağrılmalıdır.
        fn loongarch_context_switch(current: *mut LoongArchContext, next: *const LoongArchContext);

        /// LoongArch'a özgü düşük seviye kullanıcı moduna geçiş fonksiyonu.
        /// Çekirdek modundan kullanıcı moduna ilk geçişi yapar.
        /// Verilen bağlamın stack'ini, entry point'ini ve ayrıcalık seviyesini kurar ve o adrese atlar.
        /// Bu fonksiyon geri dönmez (`!` dönüş tipi).
        ///
        /// # Safety
        /// Bu fonksiyonun çağrılması **güvenli değildir** ve sadece çekirdek görev başlatma
        /// kodu gibi düşük seviye kodlardan çağrılmalıdır. Bağlamın doğru kurulduğundan,
        /// MMU'nun doğru ayarlandığından (eğer sanal adres kullanılıyorsa) emin olunmalıdır.
        fn loongarch_enter_user_mode(initial_context: *const LoongArchContext) -> !;
    }
}


// --- Çekirdek Ktask Modülü Tarafından Kullanılan LoongArch Spesifik Fonksiyonlar ---
// Bu fonksiyonlar, karnal64.rs içindeki ktask modülünün (veya genel görev yöneticisinin)
// mimariye bağımlı işlemleri gerçekleştirmek için çağıracağı arayüzü oluşturur.

/// Yeni bir LoongArch görev/iş parçacığı için başlangıç bağlamını hazırlar.
/// Genel görev yöneticisi bu fonksiyonu çağırır.
///
/// # Arguments
/// * `entry_point`: Görev kodunun başlayacağı sanal adres.
/// * `stack_base`: Görev stack'inin ayrıldığı bellek alanının başlangıcı.
/// * `stack_size`: Görev stack'inin boyutu.
/// * `arg`: Göreve geçirilecek argüman.
/// * `is_user`: Kullanıcı (true) veya çekirdek (false) görevi mi olduğunu belirtir.
///
/// # Returns
/// Başarılı olursa ilk `LoongArchContext` yapısını, hata durumunda `KError` döner.
pub fn arch_create_task_context(
    entry_point: u64,
    stack_base: u64,
    stack_size: usize,
    arg: u64,
    is_user: bool,
) -> Result<LoongArchContext, KError> {
    // TODO: stack_base ve stack_size'ın geçerli bellek alanlarını temsil ettiğini doğrula?
    // Bu doğrulama genellikle bellek yöneticisi (kmemory) veya daha üst katmanda yapılır.
    // TODO: entry_point adresinin geçerli ve yürütülebilir bir alanda olduğunu doğrula?

    let stack_top = stack_base.checked_add(stack_size as u64) // Stack yukarıdan aşağı büyüyorsa son adresi
        .ok_or(KError::InvalidArgument)?; // Taşma kontrolü

    // Stack pointer genellikle stack top'a veya biraz altına ayarlanır.
    // ABI'ye göre tam başlangıç değeri belirlenmelidir.
    // LoongArchContext::create_initial_context stack_top'ı bekliyor.

    Ok(LoongArchContext::create_initial_context(entry_point, stack_top, arg, is_user))
}

/// İki LoongArch görevi/iş parçacığı arasında bağlam değiştirmeyi gerçekleştirir.
/// Genel zamanlayıcı tarafından, hangi görevin durdurulup hangisinin çalıştırılacağına karar verildikten sonra çağrılır.
///
/// # Arguments
/// * `current_context`: Mevcut (ayrılacak) görevin `LoongArchContext` yapısının mutable pointer'ı.
///                                                    Mevcut görev durdurulduğunda registerlar bu adrese kaydedilecektir.
/// * `next_context`: Çalıştırılacak görevin `LoongArchContext` yapısının immutable pointer'ı.
///                                                    Yeni görev başladığında registerlar buradan yüklenecektir.
///
/// # Safety
/// Bu fonksiyon **güvenli değildir**. Çağıran kod, sağlanan pointer'ların geçerli,
/// tahsis edilmiş ve doğru `LoongArchContext` yapılarına işaret ettiğinden ve bu yapılara
/// eş zamanlı erişim sorunları olmayacağından emin olmalıdır (genellikle zamanlayıcı kilitleri altında çalışır).
#[inline] // Bağlam değiştirme kritik yol olduğu için inlining faydalı olabilir
pub unsafe fn arch_switch_context(
    current_context: *mut LoongArchContext,
    next_context: *const LoongArchContext,
) {
    // TODO: Düşük seviye assembly bağlam değiştirme fonksiyonunu çağır.
    // loongarch_context_switch fonksiyonu ayrı bir assembly dosyasında implemente edilmelidir.
    // Bu çağrı, mevcut görev bağlamını kaydeder ve yeni görev bağlamını yükleyip yeni göreve atlar.
    loongarch_context_switch(current_context, next_context);

    // Bu noktaya sadece bir başka görev (daha önce durdurulan 'current_context' görevi)
    // bu fonksiyona yapılan bir çağrı ile geri döndüğünde ulaşılır.
}

// TODO: İlk görev başlatılırken kullanılan özel fonksiyon.
// Normal bağlam değiştirmeden farklı olabilir çünkü çekirdek stack'inden
// kullanıcı stack'ine ve çekirdek modundan kullanıcı moduna geçiş içerir.
/// İlk kez bir görevi çalıştırmak için LoongArch donanım bağlamını kurar ve kullanıcı moduna geçer.
/// Bu fonksiyon çekirdek zamanlayıcısı tarafından, ilk defa bir kullanıcı görevini başlatırken çağrılır.
///
/// # Arguments
/// * `initial_context`: Çalıştırılacak görevin `arch_create_task_context` ile hazırlanmış ilk bağlam yapısı.
///
/// # Safety
/// Bu fonksiyon **güvenli değildir**. Çekirdekten kullanıcı alanına son derece dikkatli bir geçiş yapar.
/// Bağlam yapısının doğruluğu, MMU ayarları (varsa), ayrıcalık seviyesinin doğru ayarlanması ve
/// kesme/istisna vektörlerinin doğru kurulmuş olması kritiktir. Yanlış implementasyonlar güvenlik açıklarına yol açar.
pub unsafe fn arch_enter_user_mode(
    initial_context: LoongArchContext // Bağlamı değer olarak alıyoruz, çünkü assembly fonksiyonuna pointer'ını vereceğiz
) -> ! // Bu fonksiyon başarılı olursa geri dönmez
{
    // TODO: Düşük seviye assembly kullanıcı moduna geçiş fonksiyonunu çağır.
    // loongarch_enter_user_mode fonksiyonu ayrı bir assembly dosyasında implemente edilmelidir.
    // Bu fonksiyon, sağlanan bağlamdaki stack, entry point ve CSR'ları kullanarak
    // kullanıcı moduna geçişi ve ilk talimata atlamayı yapar.
    loongarch_enter_user_mode(&initial_context as *const LoongArchContext);

    // Bu noktaya asla ulaşılmamalıdır. Eğer ulaşılırsa bir hata var demektir.
     panic!("arch_enter_user_mode returned!"); // Geliştirme sırasında hata ayıklama için eklenebilir
}


// TODO: Diğer LoongArch spesifik task/thread yönetimi fonksiyonları (gereksinime göre eklenebilir)
// - `arch_get_current_thread_id() -> KThreadId`: Mevcut LoongArch Thread ID'sini döndürür.
// - `arch_setup_kernel_thread_context(...) -> LoongArchContext`: Çekirdek modunda çalışan bir iş parçacığı için bağlam hazırlar.
// - `arch_setup_syscall_stack(...)`: Sistem çağrısı sırasında kullanıcı stack'inin çekirdek stack'ine nasıl kaydedileceğini veya erişileceğini yönetir.
// - `arch_restore_syscall_stack(...)`: Sistem çağrısı dönüşünde stack'i geri yükler.
// - `arch_yield()`: Mevcut iş parçacığının bilerek CPU'yu bırakmasını sağlar (zamanlayıcıyı çağırır).
// - Kesme işleyicileri içinde bağlam kaydetme/geri yükleme fonksiyonları.

// --- Yardımcı Sabitler (LoongArch ABI'ye göre doldurulmalı) ---
// TODO: LoongArch CRMD registerındaki bit bayrakları gibi mimariye özgü sabitler
 pub const LOONGARCH_CRMD_USER_MODE_BITS: u64 = ...;
 pub const LOONGARCH_CRMD_KERNEL_MODE_BITS: u64 = ...;
 pub const LOONGARCH_CRMD_INTERRUPT_ENABLE_BIT: u64 = ...;
