use core::fmt;
use spin::Mutex;

// EFI Sistem Tablosu (gerekli yapılar burada tanımlanır)
#[repr(C)]
pub struct SystemTable {
    _unused: [u8; 8],
    pub con_out: *mut ConsoleOutput, // Konsol çıktı arayüzüne işaretçi
    _unused1: [u8; 40],
}

// Konsol Çıktı Arayüzü (metin çıktı fonksiyonunu içerir)
#[repr(C)]
pub struct ConsoleOutput {
    _unused: *mut (),
    pub output_string: OutputStringFunc, // Metin çıktı fonksiyonu
    _unused1: [u8; 20],
}

// OutputString fonksiyonunun tipi tanımlanıyor
type OutputStringFunc = unsafe extern "system" fn(
    *mut ConsoleOutput,
    *const u16,
    usize,
) -> usize;

// EFI Sistem Tablosuna global erişim (Mutex ile güvenli erişim)
pub static ST: Mutex<Option<&'static SystemTable>> = Mutex::new(None);

// Konsola çıktı yazmak için makro
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ({
        $crate::drivers::efi::efi_println(format_args!($($arg)*));
    });
}

// EFI yazıcısı yapısı (fmt::Write trait'ini uygular)
pub struct EfiWriter;

impl fmt::Write for EfiWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        // Sistem tablosuna eriş ve konsola yaz
        if let Some(st_ref) = ST.lock().as_ref() {
            let con_out = st_ref.con_out;
            let output_string = unsafe { (*con_out).output_string };

            // UTF-16'ya dönüştür ve null sonlandırıcı ekle
            let mut utf16_buf: Vec<u16> = s.encode_utf16().collect();
            utf16_buf.push(0); // EFI OutputString fonksiyonu null sonlandırılmış dize bekler

            // Güvenli olmayan (unsafe) fonksiyon çağrısı
            unsafe {
                output_string(con_out, utf16_buf.as_ptr(), utf16_buf.len() - 1);
            }
        }
        Ok(())
    }
}

// EFI konsoluna formatlanmış çıktı yazma fonksiyonu
pub fn efi_println(args: fmt::Arguments) {
    use fmt::Write;
    let mut writer = EfiWriter;
    writer.write_fmt(args).unwrap(); // Hata durumunda unwrap (basit örnek için)
}

// EFI'yi başlatma fonksiyonu
pub fn init(system_table: &'static SystemTable) {
    *ST.lock() = Some(system_table);
    println!("EFI başlatıldı.");
}