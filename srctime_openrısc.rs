#![no_std]
use crate::interrupt;
use crate::drivers::opbis;
use crate::platform;
use core::sync::atomic::{AtomicU64, Ordering};

static TICKS: AtomicU64 = AtomicU64::new(0);

// Define a constant for the timer interval to improve readability and maintainability.
const TIMER_INTERVAL_MS: u64 = 10;

#[no_mangle]
extern "C" fn timer_interrupt_handler() {
    TICKS.fetch_add(1, Ordering::SeqCst);
    opbis::clear_timer_interrupt();
    opbis::set_timer(platform::hart_id(), TIMER_INTERVAL_MS); // Use the defined constant
}

pub fn init() {
    interrupt::set_interrupt_handler(platform::timer_interrupt_number(), timer_interrupt_handler);
    opbis::set_timer(platform::hart_id(), TIMER_INTERVAL_MS); // Use the defined constant for initial timer setup
}

pub fn ticks() -> u64 {
    TICKS.load(Ordering::SeqCst)
}

pub fn delay(ms: u64) {
    let target_ticks = ticks() + ms;
    while ticks() < target_ticks {}
}