#[cfg(target_arch = "x86")] // Sadece x86 mimarisinde derlenecek kodlar
pub mod x86 {
    // x86 platformuna özel fonksiyonlar ve yapılar buraya gelecek.

    // Örnek:
    // x86 platformunda kullanılan özel bir veri yapısı
    pub struct X86Veri {
        pub deger: u32,
    }

    impl X86Veri {
        pub fn yeni(deger: u32) -> Self {
            X86Veri { deger }
        }

        pub fn degeri_al(&self) -> u32 {
            self.deger
        }
    }

    // Örnek:
    // x86 platformuna özel bir fonksiyon
    pub fn x86_ozel_fonksiyon() {
        println!("x86 platformuna özel fonksiyon çalıştı!");
    }

      // Örnek: Inline assembly kullanımı (dikkatli olun!)
    #[cfg(target_arch = "x86")]
    pub fn inline_assembly_ornegi() {
        unsafe {
            asm!("nop"); // "No operation" instruction
        }
    }


    // ... diğer x86'ya özgü kodlar ...
}

#[cfg(not(target_arch = "x86"))] // Eğer x86 değilse, boş bir modül tanımla
pub mod x86 {
    // Bu modül boş kalacak çünkü x86 platformunda değiliz.
    // İstenirse, diğer platformlar için varsayılan (boş) uygulamalar buraya konulabilir.

    // Örnek (Boş uygulama):
    pub struct X86Veri {} // Boş bir yapı

    impl X86Veri {
        pub fn yeni(_deger: u32) -> Self {
            X86Veri {}
        }
        pub fn degeri_al(&self) -> u32 {
            0 // Varsayılan değer
        }
    }

    pub fn x86_ozel_fonksiyon() {
        println!("x86 platformu değil. Bu fonksiyon boş bir uygulama.");
    }

    pub fn inline_assembly_ornegi() {
        println!("x86 platformu değil. Inline assembly örneği kullanılamaz.");
    }
}