#![no_std]

use core::arch::asm;

// OpenSBI'deki SBI çağrı numaraları (aynı kalır)
const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;
const SBI_CLEAR_IPI: usize = 3;
const SBI_SEND_IPI: usize = 4;
const SBI_REMOTE_FENCE_I: usize = 5;
const SBI_REMOTE_SFENCE_VMA: usize = 6;
const SBI_REMOTE_SFENCE_VMA_ASID: usize = 7;
const SBI_SHUTDOWN: usize = 8;
const SBI_HART_SUSPEND: usize = 10;

// SBI çağrısı yapan yardımcı fonksiyon (aynı kalır)
#[inline(always)]
fn sbi_call(eid: usize, fid: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let mut ret;
    unsafe {
        asm!(
            "ecall",
            inout("a0") arg0 => ret,
            in("a1") arg1,
            in("a2") arg2,
            in("a6") fid,
            in("a7") eid,
        );
    }
    ret
}

// Hart'ı askıya alan fonksiyon (iyileştirilmiş - hata kontrolü eklendi)
fn suspend_hart(suspend_type: u32, resume_addr: usize) -> Result<(), usize> {
    let result = sbi_call(SBI_HART_SUSPEND, 0, suspend_type as usize, resume_addr, 0);
    if result == 0 {
        Ok(()) // Başarılı askıya alma
    } else {
        Err(result) // Hata durumunda, hata kodunu döndür
    }
}

// Bir karakteri konsola yazdıran fonksiyon (aynı kalır)
fn console_putchar(c: char) {
    sbi_call(SBI_CONSOLE_PUTCHAR, 0, c as usize, 0, 0);
}

// Test fonksiyonu (main yerine)
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Test mesajı (aynı kalır)
    let message = "Askıya alınıyor...\n";
    for c in message.chars() {
        console_putchar(c);
    }

    // Hart'ı askıya al ve sonucu kontrol et
    match suspend_hart(1, 0) {
        Ok(()) => {
            // Başarılı askıya alma durumunda yapılacak bir şey yok, çünkü hart askıya alınacak.
        }
        Err(error_code) => {
            // Eğer askıya alma başarısız olursa hata mesajı
            let error_message = "Askıya alma başarısız! Hata kodu: ";
            for c in error_message.chars() {
                console_putchar(c);
            }
            // Hata kodunu konsola yazdır (isteğe bağlı, hata kodunu sayıya dönüştürmek gerekebilir)
            let error_code_str = format!("{}", error_code); // Format makrosunu kullanabilmek için core::fmt eklenebilir veya basit bir sayı-karakter dönüşümü yapılabilir. Bu örnekte format kullanılıyor.
            for c in error_code_str.chars() {
                console_putchar(c);
            }
            console_putchar('\n');
        }
    }

    loop {} // Sonsuz döngü (her durumda, askıya alma başarılı olsa da olmasa da)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}