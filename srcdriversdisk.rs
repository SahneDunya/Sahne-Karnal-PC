#![no_std]
use core::ptr::NonNull;

use crate::drivers::pci; // Assuming this module exists and provides find_ahci_controller

// AHCI MMIO adresleri (PCI üzerinden bulunur)
#[derive(Debug)]
pub struct AhciRegs {
    pub cap: NonNull<u32>,      // Host Capabilities Register
    pub pi: NonNull<u32>,       // Ports Implemented Register
    pub is: NonNull<u32>,       // Interrupt Status Register
    pub ie: NonNull<u32>,       // Interrupt Enable Register
    pub cmd: NonNull<u32>,      // Command List Base Address Register
    pub fb: NonNull<u32>,       // FIS Base Address Register
    pub vendor: NonNull<u32>,    // Vendor Specific Registers
}

#[derive(Debug)]
pub struct AhciPort {
    pub clb: NonNull<u64>, // Command List Base Address Register (64-bit)
    pub fb: NonNull<u64>,  // FIS Base Address Register (64-bit)
    pub is: NonNull<u32>,  // Interrupt Status Register (Port specific)
    pub ie: NonNull<u32>,  // Interrupt Enable Register (Port specific)
    pub cmd: NonNull<u32>, // Command Register (Port specific)
    pub tfd: NonNull<u32>, // Task File Data Register
    pub sig: NonNull<u32>, // Signature Register
    pub ssts: NonNull<u32>,// SATA Status Register (SSTS)
    pub serr: NonNull<u32>,// SATA Error Register (SERR)
    pub sact: NonNull<u32>,// SATA Active Register (SACT)
    pub ci: NonNull<u32>,  // Command Issue Register
    pub sntf: NonNull<u32>,// SATA Notification Register (SNTF)
    pub fbs: NonNull<u32>, // FIS-Based Switching Control Register
}

pub struct AhciDevice {
    regs: AhciRegs,
    ports: [Option<AhciPort>; 32], // Maximum 32 ports as per AHCI spec
}

impl AhciDevice {
    pub fn init() -> Option<Self> {
        // PCI üzerinden AHCI denetleyicisini bul
        let ahci_base = pci::find_ahci_controller()?;

        // MMIO adreslerini yapıya kaydet
        let regs = AhciRegs {
            cap: NonNull::new((ahci_base + 0x00) as *mut u32)?,
            pi: NonNull::new((ahci_base + 0x04) as *mut u32)?,
            is: NonNull::new((ahci_base + 0x08) as *mut u32)?,
            ie: NonNull::new((ahci_base + 0x0C) as *mut u32)?,
            cmd: NonNull::new((ahci_base + 0x10) as *mut u32)?,
            fb: NonNull::new((ahci_base + 0x14) as *mut u32)?,
            vendor: NonNull::new((ahci_base + 0x20) as *mut u32)?,
        };

        // Ports Implemented register indicates which ports are implemented
        let pi_reg = unsafe { regs.pi.as_ref() };
        let mut ports: [Option<AhciPort>; 32] = core::array::from_fn(|_| None);

        for i in 0..32 {
            // Check if port i is implemented
            if (pi_reg >> i) & 1 == 1 {
                let port_base = ahci_base + 0x100 + (i * 0x80); // Port register block offset
                ports[i] = Some(AhciPort{
                    clb: NonNull::new(port_base as *mut u64)?,
                    fb: NonNull::new((port_base + 0x08) as *mut u64)?,
                    is: NonNull::new((port_base + 0x10) as *mut u32)?,
                    ie: NonNull::new((port_base + 0x18) as *mut u32)?,
                    cmd: NonNull::new((port_base + 0x20) as *mut u32)?,
                    tfd: NonNull::new((port_base + 0x28) as *mut u32)?,
                    sig: NonNull::new((port_base + 0x30) as *mut u32)?,
                    ssts: NonNull::new((port_base + 0x38) as *mut u32)?,
                    serr: NonNull::new((port_base + 0x40) as *mut u32)?,
                    sact: NonNull::new((port_base + 0x48) as *mut u32)?,
                    ci: NonNull::new((port_base + 0x50) as *mut u32)?,
                    sntf: NonNull::new((port_base + 0x58) as *mut u32)?,
                    fbs: NonNull::new((port_base + 0x60) as *mut u32)?,
                });
            }
        }

        Some(AhciDevice{regs, ports})
    }

    pub fn read_sectors(
        &self,
        port_num: usize,
        lba: u64,
        count: u32,
        buffer: *mut u8,
    ) -> Result<(), &'static str> {
        if let Some(port) = &self.ports[port_num]{
            // Port başlatma ve komut gönderme işlemleri burada yapılacak.
            // Bu kısım oldukça karmaşık ve örnekte basitleştirilmiştir.
            // Gerçek bir uygulamada çok daha fazla detay bulunur.
            unsafe {
                let ssts_reg = port.ssts.as_ref();
                // Check for device presence and power status (SSTS.DET - Device Detection, bits 4:0 and SSTS.IPM - Interface Power Management, bits 8:11)
                // SSTS.DET value 0x3 indicates "Device Present PHY Communication Established"
                if (ssts_reg >> 4) & 0x0F != 0x03 { // Checking DET field (Device Detection) - Corrected bit shift to 4:7 from 8:11, and mask to 0x0F from 0xF. Also corrected value to 0x3 from 0x03, although both are the same.
                    return Err("Sürücü hazır değil! (Device not present or no PHY communication)"); // More descriptive error message
                }
                // SSTS.IPM value 0x1 indicates "Active" power mode
                if (ssts_reg >> 8) & 0x0F != 0x01 { // Checking IPM field (Interface Power Management) - Corrected bit shift to 8:11 from 8. Mask and value are correct.
                    return Err("Sürücü hazır değil! (Interface not in active power mode)"); // More descriptive error message
                }
            }
            Ok(())
        } else {
            Err("Geçersiz port numarası")
        }
    }

    pub fn write_sectors(
        &self,
        port_num: usize,
        lba: u64,
        count: u32,
        buffer: *const u8,
    ) -> Result<(), &'static str> {
        if let Some(port) = &self.ports[port_num]{
            unsafe {
                let ssts_reg = port.ssts.as_ref();
                // Check for device presence and power status (SSTS.DET and SSTS.IPM)
                if (ssts_reg >> 4) & 0x0F != 0x03 { // Checking DET field
                    return Err("Sürücü hazır değil! (Device not present or no PHY communication)");
                }
                if (ssts_reg >> 8) & 0x0F != 0x01 { // Checking IPM field - Corrected to 0x01 for Active power mode
                    return Err("Sürücü hazır değil! (Interface not in active power mode)");
                }
            }
            Ok(())
        } else {
            Err("Geçersiz port numarası")
        }
    }
}


// Example usage (for demonstration purposes, needs proper pci::find_ahci_controller implementation)
/*
mod drivers {
    pub mod pci {
        pub fn find_ahci_controller() -> Option<u64> {
            // Dummy implementation for example - replace with actual PCI scan logic
            // Example: return a dummy AHCI base address for testing
            Some(0xF0000000)
        }
    }
}

pub fn main() {
    let ahci_device_option = AhciDevice::init();

    match ahci_device_option {
        Some(ahci_device) => {
            println!("AHCI device initialized: {:?}", ahci_device.regs);

            match ahci_device.read_sectors(0, 0, 1, core::ptr::null_mut()) {
                Ok(_) => println!("Read sectors command (dummy) successful (port 0)"),
                Err(e) => println!("Read sectors command (dummy) failed (port 0): {}", e),
            }

            match ahci_device.write_sectors(0, 0, 1, core::ptr::null()) {
                Ok(_) => println!("Write sectors command (dummy) successful (port 0)"),
                Err(e) => println!("Write sectors command (dummy) failed (port 0): {}", e),
            }


            if let Some(port0) = &ahci_device.ports[0] {
                println!("Port 0 registers: {:?}", port0);
            } else {
                println!("Port 0 is not implemented or not detected.");
            }


        }
        None => {
            println!("AHCI controller not found!");
        }
    }
}
*/