#![no_std]

// Allow unused variables and dead code during development
#![allow(unused_variables)]
#![allow(dead_code)]

// Assuming KError and MmuFlags are defined in the parent karnal64 crate or a common module
// Adjust the path based on your actual module structure.
use crate::karnal64::KError;
// Define MmuFlags and MmuDriver trait here or import them from a common place if shared.
// For this example, let's assume MmuFlags and MmuFaultType are defined here
// and MmuDriver trait is defined here or in a dedicated kernel/mmu.rs file.

// --- Assumed Karnal64/kmemory Interfaces and Common MMU Definitions ---
// These types and the MmuDriver trait are assumed to be part of the Karnal64 framework,
// likely in a shared kernel::mmu module or directly in kmemory.
// We define them here locally for clarity in this example file,
// but they would ideally be imported from a common kernel crate or module.

#[derive(Debug, Copy, Clone)]
pub struct MmuFlags(u32);

impl MmuFlags {
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const EXECUTE: Self = Self(1 << 2);
    pub const USER: Self = Self(1 << 3); // Accessible by user space (Non-privileged)
    pub const PRIVILEGED: Self = Self(1 << 4); // Accessible only by privileged code
    pub const CACHED: Self = Self(1 << 5);
    pub const WAS_WRITTEN: Self = Self(1 << 6); // Hardware updated flag (Dirty)
    pub const WAS_ACCESSED: Self = Self(1 << 7); // Hardware updated flag (Accessed)
    // SPARC V8 specific flags might include:
    pub const VALID: Self = Self(1 << 31); // PTE Valid bit

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Converts MmuFlags to SPARC V8 PTE permission/attribute bits.
    /// This is a simplified conversion and depends heavily on the exact SPARC MMU model.
    pub fn to_sparc_pte_flags(&self) -> u32 {
        let mut sparc_flags = 0;
        if self.contains(MmuFlags::VALID) { sparc_flags |= 1 << 31; } // Valid bit
        // Simplified mapping: SPARC V8 typically has bits for Writeable (W), Executable (E), System (S)
        // S=0 for User, S=1 for Supervisor
        if self.contains(MmuFlags::USER) {
             // User access requires S=0. Read/Write/Execute permissions encoded differently
             // This mapping is highly dependent on the specific SPARC MMU implementation (e.g., MB8690x)
             // Dummy mapping for demonstration:
             if self.contains(MmuFlags::READ) { sparc_flags |= 1 << 20; } // Example read bit
             if self.contains(MmuFlags::WRITE) { sparc_flags |= 1 << 19; } // Example write bit
             if self.contains(MmuFlags::EXECUTE) { sparc_flags |= 1 << 18; } // Example execute bit
             sparc_flags |= 0 << 17; // Example System (S) bit for User
        } else if self.contains(MmuFlags::PRIVILEGED) {
             // Supervisor access requires S=1.
             // Dummy mapping:
             if self.contains(MmuFlags::READ) { sparc_flags |= 1 << 20; } // Example read bit
             if self.contains(MmuFlags::WRITE) { sparc_flags |= 1 << 19; } // Example write bit
             if self.contains(MmuFlags::EXECUTE) { sparc_flags |= 1 << 18; } // Example execute bit
             sparc_flags |= 1 << 17; // Example System (S) bit for Supervisor
        }
        if self.contains(MmuFlags::CACHED) { sparc_flags |= 1 << 16; } // Example cacheable bit
        // Add other SPARC specific bits (e.g., dirty, accessed - often updated by HW)
        sparc_flags
    }

    /// Converts SPARC V8 PTE flags to MmuFlags.
    pub fn from_sparc_pte_flags(sparc_flags: u32) -> Self {
         let mut flags = 0;
         if (sparc_flags >> 31) & 1 == 1 { flags |= MmuFlags::VALID.0; }
         // Dummy mapping based on to_sparc_pte_flags:
         if (sparc_flags >> 20) & 1 == 1 { flags |= MmuFlags::READ.0; }
         if (sparc_flags >> 19) & 1 == 1 { flags |= MmuFlags::WRITE.0; }
         if (sparc_flags >> 18) & 1 == 1 { flags |= MmuFlags::EXECUTE.0; }
         if (sparc_flags >> 17) & 1 == 1 {
             flags |= MmuFlags::PRIVILEGED.0;
         } else {
             flags |= MmuFlags::USER.0;
         }
         if (sparc_flags >> 16) & 1 == 1 { flags |= MmuFlags::CACHED.0; }
         // Assume bits 7 and 6 are HW updated Accessed and Dirty flags
         if (sparc_flags >> 7) & 1 == 1 { flags |= MmuFlags::WAS_ACCESSED.0; }
         if (sparc_flags >> 6) & 1 == 1 { flags |= MmuFlags::WAS_WRITTEN.0; }

         Self(flags)
    }
}


#[derive(Debug, Copy, Clone)]
pub enum MmuFaultType {
    PageFault,
    AccessFault, // Permission violation
    // SPARC V8 specific fault types would include:
    // Protection Violation
    // Invalid Address
    // Privileged Violation
    // etc.
}

// Placeholder TrapFrame - structure is highly SPARC specific
#[repr(C)]
pub struct TrapFrame {
    // SPARC register state at the time of the trap
    // In/Local/Global/Out registers, PSR, WIM, TBR, PC, nPC, etc.
    // This needs to match the actual trap handler's stack layout.
    pub tt: u32, // Trap Type
    pub psr: u33, // Processor State Register (assuming SPARC V9 for u33, V8 is u32) - Let's stick to V8 u32 for simplicity
    pub pc: u32, // Program Counter
    pub npc: u32, // Next Program Counter
    // ... other registers
}

/// Trait that the architecture-specific MMU implementation must fulfill
/// for the kernel's memory manager (kmemory).
/// This trait would ideally be defined in a architecture-neutral kernel::mmu module.
pub trait MmuDriver {
    /// Initializes the MMU hardware and internal structures.
    fn init(&self);

    /// Maps a range of virtual memory to physical memory with given flags.
    /// The mapping must be page-aligned and size must be a multiple of page size.
    /// root_page_table is the physical address of the top-level page table
    /// (e.g., Context Table Pointer in SPARC V8).
    /// Returns Ok(()) on success, or a KError on failure.
    ///
    /// Note: The MMU driver is responsible for allocating intermediate page table levels
    /// if they don't exist, potentially using a frame allocator provided by kmemory.
    fn map_memory(&self, root_page_table_phys: usize, virt_addr: usize, phys_addr: usize, size: usize, flags: MmuFlags) -> Result<(), KError>;

    /// Unmaps a range of virtual memory.
    /// The range must be page-aligned and size must be a multiple of page size.
    /// root_page_table is the physical address of the top-level page table.
    /// Returns Ok(()) on success, or a KError on failure.
    ///
    /// Note: The MMU driver should free intermediate page table levels if they become empty,
    /// using a frame deallocator from kmemory.
    fn unmap_memory(&self, root_page_table_phys: usize, virt_addr: usize, size: usize) -> Result<(), KError>;

    /// Changes protection flags for a range of virtual memory.
    /// The range must be page-aligned and size must be a multiple of page size.
    /// root_page_table is the physical address of the top-level page table.
    /// Returns Ok(()) on success, or a KError on failure.
    fn protect_memory(&self, root_page_table_phys: usize, virt_addr: usize, size: usize, flags: MmuFlags) -> Result<(), KError>;

    /// Translates a virtual address to its corresponding physical address
    /// for a given root page table.
    /// Returns Some(physical_address) if mapped, None otherwise.
    fn translate_address(&self, root_page_table_phys: usize, virt_addr: usize) -> Option<usize>;

    /// Handles an MMU-specific interrupt or trap (e.g., page fault).
    /// This function is called from the architecture-specific trap handler.
    /// It needs to determine the cause of the fault (read/write/exec, address)
    /// and potentially delegate handling to a higher-level page fault handler in kmemory.
    /// Returns true if the fault was handled and the trapped instruction should be retried,
    /// false otherwise (indicating an unrecoverable fault).
    fn handle_fault(&self, fault_address: usize, fault_type: MmuFaultType, trap_frame: &mut TrapFrame) -> bool;

    /// Flushes TLB entries for a specific virtual address range, or the entire TLB if range is None.
    fn flush_tlb(&self, virt_addr: Option<usize>, size: Option<usize>);

    /// Gets the size of a memory page in bytes.
    fn page_size(&self) -> usize;

    /// Creates a new root page table structure.
    /// Returns the physical address of the new root table or KError.
    /// The MMU driver must obtain physical memory frames for the root table.
    fn create_root_page_table(&self) -> Result<usize, KError>;

    /// Destroys a root page table structure and frees associated memory.
    /// Returns Ok(()) on success, or KError on failure.
    fn destroy_root_page_table(&self, root_page_table_phys: usize) -> Result<(), KError>;
}


// --- SPARC V8 MMU Implementation Details ---

/// Page size for SPARC V8, typically 4KB.
const SPARC_PAGE_SIZE: usize = 4096;
const PAGE_SIZE_SHIFT: usize = 12; // log2(4096)

/// Addresses are typically 32-bit on SPARC V8.
type SpArcAddr = u32;
type SpArcPte = u32; // SPARC V8 PTEs are typically 32-bit

/// Number of entries in each page table level on SPARC V8 (4KB pages).
/// A 32-bit address space (4GB) with 4KB pages needs 20 bits for the page number (4G / 4K = 2^20).
/// If we have 3 levels (Context, Segment, Page), each index needs ~20/3 bits.
/// Typical V8 structure:
/// Virtual Address [31:0]: Context [31:24] | Segment [23:18] | Page [17:12] | Offset [11:0]
/// Context Index: 8 bits -> 256 entries per Context Table
/// Segment Index: 6 bits -> 64 entries per Segment Table
/// Page Index: 6 bits -> 64 entries per Page Table
const CT_ENTRIES: usize = 1 << 8; // 256
const ST_ENTRIES: usize = 1 << 6; // 64
const PT_ENTRIES: usize = 1 << 6; // 64

/// Sizes of the page table levels in bytes.
const CT_SIZE: usize = CT_ENTRIES * core::mem::size_of::<SpArcAddr>(); // CT points to ST (physical address)
const ST_SIZE: usize = ST_ENTRIES * core::mem::size_of::<SpArcAddr>(); // ST points to PT (physical address)
const PT_SIZE: usize = PT_ENTRIES * core::mem::size_of::<SpArcPte>();   // PT contains PTEs

/// Masks and shifts for extracting indices from a virtual address.
const CT_SHIFT: usize = 24;
const ST_SHIFT: usize = 18;
const PT_SHIFT: usize = 12; // This is the PAGE_SIZE_SHIFT

const CT_MASK: usize = (CT_ENTRIES - 1) << CT_SHIFT;
const ST_MASK: usize = (ST_ENTRIES - 1) << ST_SHIFT;
const PT_MASK: usize = (PT_ENTRIES - 1) << PT_SHIFT;

// SPARC V8 PTE Bits (Simplified example - actual bits vary per MMU)
// Let's assume a format like: [Valid:1 | Reserved:1 | Perm:3 | S:1 | C:1 | Reserved:8 | PPN:17] (32 bits)
// PPN (Physical Page Number) is typically bits [17:0] or [18:0] depending on total addressable physical memory and page size.
// For 4KB pages, PPN [31:12] in a 32-bit word would directly give the page frame physical address.
// Let's use a simple mapping where the high bits of the PTE store the PPN.
const PTE_VALID_BIT: u32 = 1 << 31;
const PTE_PRIVILEGED_BIT: u32 = 1 << 17; // Example S bit (System = Privileged)
const PTE_CACHED_BIT: u32 = 1 << 16;    // Example C bit (Cached)
// Permission bits mapping (example):
const PTE_READ_BIT: u32 = 1 << 20;
const PTE_WRITE_BIT: u32 = 1 << 19;
const PTE_EXECUTE_BIT: u32 = 1 << 18;

const PTE_PPN_SHIFT: u32 = 12; // Physical Page Number starts at bit 12 for 4KB pages
const PTE_PPN_MASK: u32 = 0xFFFFF000; // Mask for bits 31 down to 12

/// Represents a SPARC V8 Page Table Entry (PTE).
#[derive(Copy, Clone)]
#[repr(transparent)] // Ensure it's just a u32 in memory
struct SpArcPteEntry(u32);

impl SpArcPteEntry {
    /// Creates a new PTE from physical address and flags.
    fn new(phys_addr: usize, flags: MmuFlags) -> Self {
        let ppn = (phys_addr as u32) & PTE_PPN_MASK;
        let sparc_flags = flags.to_sparc_pte_flags(); // Convert generic flags to SPARC bits
        Self(ppn | sparc_flags)
    }

    /// Returns the physical address encoded in the PTE.
    fn physical_address(&self) -> usize {
        (self.0 & PTE_PPN_MASK) as usize
    }

    /// Returns true if the PTE is marked as valid.
    fn is_valid(&self) -> bool {
        (self.0 & PTE_VALID_BIT) != 0
    }

    /// Returns the MmuFlags represented by the PTE.
    fn flags(&self) -> MmuFlags {
        MmuFlags::from_sparc_pte_flags(self.0)
    }

    /// Sets the valid bit.
    fn set_valid(&mut self, valid: bool) {
        if valid {
            self.0 |= PTE_VALID_BIT;
        } else {
            self.0 &= !PTE_VALID_BIT;
        }
    }

    /// Sets the flags, preserving PPN.
    fn set_flags(&mut self, flags: MmuFlags) {
        let ppn = self.physical_address() as u32; // Keep existing PPN
        let sparc_flags = flags.to_sparc_pte_flags(); // Convert new flags to SPARC bits
        // Clear old flags (except PPN area) and apply new flags
        self.0 = (self.0 & PTE_PPN_MASK) | sparc_flags;
    }
}


/// Accesses a page table entry given the base physical address of the table
/// and the index within that table.
/// Returns a mutable reference to the entry.
///
/// SAFETY: Caller must ensure `table_phys_addr` is a valid physical address
/// of a page table (CT, ST, or PT) and `index` is within bounds.
/// This function performs raw pointer arithmetic and assumes a direct
/// physical-to-virtual mapping in the kernel's address space or uses
/// specific functions to access physical memory from the kernel.
/// For simplicity here, we assume direct physical access is possible via pointers
/// after validating the address range is within physical memory controlled by the kernel.
unsafe fn get_pte_entry_mut(table_phys_addr: usize, index: usize) -> &'static mut SpArcPteEntry {
    // In a real kernel, accessing arbitrary physical memory requires
    // careful mapping or special access functions.
    // This is a simplification assuming kernel can map/access physical memory directly.
    let ptr = (table_phys_addr + index * core::mem::size_of::<SpArcPte>()) as *mut SpArcPteEntry;
    &mut *ptr
}

/// Accesses a page table entry given the base physical address of the table
/// and the index within that table.
/// Returns an immutable reference to the entry.
unsafe fn get_pte_entry(table_phys_addr: usize, index: usize) -> &'static SpArcPteEntry {
     let ptr = (table_phys_addr + index * core::mem::size_of::<SpArcPte>()) as *const SpArcPteEntry;
     &*ptr
}

/// Calculates the indices for CT, ST, and PT from a virtual address.
fn get_page_table_indices(virt_addr: usize) -> (usize, usize, usize) {
    let ct_idx = (virt_addr >> CT_SHIFT) & (CT_ENTRIES - 1);
    let st_idx = (virt_addr >> ST_SHIFT) & (ST_ENTRIES - 1);
    let pt_idx = (virt_addr >> PT_SHIFT) & (PT_ENTRIES - 1);
    (ct_idx, st_idx, pt_idx)
}

/// A struct representing the SPARC V8 MMU driver.
/// This struct will implement the `MmuDriver` trait.
pub struct SparcV8Mmu;

impl SparcV8Mmu {
    /// Creates a new instance of the SPARC V8 MMU driver.
    pub const fn new() -> Self {
        SparcV8Mmu
    }

    // Helper function to traverse the page table structure.
    // Returns the physical address of the PTE for virt_addr, or None if not mapped.
    // Optionally creates intermediate tables if `create` is true,
    // using the provided frame allocator callback.
    unsafe fn walk_page_tables(
        &self,
        root_ct_phys: usize,
        virt_addr: usize,
        create: bool,
        // A real implementation would need a way to request physical frames
        // from kmemory, e.g., a closure `frame_alloc: &dyn Fn() -> Result<usize, KError>`.
        // For this placeholder, we'll assume an external allocator call exists.
        frame_alloc: Option<&dyn Fn() -> Result<usize, KError>>,
    ) -> Result<*mut SpArcPteEntry, KError> {
        let (ct_idx, st_idx, pt_idx) = get_page_table_indices(virt_addr);

        // 1. Access Context Table (CT)
        let ct_entry_ptr = (root_ct_phys + ct_idx * core::mem::size_of::<SpArcAddr>()) as *mut SpArcAddr;
        let mut st_phys = *ct_entry_ptr as usize;

        if st_phys == 0 { // Segment Table does not exist
            if create {
                // Need to allocate a physical frame for the Segment Table
                let new_st_phys = if let Some(alloc) = frame_alloc {
                     alloc()? // Allocate a frame (ST_SIZE might require multiple frames)
                } else {
                    // Cannot allocate without a provider
                    return Err(KError::OutOfMemory);
                };
                // Initialize the new Segment Table to zeros
                core::ptr::write_bytes(new_st_phys as *mut u8, 0, ST_SIZE);
                *ct_entry_ptr = new_st_phys as u32; // Store physical address in CT
                st_phys = new_st_phys;
            } else {
                return Err(KError::NotFound); // Mapping doesn't exist
            }
        }

        // 2. Access Segment Table (ST)
        let st_entry_ptr = (st_phys + st_idx * core::mem::size_of::<SpArcAddr>()) as *mut SpArcAddr;
        let mut pt_phys = *st_entry_ptr as usize;

        if pt_phys == 0 { // Page Table does not exist
             if create {
                // Need to allocate a physical frame for the Page Table
                 let new_pt_phys = if let Some(alloc) = frame_alloc {
                     alloc()? // Allocate a frame (PT_SIZE might require multiple frames)
                 } else {
                     return Err(KError::OutOfMemory);
                 };
                 // Initialize the new Page Table to zeros
                 core::ptr::write_bytes(new_pt_phys as *mut u8, 0, PT_SIZE);
                 *st_entry_ptr = new_pt_phys as u32; // Store physical address in ST
                 pt_phys = new_pt_phys;
             } else {
                return Err(KError::NotFound); // Mapping doesn't exist
             }
        }

        // 3. Access Page Table (PT) and return the PTE pointer
        let pte_ptr = (pt_phys + pt_idx * core::mem::size_of::<SpArcPte>()) as *mut SpArcPteEntry;

        Ok(pte_ptr)
    }

    // Helper to deallocate intermediate tables if they become empty.
    // This is complex and needs careful reference counting or traversal.
    // Placeholder for now.
    unsafe fn maybe_free_intermediate_tables(&self, root_ct_phys: usize, virt_addr: usize, frame_dealloc: &dyn Fn(usize, usize) -> Result<(), KError>) -> Result<(), KError> {
         // TODO: Traverse back up the tree. If a PT or ST becomes all zeros,
         // deallocate its physical frame using `frame_dealloc` and zero out
         // the parent table entry. This needs careful logic to avoid premature freeing.
         Ok(()) // Placeholder
    }
}


impl MmuDriver for SparcV8Mmu {
    fn init(&self) {
        // TODO: Initialize SPARC MMU hardware registers (e.g., disable MMU initially,
        // set initial Context Table Pointer if applicable, configure traps).
        // This requires low-level assembly or volatile register access.
         print!("SparcV8Mmu: Initializing MMU hardware (placeholder)...");
        // Example (conceptual):
         unsafe {
        //     // Write to SPARC MMU control registers
              core::arch::asm!("...") // Use assembly for specific instructions
        //     // Assuming volatile writes for memory-mapped registers
              let mmu_control_reg = 0xFFF00000 as *mut u32; // Example address
              core::ptr::write_volatile(mmu_control_reg, initial_config);
         }
         println!("Done."); // Need kernel print!
    }

    fn map_memory(&self, root_page_table_phys: usize, virt_addr: usize, phys_addr: usize, size: usize, flags: MmuFlags) -> Result<(), KError> {
        // Ensure addresses and size are page-aligned
        if virt_addr % SPARC_PAGE_SIZE != 0 || phys_addr % SPARC_PAGE_SIZE != 0 || size % SPARC_PAGE_SIZE != 0 {
            return Err(KError::InvalidArgument);
        }

        let num_pages = size / SPARC_PAGE_SIZE;
        let mut current_virt = virt_addr;
        let mut current_phys = phys_addr;

        // We need a frame allocator from kmemory to create intermediate page tables.
        // Assume kmemory provides a global function or is accessible.
        // For this example, we'll pass a dummy allocator closure.
        // In a real scenario, `kmemory` would likely provide this contextually.
        let dummy_frame_alloc = || {
             // TODO: Call into kmemory's physical frame allocator
              crate::kmemory::allocate_physical_frame() -> Result<usize, KError>
             // Placeholder: Simulate allocation success
             Ok(0xdeadbeef as usize) // Return a dummy physical address
        };
        let frame_alloc_ref: &dyn Fn() -> Result<usize, KError> = &dummy_frame_alloc;


        unsafe {
            for _i in 0..num_pages {
                // Walk table, creating entries if necessary
                let pte_ptr = self.walk_page_tables(root_page_table_phys, current_virt, true, Some(frame_alloc_ref))?;

                // Check if a valid mapping already exists (potential error or update)
                let pte_entry = &mut *pte_ptr;
                if pte_entry.is_valid() {
                    // TODO: Handle existing mapping. Depends on policy (overwrite? error?)
                    // For simplicity, let's assume error for now.
                      return Err(KError::AlreadyExists); // Or update flags?
                     // If updating flags, ensure permissions aren't escalated unsafely.
                }

                // Create the new PTE
                let page_flags = flags | MmuFlags::VALID; // Always set valid bit for a new map
                *pte_entry = SpArcPteEntry::new(current_phys, page_flags);

                // Flush TLB entry for the mapped virtual address
                self.flush_tlb(Some(current_virt), Some(SPARC_PAGE_SIZE));

                current_virt += SPARC_PAGE_SIZE;
                current_phys += SPARC_PAGE_SIZE;
            }
        }

        Ok(())
    }

    fn unmap_memory(&self, root_page_table_phys: usize, virt_addr: usize, size: usize) -> Result<(), KError> {
        // Ensure addresses and size are page-aligned
        if virt_addr % SPARC_PAGE_SIZE != 0 || size % SPARC_PAGE_SIZE != 0 {
            return Err(KError::InvalidArgument);
        }

        let num_pages = size / SPARC_PAGE_SIZE;
        let mut current_virt = virt_addr;

        // We need a frame deallocator from kmemory.
        // Assume kmemory provides a global function or is accessible.
        // For this example, we'll pass a dummy deallocator closure.
        let dummy_frame_dealloc = |phys: usize, size: usize| {
            // TODO: Call into kmemory's physical frame deallocator
             crate::kmemory::free_physical_frame(phys, size) -> Result<(), KError>
            // Placeholder: Simulate deallocation success
            Ok(())
        };
        let frame_dealloc_ref: &dyn Fn(usize, usize) -> Result<(), KError> = &dummy_frame_dealloc;


        unsafe {
            for _i in 0..num_pages {
                // Walk table to find the PTE. Don't create intermediate tables (`create=false`).
                let pte_ptr = self.walk_page_tables(root_page_table_phys, current_virt, false, None)?;

                let pte_entry = &mut *pte_ptr;

                // Invalidate the PTE
                pte_entry.set_valid(false);
                // TODO: Optionally clear permission bits or the PPN for safety?

                // Flush TLB entry for the unmapped virtual address
                self.flush_tlb(Some(current_virt), Some(SPARC_PAGE_SIZE));

                // Attempt to free intermediate tables if they are now empty
                self.maybe_free_intermediate_tables(root_page_table_phys, current_virt, frame_dealloc_ref)?;

                current_virt += SPARC_PAGE_SIZE;
            }
        }

        Ok(())
    }

    fn protect_memory(&self, root_page_table_phys: usize, virt_addr: usize, size: usize, flags: MmuFlags) -> Result<(), KError> {
        // Ensure addresses and size are page-aligned
         if virt_addr % SPARC_PAGE_SIZE != 0 || size % SPARC_PAGE_SIZE != 0 {
            return Err(KError::InvalidArgument);
        }

        let num_pages = size / SPARC_PAGE_SIZE;
        let mut current_virt = virt_addr;

        unsafe {
            for _i in 0..num_pages {
                // Walk table to find the PTE. Don't create intermediate tables (`create=false`).
                let pte_ptr = self.walk_page_tables(root_page_table_phys, current_virt, false, None)?;

                let pte_entry = &mut *pte_ptr;

                // Only update if the entry is valid
                if pte_entry.is_valid() {
                     // Update the flags, preserving the valid bit and PPN.
                     // Need to be careful here: ensure new flags don't grant more permissions
                     // than allowed by a higher-level policy (handled by kmemory?).
                     let existing_ppn = pte_entry.physical_address();
                     let updated_flags = flags | MmuFlags::VALID; // Ensure valid bit remains set
                     *pte_entry = SpArcPteEntry::new(existing_ppn, updated_flags);

                     // Flush TLB entry as permissions changed
                     self.flush_tlb(Some(current_virt), Some(SPARC_PAGE_SIZE));
                } else {
                    // Attempting to protect an unmapped page? Depends on policy.
                    // For now, treat as not found or invalid argument.
                    return Err(KError::NotFound);
                }

                current_virt += SPARC_PAGE_SIZE;
            }
        }

        Ok(())
    }

    fn translate_address(&self, root_page_table_phys: usize, virt_addr: usize) -> Option<usize> {
         // Address doesn't need to be page-aligned for translation, but we translate page-by-page
        let page_virt_addr = virt_addr & !(SPARC_PAGE_SIZE - 1);
        let offset = virt_addr & (SPARC_PAGE_SIZE - 1);

        unsafe {
             // Walk table to find the PTE. Don't create intermediate tables (`create=false`).
             match self.walk_page_tables(root_page_table_phys, page_virt_addr, false, None) {
                 Ok(pte_ptr) => {
                     let pte_entry = &*pte_ptr;
                     if pte_entry.is_valid() {
                         Some(pte_entry.physical_address() + offset)
                     } else {
                         None // PTE is not valid
                     }
                 }
                 Err(_) => None, // Error during walk (e.g., intermediate table missing)
             }
        }
    }

    fn handle_fault(&self, fault_address: usize, fault_type: MmuFaultType, trap_frame: &mut TrapFrame) -> bool {
        // TODO: Analyze the trap_frame and fault_address to determine the cause.
        // SPARC traps provide specific type codes (TT) and possibly other state.
        // Example:
         println!("SparcV8Mmu: Handling MMU Fault at {:x}, Type: {:?}", fault_address, fault_type); // Need kernel print!
        // Check fault_type and trap_frame->tt
        // Delegate to a higher-level kmemory page fault handler?
         let handled = crate::kmemory::handle_page_fault(fault_address, fault_type, trap_frame);
        // handled // Return true if kmemory handled it, false otherwise.

        // Placeholder: Just indicate the fault occurred.
        false // Not handled by this minimal implementation
    }

    fn flush_tlb(&self, virt_addr: Option<usize>, size: Option<usize>) {
        // TODO: Implement SPARC-specific TLB flush instructions.
        // SPARC V8 has instructions like `flush` (global), `flush <addr>` (per-address), etc.
        // This requires inline assembly.
        // Example (conceptual):
         unsafe {
             match virt_addr {
                 Some(vaddr) => {
        //             // Flush single address or range
                      core::arch::asm!("flush %0", in(reg) vaddr); // Example syntax
                 }
                 None => {
        //             // Global flush
                      core::arch::asm!("flush");
                 }
             }
         }
          print!("SparcV8Mmu: Flushing TLB"); // Need kernel print!
          if let Some(vaddr) = virt_addr { print!(" for {:x}", vaddr); }
          println!(" (placeholder)...");
    }

    fn page_size(&self) -> usize {
        SPARC_PAGE_SIZE
    }

    fn create_root_page_table(&self) -> Result<usize, KError> {
        // Need to allocate a physical frame for the Context Table (CT).
        // Assume kmemory provides a physical frame allocator.
         // TODO: Call into kmemory's physical frame allocator
          let ct_phys_addr = crate::kmemory::allocate_physical_frame(CT_SIZE)?;
         // Placeholder: Simulate allocation
         let ct_phys_addr = 0x10000000; // Dummy physical address for a new CT

        // Initialize the new Context Table to zeros
        unsafe {
            // Need a way to get a virtual address for this physical frame
            // (if kernel doesn't have direct physical mapping)
            // For simplicity, assume direct physical mapping or a helper:
             let ct_virt_ptr = crate::kmemory::phys_to_virt(ct_phys_addr)? as *mut u8;
            let ct_virt_ptr = ct_phys_addr as *mut u8; // Assume direct mapping for kernel
            core::ptr::write_bytes(ct_virt_ptr, 0, CT_SIZE);
        }

        Ok(ct_phys_addr)
    }

    fn destroy_root_page_table(&self, root_page_table_phys: usize) -> Result<(), KError> {
         // TODO: Traverse the page table tree (CT -> STs -> PTs).
         // For each valid PTE, if it maps a page that is *owned* by this address space
         // (i.e., not a shared kernel page), the corresponding physical frame needs to be freed.
         // Free intermediate ST and PT physical frames that become empty.
         // Finally, free the root CT physical frame.
         // This requires a physical frame deallocator from kmemory.

         // Placeholder: Simulate deallocation
         // print!("SparcV8Mmu: Destroying root page table at {:x} (placeholder)...", root_page_table_phys); // Need kernel print!
         // TODO: Call into kmemory's physical frame deallocator for the CT frame.
          crate::kmemory::free_physical_frame(root_page_table_phys, CT_SIZE)?;
          println!("Done.");
         Ok(())
    }
}

// --- Potential integration point with kmemory ---
// The kmemory module in karnal64.rs would likely hold an instance of SparcV8Mmu
// and delegate MMU-specific operations to it via the MmuDriver trait.

// Conceptual kmemory.rs part
mod kmemory {
    use super::*; // Import types from karnal64.rs
    use crate::mmu_sparc::SparcV8Mmu; // Import the specific MMU driver

    // Static instance of the MMU driver
    static MMU_DRIVER: SparcV8Mmu = SparcV8Mmu::new();

    pub fn init_manager() {
        // Initialize MMU hardware early
        MMU_DRIVER.init();
        // TODO: Initialize physical and virtual memory allocators, kernel address space, etc.
          println!("Karnal64: Bellek Yöneticisi Başlatıldı"); // Use kernel print!
    }

    // Functions called by the Karnal64 API (e.g., kmemory::allocate_user_memory)
    // These functions would manage address spaces (root page tables) and delegate
    // page table manipulation to the MMU_DRIVER.

    pub fn allocate_user_memory(size: usize) -> Result<*mut u8, KError> {
        // TODO: Get current task's root page table physical address
        // TODO: Find a free virtual address range in the task's address space
        // TODO: Allocate physical frames for the requested size using a physical frame allocator
        // TODO: Call MMU_DRIVER.map_memory to map the virtual range to physical frames
        // TODO: Return the starting virtual address (as a user-space pointer)

        // Placeholder
         println!("kmemory::allocate_user_memory(size={}) (placeholder)", size);
        // Simulate allocation and mapping
        let dummy_virt_addr = 0x80001000; // Example user space virtual address
        let dummy_phys_addr = 0x20001000; // Example physical address
        let dummy_root_pt = 0x10000000; // Example root page table address

        // Assume we have a root page table and allocated physical memory
         MMU_DRIVER.map_memory(dummy_root_pt, dummy_virt_addr, dummy_phys_addr, size, MmuFlags::READ | MmuFlags::WRITE | MmuFlags::USER)?;

        Ok(dummy_virt_addr as *mut u8)
    }

    pub fn free_user_memory(ptr: *mut u8, size: usize) -> Result<(), KError> {
        // TODO: Get current task's root page table physical address
        // TODO: Call MMU_DRIVER.unmap_memory to unmap the virtual range
        // TODO: Free the corresponding physical frames using a physical frame deallocator
        // TODO: Potentially destroy page table levels if they become empty (handled by MMU_DRIVER.unmap_memory/maybe_free)

         // Placeholder
          println!("kmemory::free_user_memory(ptr={:p}, size={}) (placeholder)", ptr, size);
         // Simulate unmapping
         let dummy_virt_addr = ptr as usize;
         let dummy_root_pt = 0x10000000; // Example root page table address
          MMU_DRIVER.unmap_memory(dummy_root_pt, dummy_virt_addr, size)?;

        Ok(())
    }

    // TODO: Implement allocate_physical_frame, free_physical_frame, phys_to_virt, virt_to_phys (for kernel space), etc.
    // TODO: Implement address space management (creating/destroying root page tables per task).
    // TODO: Implement the high-level page fault handler that SparcV8Mmu::handle_fault calls.

    // Example placeholder for physical frame allocation
     pub fn allocate_physical_frame(size: usize) -> Result<usize, KError> {
          println!("kmemory::allocate_physical_frame(size={}) (placeholder)", size);
         // TODO: Implement actual physical memory allocation
         // For large sizes like CT_SIZE, ST_SIZE, PT_SIZE, this might allocate multiple frames.
         Ok(0x50000000) // Dummy address
     }

     // Example placeholder for physical frame deallocation
      pub fn free_physical_frame(phys: usize, size: usize) -> Result<(), KError> {
          println!("kmemory::free_physical_frame(phys={:x}, size={}) (placeholder)", phys, size);
         // TODO: Implement actual physical memory deallocation
         Ok(())
      }

      // Example placeholder for getting current task's root page table (physical address)
      pub fn get_current_task_root_page_table() -> Result<usize, KError> {
           // TODO: Retrieve from the current Task Control Block
           Ok(0x10000000) // Dummy root table address
      }

       // High-level page fault handler called by MMU driver
       pub fn handle_page_fault(fault_address: usize, fault_type: MmuFaultType, trap_frame: &mut TrapFrame) -> bool {
            println!("kmemory::handle_page_fault at {:x} ({:?}) (placeholder)", fault_address, fault_type);
           // TODO: Determine if this is a valid fault (e.g., stack growth, demand paging).
           // If yes, allocate memory, map it, and return true.
           // If no, terminate the task and return false.
           false // Not handled
       }
}
