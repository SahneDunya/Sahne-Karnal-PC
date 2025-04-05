#[cfg(target_arch = "arm")]
pub mod arm {
    // ARM mimarisine özel kod buraya gelecek

    // Örnek bir fonksiyon:
    pub fn arm_ozel_fonksiyon() {
        println!("Bu fonksiyon ARM mimarisi için özeldir.");
        // ARM'e özgü işlemler burada yapılabilir.
        // Örneğin, donanım erişimi veya düşük seviyeli optimizasyonlar.
    }

    // Başka ARM'e özel yapılar, fonksiyonlar veya trait'ler tanımlanabilir.

    // Örnek bir struct:
    pub struct ArmOzelVeri {
        pub deger: u32,
    }

    impl ArmOzelVeri {
        pub fn yeni(deger: u32) -> Self {
            Self { deger }
        }
    }


    // Örnek bir trait:
    pub trait ArmOzelTrait {
        fn arm_ozel_metod(&self);
    }

    impl ArmOzelTrait for ArmOzelVeri {
        fn arm_ozel_metod(&self) {
            println!("ArmOzelVeri için ARM özel metodu: {}", self.deger);
        }
    }


    #[cfg(test)]
    mod testler {
        use super::*;

        #[test]
        fn arm_ozel_fonksiyon_test() {
            arm_ozel_fonksiyon();
        }

        #[test]
        fn arm_ozel_veri_test() {
            let veri = ArmOzelVeri::yeni(42);
            assert_eq!(veri.deger, 42);
        }

        #[test]
        fn arm_ozel_trait_test() {
            let veri = ArmOzelVeri::yeni(123);
            veri.arm_ozel_metod();
        }
    }
}


#[cfg(not(target_arch = "arm"))]
pub mod arm {
    // ARM olmayan mimariler için boş bir mod veya alternatif bir uygulama
    // sağlayabiliriz. Bu, kodun diğer mimarilerde de derlenmesini sağlar.

    // Örneğin, boş bir fonksiyon:
    pub fn arm_ozel_fonksiyon() {
        println!("Bu fonksiyon ARM mimarisi için özeldir (ARM olmayan platform).");
        // ARM olmayan platformda yapılacak işlemler buraya gelebilir.
    }

      // Veya, ARM olmayan platform için farklı bir uygulama:
    // pub fn arm_ozel_fonksiyon() {
    //     println!("Bu fonksiyon ARM mimarisi için özeldir (ARM olmayan platform - alternatif uygulama).");
    //     // ARM olmayan platformda farklı işlemler yapılabilir.
    // }

    #[cfg(test)]
    mod testler {
        #[test]
        fn arm_ozel_fonksiyon_test() {
            arm_ozel_fonksiyon();
        }
    }
}

// Örnek kullanım:
fn main() {
    arm::arm_ozel_fonksiyon();

    #[cfg(target_arch = "arm")]
    {
       let veri = arm::ArmOzelVeri::yeni(99);
       veri.arm_ozel_metod();
    }
}