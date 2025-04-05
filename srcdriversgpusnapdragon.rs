use core::fmt;

// Hata türlerini tanımlayan enum
#[derive(Debug)]
pub enum GPUError {
    CommandQueueFull, // Komut kuyruğu dolu hatası
    InvalidCommand,   // Geçersiz komut hatası
    MemoryAllocationError, // Bellek ayırma hatası (örneğin, yetersiz bellek)
    UnknownError,       // Bilinmeyen hata
}

// GPU işlemlerinin sonuçlarını temsil etmek için Result kullanıyoruz.
// Başarılı olursa Ok(()), hata durumunda Err(GPUError) dönecek.
type Result<T> = std::result::Result<T, GPUError>;

// Snapdragon GPU'sunun temel özellikleri ve yetenekleri
pub struct SnapdragonGPU {
    model: String,
    memory: u64, // Toplam bellek boyutu (bayt cinsinden)
    clock_speed: u32, // Saat hızı (MHz cinsinden)
    command_queue: Vec<Command>, // Komut kuyruğu
    max_queue_size: usize, // Maksimum komut kuyruğu boyutu
    allocated_memory: u64, // Şu anda ayrılmış bellek miktarı
}

impl SnapdragonGPU {
    // Yeni bir Snapdragon GPU örneği oluşturur
    pub fn new(model: &str, memory: u64, clock_speed: u32, max_queue_size: usize) -> Self {
        Self {
            model: model.to_string(),
            memory,
            clock_speed,
            command_queue: Vec::with_capacity(max_queue_size), // Başlangıç kapasitesi ayarlandı
            max_queue_size,
            allocated_memory: 0, // Başlangıçta ayrılmış bellek yok
        }
    }

    // GPU bilgilerini ekrana yazdırır, bellek boyutunu MB cinsinden gösterir
    pub fn print_info(&self) {
        println!("Model: {}", self.model);
        println!("Bellek: {} MB", self.memory / (1024 * 1024)); // Belleği MB olarak göster
        println!("Saat Hızı: {} MHz", self.clock_speed);
        println!("Maksimum Komut Kuyruğu Boyutu: {}", self.max_queue_size);
        println!("Kuyruktaki Komut Sayısı: {}", self.command_queue.len());
        println!("Ayrılmış Bellek: {} MB", self.allocated_memory / (1024 * 1024));
    }

    // Komut kuyruğuna bir komut ekler, kuyruk doluysa hata döner
    pub fn enqueue_command(&mut self, command: Command) -> Result<()> {
        if self.command_queue.len() < self.max_queue_size {
            self.command_queue.push(command);
            println!("Komut kuyruğuna eklendi: {:?}", command);
            Ok(()) // Başarılı, Ok(Unit) döndür
        } else {
            eprintln!("Komut kuyruğu dolu!"); // Hata mesajını standart hata akışına yazdır
            Err(GPUError::CommandQueueFull) // Komut kuyruğu dolu hatası döndür
        }
    }

    // GPU'ya bellek ayırma isteği, yetersiz bellek durumunda hata döner
    pub fn allocate_memory(&mut self, size: u64) -> Result<()> {
        if self.allocated_memory + size <= self.memory {
            self.allocated_memory += size;
            println!("{} bayt bellek ayrıldı.", size);
            Ok(())
        } else {
            eprintln!("Yetersiz bellek!");
            Err(GPUError::MemoryAllocationError)
        }
    }

    // Ayrılan belleği serbest bırakır
    pub fn deallocate_memory(&mut self, size: u64) -> Result<()> {
        if size <= self.allocated_memory {
            self.allocated_memory -= size;
            println!("{} bayt bellek serbest bırakıldı.", size);
            Ok(())
        } else {
            eprintln!("Serbest bırakılacak bellek ayrılan bellekten fazla!");
            Err(GPUError::MemoryAllocationError) // Veya başka bir hata türü belirleyebilirsiniz.
        }
    }

    // Komut kuyruğunu işler (örnek olarak sadece kuyruğu temizler)
    pub fn process_commands(&mut self) {
        println!("Komut kuyruğu işleniyor...");
        for command in &self.command_queue {
            println!("Komut işleniyor: {:?}", command);
            // Burada komut işleme mantığı yer alabilir.
        }
        self.command_queue.clear(); // Kuyruğu temizle
        println!("Komut kuyruğu işlendi ve temizlendi.");
    }
}


// GPU komutlarını temsil eden bir enum
#[derive(Debug, Clone, Copy)] // Debug ve Clone traitleri eklendi
pub enum Command {
    RenderTriangle,
    ComputeKernel,
    CopyMemory,
    LoadTexture, // Yeni komut eklendi
}

// SnapdragonGPU yapısı için trait uygulamaları
impl fmt::Display for SnapdragonGPU {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Snapdragon GPU: {} ({} MB Bellek)", self.model, self.memory / (1024 * 1024))
    }
}

fn main() -> Result<()> { // main fonksiyonu artık Result<()> döndürüyor
    // Örnek bir Snapdragon GPU oluştur, maksimum komut kuyruğu boyutu 5 olarak ayarlandı
    let mut gpu = SnapdragonGPU::new("Adreno 740", 8 * 1024 * 1024 * 1024, 900, 5); // 8GB bellek, 900MHz saat hızı, kuyruk boyutu 5

    // GPU bilgilerini yazdır
    gpu.print_info();
    println!("{}", gpu);

    // Bellek ayırma
    gpu.allocate_memory(2 * 1024 * 1024 * 1024)?; // 2GB bellek ayır, hata kontrolü yapıldı

    // Komut kuyruğuna komutlar ekle ve hataları işle
    gpu.enqueue_command(Command::RenderTriangle)?;
    gpu.enqueue_command(Command::ComputeKernel)?;
    gpu.enqueue_command(Command::CopyMemory)?;
    gpu.enqueue_command(Command::LoadTexture)?;

    // Kuyruk dolu hatasını tetikleyecek bir komut daha eklemeye çalış
    if let Err(error) = gpu.enqueue_command(Command::RenderTriangle) {
        eprintln!("Komut kuyruğuna ekleme hatası: {:?}", error); // Hata durumunda mesaj yazdır
    }

    // Komut kuyruğunu işle
    gpu.process_commands();

    // Belleği serbest bırak
    gpu.deallocate_memory(1 * 1024 * 1024 * 1024)?; // 1GB bellek serbest bırak, hata kontrolü yapıldı
    gpu.print_info(); // Güncellenmiş GPU bilgilerini yazdır

    Ok(()) // Başarılı çalıştırma
}