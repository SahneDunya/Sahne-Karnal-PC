const POWER_CONTROL_REGISTER: u32 = 0x1000_0000;
const POWER_STATE_ON: u32 = 0x0000_0001;
const POWER_STATE_OFF: u32 = 0x0000_0000;

// Güç yönetimi fonksiyonları
pub fn power_on() {
    // Gücü açmak için ilgili registre doğru değeri yaz
    unsafe {
        *(POWER_CONTROL_REGISTER as *mut u32) = POWER_STATE_ON;
    }
}

pub fn power_off() {
    // Gücü kapatmak için ilgili registre doğru değeri yaz
    unsafe {
        *(POWER_CONTROL_REGISTER as *mut u32) = POWER_STATE_OFF;
    }
}

pub fn get_power_state() -> bool {
    // Güç durumunu okuyarak döndür
    unsafe {
        *(POWER_CONTROL_REGISTER as *mut u32) & POWER_STATE_ON != 0
    }
}

// Örnek kullanım
fn main() {
    println!("Güç durumu: {}", get_power_state());

    println!("Güç açılıyor...");
    power_on();
    println!("Güç durumu: {}", get_power_state());

    println!("Güç kapanıyor...");
    power_off();
    println!("Güç durumu: {}", get_power_state());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_on_off() {
        // Test sırasında gerçek donanımı etkilememek için
        // sanal bir donanım modeli kullanılabilir.
        // Bu örnekte, sadece fonksiyonların çağrılabilirliği test ediliyor.

        power_on();
        assert_eq!(get_power_state(), true);

        power_off();
        assert_eq!(get_power_state(), false);
    }
}