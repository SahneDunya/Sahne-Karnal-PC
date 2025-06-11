#![no_std]

// Import necessary components from core and potentially karnal64
use core::panic::PanicInfo;
// Assuming karnal64 types might be useful, though direct use in panic is limited
 use karnal64::{KError, KHandle, KTaskId}; // Example imports if needed

// --- Architecture Specific Imports ---
// Need to import functions for disabling interrupts and potentially halting
// These would typically come from a low-level architecture-specific module.
// Placeholder:
 use super::cpu; // Assuming a sibling module 'cpu' exists for arch-specific functions
// Or directly using assembly/intrinsics

// --- Panic Console Output (Placeholder) ---
// In a real kernel, this would interact with a minimal, panic-safe console driver.
// It cannot rely on the full kresource system or task scheduling, as they might be
// corrupted or the source of the panic.
// We'll use a placeholder function that assumes a basic printing capability.
#[cfg(target_arch = "aarch64")] // Example: Define for 64-bit ARM
mod panic_console {
    use core::fmt::{Write, Arguments};

    // A very basic, panic-safe writer struct
    struct PanicWriter;

    // Implement the Write trait for formatting
    impl Write for PanicWriter {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            // Placeholder: In a real kernel, this would write directly to a UART
            // or a pre-initialized framebuffer in a panic-safe manner.
            // It MUST NOT allocate memory or rely on complex locks/syscalls.
            #[cfg(debug_assertions)] // Optional: print in debug builds
            {
                // Example: A hypothetical low-level UART write function
                 super::super::uart::uart_put_str(s); // Assuming uart driver exists
                // For now, we'll just ignore it or use a dummy print if available
                #[cfg(feature = "enable_panic_debug_print")] // A feature flag to enable this
                {
                    // A very basic (potentially platform-specific) raw byte output
                    // Example: Assumes a global UART base address or specific function
                    // unsafe { super::super::uart::write_bytes_raw(s.as_bytes()); }
                    // In a simulator or QEMU, semi-hosting might be an option, but not for real hardware panic.
                    // Let's use a dummy print marker for now.
                    // print! is not available in no_std panic handler without a global _print!
                    // Let's use a simple loop over bytes if we can assume a raw byte output function.
                    #[cfg(all(target_arch = "aarch64", target_os = "none"))] // Only for bare-metal AArch64
                    {
                        // This is a highly simplified placeholder.
                        // A real implementation needs a dedicated panic-safe UART driver.
                        extern "C" {
                            // Assume a minimal C function for raw output exists
                            fn panic_put_byte(byte: u8);
                        }
                        for byte in s.as_bytes() {
                             // Avoid printing non-ASCII in panic for simplicity
                            if byte.is_ascii() || *byte == b'\n' || *byte == b'\r' {
                                unsafe { panic_put_byte(*byte); }
                            } else {
                                unsafe { panic_put_byte(b'.'); } // Replace non-ascii
                            }
                        }
                    }
                }
            }
            Ok(())
        }
    }

    // A macro to print during panic using the PanicWriter
    // Similar to println!, but for panic context.
    macro_rules! panic_print {
        ($($arg:tt)*) => {{
            use core::fmt::Write;
            let mut writer = PanicWriter;
            let _ = write!(writer, $($arg)*);
        }};
    }

    // Optional: A macro similar to println! for panic
    macro_rules! panic_println {
        () => ($crate::panic_console::panic_print!("\n"));
        ($($arg:tt)*) => ({
            $crate::panic_console::panic_print!($($arg)*);
            $crate::panic_console::panic_print!("\n");
        });
    }

    pub(crate) use panic_print; // Export for use in the panic handler
    pub(crate) use panic_println; // Export for use in the panic handler

    // A hypothetical panic-safe way to try and get info from kresource
    // This is highly speculative - depends on kresource being panic-safe
    
    use crate::karnal64::kresource; // Need access to kresource module
    pub(crate) fn try_get_console_provider() -> Option<&'static dyn karnal64::ResourceProvider> {
        // This function within kresource would need to be specially designed
        // to avoid locks, allocations, or other unsafe operations during panic.
        // It might return a pre-registered panic console provider if one exists.
        kresource::get_panic_safe_console_provider() // Hypothetical function call
    }
     
}
#[cfg(target_arch = "aarch64")] // Export macros only for AArch64
use panic_console::{panic_print, panic_println};


/// The kernel's panic handler for ARM (aarch64).
/// This function is called when a panic occurs anywhere in the kernel.
#[cfg(target_arch = "aarch64")] // Define panic handler specifically for AArch64
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // 1. Disable interrupts as early as possible.
    // This prevents the panic from being interrupted or causing further issues.
    // This is architecture specific. For AArch64, this might involve writing to system registers.
    // Placeholder for ARM interrupt disable:
    #[cfg(target_arch = "aarch64")]
    unsafe {
        // Example: Disable IRQ and FIQ (requires specific ARM architecture knowledge and potentially assembly)
        // For demonstration, we'll assume a function like `disable_all_interrupts` exists.
         super::cpu::disable_all_interrupts(); // Call hypothetical CPU function
        // Or inline assembly:
        core::arch::asm!(
            "mrs x0, daif", // Read interrupt mask bits
            "orr x0, x0, #0b1111", // Set bits A, I, F, D (SError, IRQ, FIQ, Debug)
            "msr daif, x0", // Write back
            options(nomem, nostack)
        );
    }


    // 2. Print panic location and message.
    // Use the panic-safe console output mechanism.
    panic_println!("--- KERNEL PANIC ---");

    if let Some(location) = info.location() {
        panic_println!("Location: {}:{}:{}", location.file(), location.line(), location.column());
    } else {
        panic_println!("Location: Unknown");
    }

    if let Some(message) = info.message() {
        panic_print!("Message: ");
        // The message is a FormattedArguments, need to write it to our panic writer
        let mut writer = panic_console::PanicWriter;
        let _ = writer.write_fmt(*message); // Write the formatted message
        panic_println!(""); // Newline after message
    } else {
        panic_println!("Message: None");
    }

    // 3. (Optional) Print Task/Thread ID.
    // This requires a panic-safe way to access the current task info from the ktask module.
    
    // Hypothetical panic-safe access to current task ID
    if let Some(current_task_id) = crate::karnal64::ktask::get_current_task_id_panic_safe() {
         panic_println!("Task ID: {}", current_task_id.0); // Assuming KTaskId is transparent u64
    }
     

    // 4. (Optional) Print CPU State (Registers).
    // This is highly architecture-specific and requires reading CPU registers.
    // Placeholder:
    #[cfg(target_arch = "aarch64")]
    {
        panic_println!("CPU State (Partial):");
        // Example: Read a few general purpose registers (requires assembly)
        unsafe {
             let mut x0: u64;
             let mut x1: u64;
             // ... read more registers as needed
             core::arch::asm!("mov {0}, x0", out(reg) x0, options(nomem, nostack));
             core::arch::asm!("mov {0}, x1", out(reg) x1, options(nomem, nostack));
             // Print them using panic_println
             panic_println!("  x0 = {:#x}, x1 = {:#x}, ...", x0, x1);
             // Reading all registers in a generic way in Rust `asm!` is complex.
             // Often requires a dedicated assembly helper function called from here.
        }
    }


    // 5. Halt the system.
    // Enter an infinite loop or trigger a system reset if desired/possible.
    panic_println!("Kernel halted.");
    loop {
        // Architecture-specific halt instruction might go here
        // For AArch64, often a WFI (Wait For Interrupt) or a simple infinite loop is used.
        core::arch::asm!("wfi", options(nomem, nostack));
    }
}

// Required for `no_std` when `panic_handler` is defined, if global allocator is not used.
// This is typically not needed for a panic handler itself, but might be required
// by the build system depending on overall project setup.
 #[cfg(not(test))] // Typically only needed if global_allocator is used elsewhere
 #[alloc_error_handler]
 fn alloc_error(_layout: core::alloc::Layout) -> ! {
     // If allocation fails during panic handling (which should be avoided!),
     // this would be called. It's a very bad state.
     panic_println!("--- KERNEL PANIC: ALLOC ERROR ---");
     // Call the panic handler again? This might be recursive.
     // Simplest is to just halt.
     loop {
         #[cfg(target_arch = "aarch64")]
         unsafe { core::arch::asm!("wfi", options(nomem, nostack)); }
     }
 }
