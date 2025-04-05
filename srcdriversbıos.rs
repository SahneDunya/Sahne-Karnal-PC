use core::fmt;

// BIOS teletekst modu için video belleği adresi
const VIDEO_MEMORY_ADDRESS: *mut u8 = 0xB8000 as *mut u8;

// Ekran boyutları (80x25 tipik teletekst modu)
const SCREEN_WIDTH: usize = 80;
const SCREEN_HEIGHT: usize = 25;

// Renk kodları için enum
#[allow(dead_code)]
#[repr(u8)]
pub enum ColorCode {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

// Renk kodunu ve arka plan rengini birleştiren fonksiyon
pub fn make_color_code(foreground: ColorCode, background: ColorCode) -> u8 {
    (background as u8) << 4 | (foreground as u8)
}

// Varsayılan renk kodu: Açık gri yazı, siyah arka plan
const DEFAULT_COLOR: u8 = make_color_code(ColorCode::LightGray, ColorCode::Black);

// Ekrana bir karakter yazdırma fonksiyonu (row ve col sınır kontrolü ile)
pub fn write_char(c: char, row: u8, col: u8, color_code: u8) {
    if row < SCREEN_HEIGHT as u8 && col < SCREEN_WIDTH as u8 {
        let index = (row as usize * SCREEN_WIDTH) + col as usize;
        unsafe {
            let video_mem_ptr = VIDEO_MEMORY_ADDRESS.add(index * 2);
            *video_mem_ptr = c as u8; // Karakter
            *video_mem_ptr.add(1) = color_code; // Renk
        }
    }
}

// Ekrana bir dize yazdırma fonksiyonu (row ve col sınır kontrolü ile)
pub fn write_string(s: &str, row: u8, col: u8, color_code: u8) {
    let mut current_col = col;
    let mut current_row = row;
    for c in s.chars() {
        match c {
            '\n' => { // Yeni satır karakteri işleme
                current_row += 1;
                current_col = 0;
                if current_row >= SCREEN_HEIGHT as u8 { // Ekran sonuna ulaşıldıysa aşağı kaydırma (basit örnekte kaydırma yok, satır sonuna gelinirse durur)
                    return; // Veya ekranı kaydırma fonksiyonu çağrılabilir
                }
            },
            _ => {
                write_char(c, current_row, current_col, color_code);
                current_col += 1;
                if current_col >= SCREEN_WIDTH as u8 {
                    current_col = 0;
                    current_row += 1;
                    if current_row >= SCREEN_HEIGHT as u8 {
                        return; // Veya ekranı kaydırma fonksiyonu çağrılabilir
                    }
                }
            }
        }

    }
}

// Format! makrosu için destek (BiosWriter yapısı üzerinden renk kodu ile)
impl fmt::Write for BiosWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_string(s, self.row, self.col, self.color_code);
        // `write_string` fonksiyonu satır ve sütun yönetimini yapıyor, burada sadece sütunu güncellemeye gerek yok.
        // `BiosWriter` yapısında satır ve sütun takibi yapılması daha uygun olabilir,
        // ancak bu örnekte `write_string` fonksiyonunun yönetimi daha basit tutulmuştur.
        Ok(())
    }
}

// Ekrana formatlı çıktı yazdırmak için bir yapı (renk kodu ile)
pub struct BiosWriter {
    pub row: u8,
    pub col: u8,
    pub color_code: u8, // Renk kodu eklendi
}

// BiosWriter için yeni bir örnek oluşturma fonksiyonu (varsayılan renk kodu ile)
pub fn new_writer(row: u8, col: u8) -> BiosWriter {
    BiosWriter { row, col, color_code: DEFAULT_COLOR }
}

// BiosWriter için yeni bir örnek oluşturma fonksiyonu (belirtilen renk kodu ile)
pub fn new_writer_with_color(row: u8, col: u8, color_code: ColorCode) -> BiosWriter {
    BiosWriter { row, col, color_code: make_color_code(color_code, ColorCode::Black) } // Örnekte arka plan siyah yapılıyor
}

// Örnek kullanım fonksiyonu
pub fn example_usage() {
    let mut writer = new_writer(0, 0); // Varsayılan renklerle writer oluştur
    write_string("Merhaba, Rust BIOS dünyası!\n", 0, 0, DEFAULT_COLOR); // Direkt fonksiyon ile yazdırma
    write_string("Alt satıra geçtim.\n", 1, 0, DEFAULT_COLOR);

    let mut colored_writer = new_writer_with_color(5, 0, ColorCode::LightGreen); // Yeşil renkli writer
    write_string("Yeşil renk ile yazıyorum!\n", 5, 0, colored_writer.color_code);

    use core::fmt::Write; // fmt::Write trait'ini kullanmak için import

    let mut fmt_writer = new_writer_with_color(10, 0, ColorCode::Yellow); // Sarı renkli writer
    fmt::write(&mut fmt_writer, format_args!("Format! makrosu ile sarı renk {}\n", 123)).unwrap(); // format! makrosu ile yazdırma
}

// Ana fonksiyon (örn: #[no_mangle] pub extern "C" fn _start() { ... } içinde kullanılabilir)
fn main() {
    example_usage();
    // Döngüye girerek işlemciyi durdur (BIOS ortamında genellikle sonsuz döngü kullanılır)
    loop {}
}