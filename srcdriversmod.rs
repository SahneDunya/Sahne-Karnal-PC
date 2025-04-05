#![no_std]
pub mod usb;
pub mod keyboard;
pub mod clint;
pub mod pci;
pub mod disk;

pub trait Driver {
    fn init(&self);
}