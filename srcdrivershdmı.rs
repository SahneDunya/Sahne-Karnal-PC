use crate::hdmı_apı::{HdmiController, HdmiError, Resolution};

 pub struct HdmiDriver {
     controller: HdmiController,
 }

 impl HdmiDriver {
     // Yeni bir HDMI sürücüsü örneği oluşturur
     pub fn new() -> Result<Self, HdmiError> {
         println!("HDMI Sürücüsü başlatılıyor...");
         let controller = HdmiController::new()?;
         Ok(HdmiDriver { controller })
     }

     // HDMI çözünürlüğünü ayarlar
     pub fn set_resolution(&self, resolution: Resolution) -> Result<(), HdmiError> {
         println!("HDMI Sürücüsü: Çözünürlük ayarlanıyor: {:?}", resolution);
         self.controller.set_resolution(resolution)
     }

     // HDMI çıkışını etkinleştirir
     pub fn enable(&self) -> Result<(), HdmiError> {
         println!("HDMI Sürücüsü: HDMI etkinleştiriliyor...");
         self.controller.enable()
     }

     // HDMI çıkışını devre dışı bırakır
     pub fn disable(&self) -> Result<(), HdmiError> {
         println!("HDMI Sürücüsü: HDMI devre dışı bırakılıyor...");
         self.controller.disable()
     }

     // Örnek bir kullanım fonksiyonu (gerekirse genişletilebilir)
     pub fn initialize_hdmi(&self, resolution: Resolution) -> Result<(), HdmiError> {
         self.set_resolution(resolution)?;
         self.enable()?;
         Ok(())
     }
 }

 // Örnek bir kullanım (çekirdek veya ana modül içinden çağrılabilir)
 // #[cfg(test)] // Eğer test ortamında çalıştırılacaksa
 fn main() -> Result<(), HdmiError> {
     let hdmi_driver = HdmiDriver::new()?;

     // Örnek bir çözünürlük ayarla
     hdmi_driver.set_resolution(Resolution::_1920x1080)?;

     // HDMI'yı etkinleştir
     hdmi_driver.enable()?;

     println!("HDMI başarıyla başlatıldı ve yapılandırıldı.");

     // Bir süre sonra devre dışı bırakılabilir (isteğe bağlı)
     // hdmi_driver.disable()?;

     Ok(())
 }