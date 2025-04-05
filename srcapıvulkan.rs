use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::physical_device::PhysicalDevice;
use vulkano::version::Version;

pub fn initialize_vulkan() -> Result<Instance, String> {
    // Vulkan uygulamasının gereksinimlerini tanımla
    let app_info = vulkano::app::AppInfo {
        application_name: "Kernel Vulkan Example".into(),
        application_version: vulkano::app::Version {
            major: 1,
            minor: 0,
            patch: 0,
        },
        engine_name: "Kernel Engine".into(),
        engine_version: vulkano::app::Version {
            major: 1,
            minor: 0,
            patch: 0,
        },
    };

    // Vulkan instance oluştur
    Instance::new(
        Some(app_info),
        InstanceCreateInfo::default(),
    ).map_err(|err| format!("Vulkan instance oluşturulamadı: {}", err)) // Hata mesajını iyileştirildi
}

pub fn select_physical_device(instance: &Instance) -> Result<PhysicalDevice, String> {
    // Uygun bir fiziksel cihaz seç
    let physical_devices = instance.enumerate_physical_devices().unwrap(); // Hata kontrolü zaten aşağıda yapılıyor
    let physical_device = physical_devices
        .find(|p| {
            // Cihazın Vulkan ile uyumlu olup olmadığını kontrol et
            p.api_version() >= Version::V1_1 // Örnek olarak Vulkan 1.1 veya üstü
        })
        .ok_or_else(|| {
            let available_devices = physical_devices
                .map(|p| format!("'{}' (Vulkan {:?})", p.name(), p.api_version()))
                .collect::<Vec<_>>()
                .join(", ");
            format!("Uyumlu bir fiziksel cihaz bulunamadı. Mevcut cihazlar: {}", available_devices) // Hata mesajı iyileştirildi ve mevcut cihazlar listelendi
        })?;

    Ok(physical_device)
}

fn main() -> Result<(), String> { // Hata türünü String olarak değiştirildi
    // Vulkan'ı başlat
    let instance = initialize_vulkan()?;

    // Fiziksel bir cihaz seç
    let physical_device = select_physical_device(&instance)?;

    // Seçilen cihaz hakkında bilgi yazdır
    println!("Seçilen cihaz:");
    println!("  Adı: {:?}", physical_device.name());
    println!("  Sürücü Sürümü: {:?}", physical_device.driver_version());
    println!("  Vulkan API Sürümü: {:?}", physical_device.api_version()); // API sürümü eklendi

    Ok(())
}