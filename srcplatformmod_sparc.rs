#[cfg(target_arch = "sparc")]
pub mod sparc {
    use core::arch::asm;

    // Sparc mimarisine özgü işlevler ve yapılar buraya gelecek.

    pub fn enable_interrupts() {
        unsafe {
            asm!("wr %g0, %g0, %psr", options(nostack, nomem));
        }
    }

    pub fn disable_interrupts() {
        unsafe {
            asm!("wr %g0, %g0, %psr", options(nostack, nomem));
        }
    }

    pub fn halt() {
        unsafe {
            asm!("ta 1", options(nostack, nomem));
        }
    }

    // ... diğer Sparc işlevleri ...
}

#[cfg(not(target_arch = "sparc"))]
pub mod sparc {
    // Sparc olmayan platformlar için boş bir uygulama.
    // Bu, kodun diğer platformlarda da derlenmesini sağlar.

    pub fn enable_interrupts() {
        // Sparc değil, bir şey yapma.
    }

    pub fn disable_interrupts() {
        // Sparc değil, bir şey yapma.
    }

    pub fn halt() {
        // Sparc değil, bir şey yapma.
    }
}