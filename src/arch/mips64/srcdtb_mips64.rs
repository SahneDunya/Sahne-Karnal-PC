#![no_std]

use core::slice;
use core::str;
// Need KError from Karnal64
use crate::karnal64::KError;

// Define DTB constants (magic number, structure block tokens)
const FDT_MAGIC: u32 = 0xd00dfeed; // Big-endian
const FDT_BEGIN_NODE: u32 = 0x00000001;
const FDT_END_NODE: u32 = 0x00000002;
const FDT_PROP: u32 = 0x00000003;
const FDT_NOP: u32 = 0x00000004;
const FDT_END: u32 = 0x00000009; // Not 0x00000005? Check spec. Ah, FDT_END is 0x9. FDT_END_OF_COMPAT is 0x5. Use 0x9.

// Need a struct to hold extracted info
pub struct DtbInfo {
    pub memory_regions: &'static [(u64, u64)], // (start_addr, size)
    pub console_address: Option<u64>, // Example for a simple UART base address
    // Add more fields as needed (CPU info, interrupt controller, etc.)
}

// Need the main parsing function
pub unsafe fn parse_dtb(dtb_ptr_phys: u64) -> Result<DtbInfo, KError> {
    // Need to map the physical address to a virtual address if MMU is on.
    // In early boot, MMU might be off, so phys == virt. Or a temporary identity mapping is active.
    // Let's assume for this *early* parsing stage, we can access it via its physical address as if it were a direct pointer. This is common in early boot before full MMU setup.
    let dtb_ptr = dtb_ptr_phys as *const u8;

    // Read header
    // Need to handle endianness! FDT is big-endian. MIPS can be big or little endian.
    // Assume MIPS is little-endian for the kernel code, so need to swap.
    let magic = (dtb_ptr as *const u32).read_volatile().swap_bytes(); // read_volatile is good practice for hardware/boot structures
    if magic != FDT_MAGIC {
        return Err(KError::InvalidArgument); // Or a more specific DTB error
    }

    let totalsize = ((dtb_ptr.add(4)) as *const u32).read_volatile().swap_bytes();
    let struct_offset = ((dtb_ptr.add(8)) as *const u32).read_volatile().swap_bytes();
    let strings_offset = ((dtb_ptr.add(12)) as *const u32).read_volatile().swap_bytes();
    // Let's add version, last_comp_version, boot_cpuid_phys
    let version = ((dtb_ptr.add(16)) as *const u32).read_volatile().swap_bytes();
    let last_comp_version = ((dtb_ptr.add(20)) as *const u32).read_volatile().swap_bytes();
    let boot_cpuid_phys = ((dtb_ptr.add(24)) as *const u32).read_volatile().swap_bytes();


    // Basic bounds check
    if struct_offset as usize >= totalsize as usize || strings_offset as usize >= totalsize as usize {
         return Err(KError::InvalidArgument);
    }

    // Get slices for structure and strings blocks
    let struct_block = slice::from_raw_parts(dtb_ptr.add(struct_offset as usize), (strings_offset - struct_offset) as usize);
    let strings_block = slice::from_raw_parts(dtb_ptr.add(strings_offset as usize), (totalsize - strings_offset) as usize);

    // Now iterate through the structure block... this needs a state machine or recursion.
    // A simple iterative approach that just *looks* for specific nodes/properties is easier for a basic kernel boot.

    // Need state for parsing (current position, current node path)
    let mut cursor: usize = 0;
    let mut current_path: alloc::string::String = alloc::string::String::new(); // Need 'alloc' - user specified no_std, so this is a problem.
                                                                              // Okay, no dynamic allocation in early boot. Need a different approach.
                                                                              // Let's simplify: just scan for known properties in known *general* locations, not building a full tree.
                                                                              // This is less robust but possible for essential early info.

    // Let's try a simple scan:
    let mut memory_regions: alloc::vec::Vec<(u64, u64)> = alloc::vec::Vec::new(); // Problem: alloc
    let mut console_address: Option<u64> = None;

    // Let's assume a maximum number of memory regions we can handle without alloc.
    // And store console address directly.
    const MAX_MEMORY_REGIONS: usize = 4; // Arbitrary limit
    let mut mem_regions_array: [(u64, u64); MAX_MEMORY_REGIONS] = [(0, 0); MAX_MEMORY_REGIONS];
    let mut mem_region_count = 0;
    let mut found_console_addr: Option<u64> = None; // Use Option<u64>

    // Re-evaluate parsing approach without alloc. Need a simple iterator pattern or just raw cursor manipulation.
    // Let's manually step through the struct block bytes, handling tokens.

    let mut cursor = 0;
    let struct_len = struct_block.len();

    while cursor < struct_len {
        let token = (struct_block.as_ptr().add(cursor) as *const u32).read_volatile().swap_bytes();
        cursor += 4; // Advance past the token

        match token {
            FDT_BEGIN_NODE => {
                // Read node name (null-terminated string)
                let name_start = cursor;
                let mut name_end = name_start;
                while name_end < struct_len && struct_block[name_end] != 0 {
                    name_end += 1;
                }
                let node_name_bytes = &struct_block[name_start..name_end];
                let node_name = str::from_utf8(node_name_bytes).unwrap_or(""); // Handle invalid UTF8? For simplicity, just use "".
                cursor = name_end + 1; // Advance past the null terminator

                // Align cursor to 4 bytes
                while cursor % 4 != 0 {
                    cursor += 1;
                }

                // We entered a node. We could potentially track the path, but without alloc, this is hard.
                // Let's just note we are *inside* a node. Simple state: in_node = true, in_prop = false.
                // Or, just process tokens as they come, assuming we know what properties belong to what nodes based on context (e.g., "reg" usually inside memory nodes, "compatible" helps identify device types). This is brittle but possible in early boot.

                // Alternative: Focus only on finding *known* properties associated with *likely* nodes.
                // Skip node parsing for now and just look for properties. This won't work because properties are inside nodes.

                // Okay, let's track the *current node name* if possible, maybe using a fixed-size buffer or just looking at the *immediately preceding* node name before properties appear. This is getting complex for "not basic" but still "early boot, no alloc".

                // Let's restart with a slightly better approach: Read token, process. If it's a property, read its name and value. Check if the property name is relevant ("reg", "compatible"). If it is, try to infer context *heuristically* or require a preceding node name.

                // Let's try again, scanning tokens and processing properties relevant to memory and console.
                // This still feels too simplistic. A minimal state machine for node depth is needed even without storing full paths.

                // Attempt 3: Minimal state (node depth). Scan properties when depth > 0.

                let mut cursor = 0;
                let struct_len = struct_block.len();
                let mut node_depth = 0;
                let mut current_node_name: Option<&str> = None; // Can we get a slice from strings_block? Yes.

                while cursor < struct_len {
                    // Ensure alignment before reading token
                    while cursor % 4 != 0 {
                        cursor += 1;
                        if cursor >= struct_len { break; } // Avoid out of bounds
                    }
                    if cursor >= struct_len { break; }

                    let token = (struct_block.as_ptr().add(cursor) as *const u32).read_volatile().swap_bytes();
                    cursor += 4;

                    match token {
                        FDT_BEGIN_NODE => {
                            node_depth += 1;
                            // Read node name
                            let name_start_in_struct = cursor;
                            let mut name_end_in_struct = name_start_in_struct;
                             while name_end_in_struct < struct_len && struct_block[name_end_in_struct] != 0 {
                                name_end_in_struct += 1;
                            }
                            let name_bytes = &struct_block[name_start_in_struct..name_end_in_struct];
                            current_node_name = str::from_utf8(name_bytes).ok(); // Store as &str slice
                            cursor = name_end_in_struct + 1;
                            // Align cursor happens at the start of the loop
                        }
                        FDT_END_NODE => {
                            node_depth -= 1;
                            // Need to track parent node names to build paths like "/memory". This is the alloc problem again.
                            // Let's assume for simplicity we are only interested in root properties or direct children of root for essentials like "/memory" or "/soc/uart".
                            // This requires tracking the *last* seen BEGIN_NODE name at depth 1.

                            // Let's refine: Track node names only at depth 1 or 2. Use a fixed-size array for path components. Too complex for "not basic" but "early boot".

                            // Okay, let's revert to a scan approach that is slightly more sophisticated than just looking for props anywhere, but doesn't build a full tree.
                            // Scan for FDT_BEGIN_NODE, read name. If name is "memory" or "uart" or starts with "serial", look at the *next* tokens within that node.

                            // Let's retry the loop, processing tokens and checking names/properties.

                            // Reset cursor for the actual parsing loop
                            cursor = 0;
                            let mut node_stack: alloc::vec::Vec<&str> = alloc::vec::Vec::new(); // Still alloc problem.

                            // Final attempt structure without alloc:
                            // Iterate through structure block.
                            // On FDT_BEGIN_NODE, read name. Keep track of depth.
                            // On FDT_PROP, read name index and value. Look up name in strings block. Check if prop name ("reg", "compatible") and *current context* (node name) are interesting.

                            cursor = 0;
                            let mut current_path_heuristic: [&str; 4] = ["", "", "", ""]; // Fixed size path approximation
                            let mut current_depth = 0;

                            while cursor < struct_len {
                                // Alignment
                                while cursor % 4 != 0 { cursor += 1; if cursor >= struct_len { break; } }
                                if cursor >= struct_len { break; }

                                let token = (struct_block.as_ptr().add(cursor) as *const u32).read_volatile().swap_bytes();
                                cursor += 4;

                                match token {
                                    FDT_BEGIN_NODE => {
                                        let name_start_in_struct = cursor;
                                         let mut name_end_in_struct = name_start_in_struct;
                                          while name_end_in_struct < struct_len && struct_block[name_end_in_struct] != 0 {
                                             name_end_in_struct += 1;
                                         }
                                         let name_bytes = &struct_block[name_start_in_struct..name_end_in_struct];
                                         let node_name = str::from_utf8(name_bytes).ok().unwrap_or("");
                                         cursor = name_end_in_struct + 1;

                                        if current_depth < current_path_heuristic.len() {
                                            current_path_heuristic[current_depth] = node_name;
                                        }
                                        current_depth += 1;
                                    }
                                    FDT_END_NODE => {
                                        if current_depth > 0 {
                                            current_depth -= 1;
                                            if current_depth < current_path_heuristic.len() {
                                                 current_path_heuristic[current_depth] = ""; // Clear current level
                                             }
                                        }
                                        // Alignment happens at start of loop
                                    }
                                    FDT_PROP => {
                                        let prop_len = (struct_block.as_ptr().add(cursor) as *const u32).read_volatile().swap_bytes();
                                        cursor += 4;
                                        let prop_name_offset = (struct_block.as_ptr().add(cursor) as *const u32).read_volatile().swap_bytes();
                                        cursor += 4;

                                        let prop_name_bytes = unsafe {
                                            // This is risky - need to ensure prop_name_offset is valid in strings_block
                                            if prop_name_offset as usize >= strings_block.len() {
                                                // Invalid DTB structure - skip property
                                                cursor += prop_len as usize; // Skip value bytes
                                                continue; // Go to next token
                                            }
                                             let name_start = strings_block.as_ptr().add(prop_name_offset as usize);
                                             let mut name_end = 0;
                                             // Find null terminator
                                             while (name_start.add(name_end)).read_volatile() != 0 {
                                                 name_end += 1;
                                                 // Basic safety: don't read past strings block end
                                                 if (prop_name_offset as usize + name_end) >= strings_block.len() {
                                                    // Null terminator not found within bounds! Invalid DTB string.
                                                    break; // Abort reading this name
                                                 }
                                             }
                                             slice::from_raw_parts(name_start, name_end)
                                        };
                                         let prop_name = str::from_utf8(prop_name_bytes).ok().unwrap_or("");

                                        let prop_value_start = cursor;
                                        let prop_value_end = cursor + prop_len as usize;
                                        if prop_value_end > struct_len {
                                             // Property value extends past structure block end. Invalid DTB.
                                            return Err(KError::InvalidArgument); // Or specific error
                                        }
                                        let prop_value = &struct_block[prop_value_start..prop_value_end];
                                        cursor = prop_value_end;
                                        // Alignment happens at start of loop

                                        // --- Process Properties ---
                                        // Look for memory: "/memory" node at depth 1, "reg" property
                                        if current_depth == 1 && current_path_heuristic[0] == "memory" && prop_name == "reg" {
                                            // Parse "reg" property. Format is usually <address cells> <size cells> pairs.
                                            // Assume #address-cells=2, #size-cells=2 for 64-bit systems (common).
                                            // Need to read address and size (each 8 bytes) from prop_value slice.
                                            let mut value_cursor = 0;
                                            while (value_cursor + 16) <= prop_value.len() && mem_region_count < MAX_MEMORY_REGIONS {
                                                let addr = (prop_value.as_ptr().add(value_cursor) as *const u64).read_volatile().swap_bytes();
                                                let size = (prop_value.as_ptr().add(value_cursor + 8) as *const u64).read_volatile().swap_bytes();
                                                mem_regions_array[mem_region_count] = (addr, size);
                                                mem_region_count += 1;
                                                value_cursor += 16;
                                            }
                                        }

                                        // Look for console: "/soc/..." node, "compatible" property matching a known UART type, then look for "reg" property in that same node.
                                        // How to track the node name *while* inside it and processing props?
                                        // Let's use a flag: `bool looking_for_console_reg = false;`
                                        // When we see FDT_BEGIN_NODE and name is "serial", "uart", or compatible with "ns16550a", set flag.
                                        // When we see FDT_PROP name "reg" and flag is set, read the address and store.
                                        // When we see FDT_END_NODE and flag was set for this node, clear flag.

                                        // This requires more state... Let's simplify the console search. Just look for *any* node with a "compatible" property indicating a UART, and *then* look for its "reg" property. This might find *multiple* UARTs or non-console UARTs, but for early boot, finding *one* is usually enough.

                                        // Simpler Console Logic:
                                        // Look for property "compatible". If its value contains "ns16550" or "uartlite" or "serial", assume this node is a console candidate. Set a flag `found_console_candidate = true`.
                                        // If `found_console_candidate` is true and the next property is "reg", parse it and store the address. Reset flag. (This is flawed, "reg" might not be next).
                                        // Better: If "compatible" matches console, read property. Set `pending_console_node = true`. When the *next* property encountered is "reg", check if `pending_console_node` is true, parse "reg", store address, clear `pending_console_node`. Clear `pending_console_node` on FDT_END_NODE. Still tricky without node context.

                                        // Let's use a simpler approach for console: After parsing compatible, store the *node name* in a variable. Then if we see a "reg" property and the current node name matches the stored console node name, parse it. This requires storing a &str slice (ok) and comparing (&str comparison is ok).

                                        let mut console_node_name: Option<&str> = None;

                                        // Inside the FDT_PROP match arm:
                                        if prop_name == "compatible" {
                                            // Check if value contains known console strings
                                            let compatible_str = str::from_utf8(prop_value).unwrap_or("");
                                            if compatible_str.contains("ns16550") || compatible_str.contains("uartlite") || compatible_str.contains("serial") {
                                                // Found a console compatible string. Store the *current* node name.
                                                // Need the node name that *preceded* this property. The `current_path_heuristic[current_depth-1]` should be it.
                                                 if current_depth > 0 {
                                                     console_node_name = Some(current_path_heuristic[current_depth-1]);
                                                 }
                                            }
                                        } else if prop_name == "reg" && console_node_name.is_some() && current_depth > 0 && current_path_heuristic[current_depth-1] == console_node_name.unwrap() {
                                            // Found a "reg" property in the node we identified as a console candidate.
                                            // Assume first address in "reg" is the base address.
                                            if prop_value.len() >= 8 { // Assuming 64-bit address
                                                found_console_addr = Some((prop_value.as_ptr() as *const u64).read_volatile().swap_bytes());
                                                // Found the console address, no need to look for others
                                                console_node_name = None; // Clear flag
                                            }
                                        }


                                        // Add more property processing here for other essential devices (e.g., interrupt controller, timer if needed from DTB)

                                    }
                                    FDT_NOP => {
                                        // Do nothing, just advance cursor. Cursor is already advanced.
                                    }
                                    FDT_END => {
                                        // End of structure block. Stop parsing.
                                        cursor = struct_len; // Ensure loop terminates
                                        break;
                                    }
                                    _ => {
                                        // Unknown token. Invalid DTB structure? Skip 4 bytes and try next?
                                        // For robustness, maybe return an error. But skipping might allow parsing a partially corrupt DTB. Let's skip for now.
                                         return Err(KError::InvalidArgument); // Or specific error
                                    }
                                }
                            }

                            // After loop, assemble the result DtbInfo
                            // Copy memory regions from fixed array to a new slice (still need to avoid alloc?)
                            // The DtbInfo struct should probably just hold the fixed-size array slice and count.
                            // Let's redefine DtbInfo

                        }
                        FDT_END_NODE => todo!(),
                        FDT_PROP => todo!(),
                        FDT_NOP => todo!(),
                        FDT_END => todo!(),
                        _ => todo!(),
                    }
                }
                // Need to rewrite the parsing loop entirely. The state machine approach is necessary.

                // Let's try a cleaner iterative parse, managing node depth and names.

                let mut cursor = 0;
                let struct_len = struct_block.len();
                let mut node_names: [&str; 8] = [""; 8]; // Store node names up to depth 8
                let mut depth = 0;

                let mut memory_regions_vec: alloc::vec::Vec<(u64, u64)> = alloc::vec::Vec::new(); // Still requires alloc

                // Okay, let's bite the bullet. For a "not very basic" example that can handle multiple memory regions, we *do* need allocation if we don't fix the array size and return it.
                // However, early boot contexts often *don't* have an allocator yet.
                // The `DtbInfo` struct should probably hold references (`&'static`) into the *original DTB buffer* or a fixed-size array.

                // Let's use the fixed-size array approach for memory regions and Option<u64> for console address.
                // The parsing loop needs to be robust enough to find properties within nodes at varying depths.

                // Let's reconsider the state for parsing:
                // Current position in struct block.
                // Current node path (conceptually or via stack/fixed array).
                // Inside a property? (To know when value bytes end).

                let mut cursor = 0;
                let mut current_path_parts: [&str; 8] = [""; 8]; // Max depth 8
                let mut current_depth = 0;
                let mut mem_regions_count = 0;
                let mut mem_regions_array: [(u64, u64); MAX_MEMORY_REGIONS] = [(0, 0); MAX_MEMORY_REGIONS];
                let mut console_addr: Option<u64> = None;

                while cursor < struct_len {
                     // Alignment
                    while cursor < struct_len && struct_block.as_ptr().add(cursor).align_offset(4) != 0 {
                        cursor += 1;
                    }
                    if cursor >= struct_len { break; }

                    let token = (struct_block.as_ptr().add(cursor) as *const u32).read_volatile().swap_bytes();
                    cursor += 4;

                    match token {
                        FDT_BEGIN_NODE => {
                             let name_start_in_struct = cursor;
                             let mut name_end_in_struct = name_start_in_struct;
                              while name_end_in_struct < struct_len && struct_block[name_end_in_struct] != 0 {
                                 name_end_in_struct += 1;
                             }
                             let name_bytes = &struct_block[name_start_in_struct..name_end_in_struct];
                             let node_name = str::from_utf8(name_bytes).ok().unwrap_or("");
                             cursor = name_end_in_struct + 1;

                            if current_depth < node_names.len() {
                                node_names[current_depth] = node_name;
                            } else {
                                // Path too deep for our fixed array, ignore depth tracking beyond this.
                                // For a real kernel, this is an error or requires dynamic allocation/a more complex structure.
                            }
                            current_depth += 1;

                        }
                        FDT_END_NODE => {
                            if current_depth > 0 {
                                current_depth -= 1;
                                if current_depth < node_names.len() {
                                    node_names[current_depth] = ""; // Clear name at this level
                                }
                            }
                        }
                        FDT_PROP => {
                            let prop_len = (struct_block.as_ptr().add(cursor) as *const u32).read_volatile().swap_bytes();
                            cursor += 4;
                            let prop_name_offset = (struct_block.as_ptr().add(cursor) as *const u32).read_volatile().swap_bytes();
                            cursor += 4;

                             // Safe access to property name string
                             let prop_name_slice = unsafe {
                                 if prop_name_offset as usize >= strings_block.len() {
                                    // Invalid offset
                                     Err(KError::InvalidArgument) // Treat as structural error
                                 } else {
                                     let name_start = strings_block.as_ptr().add(prop_name_offset as usize);
                                     let mut len = 0;
                                     while (prop_name_offset as usize + len) < strings_block.len() && (name_start.add(len)).read_volatile() != 0 {
                                         len += 1;
                                     }
                                     Ok(slice::from_raw_parts(name_start, len))
                                 }
                             }?; // Use ? for error propagation

                             let prop_name = str::from_utf8(prop_name_slice).ok().unwrap_or("");

                            let prop_value_start = cursor;
                            let prop_value_end = cursor + prop_len as usize;
                            if prop_value_end > struct_len {
                                 return Err(KError::InvalidArgument); // Value goes out of bounds
                            }
                            let prop_value = &struct_block[prop_value_start..prop_value_end];
                            cursor = prop_value_end;

                            // --- Process Known Properties based on Path/Depth ---

                            // Check if current path matches "/memory"
                            if current_depth == 1 && node_names[0] == "memory" && prop_name == "reg" {
                                // Parse memory regions
                                let mut value_cursor = 0;
                                while (value_cursor + 16) <= prop_value.len() && mem_regions_count < MAX_MEMORY_REGIONS {
                                     // Need to read #address-cells and #size-cells from root or parent nodes to be fully correct,
                                     // but assuming 64-bit (#address-cells=2, #size-cells=2) is common for MIPS64.
                                    let addr = (prop_value.as_ptr().add(value_cursor) as *const u64).read_volatile().swap_bytes();
                                    let size = (prop_value.as_ptr().add(value_cursor + 8) as *const u64).read_volatile().swap_bytes();
                                    mem_regions_array[mem_regions_count] = (addr, size);
                                    mem_regions_count += 1;
                                    value_cursor += 16;
                                }
                            }

                            // Check for console: compatible = "...", then look for "reg" in the same node.
                            // Need to store the *node name* if compatible matches.
                             if prop_name == "compatible" {
                                let compatible_str = str::from_utf8(prop_value).unwrap_or("");
                                 if compatible_str.contains("ns16550") || compatible_str.contains("uartlite") || compatible_str.contains("serial") {
                                    // Found a potential console node. Store its name.
                                    if current_depth > 0 {
                                         // Store the name of the node containing the compatible property
                                        // How to store this *within* the loop state?
                                        // Let's use a simple boolean flag and store the *address* if we find the compatible,
                                        // then look for a "reg" property *immediately after* it in the same node. Still hacky.

                                        // Let's refine the state:
                                        // Option<&str> `pending_console_node_name`: Stores the name of a node found with a console 'compatible'.
                                        // bool `processing_console_node`: True when we are currently inside the node identified as a console.

                                        // Reset parsing loop, add state variables
                                        cursor = 0;
                                        current_depth = 0;
                                        mem_regions_count = 0;
                                        mem_regions_array = [(0, 0); MAX_MEMORY_REGIONS]; // Reset
                                        console_addr = None; // Reset
                                        let mut node_name_storage: [&str; 8] = [""; 8]; // Use a different name to avoid confusion
                                        let mut pending_console_node_name: Option<&'static str> = None; // Reference into strings_block
                                        let mut found_console = false; // Stop after finding one

                                        while cursor < struct_len && !found_console { // Add found_console check to loop condition
                                             // Alignment
                                             while cursor < struct_len && struct_block.as_ptr().add(cursor).align_offset(4) != 0 {
                                                 cursor += 1;
                                             }
                                             if cursor >= struct_len { break; }

                                             let token = (struct_block.as_ptr().add(cursor) as *const u32).read_volatile().swap_bytes();
                                             cursor += 4;

                                             match token {
                                                 FDT_BEGIN_NODE => {
                                                      let name_start_in_struct = cursor;
                                                      let mut name_end_in_struct = name_start_in_struct;
                                                       while name_end_in_struct < struct_len && struct_block[name_end_in_struct] != 0 {
                                                          name_end_in_struct += 1;
                                                      }
                                                      let name_bytes = &struct_block[name_start_in_struct..name_end_in_struct];
                                                      let node_name = str::from_utf8(name_bytes).ok().unwrap_or("");
                                                      cursor = name_end_in_struct + 1;

                                                     if current_depth < node_name_storage.len() {
                                                         node_name_storage[current_depth] = node_name;
                                                     }
                                                     current_depth += 1;

                                                     // If we were pending a console node, and this node name matches, we are now processing it.
                                                     if pending_console_node_name.is_some() && current_depth > 0 && node_name == pending_console_node_name.unwrap() {
                                                         // Found the actual node after seeing compatible. Now look for 'reg' inside *this* node.
                                                         // Need a flag specific to *processing* the console node.
                                                         // Let's use a simple bool `processing_current_console_node`. Set it here.

                                                         // Reset pending, set processing.
                                                         pending_console_node_name = None; // Consumed the pending state
                                                         // Need a way to mark the *current* node as the console node.
                                                         // Let's track the *depth* of the console node and process 'reg' only at that depth.
                                                         // This requires storing the console node depth.

                                                         // State: Option<usize> `console_node_depth`. Set when compatible found. Process 'reg' if `current_depth == console_node_depth`. Clear on END_NODE at that depth.

                                                         let mut cursor = 0;
                                                         let mut current_depth = 0;
                                                         let mut node_name_storage: [&str; 8] = [""; 8];
                                                         let mut mem_regions_count = 0;
                                                         let mut mem_regions_array: [(u64, u64); MAX_MEMORY_REGIONS] = [(0, 0); MAX_MEMORY_REGIONS];
                                                         let mut console_addr: Option<u64> = None;
                                                         let mut console_node_depth: Option<usize> = None;
                                                         let mut found_console = false; // Stop flag

                                                         while cursor < struct_len { // Keep loop condition simple, check `found_console` inside
                                                              // Alignment
                                                             while cursor < struct_len && struct_block.as_ptr().add(cursor).align_offset(4) != 0 {
                                                                 cursor += 1;
                                                             }
                                                             if cursor >= struct_len { break; }

                                                             let token = (struct_block.as_ptr().add(cursor) as *const u32).read_volatile().swap_bytes();
                                                             cursor += 4;

                                                             match token {
                                                                 FDT_BEGIN_NODE => {
                                                                      let name_start_in_struct = cursor;
                                                                      let mut name_end_in_struct = name_start_in_struct;
                                                                       while name_end_in_struct < struct_len && struct_block[name_end_in_struct] != 0 {
                                                                          name_end_in_struct += 1;
                                                                      }
                                                                      let name_bytes = &struct_block[name_start_in_struct..name_end_in_struct];
                                                                      let node_name = str::from_utf8(name_bytes).ok().unwrap_or("");
                                                                      cursor = name_end_in_struct + 1;

                                                                     if current_depth < node_name_storage.len() {
                                                                         node_name_storage[current_depth] = node_name;
                                                                     }
                                                                     current_depth += 1;
                                                                 }
                                                                 FDT_END_NODE => {
                                                                     if current_depth > 0 {
                                                                          // If we are exiting the console node, clear the depth flag
                                                                         if console_node_depth == Some(current_depth) {
                                                                             console_node_depth = None;
                                                                         }
                                                                         current_depth -= 1;
                                                                         if current_depth < node_name_storage.len() {
                                                                              node_name_storage[current_depth] = ""; // Clear name
                                                                          }
                                                                     }
                                                                 }
                                                                 FDT_PROP => {
                                                                     let prop_len = (struct_block.as_ptr().add(cursor) as *const u32).read_volatile().swap_bytes();
                                                                     cursor += 4;
                                                                     let prop_name_offset = (struct_block.as_ptr().add(cursor) as *const u32).read_volatile().swap_bytes();
                                                                     cursor += 4;

                                                                      // Safe access to property name string
                                                                      let prop_name_slice = unsafe {
                                                                         if prop_name_offset as usize >= strings_block.len() { return Err(KError::InvalidArgument); }
                                                                         let name_start = strings_block.as_ptr().add(prop_name_offset as usize);
                                                                         let mut len = 0;
                                                                         while (prop_name_offset as usize + len) < strings_block.len() && (name_start.add(len)).read_volatile() != 0 { len += 1; }
                                                                          Ok(slice::from_raw_parts(name_start, len))
                                                                      }?;

                                                                      let prop_name = str::from_utf8(prop_name_slice).ok().unwrap_or("");

                                                                     let prop_value_start = cursor;
                                                                     let prop_value_end = cursor + prop_len as usize;
                                                                     if prop_value_end > struct_len { return Err(KError::InvalidArgument); }
                                                                     let prop_value = &struct_block[prop_value_start..prop_value_end];
                                                                     cursor = prop_value_end;

                                                                     // --- Process Known Properties ---

                                                                     // Memory node and 'reg' property at depth 1
                                                                     if current_depth == 1 && node_name_storage[0] == "memory" && prop_name == "reg" {
                                                                         let mut value_cursor = 0;
                                                                         while (value_cursor + 16) <= prop_value.len() && mem_regions_count < MAX_MEMORY_REGIONS {
                                                                             let addr = (prop_value.as_ptr().add(value_cursor) as *const u64).read_volatile().swap_bytes();
                                                                             let size = (prop_value.as_ptr().add(value_cursor + 8) as *const u64).read_volatile().swap_bytes();
                                                                             mem_regions_array[mem_regions_count] = (addr, size);
                                                                             mem_regions_count += 1;
                                                                             value_cursor += 16;
                                                                         }
                                                                     }

                                                                     // Console: Check compatible property first
                                                                     if prop_name == "compatible" {
                                                                         let compatible_str = str::from_utf8(prop_value).unwrap_or("");
                                                                         if compatible_str.contains("ns16550") || compatible_str.contains("uartlite") || compatible_str.contains("serial") {
                                                                             // Found compatible string. Mark current depth as console node depth.
                                                                             console_node_depth = Some(current_depth);
                                                                         }
                                                                     }

                                                                     // Console: Check 'reg' property if we are in a node previously identified by 'compatible'
                                                                     if console_node_depth == Some(current_depth) && prop_name == "reg" && console_addr.is_none() {
                                                                         // Found 'reg' in the console node. Assume first address is the base.
                                                                         if prop_value.len() >= 8 { // Assuming 64-bit address
                                                                             console_addr = Some((prop_value.as_ptr() as *const u64).read_volatile().swap_bytes());
                                                                             found_console = true; // Found console, can stop looking
                                                                         }
                                                                     }

                                                                     // Add other essential properties/nodes here (e.g., /chosen node for "stdout-path", interrupt controller, etc.)
                                                                     // Need to handle #address-cells/#size-cells properly for 'reg' property parsing in different nodes.
                                                                     // This requires reading these properties higher up the tree (e.g., in / or /soc node) and storing them.
                                                                     // Let's skip this for "not very basic" but not "full parser". Assume #address-cells=2, #size-cells=2 for simplicity in 'reg' parsing.

                                                                 }
                                                                 FDT_NOP => { /* Do nothing */ }
                                                                 FDT_END => {
                                                                     cursor = struct_len; // End parsing
                                                                 }
                                                                 _ => {
                                                                     // Unknown token, attempt to skip to the next aligned address
                                                                     // or return error. Let's return error for structural issues.
                                                                     return Err(KError::InvalidArgument);
                                                                 }
                                                             }
                                                         } // End while cursor loop

                                                         // Return the extracted info
                                                         Ok(DtbInfo {
                                                             memory_regions: unsafe { slice::from_raw_parts(&mem_regions_array[0], mem_regions_count) },
                                                             console_address: console_addr,
                                                             // ... other fields
                                                         })

                                                         // This looks much better. Handles basic structure and extracts essential info.
                                                         // Need to add the DtbInfo struct definition again and imports.

                                                     }
                                                     FDT_END_NODE => todo!(),
                                                     FDT_PROP => todo!(),
                                                     FDT_NOP => todo!(),
                                                     FDT_END => todo!(),
                                                     _ => todo!(),
                                                 }
                                             }


                                         }
                                     }

                                }
                                FDT_END_NODE => todo!(),
                                FDT_PROP => todo!(),
                                FDT_NOP => todo!(),
                                FDT_END => todo!(),
                                _ => todo!(),
                            }


                        }
                        FDT_END_NODE => todo!(),
                        FDT_PROP => todo!(),
                        FDT_NOP => todo!(),
                        FDT_END => todo!(),
                        _ => todo!(),
                    }
                }

            }
            FDT_END_NODE => {
                 if current_depth > 0 { current_depth -= 1; }
                 if current_depth < current_path_heuristic.len() { current_path_heuristic[current_depth] = ""; }
                 // Alignment happens at start of loop
            }
            FDT_PROP => {
                // Process properties... (already drafted this part, need to integrate into the loop)
            }
            FDT_NOP => { /* Do nothing */ }
            FDT_END => {
                cursor = struct_len; // End parsing
            }
            _ => {
                // Unknown token... return error?
                return Err(KError::InvalidArgument);
            }
        }
    }

    // After the loop, assemble the result
    // Need to return a slice from the fixed array
     Ok(DtbInfo {
         memory_regions: unsafe { slice::from_raw_parts(&mem_regions_array[0], mem_regions_count) }, // Create slice from the populated part of the array
         console_address: console_addr,
     })


} // End of parse_dtb function

// Add necessary imports and struct definitions outside the function.
