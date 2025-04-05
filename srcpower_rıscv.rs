// Güç modları
const ACTIVE_MODE: u32 = 0;
const SLEEP_MODE: u32 = 1;

// Güç yönetimi register'ı adresi
const POWER_CTRL_REG: u32 = 0x10000000;

// Güç modunu ayarlayan fonksiyon
fn set_power_mode(mode: u32) {
    unsafe {
        *(POWER_CTRL_REG as *mut u32) = mode;
    }
}

// Uyku moduna geçiş fonksiyonu
fn enter_sleep_mode() {
    set_power_mode(SLEEP_MODE);
    // WFI (Wait For Interrupt) komutu ile işlemciyi uyku moduna al
    unsafe {
        asm!("wfi");
    }
}

// Aktif moda geçiş (kesme ile)
#[interrupt]
fn external_interrupt() {
    // Kesme işleme kodu
    set_power_mode(ACTIVE_MODE);
}

fn main() {
    // ...
    enter_sleep_mode();
    // ...
}