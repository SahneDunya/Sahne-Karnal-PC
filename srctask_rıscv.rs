#![no_std]
use core::ptr::NonNull;
use core::panic::PanicInfo;

// A simple task structure
pub struct Task {
    stack: [u8; 1024], // 1KB stack for each task
    context: TaskContext,
}

#[derive(Default, Copy, Clone)]
#[repr(C)]
struct TaskContext{
    x:[usize; 32], // General purpose registers
    sstatus: usize, // Supervisor status register
    sepc: usize,  // Supervisor exception program counter
}

static mut CURRENT_TASK: Option<NonNull<Task>> = None;
static mut TASKS: [Option<Task>; 2] = [None, None]; // Static array to hold tasks, fixed size of 2 for this example

pub fn init(){
    unsafe{
        // Initialize the first task (task 0)
        TASKS[0] = Some(Task{stack: [0; 1024], context: TaskContext::default()});
        // Set the current task to the first task
        CURRENT_TASK = NonNull::new(TASKS[0].as_mut().expect("Failed to get mutable reference to TASKS[0]"));
    }
}

pub fn create_task(entry_point: fn()){
    static mut TASK_ID: usize = 1; // Static task ID, starts from 1 as task 0 is already initialized
    unsafe{
        // Get a mutable reference to the task slot based on TASK_ID
        let task_slot = TASKS.get_mut(TASK_ID).expect("TASK_ID out of bounds for TASKS array");
        // Create a new task in the task slot
        *task_slot = Some(Task{
            stack: [0; 1024],
            context: TaskContext{
                sepc: entry_point as usize, // Set the entry point of the task
                ..Default::default() // Inherit other context fields from default
            }
        });
        TASK_ID += 1; // Increment task ID for the next task creation
    }
}

pub fn switch_task(){
    unsafe{
        // Get mutable references to current and next tasks (task 1 is assumed to be the next task for simplicity in this example)
        let current_task = CURRENT_TASK.expect("CURRENT_TASK is None").as_mut();
        let next_task = TASKS.get_mut(1).expect("TASKS[1] is None").as_mut().expect("Failed to get mutable reference to TASKS[1]");

        // Save current task's context
        // Save sstatus register
        asm!("csrr {}, sstatus", out(reg) current_task.context.sstatus);
        // Save sepc register
        asm!("csrr {}, sepc", out(reg) current_task.context.sepc);
        // Save stack pointer (x[2] is typically used as stack pointer in RISC-V convention, though 'sp' alias is preferred for readability usually)
        asm!("mv {}, sp", out(reg) current_task.context.x[2]); // Or asm!("mv {}, sp", out(reg) current_task.context.x[2] : : "sp"); for explicit 'sp' input

        // Switch to the next task's context
        // Load next task's sstatus register
        asm!("csrw sstatus, {}", in(reg) next_task.context.sstatus);
        // Load next task's sepc register
        asm!("csrw sepc, {}", in(reg) next_task.context.sepc);
        // Load next task's stack pointer
        asm!("mv sp, {}", in(reg) next_task.context.x[2]);     // Or asm!("mv sp, {},", in(reg) next_task.context.x[2] : : "sp"); for explicit 'sp' output

        // Update CURRENT_TASK to the next task
        CURRENT_TASK = NonNull::new(next_task as *mut Task);
    }
}

// Example task entry point (just for demonstration)
fn task1_entry() {
    let mut count = 0;
    loop {
        count += 1;
        // In a real no_std environment, you might use a hardware timer or similar for delays.
        // For this example, a simple loop serves as a placeholder.
        for _ in 0..100000 {
            // waste time
        }
        unsafe {
            // Basic output to indicate task switching (assuming some form of output like serial is initialized elsewhere)
            let ptr = 0x80000000 as *mut u32; // Example memory address for output
            ptr.write_volatile(count);
        }
    }
}

// Example usage (conceptual main function for demonstration)
fn main() {
    init(); // Initialize task management

    create_task(task1_entry); // Create task 1 and set its entry point

    // Start running the first task (task 0 is initialized in init).
    // In a real OS, task 0 might do initial setup and then start task switching.

    loop {
        unsafe {
            // For demonstration, switch tasks in a loop.
            // In a real system, task switching would be triggered by events (e.g., timer interrupt).
            switch_task();
            for _ in 0..200000 { // waste time in task 0
                // waste time
            }
            let ptr = 0x80000004 as *mut u32; // Example memory address for output for task 0
            ptr.write_volatile(0); // Indicate task 0 is running
        }
    }
}

// Required for no_std environment, define panic handler
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}