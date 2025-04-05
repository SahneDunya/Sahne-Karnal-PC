use std::error::Error;
use std::fmt;

/// USB cihazlarıyla ilgili genel hataları temsil eden bir enum.
#[derive(Debug)]
pub enum UsbHata {
    AygıtBulunamadı,
    ErişimHatası,
    VeriOkumaHatası,
    VeriYazmaHatası,
    Diğer(String),
}

impl fmt::Display for UsbHata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UsbHata::AygıtBulunamadı => write!(f, "USB aygıtı bulunamadı."),
            UsbHata::ErişimHatası => write!(f, "USB aygıtına erişim hatası."),
            UsbHata::VeriOkumaHatası => write!(f, "USB aygıtından veri okuma hatası."),
            UsbHata::VeriYazmaHatası => write!(f, "USB aygıtına veri yazma hatası."),
            UsbHata::Diğer(s) => write!(f, "Diğer USB hatası: {}", s),
        }
    }
}

impl Error for UsbHata {}

/// Bir USB cihazını temsil eden yapı.
pub struct UsbAygıt {
    /// Aygıtın benzersiz tanımlayıcısı.
    pub aygıt_adı: String,
    // Diğer aygıt özellikleri eklenebilir (örneğin, satıcı kimliği, ürün kimliği).
}

impl UsbAygıt {
    /// Yeni bir `UsbAygıt` örneği oluşturur.
    pub fn yeni(aygıt_adı: String) -> Self {
        UsbAygıt { aygıt_adı }
    }

    /// Aygıttan veri okur. Bu örnekte sadece bir yer tutucudur.
    pub fn veri_oku(&self, boyut: usize) -> Result<Vec<u8>, UsbHata> {
        println!("{} aygıtından {} bayt veri okunmaya çalışılıyor.", self.aygıt_adı, boyut);
        // Gerçek okuma işlemleri burada gerçekleştirilir.
        // Örneğin, işletim sistemi API'lerini kullanarak.
        if boyut > 1024 {
            Err(UsbHata::VeriOkumaHatası)
        } else {
            Ok(vec![0; boyut]) // Örnek olarak sıfırlarla dolu bir vektör döndürülüyor.
        }
    }

    /// Aygıta veri yazar. Bu örnekte sadece bir yer tutucudur.
    pub fn veri_yaz(&self, veri: &[u8]) -> Result<(), UsbHata> {
        println!("{} aygıtına {} bayt veri yazılmaya çalışılıyor.", self.aygıt_adı, veri.len());
        // Gerçek yazma işlemleri burada gerçekleştirilir.
        // Örneğin, işletim sistemi API'lerini kullanarak.
        if veri.len() > 2048 {
            Err(UsbHata::VeriYazmaHatası)
        } else {
            Ok(())
        }
    }
}

/// USB aygıtlarını yönetmek için ana API yapısı.
pub struct UsbYönetici {}

impl UsbYönetici {
    /// Yeni bir `UsbYönetici` örneği oluşturur.
    pub fn yeni() -> Self {
        UsbYönetici {}
    }

    /// Şu anda bağlı olan tüm USB aygıtlarının bir listesini alır.
    pub fn aygıtları_listele(&self) -> Result<Vec<UsbAygıt>, UsbHata> {
        println!("Bağlı USB aygıtları listeleniyor.");
        // Gerçek aygıt listeleme işlemleri burada gerçekleştirilir.
        // Örneğin, işletim sistemi API'lerini kullanarak.
        // Bu örnekte, bazı sahte aygıtlar döndürülüyor.
        Ok(vec![
            UsbAygıt::yeni("aygıt1".to_string()),
            UsbAygıt::yeni("aygıt2".to_string()),
        ])
    }

    /// Belirli bir ada sahip bir USB aygıtını açar.
    pub fn aygıtı_aç(&self, aygıt_adı: &str) -> Result<UsbAygıt, UsbHata> {
        println!("{} adlı USB aygıtı açılmaya çalışılıyor.", aygıt_adı);
        // Gerçek aygıt açma işlemleri burada gerçekleştirilir.
        // Örneğin, işletim sistemi API'lerini kullanarak.
        if aygıt_adı == "aygıt1" {
            Ok(UsbAygıt::yeni(aygıt_adı.to_string()))
        } else {
            Err(UsbHata::AygıtBulunamadı)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aygıt_listeleme_testi() {
        let yönetici = UsbYönetici::yeni();
        let sonuç = yönetici.aygıtları_listele();
        assert!(sonuç.is_ok());
        assert_eq!(sonuç.unwrap().len(), 2);
    }

    #[test]
    fn aygıt_açma_testi() {
        let yönetici = UsbYönetici::yeni();
        let sonuç = yönetici.aygıtı_aç("aygıt1");
        assert!(sonuç.is_ok());
        assert_eq!(sonuç.unwrap().aygıt_adı, "aygıt1");

        let sonuç = yönetici.aygıtı_aç("olmayan_aygıt");
        assert!(sonuç.is_err());
        assert!(matches!(sonuç.unwrap_err(), UsbHata::AygıtBulunamadı));
    }

    #[test]
    fn veri_okuma_yazma_testi() {
        let yönetici = UsbYönetici::yeni();
        let aygıt_sonucu = yönetici.aygıtı_aç("aygıt1");
        assert!(aygıt_sonucu.is_ok());
        let aygıt = aygıt_sonucu.unwrap();

        let okuma_sonucu = aygıt.veri_oku(512);
        assert!(okuma_sonucu.is_ok());
        assert_eq!(okuma_sonucu.unwrap().len(), 512);

        let yazma_sonucu = aygıt.veri_yaz(&[1; 1024]);
        assert!(yazma_sonucu.is_ok());

        let büyük_okuma_sonucu = aygıt.veri_oku(2048);
        assert!(büyük_okuma_sonucu.is_err());
        assert!(matches!(büyük_okuma_sonucu.unwrap_err(), UsbHata::VeriOkumaHatası));

        let büyük_yazma_sonucu = aygıt.veri_yaz(&[1; 4096]);
        assert!(büyük_yazma_sonucu.is_err());
        assert!(matches!(büyük_yazma_sonucu.unwrap_err(), UsbHata::VeriYazmaHatası));
    }
}

fn main() -> Result<(), UsbHata> {
    // Bir UsbYönetici örneği oluşturun.
    let yönetici = UsbYönetici::yeni();

    // Bağlı USB aygıtlarını listeleyin.
    println!("Bağlı USB Aygıtları:");
    match yönetici.aygıtları_listele() {
        Ok(aygıtlar) => {
            for aygıt in aygıtlar {
                println!("- {}", aygıt.aygıt_adı);
            }
        }
        Err(hata) => {
            eprintln!("Aygıt listeleme hatası: {}", hata);
            return Err(hata);
        }
    }

    // "aygıt1" adlı bir USB aygıtını açmaya çalışın.
    let aygıt_adı = "aygıt1";
    println!("\n{} adlı aygıt açılıyor...", aygıt_adı);
    let aygıt_sonucu = yönetici.aygıtı_aç(aygıt_adı);
    match aygıt_sonucu {
        Ok(aygıt) => {
            println!("{} adlı aygıt başarıyla açıldı.", aygıt.aygıt_adı);

            // Aygıttan veri okumayı deneyin.
            let okunacak_boyut = 512;
            println!("{} aygıtından {} bayt okunmaya çalışılıyor.", aygıt.aygıt_adı, okunacak_boyut);
            match aygıt.veri_oku(okunacak_boyut) {
                Ok(veri) => {
                    println!("{} bayt veri başarıyla okundu.", veri.len());
                    // Okunan verilerle ilgili işlemler burada yapılabilir.
                }
                Err(hata) => {
                    eprintln!("Veri okuma hatası: {}", hata);
                    return Err(hata);
                }
            }

            // Aygıta veri yazmayı deneyin.
            let yazılacak_veri = vec![1; 1024];
            println!("{} aygıtına {} bayt yazılmaya çalışılıyor.", aygıt.aygıt_adı, yazılacak_veri.len());
            match aygıt.veri_yaz(&yazılacak_veri) {
                Ok(_) => {
                    println!("{} bayt veri başarıyla yazıldı.", yazılacak_veri.len());
                }
                Err(hata) => {
                    eprintln!("Veri yazma hatası: {}", hata);
                    return Err(hata);
                }
            }

            // Büyük boyutlu veri okumayı deneyin (hata bekleniyor).
            let büyük_okuma_boyutu = 2048;
            println!("{} aygıtından {} bayt okunmaya çalışılıyor (hata bekleniyor).", aygıt.aygıt_adı, büyük_okuma_boyutu);
            match aygıt.veri_oku(büyük_okuma_boyutu) {
                Ok(_) => {
                    println!("Hata bekleniyordu ancak okuma başarılı oldu!");
                }
                Err(hata) => {
                    println!("Beklenen hata alındı: {}", hata);
                }
            }

            // Büyük boyutlu veri yazmayı deneyin (hata bekleniyor).
            let büyük_yazılacak_veri = vec![1; 4096];
            println!("{} aygıtına {} bayt yazılmaya çalışılıyor (hata bekleniyor).", aygıt.aygıt_adı, büyük_yazılacak_veri.len());
            match aygıt.veri_yaz(&büyük_yazılacak_veri) {
                Ok(_) => {
                    println!("Hata bekleniyordu ancak yazma başarılı oldu!");
                }
                Err(hata) => {
                    println!("Beklenen hata alındı: {}", hata);
                }
            }
        }
        Err(hata) => {
            eprintln!("Aygıt açma hatası: {}", hata);
            return Err(hata);
        }
    }

    // Olmayan bir aygıtı açmayı deneyin (hata bekleniyor).
    let olmayan_aygıt_adı = "olmayan_aygıt";
    println!("\n{} adlı aygıt açılmaya çalışılıyor (hata bekleniyor)...", olmayan_aygıt_adı);
    match yönetici.aygıtı_aç(olmayan_aygıt_adı) {
        Ok(_) => {
            println!("Hata bekleniyordu ancak aygıt başarıyla açıldı!");
        }
        Err(hata) => {
            println!("Beklenen hata alındı: {}", hata);
        }
    }

    Ok(())
}