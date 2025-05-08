# Sahne-Karnal
Sahne Karnal is a kernel family developed by Sahne Dünya. Sahne Dünya also developed this project through an independent operating system project. Many operating systems use Linux kernels, most operating systems that use their own kernels have a Unix infrastructure, but very few operating systems use both their own kernel and their own infrastructure. Sahne Karnal falls into this percentage. What are the features of Sahne Karnal? First of all, Sahne Karnal uses Rust programming language instead of C, which is the programming language used by many operating systems. Thanks to this, it can benefit from modern features. Sahne Karnal is closer to Micro Architecture, which means that basic operating system services are located in the kernel space. Sahne Karnal architecture uses Sahne64 API for kernel-user space communication, this API is the most important API of the operating system. Sahne Dünya does not only stay with this in the Sahne Karnal series, it also offers Standard User Space components for some components. These are executable file: .gaxe, file system: SADAK, Package manager: Sahne Dünya Packet Liders. However, not all of them, for example, this kernel does not offer Standard Desktop Environment, Window system, Sound system, Device drivers, init system, etc. Also Sahne Karnal is an open source kernel. It is published under the MIT License. Sahne Karnal offers a single version as standard, but there may be thousands of non-standard versions, just like Linux. You are currently in the Standard Edition. There are many striking features in the Standard Edition, it is compatible with both Desktop and Mobile.

# Basic features
1. Target source file size: In Standard Edition 150 MB
2. Components that will be in the kernel area: CPU related operations, Basic functions, Sahne64
3. Components that will be in user space: File system, Sound server, Window system, GUI API, Desktop Environment, Mobile Environment, Installation system, Package manager, Library implementation, Init System, Device drivers
4. Is it Unix?: No it is not Unix instead it comes with its own principles
5. Main programming language used: Rust
6. Target CPU instruction set: In Standard Edition Cross CPU instruction set support
7. Targeted Optimization level: -Os
8. Targeted Electronic devices: In Standard Edition Deskop and Mobile (Laptop, Tablet, Phone, Desktop case)
