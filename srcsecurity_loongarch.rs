#![no_std]

use core::arch::asm;

// LoongArch MPU (Memory Protection Unit) yapılandırma baytlarını daha okunabilir hale getiren sabitler
// Lütfen dikkat: Bu sabitler ve register isimleri hipotetikdir ve LoongArch mimarisine göre değişiklik gösterebilir.
// Gerçek değerler ve isimler için LoongArch mimari referans kılavuzuna başvurulmalıdır.

const MPU_CFG_REGION_EN: u32 = 0b1;        // MPU Bölgesi Etkin (Region Enable)
const MPU_CFG_ADDR_MODE_TOR: u32 = 0b10;   // Üst Sınır (Top of Range - TOR) Adresleme Modu (hipotetik)
const MPU_CFG_ADDR_MODE_BASE_MASK: u32 = 0b00; // Taban ve Maske Adresleme Modu (hipotetik)

const MPU_CFG_PERM_NONE: u32 = 0b000;      // Erişim Yok (No Access)
const MPU_CFG_PERM_READ: u32 = 0b001;      // Sadece Okuma (Read-Only)
const MPU_CFG_PERM_WRITE: u32 = 0b010;     // Sadece Yazma (Write-Only) (hipotetik, RW daha yaygın)
const MPU_CFG_PERM_RW: u32 = 0b011;        // Okuma ve Yazma (Read-Write)
const MPU_CFG_PERM_EXECUTE: u32 = 0b100;   // Sadece Çalıştırma (Execute-Only) (hipotetik, RX daha yaygın)
const MPU_CFG_PERM_RX: u32 = 0b101;        // Okuma ve Çalıştırma (Read-Execute)
// Diğer erişim modları ve kombinasyonları LoongArch MPU spesifikasyonunda bulunabilir.

const MPU_CFG_LOCKED: u32 = 0b10000;       // Kilitli (Locked) - Yapılandırma değişikliğini engeller (hipotetik)


// MPU kayıtlarını ayarlamak için fonksiyon (indeks ve adres aralığı kontrolü eklendi)
// Lütfen dikkat: Register isimleri ve assembly komutları hipotetikdir.
// Gerçek komutlar ve register isimleri için LoongArch mimari referans kılavuzuna başvurulmalıdır.
pub fn configure_mpu(index: usize, base_addr: usize, limit_addr: usize, cfg: u32) {
    if index >= 8 { // LoongArch'da hipotetik olarak 8 MPU bölgesi olabilir, doğrulamak gerekir
        panic!("Geçersiz MPU indeksi: {}", index); // Daha açıklayıcı hata mesajı
    }

    // Adres aralığı kontrolü (isteğe bağlı)
    if base_addr >= limit_addr {
        panic!("Geçersiz MPU adres aralığı: Başlangıç adresi bitiş adresinden büyük veya eşit.");
    }

    if base_addr < 0 || limit_addr < 0 {
        panic!("Geçersiz MPU adres değeri: Negatif adres kullanılamaz.");
    }


    unsafe {
        // MPU Taban Adres Kaydını (mpu_base_addr<index>) yapılandır (hipotetik register ismi)
        asm!(
            "csrwr mpu_base_addr{}, {}", // csrwr: Control and Status Register Write (hipotetik komut)
            index,                        // mpu_base_addr kaydının indeksi (0, 1, 2, ...)
            in(reg) base_addr,           // Taban adres değeri
            options(nostack, nomem)
        );

        // MPU Sınır Adres Kaydını (mpu_limit_addr<index>) yapılandır (hipotetik register ismi)
        asm!(
            "csrwr mpu_limit_addr{}, {}", // csrwr: Control and Status Register Write (hipotetik komut)
            index,                        // mpu_limit_addr kaydının indeksi (0, 1, 2, ...)
            in(reg) limit_addr,          // Sınır adres değeri
            options(nostack, nomem)
        );


        // MPU Yapılandırma Kaydını (mpu_cfg<index>) yapılandır (hipotetik register ismi)
        asm!(
            "csrwr mpu_cfg{}, {}",     // csrwr: Control and Status Register Write (hipotetik komut)
            index,                        // mpu_cfg kaydının indeksi (0, 1, 2, ...)
            in(reg) cfg,                // Yapılandırma değeri (adresleme modu, erişim hakları, kilit durumu)
            options(nostack, nomem)
        );
    }
}


// MPU yapılandırmasını başlatmak için fonksiyon (geliştirilmiş ve açıklamalı örnek)
pub fn init_mpu() {
    // Çekirdek (Kernel) bellek bölgesi tanımları - Örnek adresler, LoongArch sistemine göre değişebilir.
    let kernel_start = 0x80000000;        // Çekirdek kodunun başlangıç adresi (Örnek - LoongArch için doğrulanmalı)
    let kernel_size = 0x4000;         // Çekirdek kodunun boyutu (16KB = 0x4000 bayt) - Örnek
    let kernel_end = kernel_start + kernel_size; // Çekirdek kodunun bitiş adresi

    // 1. MPU Bölgesi: Çekirdek kodu için (Okuma ve Çalıştırma - Read & Execute)
    // Adresleme Modu: Üst Sınır (TOR - Top of Range) - Hipotetik olarak TOR benzeri mod
    // Erişim Hakları: Okuma ve Çalıştırma (RX - Read-Execute)
    // Kilit Durumu: Kilitli (LOCKED) - Değiştirilemez
    configure_mpu(
        0,                                 // MPU Bölge İndeksi 0
        0x0,                                // Bölge Başlangıç Adresi (TOR benzeri mod için 0)
        kernel_end,                         // Bölge Bitiş Adresi (Çekirdek sonu) - TOR benzeri mod
        MPU_CFG_REGION_EN |                 // Bölgeyi etkinleştir
        MPU_CFG_ADDR_MODE_TOR |            // Üst Sınır Adresleme Modu (hipotetik)
        MPU_CFG_PERM_RX |                   // Okuma ve Çalıştırma erişim hakları
        MPU_CFG_LOCKED,                    // Kilitli yapılandırma
    );
    // Açıklama: MPU Bölge 0, 0x0 - kernel_end adres aralığını (16KB çekirdek kodu - örnek boyut)
    // Okuma ve Çalıştırma erişimine izin verir, diğer erişim türlerini engeller.
    // Bu bölge kilitlendiği için çalışma zamanında yapılandırması değiştirilemez.


    // Çekirdek yığını (Kernel Stack) bellek bölgesi tanımları - Örnek adresler, LoongArch sistemine göre değişebilir.
    let kernel_stack_start = kernel_end;    // Çekirdek yığınının başlangıç adresi (çekirdek kodundan hemen sonra)
    let kernel_stack_size = 0x2000;       // Çekirdek yığınının boyutu (8KB = 0x2000 bayt) - Örnek
    let kernel_stack_end = kernel_stack_start + kernel_stack_size; // Çekirdek yığınının bitiş adresi


    // 2. MPU Bölgesi: Çekirdek yığını için (Okuma ve Yazma - Read & Write)
    // Adresleme Modu: Üst Sınır (TOR - Top of Range) - Hipotetik olarak TOR benzeri mod
    // Erişim Hakları: Okuma ve Yazma (RW - Read-Write)
    // Kilit Durumu: Kilitli (LOCKED) - Değiştirilemez
    configure_mpu(
        1,                                 // MPU Bölge İndeksi 1
        kernel_stack_start,                 // Bölge Başlangıç Adresi (Çekirdek yığın başlangıcı) - Taban adres
        kernel_stack_end,                   // Bölge Bitiş Adresi (Çekirdek yığın sonu) - Sınır adres
        MPU_CFG_REGION_EN |                 // Bölgeyi etkinleştir
        MPU_CFG_ADDR_MODE_BASE_MASK |       // Taban ve Maske Adresleme Modu (hipotetik olarak uygun olabilir)
        MPU_CFG_PERM_RW |                    // Okuma ve Yazma erişim hakları
        MPU_CFG_LOCKED,                    // Kilitli yapılandırma
    );
    // Açıklama: MPU Bölge 1, kernel_stack_start - kernel_stack_end adres aralığını (8KB çekirdek yığını - örnek boyut)
    // Okuma ve Yazma erişimine izin verir, çalıştırma erişimini engeller.
    // Bu bölge de kilitlendiği için çalışma zamanında yapılandırması değiştirilemez.


    // 3. MPU Bölgesi: "Her şeyi engelle" (Default Deny) bölgesi - Kalan tüm adres alanını kapsar
    // Adresleme Modu: Üst Sınır (TOR - Top of Range) - Hipotetik olarak TOR benzeri mod
    // Erişim Hakları: Erişim Yok (NONE - No Access)
    // Kilit Durumu: Kilitli (LOCKED) - Değiştirilemez
    configure_mpu(
        2,                                 // MPU Bölge İndeksi 2
        0x0,                                // Bölge Başlangıç Adresi (TOR benzeri mod için 0)
        0x0,                                // Bölge Bitiş Adresi: 0 - TOR benzeri mod için *tüm adres uzayını kapsar*
        MPU_CFG_REGION_EN |                 // Bölgeyi etkinleştir
        MPU_CFG_ADDR_MODE_TOR |            // Üst Sınır Adresleme Modu (hipotetik)
        MPU_CFG_PERM_NONE |                 // Erişim Yok - Tüm erişimleri engeller
        MPU_CFG_LOCKED,                    // Kilitli yapılandırma
    );
    // Açıklama: MPU Bölge 2, 0x0 - 0x0 adres aralığını (yani *tüm adres uzayı*)
    // Hiçbir erişime izin vermez. TOR benzeri modu ve 0x0 adresi ile, *tüm adres uzayı* etkili bir şekilde
    // "üst sınır" olarak tanımlanır ve erişime kapatılır. Bu, MPU'nun varsayılan olarak güvenli
    // (her şeyi engelle) bir başlangıç noktası sağlamasına olanak tanır.


    // Diğer MPU bölgelerini gerektiği gibi yapılandırın...
    // Örneğin, çevre birimleri (peripherals), farklı görevler (tasks) için bölgeler tanımlanabilir.
    // MPU bölge indeksleri 3'ten 7'ye kadar (toplamda 8 bölge varsayımıyla) kullanılabilir.
}