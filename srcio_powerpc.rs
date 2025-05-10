#![no_std] // Bu modül de çekirdek alanında çalışacak

// Karnal64 API'sından gerekli tipleri ve trait'leri içeri aktar
// Varsayım: karnal64.rs dosyasındaki public öğelere buradan erişilebilir.
use karnal64::{
    KError,         // Çekirdek hata türü
    ResourceProvider, // Kaynak sağlayıcı trait'i
    // Aşağıdaki tipler karnal64.rs kodunuzdaki yorumlarda geçtiği için dahil edildi
    KseekFrom,      // Seek pozisyonu enum'ı
    KResourceStatus,// Kaynak durumu struct'ı
};

// MMIO (Memory-Mapped I/O) adreslerine doğrudan erişim için volatile okuma/yazma fonksiyonları
// Bu, derleyicinin G/Ç işlemlerini optimize etmesini veya yeniden sıralamasını önler.
#[inline(always)]
fn mmio_readb(addr: usize) -> u8 {
    unsafe {
        core::ptr::read_volatile(addr as *const u8)
    }
}

#[inline(always)]
fn mmio_writeb(addr: usize, value: u8) {
    unsafe {
        core::ptr::write_volatile(addr as *mut u8, value)
    }
}


// --- PowerPC UART Donanım Detayları (Yer Tutucu) ---
// Bunlar, kullandığınız spesifik PowerPC kartının donanım kılavuzundan alınmalıdır.
// Burada tipik bir UART'a ait varsayımsal ofsetler ve bitler kullanılmıştır (örn. 16550 benzeri).

/// Varsayımsal PowerPC UART'ının belleğe haritalı başlangıç adresi.
/// Gerçek donanıma göre değiştirilmelidir.
const UART_BASE_ADDRESS: usize = 0xF000_0100; // Örnek bir MMIO adresi

/// UART Register Ofsetleri (UART_BASE_ADDRESS'e göre)
const RBR_THR_OFFSET: usize = 0x00; // Okuma için Alıcı Tamponu (Receiver Buffer Register - RBR)
                                    // Yazma için İletim Tutma Register'ı (Transmit Holding Register - THR)
const LSR_OFFSET: usize = 0x05;     // Hat Durumu Register'ı (Line Status Register - LSR)

/// Hat Durumu Register'ı (LSR) Bit Maskeleri
const LSR_DR: u8   = 1 << 0; // Data Ready (Okunacak veri var)
const LSR_THRE: u8 = 1 << 5; // Transmit Holding Register Empty (Yazmak için hazır)


/// PowerPC UART cihazını temsil eden ve Karnal64 ResourceProvider trait'ini implemente eden yapı.
/// Belleğe haritalı G/Ç kullandığı varsayılır.
pub struct PowerPCUart {
    base_address: usize,
    // Not: Çoklu iş parçacığı veya kesme bağlamından erişim olacaksa,
    // register erişimi için bir kilit (spin::Mutex gibi) eklemek gerekir.
    // Şimdilik basitleştirilmiş, tek iş parçacıklı erişim varsayılmıştır.
}

impl PowerPCUart {
    /// Yeni bir PowerPCUart örneği oluşturur.
    ///
    /// `base_address`: UART donanımının belleğe haritalı başlangıç adresi.
    pub const fn new(base_address: usize) -> Self {
        Self { base_address }
    }

    /// LSR register'ını okuyarak veri okunmaya hazır olana kadar bekler (polling).
    /// Gerçek bir çekirdekte, bloklama yerine görev değiştirmek (yield) veya
    /// kesme tabanlı G/Ç kullanmak daha verimli olacaktır.
    fn wait_for_read_ready(&self) {
        while (mmio_readb(self.base_address + LSR_OFFSET) & LSR_DR) == 0 {
            // CPU'yu meşgul eden bekleme (busy-wait). Verimli değildir.
            // Gerçek implementasyonda ktask::task_yield() veya olay bekleme kullanılabilir.
            core::hint::spin_loop();
        }
    }

    /// LSR register'ını okuyarak iletim tamponu boşalana kadar bekler (polling).
    fn wait_for_write_ready(&self) {
         while (mmio_readb(self.base_address + LSR_OFFSET) & LSR_THRE) == 0 {
             // Busy-wait.
             core::hint::spin_loop();
         }
    }
}


// ResourceProvider trait'ini PowerPCUart yapısı için implemente et
impl ResourceProvider for PowerPCUart {
    /// UART'tan veri okur.
    /// UART akış tabanlı bir kaynak olduğundan, ofset genellikle göz ardı edilir.
    /// Okuma, veri gelene kadar bloklar (polling kullanarak).
    fn read(&self, buffer: &mut [u8], _offset: u64) -> Result<usize, KError> {
        if buffer.is_empty() {
            return Ok(0); // Boş tampon, 0 byte okuma başarılı sayılır.
        }

        let mut bytes_read = 0;
        for byte in buffer.iter_mut() {
            // Okunacak veri gelene kadar bekle
            self.wait_for_read_ready();

            // Veriyi RBR register'ından oku
            *byte = mmio_readb(self.base_address + RBR_THR_OFFSET);
            bytes_read += 1;

            // Akış cihazlarında genellikle tek byte okunur ve hemen döndürülür.
            // Tam tamponu doldurmak istiyorsak döngü devam eder.
            // Bu örnek tamponu doldurmaya çalışır.
        }

        Ok(bytes_read)
    }

    /// UART'a veri yazar.
    /// UART akış tabanlı bir kaynak olduğundan, ofset genellikle göz ardı edilir.
    /// Yazma, iletim tamponu boşalana kadar bloklar (polling kullanarak).
    fn write(&self, buffer: &[u8], _offset: u64) -> Result<usize, KError> {
        if buffer.is_empty() {
            return Ok(0); // Boş tampon, 0 byte yazma başarılı sayılır.
        }

        let mut bytes_written = 0;
        for &byte in buffer.iter() {
            // İletim tamponu boşalana kadar bekle
            self.wait_for_write_ready();

            // Veriyi THR register'ına yaz
            mmio_writeb(self.base_address + RBR_THR_OFFSET, byte);
            bytes_written += 1;
        }

        Ok(bytes_written)
    }

    /// Kaynağa özel kontrol komutlarını işler.
    /// Temel bir UART için genellikle desteklenmez veya çok sınırlıdır.
    fn control(&self, _request: u64, _arg: u64) -> Result<i64, KError> {
        // UART ayarları (baud rate, parity vb.) burada yapılabilir, ancak
        // bu örnekte karmaşıklığı artırmamak için desteklenmedi olarak işaretlendi.
        Err(KError::NotSupported)
    }

    /// Kaynak içindeki pozisyonu değiştirir.
    /// Akış tabanlı cihazlar (UART gibi) genellikle seekable değildir.
    fn seek(&self, _position: KseekFrom) -> Result<u64, KError> {
        Err(KError::NotSupported)
    }

    /// Kaynağın güncel durumunu döndürür.
    /// UART için okunmaya veya yazılmaya hazır olup olmadığını gösterebilir.
    fn get_status(&self) -> Result<KResourceStatus, KError> {
        let lsr = mmio_readb(self.base_address + LSR_OFFSET);
        Ok(KResourceStatus {
            ready_to_read: (lsr & LSR_DR) != 0,     // Data Ready bitini kontrol et
            ready_to_write: (lsr & LSR_THRE) != 0, // THR Empty bitini kontrol et
        })
    }
}

// --- Nasıl Kullanılır (Kavramsal Notlar) ---
// Bu PowerPC UART sürücüsünü Karnal64 sistemine entegre etmek için:
// 1. Çekirdek başlatma (boot) aşamasında, PowerPC'ye özgü başlatma kodunda
//    `PowerPCUart::new(UART_BASE_ADDRESS)` çağrılarak bir örnek oluşturulur.
//    Örn: `let uart0_provider = PowerPCUart::new(UART_BASE_ADDRESS);`
// 2. Oluşturulan ResourceProvider örneği, Karnal64'ün kresource yöneticisine
//    bir isimle kaydedilir. Bu isim kullanıcı alanından `resource_acquire`
//    sistem çağrısı ile kaynağa erişmek için kullanılır.
//    Örn: `kresource::register_provider("karnal://device/uart/powerpc/0", Box::new(uart0_provider)).expect("UART0 kaydı başarısız!");`
//    (kresource modülünde register_provider fonksiyonunun public ve Box<dyn ResourceProvider> alacak şekilde tanımlı olması gerekir)
// 3. Kullanıcı alanı veya çekirdeğin diğer kısımları, kaydedilen ismi kullanarak
//    bu UART kaynağına handle (tanıtıcı) edinebilir ve ardından read/write çağrıları yapabilir.
    resource_acquire("karnal://device/uart/powerpc/0", MODE_READ | MODE_WRITE) -> KHandle

// Not: Bu kodda kullanılan bellek adresleri ve register ofsetleri tamamen
// varsayımsaldır. Gerçek bir PowerPC sisteminde kullanılmadan önce doğru
// donanım belgelerine başvurulmalıdır. Busy-waiting implementasyonu basittir
// ancak gerçek bir çekirdek için verimsizdir ve görev değişimine veya kesmelere
// dayalı G/Ç yönetimi gerektirir.
