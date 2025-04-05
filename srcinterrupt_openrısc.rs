#![no_std]
use core::arch::asm;

// OpenRISC'te Kesme Vektör Tablosu (IVT) adresi
const IVT_ADDRESS: usize = 0x100; // Örnek adres: Donanımınıza göre ayarlayın

// Kesme Nedenleri (Örnek): Donanımınıza göre genişletin
const TIMER_INTERRUPT: usize = 0x01;

// Kesme İşleyici Fonksiyonu
#[no_mangle]
pub extern "C" fn interrupt_handler() {
    // Kesme nedenini okumak için SPR_CSR寄存器'sini kullan
    let cause = get_interrupt_cause();

    match cause {
        TIMER_INTERRUPT => {
            handle_timer_interrupt();
        }
        _ => {
            // Beklenmeyen kesme durumu (isteğe bağlı işleme)
            // ... hata ayıklama veya varsayılan işleme ...
        }
    }
}

// Yardımcı fonksiyon: Kesme nedenini oku
fn get_interrupt_cause() -> usize {
    let mut cause: usize;
    unsafe {
        // SPR_CSR (Kontrol ve Durum 寄存器)'den kesme nedenini oku
        asm!("mfspr {0}, spr_csr", out(reg) cause);
    }
    cause
}

// Yardımcı fonksiyon: Zamanlayıcı kesmesini işle
fn handle_timer_interrupt() {
    // Zamanlayıcı kesmesi işleme kodu buraya gelecek
    // ... zamanlayıcı ile ilgili işlemler ...

    clear_timer_interrupt_flag();
}

// Yardımcı fonksiyon: Zamanlayıcı kesme bayrağını temizle (DONANIMA ÖZGÜ)
fn clear_timer_interrupt_flag() {
    unsafe {
        // Örnek: Zamanlayıcı kontrol寄存器'indeki ilgili biti temizle
        // Gerçek donanımınıza uygun寄存器 ve biti kullanın
        // volatile_store!(TIMER_CONTROL_REGISTER, ...);

        // !!! DİKKAT: Aşağıdaki satır SADECE BİR ÖRNEKTİR ve ÇALIŞMAYABİLİR !!!
        // !!! Gerçek donanımınızın kılavuzuna başvurarak doğru yöntemi bulun !!!
        asm!("nop"); // veya uygun donanım komutu
    }
}


pub fn init() {
    set_ivt_address();
    enable_interrupts();
    enable_timer_interrupt();
}

// Yardımcı fonksiyon: IVT adresini ayarla
fn set_ivt_address() {
    unsafe {
        // SPR_IVTBR (Kesme Vektör Tablosu Taban 寄存器)'ye IVT adresini yaz
        asm!(
            "mtspr spr_ivtbr, {0}",
            in(reg) IVT_ADDRESS
        );
    }
}

// Yardımcı fonksiyon: Genel kesmeleri etkinleştir
fn enable_interrupts() {
    unsafe {
        // SPR_SR (Durum 寄存器)'de kesmeleri etkinleştir
        // !!! DİKKAT: 0x01 değeri SADECE BİR ÖRNEKTİR ve DONANIMA GÖRE DEĞİŞİR !!!
        // !!! Gerçek donanımınızın kılavuzuna başvurarak doğru değeri bulun !!!
        asm!(
            "mtspr spr_sr, {0}",
            in(reg) 0x01 // Örnek değer: Donanımınıza göre ayarlayın
        );
    }
}

// Yardımcı fonksiyon: Zamanlayıcı kesmesini etkinleştir (DONANIMA ÖZGÜ)
fn enable_timer_interrupt() {
    // ... zamanlayıcı ile ilgili ayarlar ...
    // !!! BU KISIM DONANIMA ÖZGÜDÜR ve AYARLANMASI GEREKİR !!!
    // !!! Zamanlayıcı kesmesini etkinleştirmek için donanım kılavuzuna bakın !!!

    unsafe {
        // Örnek olarak, sadece bir "nop" komutu ekliyoruz.
        // GERÇEK UYGULAMADA BU KISIM DONANIMA ÖZGÜ KOD İLE DOLDURULMALIDIR.
        asm!("nop"); // Yer tutucu: Donanıma özgü zamanlayıcı etkinleştirme kodu buraya
    }
}