# Sahne-Karnal-PC
Sahne Karnal PC is a kernel version designed for desktop computers by Sahne Dünya, a member of Sahne Karnal. Sahne DünyaOS is used in SQUAD and desktop versions of systems using the same kernel. Sahne Karnal series is a kernel series developed to be a completely independent operating system. It is generally famous for its Rust ecosystem! Rust language is preferred over C in Sahne Karnal series due to its ownership and borrowing, automatic memory management, zero-cost abstractions and other rich features! Sahne Karnal is not a Unix-like system in essence, but Unix-related code can be found! Sahne64 is an important component in this kernel, Sahne64 is actually the Rust API that provides communication between the user space and the kernel, the main source code is in the kernel! Sahne Karnal has basically preferred the Monotolic architecture! 

# Basic features
1. Target source file size: 461 MB
2. Components that will be in the kernel area: Device drivers, CPU related operations, Basic functions, Hardware API and Sahne64
3. Components that will be in user space: File system, Sound server, Window system, GUI API, Desktop Environment, Mobile Environment, Installation system, Package manager, Library implementation, Init System
4. Is it Unix?: No it is not Unix instead it comes with its own principles
5. Main programming language used: Rust
6. Target CPU instruction set: Cross CPU instruction set support
7. Targeted Optimization level: -Os
