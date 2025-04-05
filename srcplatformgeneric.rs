#![no_std]
use core::arch::asm;

pub fn hart_id() -> usize {
    let mut hartid: usize;
    unsafe {
        asm!("mv {}, tp", out(reg) hartid);
    }
    hartid
}