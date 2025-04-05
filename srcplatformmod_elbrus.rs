#[cfg(target_arch = "elbrus")]
pub mod elbrus {
    // Elbrus mimarisine özel kod buraya gelecek

    // Örnek bir fonksiyon:
    pub fn elbrus_ozel_fonksiyon() {
        println!("Bu fonksiyon Elbrus mimarisinde çalışıyor!");

        // Elbrus'a özgü donanım veya yazılım özelliklerine erişim burada yapılabilir.
        // Örneğin, özel register'lara erişim veya belirli Elbrus komutlarını kullanma gibi.

        // Örnek: (Bu kod tamamen hayalidir ve Elbrus mimarisine özel detayları bilmeyi gerektirir)
        /*
        unsafe {
            // Elbrus'a özel bir register'a değer yazma
            asm!("mov r15, {}", in(reg) 0x1234); // Örnek register adresi ve değeri
        }
        */
    }


    // Başka Elbrus'a özel fonksiyonlar ve yapılar buraya eklenebilir.

    // Örnek bir struct:
    pub struct ElbrusVeri {
        pub deger: i32,
    }

    impl ElbrusVeri {
        pub fn yeni(deger: i32) -> Self {
            ElbrusVeri { deger }
        }
    }


    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn elbrus_ozel_fonksiyon_test() {
            elbrus_ozel_fonksiyon();
            // Burada test assertion'ları olabilir. Örneğin, fonksiyonun beklenen sonucu üretip üretmediği kontrol edilebilir.
        }

       #[test]
        fn elbrus_veri_test() {
             let veri = ElbrusVeri::yeni(42);
             assert_eq!(veri.deger, 42);
        }
    }
}


// Diğer mimariler için de benzer modüller oluşturulabilir.
// Örneğin:
#[cfg(not(target_arch = "elbrus"))]
pub mod diger_mimari {
    // Diğer mimariler için kod buraya gelecek.
    // Bu modül boş olabilir veya farklı mimariler için alternatif implementasyonlar içerebilir.

    pub fn elbrus_ozel_fonksiyon() {
        println!("Bu fonksiyon Elbrus mimarisinde çalışmıyor!");
    }
}



fn main() {
    #[cfg(target_arch = "elbrus")]
    {
        elbrus::elbrus_ozel_fonksiyon();
        let veri = elbrus::ElbrusVeri::yeni(10);
        println!("ElbrusVeri değeri: {}", veri.deger);
    }

    #[cfg(not(target_arch = "elbrus"))]
    {
       diger_mimari::elbrus_ozel_fonksiyon();
    }

}