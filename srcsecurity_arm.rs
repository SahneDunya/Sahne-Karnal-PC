#![no_std]

use core::arch::asm;

// ARM Cortex-A MPU yapılandırma sabitleri (genel ve örnek değerler - CPU'ya özel kılavuza bakılmalıdır)

// MPU_TYPE Register - MPU'nun özelliklerini gösterir (bölgelerin sayısı vb.) - *Genellikle sadece okuma amaçlıdır*
// const MPU_TYPE_REG: u32 = 0xE000ED90; // Örnek adres - CPU kılavuzundan doğrulanmalı

// MPU Kontrol Kaydı (MPU_CTRL) - MPU'yu etkinleştirme/devre dışı bırakma, varsayılan bellek eşlemesi vb.
const MPU_CTRL_REG: u32 = 0xE000ED94; // Örnek adres - CPU kılavuzundan doğrulanmalı
const MPU_CTRL_ENABLE: u32 = 0b1; // MPU'yu etkinleştir
const MPU_CTRL_DISABLE: u32 = 0b0; // MPU'yu devre dışı bırak
const MPU_CTRL_PRIVDEFENA_ENABLE: u32 = 0b100; // Ayrıcalıklı varsayılan eşlemeyi etkinleştir (isteğe bağlı)
const MPU_CTRL_HFNMIENA_ENABLE: u32 = 0b1000; // HardFault, NMI için MPU'yu etkinleştir (isteğe bağlı)


// MPU Bölge Sayısı Kaydı (MPU_RNR) - Yapılandırılacak bölgeyi seçer
const MPU_RNR_REG: u32 = 0xE000ED98; // Örnek adres - CPU kılavuzundan doğrulanmalı

// MPU Bölge Başlangıç Adresi Kaydı (MPU_RBAR) - Bölge başlangıç adresini ve RO bitini (Salt Okunur erişim özniteliği - isteğe bağlı) ayarlar
const MPU_RBAR_REG: u32 = 0xE000ED9C; // Örnek adres - CPU kılavuzundan doğrulanmalı
// RO biti için sabit (MPU_RBAR'da, örneğin bit 1) - *CPU'ya özel kılavuza bakılmalı*
// const MPU_RBAR_RO_BIT: u32 = 0b10; // Örnek değer - CPU kılavuzundan doğrulanmalı

// MPU Bölge Limit Adresi Kaydı (MPU_RLAR) - Bölge bitiş adresini/limitini ve bölge boyutunu ayarlar, erişim özniteliklerini (örn. Tam Erişim) içerir
const MPU_RLAR_REG: u32 = 0xE000EDA0; // Örnek adres - CPU kılavuzundan doğrulanmalı

// Erişim İzinleri (AP - Access Permissions) için sabitler - *CPU mimarisine ve MPU sürümüne göre değişebilir*
const MPU_RLAR_AP_NONE: u32 = 0b000 << 1; // Erişim Yok (örn., kullanıcı ve ayrıcalıklı erişim yok) - Kaydırılmış örnek değer!
const MPU_RLAR_AP_PRIV_RW_USER_NONE: u32 = 0b001 << 1; // Ayrıcalıklı RW, Kullanıcı Erişim Yok - Kaydırılmış örnek değer!
const MPU_RLAR_AP_PRIV_RW_USER_RO: u32 = 0b010 << 1;  // Ayrıcalıklı RW, Kullanıcı RO - Kaydırılmış örnek değer!
const MPU_RLAR_AP_FULL_ACCESS: u32 =  0b011 << 1; // Tam Erişim (Ayrıcalıklı ve Kullanıcı RW) - Kaydırılmış örnek değer!
const MPU_RLAR_AP_PRIV_RO_USER_NONE: u32 = 0b101 << 1; // Ayrıcalıklı RO, Kullanıcı Erişim Yok - Kaydırılmış örnek değer!
const MPU_RLAR_AP_PRIV_RO_USER_RO: u32 = 0b110 << 1;  // Ayrıcalıklı RO, Kullanıcı RO - Kaydırılmış örnek değer!
// ... Diğer erişim izinleri kombinasyonları - ARM Cortex-A mimari kılavuzuna bakılmalı

const MPU_RLAR_REGION_ENABLE: u32 = 0b1; // Bölgeyi etkinleştirme biti (MPU_RLAR'da, örneğin bit 0)


// *ÖNEMLİ*: Yukarıdaki adresler ve sabit değerler *örnektir* ve genel bir Cortex-A işlemci içindir.
// *CPU'NUZA ÖZEL TEKNİK REFERANS KILAVUZUNA BAŞVURUN* ve doğru kayıt adreslerini,
// bit alanlarını ve yapılandırma seçeneklerini ORADAN kontrol edin. Farklı Cortex-A çekirdekleri ve
//  üreticiye özel uygulamalar farklılık gösterebilir!



// MPU Bölgesini yapılandırmak için fonksiyon
// *Adreslerin ve yapılandırmanın CPU'ya ÖZEL KILAVUZDAN kontrol edilmesi ZORUNLUDUR*
pub fn configure_mpu_region(index: usize, start_addr: usize, end_addr: usize, access_permissions: u32) {
    if index >= 8 { // Örnek olarak 8 MPU bölgesi varsayımı - CPU kılavuzundan kontrol edilmeli!
        panic!("Geçersiz MPU bölge indeksi: {}", index);
    }

    if start_addr >= end_addr {
        panic!("Geçersiz adres aralığı: başlangıç adresi bitiş adresinden büyük veya eşit");
    }


    unsafe {
        // 1. MPU Bölge Numarası Kaydını (MPU_RNR) yapılandır
        // Hangi bölgenin yapılandırılacağını seçin
        asm!(
            "ldr r0, ={}",                  // Kayıt adresini r0'a yükle
            "mov r1, {}",                  // Bölge indeksini r1'e taşı
            "str r1, [r0]",                 // r1'i [r0]'daki adrese (MPU_RNR) yaz
            in(reg) MPU_RNR_REG,
            in(reg) index,
            options(nostack, nomem)
        );

        // 2. MPU Bölge Başlangıç Adresi Kaydını (MPU_RBAR) yapılandır
        // Bölge başlangıç adresini ayarla (ve isteğe bağlı RO biti)
        asm!(
            "ldr r0, ={}",                  // Kayıt adresini r0'a yükle
            "mov r1, {}",                  // Başlangıç adresini r1'e taşı
            "str r1, [r0]",                 // r1'i [r0]'daki adrese (MPU_RBAR) yaz
            in(reg) MPU_RBAR_REG,
            in(reg) start_addr,
            options(nostack, nomem)
        );

        // 3. MPU Bölge Limit Adresi Kaydını (MPU_RLAR) yapılandır
        // Bölge limit adresini, erişim izinlerini ve etkinleştirme bitini ayarla
        let rlar_value = end_addr | access_permissions | MPU_RLAR_REGION_ENABLE; // Bit alanlarını birleştir
        asm!(
            "ldr r0, ={}",                  // Kayıt adresini r0'a yükle
            "mov r1, {}",                  // RLAR değerini r1'e taşı (limit, izinler, etkinleştirme)
            "str r1, [r0]",                 // r1'i [r0]'daki adrese (MPU_RLAR) yaz
            in(reg) MPU_RLAR_REG,
            in(reg) rlar_value,
            options(nostack, nomem)
        );
    }
}


// MPU yapılandırmasını başlatmak için fonksiyon
pub fn init_mpu() {
    // *CPU'YA ÖZEL ADRESLER VE BOYUTLAR KULLANILMALIDIR!* - Aşağıdakiler sadece örnek değerlerdir!

    // Çekirdek (Kernel) bellek bölgesi tanımları - *ÖRNEK DEĞERLER*
    let kernel_start = 0x80000000;         // Örnek başlangıç adresi - *Doğrulanmalı*
    let kernel_size = 0x2000;           // Örnek boyut (8KB) - *Doğrulanmalı*
    let kernel_end = kernel_start + kernel_size -1; // Bitiş adresi (MPU limit adresi *dahildir*)

    // 1. MPU Bölgesi: Çekirdek kodu için (Salt Okunur ve Çalıştırma - RX benzeri etki için RO + çalıştırma izinleri *CPU'ya özel kılavuzdan kontrol edilmeli*)
    configure_mpu_region(
        0,                                  // MPU Bölge İndeksi 0
        kernel_start,                       // Bölge Başlangıç Adresi
        kernel_end,                         // Bölge Bitiş Adresi
        MPU_RLAR_AP_PRIV_RO_USER_RO         // Örnek: Ayrıcalıklı ve Kullanıcı Salt Okunur erişim - *UYGUN ERİŞİM İZİNLERİ CPU KILAVUZUNDAN KONTROL EDİLMELİ*
    );
    // Açıklama: MPU Bölge 0, kernel_start - kernel_end adres aralığını
    // Salt Okunur erişime izin verir (çekirdek kodu için), diğer erişim türlerini engeller (varsayıma göre - AP değerine bağlı).
    // *UYGUN AP DEĞERİ CPU KILAVUZUNDAN KONTROL EDİLMELİ*


    // Çekirdek yığını (Kernel Stack) bellek bölgesi tanımları - *ÖRNEK DEĞERLER*
    let kernel_stack_start = kernel_end + 1;    // Yığın başlangıcı, çekirdek kodundan hemen sonra - *Adres planına göre değişebilir*
    let kernel_stack_size = 0x1000;           // Örnek boyut (4KB) - *Doğrulanmalı*
    let kernel_stack_end = kernel_stack_start + kernel_stack_size -1; // Bitiş adresi


    // 2. MPU Bölgesi: Çekirdek yığını için (Okuma ve Yazma - RW)
    configure_mpu_region(
        1,                                  // MPU Bölge İndeksi 1
        kernel_stack_start,                 // Bölge Başlangıç Adresi
        kernel_stack_end,                   // Bölge Bitiş Adresi
        MPU_RLAR_AP_FULL_ACCESS            // Tam Erişim (Ayrıcalıklı ve Kullanıcı RW) - *Gerekli erişim düzeyine göre ayarlanabilir*
    );
    // Açıklama: MPU Bölge 1, kernel_stack_start - kernel_stack_end adres aralığını
    // Okuma ve Yazma erişimine izin verir (çekirdek yığını için).
    // *GEREKLİ AP DEĞERİ CPU KILAVUZUNDAN KONTROL EDİLMELİ*


    // 3. MPU Bölgesi: Çevre Birimleri (Peripherals) bölgesi - *ÖRNEK DEĞERLER VE ADRESLER*
    let peripherals_start = 0x40000000;      // Örnek çevre birimleri başlangıç adresi - *CPU ve SoC'ye göre değişir*
    let peripherals_size = 0x10000;         // Örnek boyut (64KB) - *SoC çevre birimi haritasına göre değişir*
    let peripherals_end = peripherals_start + peripherals_size -1; // Bitiş adresi

    configure_mpu_region(
        2,                                  // MPU Bölge İndeksi 2
        peripherals_start,                  // Bölge Başlangıç Adresi
        peripherals_end,                    // Bölge Bitiş Adresi
        MPU_RLAR_AP_FULL_ACCESS             // Tam Erişim (Ayrıcalıklı ve Kullanıcı RW) - *Çevre birimlerine erişim gereksinimlerine göre ayarlanabilir*
    );
     // Açıklama: MPU Bölge 2, peripherals_start - peripherals_end adres aralığını
    // Çevre birimlerine erişim için (örnek olarak Tam Erişim verildi, erişim hakları çevre birimi gereksinimlerine göre ayarlanmalı).
    // *ÇEVRE BİRİMİ ERİŞİM GEREKSİNİMLERİNE UYGUN AP DEĞERİ CPU VE SoC KILAVUZUNDAN KONTROL EDİLMELİ*


    // Varsayılan bellek eşlemesini (isteğe bağlı) ayarlama - *CPU kılavuzuna göre gerekliyse*
    // Örnek olarak, varsayılan eşlemeyi devre dışı bırakıp MPU tarafından *korunmayan* bölgelere erişimi
    //  engelleyebilir veya ayrıcalıklı erişime izin verebilirsiniz. MPU_CTRL kaydındaki PRIVDEFENA biti ile kontrol edilir.
    // *CPU KILAVUZUNA BAKARAK VARSAYILAN EŞLEME GEREKSİNİMLERİNİ KONTROL EDİN*
    // Örneğin:
    unsafe {
        asm!(
            "ldr r0, ={}",                  // Kayıt adresini r0'a yükle
            "mov r1, {}",                  // MPU_CTRL değerini r1'e taşı (örnek: sadece etkinleştirme)
            "str r1, [r0]",                 // r1'i [r0]'daki adrese (MPU_CTRL) yaz
            in(reg) MPU_CTRL_REG,
            in(reg) MPU_CTRL_ENABLE | MPU_CTRL_PRIVDEFENA_ENABLE, // Örnek: MPU'yu etkinleştir ve Ayrıcalıklı Varsayılan Eşlemeyi etkinleştir
            options(nostack, nomem)
        );
    }

    // *DİĞER MPU BÖLGELERİNİ GEREKTİĞİ GİBİ YAPILANDIRIN (MPU_RNR indeks 3'ten 7'ye kadar örnek olarak)*
    // ...

    // *SON ADIM*: MPU'yu ETKİNLEŞTİRME - *MPU_CTRL kaydındaki etkinleştirme biti AYRICA ayarlanmalıdır*
    // (Varsayılan eşleme ayarı ile birlikte veya ayrı olarak, CPU kılavuzuna göre)
    // Yukarıdaki varsayılan eşleme örneğinde MPU etkinleştirme biti de ayarlanmıştır.
    // Ayrı etkinleştirme örneği (yukarıdaki örnekte zaten etkinleştirildi ama netlik için tekrar gösteriliyor):
     unsafe {
        asm!(
            "ldr r0, ={}",                  // Kayıt adresini r0'a yükle
            "ldr r1, [r0]",                 // Mevcut MPU_CTRL değerini r1'e oku (değiştirmeden korumak için)
            "orr r1, r1, {}",               // Etkinleştirme bitini r1 ile VEYA'la (diğer bitleri değiştirmeden ayarla)
            "str r1, [r0]",                 // Güncellenmiş r1 değerini [r0]'daki adrese (MPU_CTRL) yaz
            in(reg) MPU_CTRL_REG,
            in(reg) MPU_CTRL_ENABLE,        // Sadece etkinleştirme biti (örn. 0b1) - diğer bitler 0 olabilir veya mevcut değeri korumak için okunup OR'lanabilir
            options(nostack, nomem)
        );
    }
}