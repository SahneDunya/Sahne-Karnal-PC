#[cfg(target_arch = "openrisc")]
pub mod openrisc {
    // OpenRISC platformuna özel kod buraya gelecek

    // Örnek bir fonksiyon:
    pub fn openrisc_ozel_fonksiyon() {
        println!("Bu fonksiyon OpenRISC platformunda çalışıyor!");
        // OpenRISC'e özel işlemler burada gerçekleştirilebilir.
        // Örneğin, donanım erişimi veya özel kütüphane çağrıları gibi.
    }

    // Başka OpenRISC'e özel fonksiyonlar ve yapılar buraya eklenebilir.

    // Örnek bir struct:
    pub struct OpenriscVerisi {
        pub deger: i32,
    }

    impl OpenriscVerisi {
        pub fn yeni(deger: i32) -> Self {
            Self { deger }
        }
    }
}

#[cfg(not(target_arch = "openrisc"))]
pub mod openrisc {
    // OpenRISC dışında diğer platformlar için boş bir mod veya alternatif implementasyonlar.

    pub fn openrisc_ozel_fonksiyon() {
        println!("Bu fonksiyon OpenRISC platformunda çalışmıyor. (Diğer Platform)");
    }

    pub struct OpenriscVerisi {
        pub deger: i32,
    }

    impl OpenriscVerisi {
        pub fn yeni(deger: i32) -> Self {
            Self { deger }
        }
    }
}


#[cfg(test)]
#[cfg(target_arch = "openrisc")]
mod tests {
    use super::openrisc::*;

    #[test]
    fn openrisc_ozel_fonksiyon_testi() {
        openrisc_ozel_fonksiyon();
        // Burada OpenRISC'e özel testler yapılabilir.
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "openrisc"))]
mod tests {
    use super::openrisc::*;

    #[test]
    fn openrisc_ozel_fonksiyon_testi() {
        openrisc_ozel_fonksiyon();
        // Burada OpenRISC dışı platformlar için testler yapılabilir.
    }
}