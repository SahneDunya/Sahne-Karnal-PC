#[derive(Debug)]
pub enum CameraError {
    InitializationFailed,
    CaptureFailed,
    ParameterSetFailed,
    UnsupportedFormat,
    DeviceNotFound,
    Other(String),
}

// Kamera cihazını temsil eden bir yapı.
pub struct Camera {
    // Özel donanım veya sürücüye özgü tutamaçlar veya durum bilgileri buraya gelebilir.
    device_handle: Option<usize>, // Örneğin, bir cihaz dosya tanımlayıcısı veya benzeri bir şey.
    width: u32,
    height: u32,
    pixel_format: PixelFormat,
}

// Desteklenen piksel formatlarını tanımlayalım.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Rgb888,
    Yuv422,
    Mono8,
    // ... diğer formatlar ...
}

impl PixelFormat {
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            PixelFormat::Rgb888 => 3,
            PixelFormat::Yuv422 => 2, // Genellikle iki piksel için 4 bayt
            PixelFormat::Mono8 => 1,
        }
    }
}

impl Camera {
    /// Yeni bir kamera örneği oluşturur. Henüz başlatma yapmaz.
    pub fn new() -> Self {
        Camera {
            device_handle: None,
            width: 0,
            height: 0,
            pixel_format: PixelFormat::Rgb888, // Varsayılan bir format
        }
    }

    /// Kamerayı belirtilen genişlik, yükseklik ve piksel formatıyla başlatır.
    ///
    /// # Hatalar
    ///
    /// Eğer başlatma başarısız olursa `CameraError::InitializationFailed` döner.
    /// Eğer belirtilen format desteklenmiyorsa `CameraError::UnsupportedFormat` döner.
    /// Eğer cihaz bulunamazsa `CameraError::DeviceNotFound` döner.
    pub fn init(&mut self, width: u32, height: u32, format: PixelFormat) -> Result<(), CameraError> {
        // Düşük seviyeli donanım başlatma işlemleri burada yapılmalı.
        // Bu, CustomOS'un özel sürücüleriyle veya doğrudan donanım erişimiyle etkileşim kurmayı içerebilir.

        // Örnek olarak, bazı temel kontroller yapalım.
        if format != PixelFormat::Rgb888 && format != PixelFormat::Yuv422 && format != PixelFormat::Mono8 {
            return Err(CameraError::UnsupportedFormat);
        }

        // Gerçek donanım başlatma burada olmalı.
        // Örneğin, bir cihaz dosyasını açma veya özel bir sürücü API'sini çağırma gibi.
        let device = self.open_device()?; // Bu metot aşağıda tanımlanmıştır.
        self.device_handle = Some(device);
        self.width = width;
        self.height = height;
        self.pixel_format = format;

        println!("Kamera {width}x{height} çözünürlüğünde ve {:?} formatında başlatıldı.", format);
        Ok(())
    }

    /// Kameradan bir kare yakalar ve ham piksel verilerini bir `Vec<u8>` olarak döndürür.
    ///
    /// # Hatalar
    ///
    /// Eğer yakalama başarısız olursa `CameraError::CaptureFailed` döner.
    pub fn capture_frame(&self) -> Result<Vec<u8>, CameraError> {
        match self.device_handle {
            Some(handle) => {
                let frame_size = (self.width * self.height * self.pixel_format.bytes_per_pixel()) as usize;
                let mut frame_buffer = vec![0u8; frame_size];

                // Düşük seviyeli kare yakalama işlemleri burada yapılmalı.
                // Bu, donanımdan doğrudan veri okumayı veya özel bir sürücü API'sini kullanmayı içerebilir.
                println!("Kamera cihazı {handle} üzerinden {frame_size} bayt veri okunuyor...");

                // !!! DİKKAT !!!
                // Burası tamamen varsayımsal bir örnektir. Gerçekte, bu kısım CustomOS'un
                // donanım erişim mekanizmalarına ve kamera sürücüsüne bağlı olacaktır.
                // Örneğin, belirli bir bellek adresinden okuma yapılması gerekebilir.

                // Örnek olarak, rastgele veri dolduralım.
                for i in 0..frame_size {
                    frame_buffer[i] = (i % 256) as u8;
                }

                println!("Kare yakalandı.");
                Ok(frame_buffer)
            }
            None => Err(CameraError::InitializationFailed),
        }
    }

    /// Kameranın genişliğini döndürür.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Kameranın yüksekliğini döndürür.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Kameranın piksel formatını döndürür.
    pub fn pixel_format(&self) -> PixelFormat {
        self.pixel_format
    }

    /// Kamerayı kapatır ve kaynakları serbest bırakır.
    pub fn deinit(&mut self) {
        if let Some(handle) = self.device_handle.take() {
            // Düşük seviyeli donanım kapatma işlemleri burada yapılmalı.
            // Örneğin, açılan cihaz dosyasını kapatma veya sürücüye kapatma sinyali gönderme gibi.
            println!("Kamera cihazı {handle} kapatılıyor.");
            self.close_device(handle).unwrap_or_else(|e| eprintln!("Cihaz kapatılırken hata oluştu: {:?}", e));
        }
        println!("Kamera kapatıldı.");
    }

    // --- Özel düşük seviyeli fonksiyonlar (CustomOS'a özgü) ---

    /// Kamera cihazını açar. Bu, CustomOS'un özel bir sistem çağrısı veya mekanizması olabilir.
    fn open_device(&self) -> Result<usize, CameraError> {
        // !!! DİKKAT !!!
        // Bu kısım tamamen varsayımsal bir örnektir. Gerçekte, CustomOS'un
        // cihaz açma mekanizmasını kullanmanız gerekecektir.
        // Örneğin, belirli bir sistem çağrısı veya özel bir API olabilir.

        // Örnek olarak, sabit bir değer döndürelim.
        println!("Kamera cihazı açılıyor...");
        Ok(123) // Varsayımsal cihaz tutamağı
    }

    /// Açılan kamera cihazını kapatır. Bu da CustomOS'a özgü olacaktır.
    fn close_device(&self, handle: usize) -> Result<(), CameraError> {
        // !!! DİKKAT !!!
        // Bu kısım da tamamen varsayımsal bir örnektir. Gerçekte, CustomOS'un
        // cihaz kapatma mekanizmasını kullanmanız gerekecektir.

        println!("Kamera cihazı {} kapatılıyor...", handle);
        Ok(())
    }

    /// Kamera parametrelerini ayarlamak için (örneğin, pozlama, parlaklık vb.)
    /// özel fonksiyonlar eklenebilir.
    // pub fn set_parameter(&mut self, parameter: CameraParameter, value: u32) -> Result<(), CameraError> { ... }
}

// Örnek bir kullanım senaryosu.
fn main() {
    let mut camera = Camera::new();

    match camera.init(640, 480, PixelFormat::Rgb888) {
        Ok(_) => {
            println!("Kamera başarıyla başlatıldı.");

            match camera.capture_frame() {
                Ok(frame_data) => {
                    println!("Kare yakalandı. Boyut: {} bayt.", frame_data.len());
                    // Burada kare verileriyle bir şeyler yapabilirsiniz.
                }
                Err(e) => {
                    eprintln!("Kare yakalama hatası: {:?}", e);
                }
            }

            camera.deinit();
        }
        Err(e) => {
            eprintln!("Kamera başlatma hatası: {:?}", e);
        }
    }
}