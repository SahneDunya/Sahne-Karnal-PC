pub struct EccDriver {
    // ECC sürücüsüne özgü veriler ve donanım erişimi
}

impl EccDriver {
    pub fn new() -> Self {
        // ECC sürücüsünü başlat
        EccDriver {}
    }

    pub fn encode(&self, data: &[u8]) -> Vec<u8> {
        // Verileri ECC kodlarıyla kodla
        let mut encoded_data = Vec::new();
        for byte in data {
            encoded_data.push(*byte);
            encoded_data.push(self.calculate_ecc(*byte));
        }
        encoded_data
    }

    pub fn decode(&self, encoded_data: &[u8]) -> Result<Vec<u8>, String> {
        // Kodlanmış verileri çöz ve hataları düzelt
        if encoded_data.len() % 2 != 0 {
            return Err("Geçersiz kodlanmış veri uzunluğu".to_string());
        }

        let mut decoded_data = Vec::new();
        let mut corrected = false;

        for chunk in encoded_data.chunks(2) {
            let data = chunk[0];
            let ecc = chunk[1];
            let calculated_ecc = self.calculate_ecc(data);

            if ecc != calculated_ecc {
                // Hata algılandı, düzeltmeye çalış
                let corrected_data = self.correct_error(data, ecc, calculated_ecc);
                if self.calculate_ecc(corrected_data) == ecc {
                    decoded_data.push(corrected_data);
                    corrected = true;
                } else {
                    return Err("Düzeltilemeyen hata".to_string());
                }
            } else {
                // Hata yok
                decoded_data.push(data);
            }
        }

        if corrected {
            println!("Hata düzeltildi");
        }

        Ok(decoded_data)
    }

    fn calculate_ecc(&self, data: u8) -> u8 {
        // Basit bir XOR tabanlı ECC hesapla
        data ^ 0x5A
    }

    fn correct_error(&self, data: u8, ecc: u8, calculated_ecc: u8) -> u8 {
        // Tek bit hatasını düzeltmeye çalış
        data ^ (ecc ^ calculated_ecc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecc_encode_decode() {
        let ecc_driver = EccDriver::new();
        let data = vec![0x12, 0x34, 0x56];
        let encoded_data = ecc_driver.encode(&data);
        let decoded_data = ecc_driver.decode(&encoded_data).unwrap();
        assert_eq!(data, decoded_data);
    }

    #[test]
    fn test_ecc_error_correction() {
        let ecc_driver = EccDriver::new();
        let data = vec![0x12, 0x34, 0x56];
        let mut encoded_data = ecc_driver.encode(&data);

        // Tek bit hatası ekle (örneğin, ikinci baytın ikinci bitini değiştir)
        encoded_data[1] ^= 0b00000010; // 0x02 yerine 0x01 hataya neden oldu önceki örnekte

        let decoded_data = ecc_driver.decode(&encoded_data).unwrap();
        assert_eq!(data, decoded_data);
    }

    #[test]
    fn test_ecc_uncorrectable_error() {
        let ecc_driver = EccDriver::new();
        let data = vec![0x12, 0x34, 0x56];
        let mut encoded_data = ecc_driver.encode(&data);

        // Çoklu bit hatası ekle
        encoded_data[1] ^= 0x03;

        let decoded_data = ecc_driver.decode(&encoded_data);
        assert!(decoded_data.is_err());
    }
}