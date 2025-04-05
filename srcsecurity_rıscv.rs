#![no_std]
use core::arch::asm;

// PMP yapılandırma baytlarını daha okunabilir hale getiren sabitler
const PMP_CFG_A_TOR: u32 = 0b1000; // Üst Sınır (Top of Range - TOR) Adresleme Modu
const PMP_CFG_A_NA4: u32 = 0b0000; // 4 Bayt Hizalı (Naturally Aligned 4-byte region - NA4) Adresleme Modu
// Diğer adresleme modları: PMP_CFG_A_OFF (Kapalı), PMP_CFG_A_NAPOT (Naturally Aligned Power-of-two region)

const PMP_CFG_M_NONE: u32 = 0b0000; // Erişim Yok (No Access)
const PMP_CFG_M_READ: u32 = 0b0001; // Sadece Okuma (Read-Only)
const PMP_CFG_M_RW: u32 = 0b0011;   // Okuma ve Yazma (Read-Write)
const PMP_CFG_M_RX: u32 = 0b0101;   // Okuma ve Çalıştırma (Read-Execute)
// Diğer erişim modları ve kombinasyonları RISC-V PMP spesifikasyonunda bulunabilir.

const PMP_CFG_LOCKED: u32 = 0b0100; // Kilitli (Locked) - Yapılandırma değişikliğini engeller

// PMP kayıtlarını ayarlamak için fonksiyon (indeks ve adres aralığı kontrolü eklendi)
pub fn configure_pmp(index: usize, addr: usize, cfg: u32) {
    if index >= 16 { // RISC-V'de genellikle 16 PMP kaydı vardır (uygulamaya göre değişebilir)
        panic!("Geçersiz PMP indeksi: {}", index); // Daha açıklayıcı hata mesajı
    }

    // Adres aralığı kontrolü (isteğe bağlı, TOR modunda anlamlıdır)
    // TOR (Üst Sınır) modunda, adres kaydına *bölge sonu* adresi yazılır.
    // NA4 ve NAPOT modlarında, adres kaydına *bölge başlangıç* adresi yazılır.
    // Bu örnekte TOR modu kullanıldığı için, adresin pozitif olması yeterli bir kontrol olabilir.
    if addr < 0 {
        panic!("Geçersiz PMP adres değeri: {}", addr);
    }

    unsafe {
        // PMP Adres Kaydını (pmpaddr) yapılandır
        asm!(
            "csrw pmpaddr{}, {}", // csrw: Control and Status Register Write
            index,              // pmpaddr kaydının indeksi (0, 1, 2, ...)
            in(reg) addr,       // Adres değeri (bölge sonu adresi - TOR modu)
            options(nostack, nomem) // Derleyici optimizasyonlarını kısıtlar (stack ve bellek kullanma)
        );
        // PMP Yapılandırma Kaydını (pmpcfg) yapılandır
        asm!(
            "csrw pmpcfg{}, {}",  // csrw: Control and Status Register Write
            index,               // pmpcfg kaydının indeksi (0, 1, 2, ...)
            in(reg) cfg,        // Yapılandırma değeri (adresleme modu, erişim hakları, kilit durumu)
            options(nostack, nomem) // Derleyici optimizasyonlarını kısıtlar (stack ve bellek kullanma)
        );
    }
}

// PMP yapılandırmasını başlatmak için fonksiyon (geliştirilmiş ve açıklamalı örnek)
pub fn init_pmp() {
    // Çekirdek (Kernel) bellek bölgesi tanımları
    let kernel_start = 0x10000;           // Çekirdek kodunun başlangıç adresi
    let kernel_size = 0x2000;            // Çekirdek kodunun boyutu (8KB = 0x2000 bayt)
    let kernel_end = kernel_start + kernel_size; // Çekirdek kodunun bitiş adresi (üst sınır)

    // 1. PMP Bölgesi: Çekirdek kodu için (Okuma ve Çalıştırma - Read & Execute)
    // Adresleme Modu: Üst Sınır (TOR - Top of Range)
    // Erişim Hakları: Okuma ve Çalıştırma (RX - Read-Execute)
    // Kilit Durumu: Kilitli (LOCKED) - Değiştirilemez
    configure_pmp(
        0,                      // PMP Kayıt İndeksi 0
        kernel_end,             // Bölge Üst Sınır Adresi (Çekirdek sonu) - TOR modu
        PMP_CFG_A_TOR |        // Üst Sınır Adresleme Modu
        PMP_CFG_M_RX |         // Okuma ve Çalıştırma erişim hakları
        PMP_CFG_LOCKED,         // Kilitli yapılandırma
    );
    // Açıklama: PMP Kayıt 0, 0x0 - kernel_end adres aralığını (8KB çekirdek kodu)
    // Okuma ve Çalıştırma erişimine izin verir, diğer erişim türlerini engeller.
    // Bu bölge kilitlendiği için çalışma zamanında yapılandırması değiştirilemez.


    // Çekirdek yığını (Kernel Stack) bellek bölgesi tanımları
    let kernel_stack_start = kernel_end;    // Çekirdek yığınının başlangıç adresi (çekirdek kodundan hemen sonra)
    let kernel_stack_size = 0x1000;     // Çekirdek yığınının boyutu (4KB = 0x1000 bayt)
    let kernel_stack_end = kernel_stack_start + kernel_stack_size; // Çekirdek yığınının bitiş adresi (üst sınır)


    // 2. PMP Bölgesi: Çekirdek yığını için (Okuma ve Yazma - Read & Write)
    // Adresleme Modu: Üst Sınır (TOR - Top of Range)
    // Erişim Hakları: Okuma ve Yazma (RW - Read-Write)
    // Kilit Durumu: Kilitli (LOCKED) - Değiştirilemez
    configure_pmp(
        1,                      // PMP Kayıt İndeksi 1
        kernel_stack_end,         // Bölge Üst Sınır Adresi (Çekirdek yığın sonu) - TOR modu
        PMP_CFG_A_TOR |        // Üst Sınır Adresleme Modu
        PMP_CFG_M_RW |         // Okuma ve Yazma erişim hakları
        PMP_CFG_LOCKED,         // Kilitli yapılandırma
    );
    // Açıklama: PMP Kayıt 1, kernel_end - kernel_stack_end adres aralığını (4KB çekirdek yığını)
    // Okuma ve Yazma erişimine izin verir, çalıştırma erişimini engeller.
    // Bu bölge de kilitlendiği için çalışma zamanında yapılandırması değiştirilemez.


    // 3. PMP Bölgesi: "Her şeyi engelle" (Default Deny) bölgesi - Kalan tüm adres alanını kapsar
    // Adresleme Modu: Üst Sınır (TOR - Top of Range)
    // Erişim Hakları: Erişim Yok (NONE - No Access)
    // Kilit Durumu: Kilitli (LOCKED) - Değiştirilemez
    configure_pmp(
        2,                      // PMP Kayıt İndeksi 2
        0,                      // Bölge Üst Sınır Adresi: 0 - TOR modu için *tüm adres uzayını kapsar*
        PMP_CFG_A_TOR |        // Üst Sınır Adresleme Modu
        PMP_CFG_M_NONE |       // Erişim Yok - Tüm erişimleri engeller
        PMP_CFG_LOCKED,         // Kilitli yapılandırma
    );
    // Açıklama: PMP Kayıt 2, 0 - 0 adres aralığını (yani *tüm adres uzayı*)
    // Hiçbir erişime izin vermez. TOR modu ve 0 adresi ile, *tüm adres uzayı* etkili bir şekilde
    // "üst sınır" olarak tanımlanır ve erişime kapatılır. Bu, PMP'nin varsayılan olarak güvenli
    // (her şeyi engelle) bir başlangıç noktası sağlamasına olanak tanır.

    // Diğer PMP bölgelerini gerektiği gibi yapılandırın...
    // Örneğin, çevre birimleri (peripherals), farklı görevler (tasks) için bölgeler tanımlanabilir.
    // PMP kayıt indeksleri 3'ten 15'e kadar (toplamda 16 kayıt varsayımıyla) kullanılabilir.
}