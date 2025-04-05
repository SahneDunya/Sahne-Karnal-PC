#![no_std]
use core::ptr::{read_volatile, write_volatile};

// Message yapısı (aynı kalıyor)
#[repr(C)]
#[derive(Debug, Copy, Clone)] // Debug eklendi, hata ayıklamayı kolaylaştırır
pub struct Message {
    sender: u32,
    receiver: u32,
    message_type: u32,
    data: [u8; 4],
}

// Paylaşımlı bellek bölgesi (gerçek bir uygulamada bu, işletim sistemi tarafından ayrılmalı)
// static mut kullanimi dikkatli olunmalidir, özellikle eş zamanlı erişimde yarış durumlarına neden olabilir.
// Gerçek bir sistemde, bu bellek bölgesi işletim sistemi veya donanım seviyesinde güvenli bir şekilde yönetilmelidir.
static mut SHARED_MEMORY: [u8; 1024] = [0; 1024];

// Döngüsel tampon yapısı
pub struct RingBuffer {
    buffer: *mut u8,
    capacity: usize,
    head: *mut usize,
    tail: *mut usize,
}

impl RingBuffer {
    // Güvenli new fonksiyonu. Buffer ve capacity parametreleri alinir.
    // head ve tail pointerlari buffer'in baslangicina yerlestirilir.
    pub unsafe fn new(buffer: *mut u8, capacity: usize) -> Self {
        // **İyileştirme 1: Buffer geçerlilik kontrolü**
        // buffer'in null olup olmadığını kontrol etmek önemlidir.
        assert!(!buffer.is_null(), "RingBuffer::new: Buffer pointer cannot be null");
        // capacity'nin 0'dan büyük olduğundan emin olun.
        assert!(capacity > core::mem::size_of::<usize>() * 2, "RingBuffer::new: Capacity must be larger than head and tail size");


        let head = buffer as *mut usize;
        let tail = buffer.add(core::mem::size_of::<usize>()) as *mut usize;
        *head = 0;
        *tail = 0;
        RingBuffer { buffer, capacity, head, tail }
    }

    pub unsafe fn push(&mut self, msg: &Message) -> bool {
        // **İyileştirme 2: Volatile okuma ve yazma işlemleri açıkça belirtildi**
        // head ve tail değerleri volatile olarak okunur.
        let head = read_volatile(self.head);
        let tail = read_volatile(self.tail);
        let msg_size = core::mem::size_of::<Message>();

        // **İyileştirme 3: Tampon doluluk kontrolü daha anlaşılır hale getirildi**
        // Tamponun dolu olup olmadığını kontrol eder.
        // ((head + msg_size) % self.capacity) == tail ifadesi,
        // yeni mesaj için yer kalıp kalmadığını döngüsel tampon mantığına göre kontrol eder.
        if (head + msg_size) % self.capacity == tail {
            return false; // Tampon dolu
        }

        let write_ptr = self.buffer.add(head);
        // **İyileştirme 4: Veri kopyalama işlemi nonoverlapping olarak yapılıyor**
        // Mesaj verisi tampona kopyalanır. `copy_nonoverlapping` kullanılarak bellek bölgelerinin çakışmadığı varsayılır,
        // bu da performansı artırabilir (ancak dikkatli kullanılmalıdır). Burada tampon ve mesaj belleği ayrı olduğu için güvenlidir.
        core::ptr::copy_nonoverlapping(msg as *const Message as *const u8, write_ptr, msg_size);

        // **İyileştirme 5: head değeri volatile olarak güncelleniyor**
        // head değeri güncellenirken volatile yazma işlemi kullanılır.
        write_volatile(self.head, (head + msg_size) % self.capacity);
        true
    }

    pub unsafe fn pop(&mut self) -> Option<Message> {
        // **İyileştirme 6: Volatile okuma işlemleri vurgulandı**
        // head ve tail değerleri volatile olarak okunur.
        let head = read_volatile(self.head);
        let tail = read_volatile(self.tail);

        // **İyileştirme 7: Tampon boşluk kontrolü daha belirginleştirildi**
        // Tamponun boş olup olmadığını kontrol eder.
        if head == tail {
            return None; // Tampon boş
        }

        let read_ptr = self.buffer.add(tail);
        // **İyileştirme 8: Veri okuma işlemi volatile olarak yapılıyor**
        // Mesaj verisi tampondan volatile olarak okunur. `read_volatile` direkt olarak bellekten okuma yapar,
        // bu da paylaşımlı bellek senaryolarında veri tutarlılığını sağlamak için önemlidir.
        let msg = read_volatile(read_ptr as *mut Message);

        // **İyileştirme 9: tail değeri volatile olarak güncelleniyor**
        // tail değeri güncellenirken volatile yazma işlemi kullanılır.
        write_volatile(self.tail, (tail + core::mem::size_of::<Message>()) % self.capacity);
        Some(msg)
    }
}

// Mesaj gönderme fonksiyonu
pub fn send_message(buffer: &mut RingBuffer, msg: &Message) -> bool {
    unsafe { buffer.push(msg) }
}

// Mesaj alma fonksiyonu
pub fn receive_message(buffer: &mut RingBuffer) -> Option<Message> {
    unsafe { buffer.pop() }
}

fn main() {
    unsafe {
        // **İyileştirme 10: Buffer oluşturulurken kapasite doğru hesaplandı**
        // head ve tail için yer ayırıldıktan sonra geri kalan alan kapasite olarak belirlenir.
        let buffer_size = SHARED_MEMORY.len();
        let metadata_size = core::mem::size_of::<usize>() * 2; // head ve tail için boyut
        let buffer_capacity = buffer_size - metadata_size;

        let mut buffer = RingBuffer::new(SHARED_MEMORY.as_mut_ptr().add(metadata_size), buffer_capacity); // metadata alanını atla

        let msg = Message {
            sender: 1,
            receiver: 2,
            message_type: 10,
            data: [1, 2, 3, 4],
        };

        if send_message(&mut buffer, &msg) {
            // **İyileştirme 11: println! yerine debug çıktısı veya loglama mekanizması kullanılmalı**
            // println!("Mesaj gönderildi."); // no_std ortamda println! doğrudan çalışmaz
            // Burada gerçek bir no_std ortamında çalışacak bir loglama mekanizması veya debug arayüzü kullanılmalı.
            // Örneğin: debug_print!("Mesaj gönderildi.\r\n"); gibi bir fonksiyon kullanılabilir.
            // Şimdilik yorum satırı olarak bırakılıyor.
            // Örneğin, bir UART üzerinden çıktı vermek için özelleştirilmiş bir fonksiyon kullanılabilir.
            // debug_print!("Mesaj gönderildi.\r\n");
        }

        if let Some(received_msg) = receive_message(&mut buffer) {
            // println!("Alınan mesaj: {:?}", received_msg);
            // debug_print!("Alınan mesaj: {:?}\r\n", received_msg);
        }
    }
    // Döngü sonu, program burada sonlanır (main fonksiyonu no_std'de tipik olarak programın giriş noktasıdır).
}