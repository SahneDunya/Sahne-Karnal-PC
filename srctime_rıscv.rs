#![no_std]
use core::sync::atomic::{AtomicU64, Ordering};

use crate::interrupt;
use crate::drivers::clint;
use crate::platform;

static TICKS: AtomicU64 = AtomicU64::new(0);

#[no_mangle]
extern "C" fn timer_interrupt_handler() {
    TICKS.fetch_add(1, Ordering::Relaxed); // Atomically increment TICKS
    unsafe {
        asm!("csrw mip, zero"); // Kesme bayrağını temizle
        clint::set_timer(platform::hart_id(), 10); // 10ms sonra tekrar kesme
    }
}

pub fn init() {
    interrupt::set_interrupt_handler(7, timer_interrupt_handler);
    unsafe {
        clint::set_timer(platform::hart_id(), 10); // İlk kesmeyi ayarla
    }
}

pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed) // Atomically load TICKS value
}

pub fn delay(ms: u64) {
    let target_ticks = ticks() + ms;
    while ticks() < target_ticks {}
}