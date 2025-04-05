#![no_std]

use core::arch::asm;

// OpenRISC 1000 MPU (Memory Protection Unit) yapılandırma sabitleri

// MPURATTR - Bölge Özellik Kaydı için sabitler
const OR_MPU_ATTR_E: u32 = 1 << 0;  // Bölgeyi Etkinleştir (Enable)
const OR_MPU_ATTR_CI: u32 = 1 << 1; // Talimat Önbelleğe Alınabilir (Cacheable Instruction)
const OR_MPU_ATTR_CD: u32 = 1 << 2; // Veri Önbelleğe Alınabilir (Cacheable Data)
const OR_MPU_ATTR_WT: u32 = 1 << 3; // Yazma Geçirme (Write Through)
const OR_MPU_ATTR_G: u32 = 1 << 4;  // Koruma Bölgesi (Guard Region)
const OR_MPU_ATTR_RWX_SUP_RW: u32 = 0b11 << 5; // Süpervizör Modu için Okuma/Yazma
const OR_MPU_ATTR_RWX_SUP_R: u32 = 0b10 << 5;  // Süpervizör Modu için Sadece Okuma
const OR_MPU_ATTR_RWX_SUP_NONE: u32 = 0b00 << 5; // Süpervizör Modu için Erişim Yok
const OR_MPU_ATTR_RWX_USER_RW: u32 = 0b11 << 7; // Kullanıcı Modu için Okuma/Yazma
const OR_MPU_ATTR_RWX_USER_R: u32 = 0b10 << 7;  // Kullanıcı Modu için Sadece Okuma
const OR_MPU_ATTR_RWX_USER_NONE: u32 = 0b00 << 7; // Kullanıcı Modu için Erişim Yok
const OR_MPU_ATTR_RWX_EXEC: u32 = 1 << 9; // Çalıştırma İzni (Execute Permission)


// MPU bölgelerini ayarlamak için fonksiyon
// OpenRISC MPU, 8 bölgeye kadar destekler (MPU Bölge 0 - MPU Bölge 7)
pub fn configure_mpu_region(index: usize, start_addr: usize, end_addr: usize, attr: u32) {
    if index >= 8 { // OpenRISC 1000 MPU 8 bölge destekler
        panic!("Geçersiz MPU bölge indeksi: {}", index);
    }

    if start_addr >= end_addr {
        panic!("Geçersiz MPU adres aralığı: Başlangıç adresi bitiş adresinden büyük veya eşit olamaz.");
    }

    unsafe {
        // MPU Bölge Başlangıç Adresi Kaydını (MPURSTARTn) yapılandır
        asm!(
            "mtspr r{0}, {1}",  // mtspr: Move to Special-Purpose Register
            in(reg) 96 + index, // MPURSTARTn SPR numarası (96 + index, n=0-7) - Bölge Başlangıç Adresi Kaydı
            in(reg) start_addr,
            options(nostack, nomem)
        );

        // MPU Bölge Bitiş Adresi Kaydını (MPURENDn) yapılandır
        asm!(
            "mtspr r{0}, {1}",  // mtspr: Move to Special-Purpose Register
            in(reg) 104 + index, // MPURENDn SPR numarası (104 + index, n=0-7) - Bölge Bitiş Adresi Kaydı
            in(reg) end_addr,
            options(nostack, nomem)
        );

        // MPU Bölge Özellik Kaydını (MPURATTRn) yapılandır
        asm!(
            "mtspr r{0}, {1}",  // mtspr: Move to Special-Purpose Register
            in(reg) 112 + index, // MPURATTRn SPR numarası (112 + index, n=0-7) - Bölge Özellik Kaydı
            in(reg) attr,
            options(nostack, nomem)
        );
    }
}

// MPU yapılandırmasını başlatmak için örnek fonksiyon
pub fn init_mpu() {
    // Çekirdek (Kernel) bellek bölgesi tanımları (Örnek Adresler)
    let kernel_start = 0x10000;
    let kernel_size = 0x2000; // 8KB
    let kernel_end = kernel_start + kernel_size -1; // Bitiş adresi dahil


    // 1. MPU Bölgesi: Çekirdek kodu için (Süpervizör Modu: Okuma/Çalıştırma, Kullanıcı Modu: Çalıştırma)
    configure_mpu_region(
        0,                      // MPU Bölge İndeksi 0
        kernel_start,           // Başlangıç Adresi
        kernel_end,             // Bitiş Adresi
        OR_MPU_ATTR_E |        // Bölgeyi Etkinleştir
        OR_MPU_ATTR_CI |       // Talimat Önbelleğe Alınabilir
        OR_MPU_ATTR_CD |       // Veri Önbelleğe Alınabilir (Çekirdek veri erişimi için - isteğe bağlı)
        OR_MPU_ATTR_RWX_SUP_R | // Süpervizör modunda okuma ve çalıştırma (kod çalıştırılabilir olmalı)
        OR_MPU_ATTR_RWX_EXEC |  // Çalıştırma izni
        OR_MPU_ATTR_RWX_USER_R  // Kullanıcı modunda sadece okuma ve çalıştırma (isteğe bağlı - kullanıcı kodu çekirdekten kod okuyabilirse)
    );
    // Açıklama: MPU Bölge 0, kernel_start - kernel_end adres aralığını
    // Çekirdek kodu için korur. Süpervizör modunda okuma ve çalıştırma, kullanıcı modunda sadece okuma ve çalıştırma (örnek yapılandırma).


    // Çekirdek yığını (Kernel Stack) bölgesi (Örnek Adresler)
    let kernel_stack_start = kernel_end + 1; // Çekirdek kodundan hemen sonra
    let kernel_stack_size = 0x1000; // 4KB
    let kernel_stack_end = kernel_stack_start + kernel_stack_size - 1;

    // 2. MPU Bölgesi: Çekirdek yığını için (Süpervizör Modu: Okuma/Yazma)
    configure_mpu_region(
        1,                      // MPU Bölge İndeksi 1
        kernel_stack_start,     // Başlangıç Adresi
        kernel_stack_end,       // Bitiş Adresi
        OR_MPU_ATTR_E |        // Bölgeyi Etkinleştir
        OR_MPU_ATTR_CD |       // Veri Önbelleğe Alınabilir
        OR_MPU_ATTR_RWX_SUP_RW | // Süpervizör modunda okuma ve yazma (yığın için RW erişimi)
        OR_MPU_ATTR_RWX_USER_NONE // Kullanıcı modunda erişim yok (yığın kullanıcı modundan korunmalı)
    );
    // Açıklama: MPU Bölge 1, kernel_stack_start - kernel_stack_end adres aralığını
    // Çekirdek yığını için korur. Sadece Süpervizör modunda okuma ve yazma erişimine izin verir.


    // 3. MPU Bölgesi: Çevre Birimleri (Peripherals) bölgesi (Örnek Adresler)
    let peripheral_start = 0x40000000; // Örnek başlangıç adresi
    let peripheral_size = 0x1000; // 4KB
    let peripheral_end = peripheral_start + peripheral_size -1;

    // 3. MPU Bölgesi: Çevre birimleri için (Süpervizör Modu: Okuma/Yazma, Kullanıcı Modu: Sadece Okuma - örnek)
    configure_mpu_region(
        2,                      // MPU Bölge İndeksi 2
        peripheral_start,       // Başlangıç Adresi
        peripheral_end,         // Bitiş Adresi
        OR_MPU_ATTR_E |        // Bölgeyi Etkinleştir
        OR_MPU_ATTR_CD |       // Veri Önbelleğe Alınabilir (Çevre birimleri veri erişimi için)
        OR_MPU_ATTR_WT |       // Yazma Geçirme (Çevre birimleri için WT yaygın olabilir)
        OR_MPU_ATTR_RWX_SUP_RW | // Süpervizör modunda okuma/yazma
        OR_MPU_ATTR_RWX_USER_R  // Kullanıcı modunda sadece okuma (örnek - çevre birimi durumunu okuma)
    );
    // Açıklama: MPU Bölge 2, peripheral_start - peripheral_end adres aralığını
    // Çevre birimleri için korur. Süpervizör modunda RW, kullanıcı modunda sadece R erişimine izin verir (örnek).


    // 4. MPU Bölgesi: "Her şeyi engelle" (Default Deny) bölgesi - Kalan tüm adres alanını kapsar
    // Dikkat: OpenRISC MPU "default deny" özelliği için genellikle tüm adres uzayını kapsayan bir bölgeye
    // erişim yok (NONE) özelliği vermek yerine, *hiçbir bölge tanımlamamak* daha yaygın bir yaklaşımdır.
    // Eğer MPU'da bir eşleşme bulunamazsa, varsayılan davranış genellikle erişimi engellemektir.
    // Ancak, açıkça "her şeyi engelle" bölgesi tanımlamak istenirse, en düşük öncelikli bölgeye (örneğin Bölge 7)
    // geniş bir aralık (veya tüm adres uzayı) ve erişim yok (NONE) özelliği verilebilir.

    // Örnek olarak, Bölge 7'yi "her şeyi engelle" bölgesi olarak yapılandıralım:
    let deny_all_start = 0x0;
    let deny_all_end = 0xFFFFFFFF; // Tüm 32-bit adres uzayını kapsayacak şekilde (veya sistemin adres uzayı sınırına göre)

    configure_mpu_region(
        7,                      // MPU Bölge İndeksi 7 (En düşük öncelikli bölge - örnek)
        deny_all_start,         // Başlangıç Adresi: 0
        deny_all_end,           // Bitiş Adresi: Tüm adres uzayı
        OR_MPU_ATTR_E |        // Bölgeyi Etkinleştir
        OR_MPU_ATTR_RWX_SUP_NONE | // Süpervizör modunda erişim yok
        OR_MPU_ATTR_RWX_USER_NONE  // Kullanıcı modunda erişim yok
    );
    // Açıklama: MPU Bölge 7, 0 - 0xFFFFFFFF adres aralığını (tüm adres uzayı)
    // Herhangi bir erişimi engellemek için yapılandırılmıştır. Bu, varsayılan olarak erişimi engelleme (default deny)
    // prensibini uygulamak için kullanılabilir. Ancak OpenRISC MPU'nun doğal davranışı bölge eşleşmesi yoksa
    // erişimi engellemek olduğundan, bu bölgeye her zaman ihtiyaç duyulmayabilir.


    // Diğer MPU bölgelerini gerektiği gibi yapılandırın...
    // Örneğin, farklı görevler (tasks) için bölgeler, özel amaçlı bellek bölgeleri tanımlanabilir.
    // MPU kayıt indeksleri 3'ten 6'ya kadar (veya ihtiyaca göre) kullanılabilir.
}