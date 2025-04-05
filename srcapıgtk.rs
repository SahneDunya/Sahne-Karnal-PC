#[cfg(feature = "gtk3")]
extern crate gtk3;

#[cfg(feature = "gtk4")]
extern crate gtk4;

// Ortak trait ve fonksiyonlar
pub trait Driver {
    fn create_window(&self, title: &str, width: i32, height: i32) -> Box<dyn Window>;
}

pub trait Window {
    fn set_title(&self, title: &str);
    fn show_all(&self);
}

// GTK3 için uygulama
#[cfg(feature = "gtk3")]
pub struct Gtk3Driver;

#[cfg(feature = "gtk3")]
impl Driver for Gtk3Driver {
    fn create_window(&self, title: &str, width: i32, height: i32) -> Box<dyn Window> {
        use gtk3::{Window as Gtk3WindowConcrete, WindowType, prelude::*}; // İçe aktarmaları sadeleştir
        let window = Gtk3WindowConcrete::new(WindowType::Toplevel);
        window.set_title(title);
        window.set_default_size(width, height);
        Box::new(Gtk3Window { window })
    }
}

#[cfg(feature = "gtk3")]
pub struct Gtk3Window {
    window: gtk3::Window,
}

#[cfg(feature = "gtk3")]
impl Window for Gtk3Window {
    fn set_title(&self, title: &str) {
        self.window.set_title(title);
    }

    fn show_all(&self) {
        self.window.show_all();
    }
}

// GTK4 için uygulama
#[cfg(feature = "gtk4")]
pub struct Gtk4Driver;

#[cfg(feature = "gtk4")]
impl Driver for Gtk4Driver {
    fn create_window(&self, title: &str, width: i32, height: i32) -> Box<dyn Window> {
        use gtk4::{Window as Gtk4WindowConcrete, prelude::*}; // İçe aktarmaları sadeleştir
        let window = Gtk4WindowConcrete::new();
        window.set_title(Some(title));
        window.set_default_size(width, height);
        Box::new(Gtk4Window { window })
    }
}

#[cfg(feature = "gtk4")]
pub struct Gtk4Window {
    window: gtk4::Window,
}

#[cfg(feature = "gtk4")]
impl Window for Gtk4Window {
    fn set_title(&self, title: &str) {
        self.window.set_title(Some(title));
    }

    fn show_all(&self) {
        self.window.show_all();
    }
}

// Örnek kullanım (Tek örnek ile basitleştirilmiş)
fn main() {
    #[cfg(feature = "gtk4")] // Tek örnek olarak GTK4 seçildi
    let driver = Gtk4Driver;

    #[cfg(feature = "gtk4")] // Sadece GTK4 için main loop başlatılıyor örnekte
    {
        let window = driver.create_window("Merhaba, Dünya! (GTK4 Örneği)", 800, 600);
        window.show_all();
        gtk4::main();
    }
}