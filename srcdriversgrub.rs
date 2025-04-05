use core::fmt;

#[derive(Debug)]
pub struct GrubInfo {
    pub memory_map: MemoryMap, // Changed to owned MemoryMap for safety
    // Diğer GRUB bilgilerini buraya ekleyebilirsiniz
}

impl GrubInfo {
    pub unsafe fn new(multiboot_information: usize) -> Self {
        let multiboot_header = *(multiboot_information as *const MultibootHeader);
        let memory_map_address = multiboot_header.memory_map_address as usize; // Correctly use usize

        let memory_map = MemoryMap::new(memory_map_address); // Create MemoryMap instance

        Self {
            memory_map,
            // Diğer GRUB bilgilerini buraya ekleyebilirsiniz
        }
    }
}

#[repr(C)]
pub struct MultibootHeader {
    pub magic: u32,
    pub flags: u32,
    pub header_length: u32,
    pub checksum: u32,
    pub modules_address: u32,
    pub modules_length: u32,
    pub boot_device: u32,
    pub cmdline: u32,
    pub memory_map_address: u32,
    pub drive_info_address: u32,
    pub config_table: u32,
    pub boot_loader_name: u32,
    pub apm_table: u32,
    pub vbe_control_info: u32,
    pub vbe_mode_info: u32,
    pub vbe_interface: u32,
    pub framebuffer_address: u32,
    pub framebuffer_pitch: u32,
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
    pub framebuffer_type: u32,
    pub framebuffer_color_depth: u32,
}

#[repr(C)]
#[derive(Debug)] // Added Debug for MemoryMapEntry
pub struct MemoryMapEntry {
    pub size: u32,
    pub base_address_low: u32,
    pub base_address_high: u32,
    pub type_: u32,
    pub length_low: u32, // Added length_low and length_high as per spec (v3)
    pub length_high: u32,
}

pub struct MemoryMap {
    entries: Vec<&'static MemoryMapEntry>, // Changed to owned Vec of references for safety
}

impl MemoryMap {
    pub unsafe fn new(address: usize) -> Self {
        let mut entries = Vec::new();
        let mut current_address = address as *const MemoryMapEntry;

        loop {
            let entry = &*current_address;
            if entry.size == 0 { // Memory map entries are terminated by a zero-size entry.
                break;
            }
            entries.push(entry);
            current_address = (current_address as usize + entry.size as usize + 4) as *const MemoryMapEntry; // Move to the next entry. Size field itself is 4 bytes.
            // According to Multiboot Specification, size field indicates the size of the structure EXCEPT the size field itself.
            // But in practice, 'size' often includes the size field itself. To be safe, we advance by size + 4.

            // Added a safety break to prevent infinite loop in case of malformed memory map.
            if entries.len() > 2048 { // Arbitrary large number, adjust if needed.
                break;
            }
        }

        Self {
            entries
        }
    }
}

impl fmt::Debug for MemoryMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.entries.iter()).finish()
    }
}

// Example usage (not for real kernel, but to show how to use the structures)
fn main() {
    // This is a dummy address, in a real kernel, you'd get this from assembly or bootloader.
    let multiboot_info_address = 0x10000;

    // Create a dummy MultibootHeader at the address. In real kernel, this memory is provided by bootloader.
    let multiboot_header = MultibootHeader {
        magic: 0, // Dummy values
        flags: 0,
        header_length: 0,
        checksum: 0,
        modules_address: 0,
        modules_length: 0,
        boot_device: 0,
        cmdline: 0,
        memory_map_address: 0x20000, // Dummy memory map address
        drive_info_address: 0,
        config_table: 0,
        boot_loader_name: 0,
        apm_table: 0,
        vbe_control_info: 0,
        vbe_mode_info: 0,
        vbe_interface: 0,
        framebuffer_address: 0,
        framebuffer_pitch: 0,
        framebuffer_width: 0,
        framebuffer_height: 0,
        framebuffer_type: 0,
        framebuffer_color_depth: 0,
    };

    // Dummy MemoryMap entries
    let memory_map_entries_data: [MemoryMapEntry; 4] = [
        MemoryMapEntry { size: 20, base_address_low: 0, base_address_high: 0, type_: 1, length_low: 0x100000, length_high: 0 }, // 1MB RAM
        MemoryMapEntry { size: 20, base_address_low: 0xF0000, base_address_high: 0, type_: 2, length_low: 0x10000, length_high: 0 }, // Reserved area
        MemoryMapEntry { size: 20, base_address_low: 0x100000, base_address_high: 0, type_: 1, length_low: 0x100000, length_high: 0 }, // Another 1MB RAM
        MemoryMapEntry { size: 20, base_address_low: 0x0, base_address_high: 0, type_: 0, length_low: 0, length_high: 0 }, // End marker (size = 20 is just for demonstration, in real case it should be size of MemoryMapEntry struct without size field, or 4 if size field is included in size)
    ];

    unsafe {
        // Place dummy MultibootHeader in memory (UNSAFE, for demonstration only)
        let multiboot_header_ptr = multiboot_info_address as *mut MultibootHeader;
        *multiboot_header_ptr = multiboot_header;

        // Place dummy MemoryMap entries in memory (UNSAFE, for demonstration only)
        let memory_map_address_ptr = 0x20000 as *mut MemoryMapEntry;
        for i in 0..memory_map_entries_data.len() {
            *((memory_map_address_ptr as usize + i * 20) as *mut MemoryMapEntry) = memory_map_entries_data[i]; // Size 20 as example, real size might be different, but must be consistent with `size` field in struct and how you advance the pointer.
        }


        let grub_info = GrubInfo::new(multiboot_info_address);
        println!("{:#?}", grub_info.memory_map);
    }
}