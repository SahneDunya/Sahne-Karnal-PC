#![no_std]

use core::panic::PanicInfo;
// No need for core::fmt::Write or core::fmt if using macros directly
use core::fmt::Write; // <-- Removed

// Need access to the custom print/eprintln macros from Sahne64's stdio_impl in no_std
// Assuming these are made available in the build setup for this component crate,
// potentially via #[macro_use] extern crate sahne64; in the crate root,
// or if Sahne64 exports them directly, using `use sahne64::eprintln;`.
// For this example, we'll use the `cfg` approach assuming they are available.
#[cfg(not(feature = "std"))]
// Assuming Sahne64 provides `println` and `eprintln` macros that work in no_std
// They would likely be exported via something like:
 #[macro_export] macro_rules! println {...}
 #[macro_export] macro_rules! eprintln {...}
// And used in the dependent crate root with #[macro_use] extern crate sahne64;
// Or imported specifically if the Sahne64 crate provides a way to do so.
// Let's assume they are globally available via macro_use or similar.

// The local io module is replaced by the Sahne64 output mechanism
 mod io { /* ... */ } // <-- Removed

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Use the Sahne64 provided eprintln! macro for output
    // This macro is assumed to write to the Sahne64 console resource via syscalls.

    // eprintln! requires the formatting infrastructure and the underlying
    // writing mechanism (like writing to a resource handle).
    // This works if the Sahne64 crate exports `_eprint` or similar functions
    // that the macro expands to, AND those functions use the Sahne64 resource API.

    #[cfg(feature = "std")]
    {
        // In std environment, use standard eprintln!
        std::eprintln!("Kernel Panic:");

        if let Some(location) = info.location() {
            std::eprintln!("  File: {}", location.file());
            std::eprintln!("  Line: {}", location.line());
        }

        if let Some(message) = info.message() {
            std::eprintln!("  Message: {}", message);
        } else if let Some(payload) = info.payload().downcast_ref::<&'static str>() {
             std::eprintln!("  Payload (static str): {}", payload);
        } else if let Some(payload) = info.payload().downcast_ref::<String>() {
             // Note: This branch requires the 'alloc' crate and a global allocator
             // in a no_std environment. It might not be reachable.
             std::eprintln!("  Payload (String): {}", payload);
        } else {
            std::eprintln!("  No panic message or payload available.");
        }
         // Add backtrace if available and configured
          debug_print_backtrace(); // Needs a specific implementation
    }

    #[cfg(not(feature = "std"))]
    {
        // In no_std environment, use Sahne64's eprintln!
        // This requires that the Sahne64 crate provides this macro and it's in scope.
        // It also implies that the underlying resource writing mechanism is functional
        // even in a panic context.

        // Use direct formatting via eprintln! args
        eprintln!("Kernel Panic:"); // Macro call

        if let Some(location) = info.location() {
             eprintln!("  at {}:{}", location.file(), location.line()); // Compact form
             eprintln!("  File: {}", location.file()); // As in original code
             eprintln!("  Line: {}", location.line()); // As in original code
        } else {
            eprintln!("  at unknown location");
        }

        // Format the message or payload
        if let Some(message) = info.message() {
             eprintln!("  Message: {}", message);
        } else if let Some(payload) = info.payload().downcast_ref::<&'static str>() {
             eprintln!("  Payload (static str): {}", payload);
        }
         // Downcasting to String requires `alloc` and global allocator in no_std,
         // which might not be available during panic. So, it's safer to omit this branch
         // or assume it might not work. Let's keep the branch but add a note.
         else if let Some(payload) = info.payload().downcast_ref::<String>() {
              // This branch requires `alloc` and a global allocator.
              // In a no_std panic, these might not be available, leading to issues.
              // Consider removing or guarding with an 'alloc' feature.
              eprintln!("  Payload (String): {}", payload);
         }
         else {
            eprintln!("  No panic message or payload available.");
         }

         // Add backtrace if available and configured for no_std
          #[cfg(feature = "backtrace")]
          print_backtrace_no_std(); // Needs a specific implementation
    }


    // In a real kernel/runtime, you might want to halt the system,
    // reboot, or enter a debug loop here.
    // For no_std, a simple infinite loop is common.
    loop {
        core::hint::spin_loop(); // Hint to the CPU to idle
    }
}

// The local panic handler is now the main one in this file.
// The separate panic handler block at the end of the original code is removed.
 #[cfg(not(feature = "std"))]
 #[panic_handler]
 fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} } // <-- Removed duplicate
