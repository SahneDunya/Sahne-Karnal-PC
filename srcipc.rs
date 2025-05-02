#![no_std]
#[allow(dead_code)] // Keep if needed

use core::ptr::{read_volatile, write_volatile, self}; // Added ptr module for clarity on null pointer checks, write_bytes (example)
use core::mem; // Added for size_of, align_of

// Message yapısı
// #[repr(C)] ensures C compatibility and a predictable layout.
// Copy and Clone are needed for passing by value.
// Debug for easy printing.
#[repr(C)]
#[derive(Debug, Copy, Clone)] // Debug eklendi, hata ayıklamayı kolaylaştırır
pub struct Message {
    sender: u32,
    receiver: u32,
    message_type: u32,
    data: [u8; 4], // Fixed size data
    // Note: For variable size data, a pointer/length or other mechanism is needed.
}

// Paylaşımlı bellek bölgesi (Bu, işletim sistemi tarafından ayrılmalı veya yönetilmelidir)
// static mut kullanımı TEHLİKELİDİR ve sadece tekil erişim garanti edildiğinde veya
// dikkatli unsafe bloklarla yönetildiğinde güvenlidir. Yarış durumlarına açıktır.
// Gerçek bir IPC sisteminde, bu bellek ya OS (Sahne64) tarafından Paylaşımlı Bellek API'si ile
// sağlanmalı ya da daha gelişmiş senkronizasyon mekanizmaları kullanılmalıdır.
// Bu örnek, düşük seviyeli bir ring buffer implementasyonunu gösterir.
static mut SHARED_MEMORY: [u8; 1024] = [0; 1024];

// Döngüsel tampon yapısı (Paylaşımlı bellek üzerinde çalışacak şekilde tasarlanmıştır)
pub struct RingBuffer {
    buffer_start: *mut u8, // Tamponun başlangıç adresi (mesaj verilerinin başladığı yer)
    capacity: usize,       // Tamponun byte cinsinden kapasitesi (metadata hariç)
    head: *mut usize,      // Bir sonraki yazılacak pozisyonun indeksi
    tail: *mut usize,      // Bir sonraki okunacak pozisyonun indeksi
    // Note: head ve tail gerçekte tamponun başlangıcına göre offset'lerdir.
    // Pointerlar volatile erişim için tutulur.
}

impl RingBuffer {
    /// Güvenli olmayan (unsafe) new fonksiyonu.
    /// Paylaşımlı bellek bölgesinin başlangıç pointer'ını ve toplam byte boyutunu alır.
    /// Head ve tail sayaçları için bu belleğin başından yer ayırır ve geri kalanı mesaj tamponu olarak kullanır.
    ///
    /// # Güvenlik
    /// Çağıran kişi şunlardan sorumludur:
    /// 1. `shared_memory_ptr` geçerli, `shared_memory_size` boyutunda okunabilir/yazılabilir belleği işaret etmelidir.
    /// 2. Aynı bellek bölgesine eş zamanlı olarak birden fazla `RingBuffer` mutable referansının oluşmadığından emin olunmalıdır.
    /// 3. Bellek boyutu (`shared_memory_size`), head ve tail sayaçları için yeterli alana sahip olmalıdır.
    pub unsafe fn new(shared_memory_ptr: *mut u8, shared_memory_size: usize) -> Option<Self> {
        // **İyileştirme 1: Buffer geçerlilik ve boyut kontrolü**
        // shared_memory_ptr'in null olup olmadığını kontrol et.
        if shared_memory_ptr.is_null() {
             #[cfg(not(feature = "std"))] eprintln!("RingBuffer::new: Shared memory pointer cannot be null");
            return None; // Geçersiz pointer
        }

        // Head ve tail için gereken toplam boyut (usize tipinin boyutu * 2)
        let metadata_size = mem::size_of::<usize>().checked_mul(2).unwrap_or(usize::MAX);

        // Bellek boyutu metadata için yeterli mi kontrol et
        if shared_memory_size < metadata_size {
             // #[cfg(not(feature = "std"))] eprintln!("RingBuffer::new: Shared memory size ({}) is too small for metadata ({})", shared_memory_size, metadata_size);
             return None; // Yetersiz boyut
        }

        // Mesaj verileri için kullanılacak tamponun başlangıç adresi ve boyutu
        let buffer_start = shared_memory_ptr.add(metadata_size); // Metadata alanını atla
        let buffer_capacity = shared_memory_size.checked_sub(metadata_size).unwrap_or(0); // Güvenli çıkarma

         // **İyileştirme 1.1: Tampon kapasitesinin mesaj boyutu için yeterli olduğundan emin olun**
         // En az bir mesaj sığmalı.
        if buffer_capacity < mem::size_of::<Message>() {
              #[cfg(not(feature = "std"))] eprintln!("RingBuffer::new: Buffer capacity ({}) is too small for a Message ({})", buffer_capacity, mem::size_of::<Message>());
             return None;
        }

         // Head ve tail sayaçlarının bellekteki adresleri (paylaşımlı belleğin başında)
        let head_ptr = shared_memory_ptr as *mut usize;
        let tail_ptr = shared_memory_ptr.add(mem::size_of::<usize>()) as *mut usize;

         // **İyileştirme 1.2: Head ve tail sayaçlarını 0 olarak başlat (Sadece ilk oluşturmada)**
         // Eğer paylaşımlı bellek sıfırlandıysa veya ilk kez kullanılıyorsa yapılmalı.
         // Güvenli başlatma için harici bir 'init' fonksiyonu veya versiyonlama gerekebilir.
         // Bu örnekte, new çağrıldığında her zaman sıfırlanacağını varsayıyoruz (Dikkatli kullanılmalı!).
        ptr::write_volatile(head_ptr, 0); // Volatile başlatma
        ptr::write_volatile(tail_ptr, 0); // Volatile başlatma


        Some(RingBuffer {
            buffer_start,
            capacity: buffer_capacity,
            head: head_ptr,
            tail: tail_ptr,
        })
    }

    /// Mesaj tamponu boşsa true döner.
    #[inline] // Performans için inline edilebilir
    unsafe fn is_empty(&self) -> bool {
         // **Volatile okuma**
        read_volatile(self.head) == read_volatile(self.tail)
    }

     /// Mesaj tamponu doluysa true döner (yeni mesaj için yer yok).
     #[inline] // Performans için inline edilebilir
    unsafe fn is_full(&self) -> bool {
        // **Volatile okuma**
         let head = read_volatile(self.head);
         let tail = read_volatile(self.tail);
         let msg_size = mem::size_of::<Message>();

         // **Tampon doluluk kontrolü**
         (head.wrapping_add(msg_size) % self.capacity) == tail
    }


    /// Mesajı tampona ekler.
    ///
    /// # Güvenlik
    /// Bu fonksiyon güvenli değildir çünkü paylaşımlı belleğe volatile olarak yazma yapar.
    /// Eş zamanlı erişimde harici senkronizasyon (örn. kilitler) gereklidir.
    ///
    /// # Dönüş
    /// Başarılıysa `true` (mesaj eklendi), tampon doluysa `false`.
    pub unsafe fn push(&mut self, msg: &Message) -> bool {
        // **İyileştirme 2 & 3: Tampon dolu kontrolü is_full fonksiyonunu kullandı**
         if self.is_full() {
             return false; // Tampon dolu
         }

         // **İyileştirme 4: Veri kopyalama hedef pointer'ı hesaplandı**
         // buffer_start + head offset
         let head_offset = read_volatile(self.head); // Volatile okuma
         let write_ptr = self.buffer_start.add(head_offset); // Güvenli offset ekleme varsayılır


         // **İyileştirme 4.1: Mesaj verisi kopyalama**
         // copy_nonoverlapping kullanılır, msg ve write_ptr bölgeleri çakışmamalıdır.
         // msg referansının RingBuffer tamponuyla çakışmadığı varsayılır (genellikle böyledir).
        let msg_size = mem::size_of::<Message>();
         core::ptr::copy_nonoverlapping(msg as *const Message as *const u8, write_ptr, msg_size);

        // **İyileştirme 5: head değeri volatile olarak güncelleniyor**
        // head değeri yeni yazılan yerin sonrasını gösterecek şekilde güncellenir.
        let new_head = (head_offset.wrapping_add(msg_size)) % self.capacity; // Güvenli toplama
        write_volatile(self.head, new_head); // Volatile yazma
        true
    }

    /// Tampondan bir mesaj okur.
    ///
    /// # Güvenlik
    /// Bu fonksiyon güvenli değildir çünkü paylaşımlı bellekten volatile olarak okuma yapar.
    /// Eş zamanlı erişimde harici senkronizasyon (örn. kilitler) veya tek okuyucu/tek yazıcı garantisi gereklidir.
    ///
    /// # Dönüş
    /// Tampon boş değilse `Some(Message)`, tampon boşsa `None`.
    pub unsafe fn pop(&mut self) -> Option<Message> {
        // **İyileştirme 6 & 7: Tampon boş kontrolü is_empty fonksiyonunu kullandı**
        if self.is_empty() {
            return None; // Tampon boş
        }

        // **İyileştirme 8: Veri okuma kaynak pointer'ı hesaplandı**
        // buffer_start + tail offset
        let tail_offset = read_volatile(self.tail); // Volatile okuma
        let read_ptr = self.buffer_start.add(tail_offset); // Güvenli offset ekleme varsayılır


        // **İyileştirme 8.1: Mesaj verisi okuma (volatile)**
        // read_volatile kullanılır. Pointer'ın Message struct'ı için doğru hizalandığı varsayılır!
        // Eğer buffer_start adresi Message'ın hizalama gereksinimini karşılamıyorsa
        // veya tail_offset bu hizalamayı bozuyorsa BURASI GÜVENLİ DEĞİLDİR.
        // Güvenli bir ring buffer implementasyonu, mesaj boyutunu ve hizalamasını
        // dikkate alarak buffer'daki slotları yönetir.
        let msg = read_volatile(read_ptr as *const Message); // <-- Pointer cast ve volatile okuma


        // **İyileştirme 9: tail değeri volatile olarak güncelleniyor**
        // tail değeri okunan mesajın sonrasını gösterecek şekilde güncellenir.
         let msg_size = mem::size_of::<Message>();
        let new_tail = (tail_offset.wrapping_add(msg_size)) % self.capacity; // Güvenli toplama
        write_volatile(self.tail, new_tail); // Volatile yazma
        Some(msg)
    }

    /// Tamponda kaç byte boş yer olduğunu döndürür (yaklaşık).
    /// Eş zamanlı erişimde kilit olmadan doğru değer garanti edilmez.
    pub unsafe fn available_space(&self) -> usize {
         let head = read_volatile(self.head);
         let tail = read_volatile(self.tail);
         let msg_size = mem::size_of::<Message>();

         if head >= tail {
             self.capacity - (head - tail) - msg_size // Son mesaj için yer + boşluk
         } else {
             // Döngüsel durumda boşluk
             tail - head - msg_size
         }
         // Note: This is a simplified space calculation. A proper implementation tracks message slots, not just bytes.
    }

     /// Tamponda kaç byte veri olduğunu döndürür (yaklaşık).
     /// Eş zamanlı erişimde kilit olmadan doğru değer garanti edilmez.
    pub unsafe fn used_space(&self) -> usize {
         let head = read_volatile(self.head);
         let tail = read_volatile(self.tail);

         if head >= tail {
             head - tail
         } else {
             self.capacity - tail + head
         }
          // Note: This is a simplified space calculation. A proper implementation tracks message slots, not just bytes.
    }
}

// Mesaj gönderme fonksiyonu
/// Tampona bir mesaj göndermeyi dener.
/// Güvenli değildir, eş zamanlı erişimde dikkatli olunmalı.
pub fn send_message(buffer: &mut RingBuffer, msg: &Message) -> bool {
    // unsafe blok, RingBuffer'ın push fonksiyonunun unsafe olmasından kaynaklanır.
    // Bu fonksiyonun dışarıdan çağrılması hala unsafe'dir, bu da kullanıcının
    // senkronizasyon sorumluluğunu vurgular.
    unsafe { buffer.push(msg) }
}

// Mesaj alma fonksiyonu
/// Tampondan bir mesaj almayı dener.
/// Güvenli değildir, eş zamanlı erişimde dikkatli olunmalı.
pub fn receive_message(buffer: &mut RingBuffer) -> Option<Message> {
    // unsafe blok, RingBuffer'ın pop fonksiyonunun unsafe olmasından kaynaklanır.
    // Bu fonksiyonun dışarıdan çağrılması hala unsafe'dir.
    unsafe { buffer.pop() }
}

// Örnek Kullanım
// no_std ortamında main fonksiyonu programın giriş noktası olmayabilir
// ve çıktılar için özel makrolar gerektirir.
#[cfg(feature = "std")] // Sadece standart kütüphane varsa derlenir
fn main() {
    // Bu örnek std ortamında çalışacak şekilde yapılandırılmıştır.
    // no_std ortamında Sahne64'ün çıktı makroları veya özel bir çıktı mekanizması gerektirir.

     #[cfg(feature = "std")] std::println!("Sahne64 Basit IPC (RingBuffer) Örneği (std)");
     #[cfg(not(feature = "std"))] println!("Sahne64 Basit IPC (RingBuffer) Örneği (no_std)");


    unsafe {
        // **İyileştirme 10: Buffer oluşturulurken kapasite ve pointer doğru hesaplandı**
        // Head ve tail sayaçları için paylaşımlı belleğin başı kullanılır.
        let shared_memory_ptr = SHARED_MEMORY.as_mut_ptr();
        let shared_memory_size = SHARED_MEMORY.len();

        let ring_buffer_option = RingBuffer::new(shared_memory_ptr, shared_memory_size);

        match ring_buffer_option {
            Some(mut buffer) => {
                // Tampon başarıyla oluşturuldu
                #[cfg(feature = "std")] std::println!("RingBuffer başarıyla oluşturuldu. Kapasite: {} byte", buffer.capacity);
                #[cfg(not(feature = "std"))] println!("RingBuffer başarıyla oluşturuldu. Kapasite: {} byte", buffer.capacity);


                let msg1 = Message {
                    sender: 1,
                    receiver: 2,
                    message_type: 10,
                    data: [1, 2, 3, 4],
                };
                 let msg2 = Message {
                    sender: 3,
                    receiver: 4,
                    message_type: 11,
                    data: [5, 6, 7, 8],
                };


                // Mesaj gönderme
                #[cfg(feature = "std")] std::println!("Mesaj 1 gönderiliyor...");
                #[cfg(not(feature = "std"))] println!("Mesaj 1 gönderiliyor...");
                if send_message(&mut buffer, &msg1) {
                    #[cfg(feature = "std")] std::println!("Mesaj 1 gönderildi.");
                    #[cfg(not(feature = "std"))] println!("Mesaj 1 gönderildi.");
                } else {
                     #[cfg(feature = "std")] std::eprintln!("Mesaj 1 gönderilemedi, tampon dolu.");
                     #[cfg(not(feature = "std"))] eprintln!("Mesaj 1 gönderilemedi, tampon dolu.");
                }

                 // Başka bir mesaj gönderme
                #[cfg(feature = "std")] std::println!("Mesaj 2 gönderiliyor...");
                #[cfg(not(feature = "std"))] println!("Mesaj 2 gönderiliyor...");
                 if send_message(&mut buffer, &msg2) {
                    #[cfg(feature = "std")] std::println!("Mesaj 2 gönderildi.");
                    #[cfg(not(feature = "std"))] println!("Mesaj 2 gönderildi.");
                 } else {
                    #[cfg(feature = "std")] std::eprintln!("Mesaj 2 gönderilemedi, tampon dolu.");
                    #[cfg(not(feature = "std"))] eprintln!("Mesaj 2 gönderilemedi, tampon dolu.");
                 }


                // Mesaj alma
                #[cfg(feature = "std")] std::println!("Mesaj alınıyor...");
                #[cfg(not(feature = "std"))] println!("Mesaj alınıyor...");
                if let Some(received_msg) = receive_message(&mut buffer) {
                     #[cfg(feature = "std")] std::println!("Alınan mesaj: {:?}", received_msg);
                     #[cfg(not(feature = "std"))] println!("Alınan mesaj: {:?}", received_msg);
                     // Mesaj 1 bekleniyor
                     assert_eq!(received_msg.sender, 1);
                } else {
                     #[cfg(feature = "std")] std::println!("Tampon boş, mesaj alınamadı.");
                     #[cfg(not(feature = "std"))] println!("Tampon boş, mesaj alınamadı.");
                }

                // İkinci mesajı alma
                #[cfg(feature = "std")] std::println!("İkinci mesaj alınıyor...");
                #[cfg(not(feature = "std"))] println!("İkinci mesaj alınıyor...");
                if let Some(received_msg) = receive_message(&mut buffer) {
                     #[cfg(feature = "std")] std::println!("Alınan mesaj: {:?}", received_msg);
                     #[cfg(not(feature = "std"))] println!("Alınan mesaj: {:?}", received_msg);
                     // Mesaj 2 bekleniyor
                     assert_eq!(received_msg.sender, 3);
                } else {
                     #[cfg(feature = "std")] std::println!("Tampon boş, ikinci mesaj alınamadı.");
                     #[cfg(not(feature = "std"))] println!("Tampon boş, ikinci mesaj alınamadı.");
                }

                // Tamponun artık boş olması beklenir
                 #[cfg(feature = "std")] std::println!("Tekrar mesaj alınıyor (tampon boş olmalı)...");
                #[cfg(not(feature = "std"))] println!("Tekrar mesaj alınıyor (tampon boş olmalı)...");
                 if receive_message(&mut buffer).is_none() {
                     #[cfg(feature = "std")] std::println!("Tampon boş olduğu doğrulandı.");
                     #[cfg(not(feature = "std"))] println!("Tampon boş olduğu doğrulandı.");
                 } else {
                     #[cfg(feature = "std")] std::eprintln!("Hata: Tampon boş bekleniyordu ama mesaj alındı.");
                     #[cfg(not(feature = "std"))] eprintln!("Hata: Tampon boş bekleniyordu ama mesaj alındı.");
                 }


            }
            None => {
                // Tampon oluşturulamadı (boyut yetersiz vs.)
                #[cfg(feature = "std")] std::eprintln!("RingBuffer oluşturulamadı.");
                #[cfg(not(feature = "std"))] eprintln!("RingBuffer oluşturulamadı.");
            }
        }
    }
    // Döngü sonu
}

// no_std ortamında main fonksiyonu programın giriş noktası olmayabilir.
// Bu dosya bir kütüphane parçasıysa, main fonksiyonu sadece örnek veya test içindir.
 #[cfg(not(feature = "std"))]
 fn main() {
//     // no_std entry point here
 }
