pub fn uyku_moduna_gir() {
    // ARM işlemcisini düşük güç moduna geçirmek için gerekli adımlar
    // Örneğin, SCB_SCR register'ını yapılandırmak veya WFI (Wait For Interrupt) komutunu kullanmak.
    // ...
    println!("ARM işlemcisi uyku moduna girdi.");
}

pub fn derin_uyku_moduna_gir() {
    // ARM işlemcisini daha derin bir düşük güç moduna geçirmek için gerekli adımlar
    // Örneğin, SCB_SCR register'ını yapılandırmak ve güç tüketimini en aza indirmek.
    // ...
    println!("ARM işlemcisi derin uyku moduna girdi.");
}

pub fn uyan() {
    // ARM işlemcisini uyku modundan uyandırmak için gerekli adımlar
    // Örneğin, kesme (interrupt) veya başka bir olay tetiklemek.
    // ...
    println!("ARM işlemcisi uyandı.");
}

pub fn güç_tüketimini_ölç() -> u32 {
    // ARM işlemcisinin anlık güç tüketimini ölçmek için gerekli adımlar
    // Örneğin, PMU (Power Management Unit) register'larını okumak.
    // ...
    let güç_tüketimi = 100; // Örnek olarak sabit bir değer döndürülüyor
    println!("ARM işlemcisinin güç tüketimi: {} mW", güç_tüketimi);
    güç_tüketimi
}

// Örnek kullanım
fn main() {
    println!("Güç yönetimi örneği");

    güç_tüketimini_ölç();
    uyku_moduna_gir();
    uyan();
    derin_uyku_moduna_gir();
    uyan();
    güç_tüketimini_ölç();
}