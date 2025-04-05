use crate::error::Result;
use crate::memory::volatile::Volatile;
use crate::registers::msr;
use crate::synchronization::NullLock;
use core::sync::Mutex;

/// PowerPC için güç yönetimi.
pub struct PowerManager {
    /// Güç yönetimi modülünün taban adresi.
    base_address: usize,
}

impl PowerManager {
    /// Yeni bir `PowerManager` örneği oluşturur.
    pub const fn new(base_address: usize) -> Self {
        Self { base_address }
    }

    /// Sistemin güç durumunu alır.
    pub fn get_power_state(&self) -> Result<PowerState> {
        let state = unsafe {
            let state_register = Volatile::new((self.base_address + 0x10) as *mut u32);
            state_register.read()
        };

        match state {
            0 => Ok(PowerState::On),
            1 => Ok(PowerState::Sleep),
            2 => Ok(PowerState::DeepSleep),
            3 => Ok(PowerState::Off),
            _ => Err(Error::InvalidPowerState(state)),
        }
    }

    /// Sistemin güç durumunu ayarlar.
    pub fn set_power_state(&mut self, state: PowerState) -> Result<()> {
        let state_value = match state {
            PowerState::On => 0,
            PowerState::Sleep => 1,
            PowerState::DeepSleep => 2,
            PowerState::Off => 3,
        };

        unsafe {
            let state_register = Volatile::new((self.base_address + 0x10) as *mut u32);
            state_register.write(state_value);
        }

        Ok(())
    }

    /// Sistemin güç tüketimini alır.
    pub fn get_power_consumption(&self) -> Result<u32> {
        let consumption = unsafe {
            let consumption_register = Volatile::new((self.base_address + 0x20) as *mut u32);
            consumption_register.read()
        };

        Ok(consumption)
    }
}

/// Olası güç durumları.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerState {
    On,
    Sleep,
    DeepSleep,
    Off,
}

/// Güç yönetimiyle ilgili hatalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidPowerState(u32),
}

/// Güç yönetimi modülüne erişimi koordine etmek için bir mutex.
static POWER_MANAGER: Mutex<Option<PowerManager>> = Mutex::new(None);

/// Güç yönetimi modülünü başlatır.
pub fn init(base_address: usize) {
    let mut power_manager = POWER_MANAGER.lock();
    *power_manager = Some(PowerManager::new(base_address));
}

/// Güç yönetimi modülüne erişim sağlar.
pub fn get() -> Result<&'static mut PowerManager> {
    let mut power_manager = POWER_MANAGER.lock();
    power_manager.as_mut().ok_or(Error::NotInitialized)
}

/// Güç yönetimiyle ilgili hatalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    NotInitialized,
    InvalidPowerState(u32),
}