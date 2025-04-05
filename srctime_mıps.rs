#![no_std]

use core::arch::asm;

// CP0 register adresleri
const CP0_STATUS: u32 = 12;
const CP0_CAUSE: u32 = 13;
const CP0_COMPARE: u32 = 11;
const CP0_COUNT: u32 = 9;

// Interrupt masks and status bits
const STATUS_ENABLE_INTERRUPTS_BIT: u32 = 0x00010000; // Bit to enable interrupts in Status register

// Timer configuration
const TIMER_INTERVAL_CYCLES: u32 = 1000000; // Target interval in CPU cycles

static mut TICKS: u64 = 0;

#[no_mangle]
extern "C" fn timer_interrupt_handler() {
    unsafe {
        TICKS += 1;

        // Set next timer interrupt (acknowledges current interrupt by setting up the next event)
        let current_count: u32;
        asm!("mfc0 $t0, $9", out("$t0") current_count, in("$9") CP0_COUNT);
        let next_compare = current_count.wrapping_add(TIMER_INTERVAL_CYCLES);
        asm!("mtc0 $t0, $11", in("$t0") next_compare, in("$11") CP0_COMPARE);
    }
}

pub fn init() {
    unsafe {
        // Enable global interrupts and timer interrupts in CP0 Status register
        asm!("mfc0 $t0, $12", out("$t0") _, in("$12") CP0_STATUS);
        asm!("ori $t0, $t0, {}", out("$t0") _, const STATUS_ENABLE_INTERRUPTS_BIT); // Enable interrupts
        asm!("mtc0 $t0, $12", in("$t0") _, in("$12") CP0_STATUS);

        // Set initial timer compare value
        let current_count: u32;
        asm!("mfc0 $t0, $9", out("$t0") current_count, in("$9") CP0_COUNT);
        let next_compare = current_count.wrapping_add(TIMER_INTERVAL_CYCLES);
        asm!("mtc0 $t0, $11", in("$t0") next_compare, in("$11") CP0_COMPARE);
    }

    // Kesme işleyicisini ayarla (burası platforma özgü olmalı)
    // ...
}

pub fn ticks() -> u64 {
    unsafe { TICKS }
}

pub fn delay(ms: u64) {
    let target_ticks = ticks() + ms;
    while ticks() < target_ticks {}
}