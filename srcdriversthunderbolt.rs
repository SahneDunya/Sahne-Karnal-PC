#![no_std]
use core::ptr::write_volatile;
use crate::drivers::pci;
use crate::io::println;

// Harici kütüphaneden alınan sabitler ve yapılar (ilk kod bloğundan)
const THUNDERBOLT_BASE_ADDRESS: usize = 0x...; // Thunderbolt kontrolcüsünün temel adresi (Gerçek değer "CustomOS"a özel olacaktır)
const DEVICE_DISCOVERY_REGISTER: usize = THUNDERBOLT_BASE_ADDRESS + 0x100;
const DEVICE_COUNT_REGISTER: usize = THUNDERBOLT_BASE_ADDRESS + 0x104;
const THUNDERBOLT_CONTROLLER_STATUS_REGISTER: usize = THUNDERBOLT_BASE_ADDRESS + 0x00;
const THUNDERBOLT_CONTROLLER_CONTROL_REGISTER: usize = THUNDERBOLT_BASE_ADDRESS + 0x04;

#[repr(C)]
pub struct ThunderboltDevice {
    vendor_id: u16,
    device_id: u16,
}

#[derive(Debug, Copy, Clone)]
pub enum ThunderboltError {
    NoDataToSend,
    NoBufferToReceive,
    ControllerNotReady,
    DeviceDiscoveryFailed,
    SendDataFailed,
    ReceiveDataFailed,
}

// Harici kütüphaneden alınan fonksiyonlar (ilk kod bloğundan)
unsafe fn read_register(address: usize) -> u32 {
    (address as *const u32).read_volatile()
}

unsafe fn write_register(address: usize, value: u32) {
    (address as *mut u32).write_volatile(value);
}

pub fn init_controller() -> Result<(), ThunderboltError> {
    unsafe {
        let status = read_register(THUNDERBOLT_CONTROLLER_STATUS_REGISTER);
        if (status & 0x01) == 0 {
            return Err(ThunderboltError::ControllerNotReady);
        }
        write_register(THUNDERBOLT_CONTROLLER_CONTROL_REGISTER, 0x01);
        Ok(())
    }
}

pub fn discover_devices() -> Result<Vec<ThunderboltDevice>, ThunderboltError> {
    let mut devices = Vec::new();
    unsafe {
        let device_count = read_register(DEVICE_COUNT_REGISTER);
        for i in 0..device_count {
            let vendor_id_address = DEVICE_DISCOVERY_REGISTER + (i * 8) as usize;
            let device_id_address = vendor_id_address + 4;
            let vendor_id = read_register(vendor_id_address) as u16;
            let device_id = read_register(device_id_address) as u16;
            devices.push(ThunderboltDevice { vendor_id, device_id });
        }
    }
    if devices.is_empty() && unsafe { read_register(DEVICE_COUNT_REGISTER) } > 0 {
        return Err(ThunderboltError::DeviceDiscoveryFailed);
    }
    Ok(devices)
}

pub fn init() {
    println!("Thunderbolt başlatılıyor...");

    // Kontrolcüyü başlat
    match init_controller() {
        Ok(_) => println!("Thunderbolt kontrolcüsü başlatıldı."),
        Err(e) => {
            println!("Thunderbolt kontrolcüsü başlatılamadı: {:?}", e);
            return;
        }
    }

    // Cihazları keşfet
    match discover_devices() {
        Ok(devices) => {
            println!("Bulunan Thunderbolt cihazları:");
            for device in devices {
                println!("  Vendor ID: 0x{:x}, Device ID: 0x{:x}", device.vendor_id, device.device_id);
            }
            if devices.is_empty() {
                println!("  Hiç cihaz bulunamadı.");
            }
        }
        Err(e) => {
            println!("Cihaz keşfi başarısız oldu: {:?}", e);
        }
    }
}