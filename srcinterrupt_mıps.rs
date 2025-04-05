#![no_std]

use core::arch::asm;
use core::ptr;

// MIPS'te kesme bitleri (Örnek: MIPS32 için)
const KERNEL_MODE_BIT: u32 = 0x80000000; // Çekirdek modu biti (Status register'da KU bitleri)
const GLOBAL_INTERRUPT_ENABLE_BIT: u32 = 0x00000002; // Genel kesme etkinleştirme biti (Status register'da IE biti - bit 1)
const TIMER_INTERRUPT_BIT: u32 = 0x00000008; // Zamanlayıcı kesme biti (Cause register'da IP[2] biti) - DONANIMA ÖZGÜ!
const USB_INTERRUPT_BIT: u32 = 0x00000020; // Örnek USB kesme biti (Cause register'da IP[5] biti) - DONANIMA ÖZGÜ!

extern "C" {
    static IVT: u32; // Kesme vektör tablosu başlangıç adresi (linker script ile ayarlanmalı)
}

pub fn init() {
    unsafe {
        // Kesme vektör tablosu adresini ayarla (MIPS'te EBase register'ına yazılır - CP0 register 12)
        let ivt_address = &IVT as *const u32 as u32;
        asm!(
            "mtc0 $12, {0}", // CP0 register 12'ye (EBase) IVT adresini yükle
            in(reg) ivt_address
        );

        // Kesme maskesini ayarla (Status register'daki IM (Interrupt Mask) bitlerini kullanarak zamanlayıcı ve USB kesmelerini etkinleştir).
        // Cause register'daki IP (Interrupt Pending) bitleri, hangi kesmelerin beklediğini gösterir, maskeleme için kullanılmaz.
        // Status register'daki IM bitleri ile hangi kesmelerin *işlenmesini* istediğimizi belirtiriz.
        let interrupt_mask = TIMER_INTERRUPT_BIT | USB_INTERRUPT_BIT; // DONANIMA ÖZGÜ DEĞERLER! Cause register IP bitleri ile eşleşmeli.

        // ÖNEMLİ: Kesme maskesi Status register'ın IM bitleri ile ayarlanır. Cause register'a YAZILMAZ!
        // Yanlış: asm!("mtc0 $cause, {0}", in(reg) interrupt_mask); // BU YANLIŞ! Cause register salt okunur (çoğunlukla).

        // Doğru yaklaşım: Status register'ı oku, IM bitlerini ayarla ve geri yaz.
        asm!(
            "mfc0 $12, $status", // Status register'ı CP0 register 12'ye (geçici olarak) oku (kullanılabilecek herhangi bir register)
            "ori $12, $12, {0}", // CP0 register 12'deki değeri (eski Status) interrupt_mask ile OR'la (IM bitlerini ayarla)
            in(reg) interrupt_mask,
            "mtc0 $status, $12"  // Yeni Status değerini Status register'a geri yaz
        );


        // Genel kesmeleri etkinleştir (Status register'daki IE (Interrupt Enable) bitini ayarla - bit 1).
        asm!(
            "mfc0 $12, $status", // Status register'ı CP0 register 12'ye (geçici olarak) oku
            "ori $12, $12, {0}", // CP0 register 12'deki değeri (eski Status) GLOBAL_INTERRUPT_ENABLE_BIT ile OR'la (IE bitini ayarla)
            in(reg) GLOBAL_INTERRUPT_ENABLE_BIT,
            "mtc0 $status, $12"  // Yeni Status değerini Status register'a geri yaz
        );
    }
}

// Örnek USB kesme işleyicisi (DONANIMA ÖZGÜ UYGULAMA GEREKLİ)
#[no_mangle]
pub extern "C" fn usb_interrupt_handler() {
    // USB ile ilgili işlemleri burada gerçekleştirin.
    // Örneğin, veri okuma, veri gönderme, durum kontrolü vb.
    // BU KISIM DONANIMA ÖZGÜDÜR VE DETAYLI UYGULAMA GEREKTİRİR.
    // ... USB sürücü kodu ...

    // Kesme bayrağını temizleyin (DONANIMA ÖZGÜ). BU ÇOK ÖNEMLİDİR!
    // Aksi takdirde kesme sürekli tetiklenir.
    unsafe {
        // Örnek: USB durum kaydının ilgili bitini temizleme
        // volatile_store!(USB_STATUS_REGISTER, ...); // USB durum kaydını temizle
    }
}