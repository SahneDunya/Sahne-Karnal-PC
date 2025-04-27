#include <stdint.h>
#include <stddef.h> // For size_t

#ifdef __cplusplus
extern "C" {
#endif

// --- Basic Types ---
typedef uint64_t sahne_handle_t;
typedef uint64_t sahne_task_id_t;
typedef int32_t sahne_error_t; // Use int32_t for error codes

// --- Success/Error Codes ---
#define SAHNE_SUCCESS 0

// Map Rust SahneError to C integer constants (arbitrary positive values)
#define SAHNE_ERROR_OUT_OF_MEMORY 1
#define SAHNE_ERROR_INVALID_ADDRESS 2
#define SAHNE_ERROR_INVALID_PARAMETER 3
#define SAHNE_ERROR_RESOURCE_NOT_FOUND 4
#define SAHNE_ERROR_PERMISSION_DENIED 5
#define SAHNE_ERROR_RESOURCE_BUSY 6
#define SAHNE_ERROR_INTERRUPTED 7
#define SAHNE_ERROR_NO_MESSAGE 8
#define SAHNE_ERROR_INVALID_OPERATION 9
#define SAHNE_ERROR_NOT_SUPPORTED 10
#define SAHNE_ERROR_UNKNOWN_SYSCALL 11
#define SAHNE_ERROR_TASK_CREATION_FAILED 12
#define SAHNE_ERROR_INVALID_HANDLE 13
#define SAHNE_ERROR_HANDLE_LIMIT_EXCEEDED 14
#define SAHNE_ERROR_NAMING_ERROR 15
#define SAHNE_ERROR_COMMUNICATION_ERROR 16
// ... add mappings for any new errors ...

// --- System Call Numbers (from Rust arch module) ---
#define SAHNE_SYSCALL_MEMORY_ALLOCATE 1
#define SAHNE_SYSCALL_MEMORY_RELEASE 2
#define SAHNE_SYSCALL_TASK_SPAWN 3
#define SAHNE_SYSCALL_TASK_EXIT 4
#define SAHNE_SYSCALL_RESOURCE_ACQUIRE 5
#define SAHNE_SYSCALL_RESOURCE_READ 6
#define SAHNE_SYSCALL_RESOURCE_WRITE 7
#define SAHNE_SYSCALL_RESOURCE_RELEASE 8
#define SAHNE_SYSCALL_GET_TASK_ID 9
#define SAHNE_SYSCALL_TASK_SLEEP 10
#define SAHNE_SYSCALL_LOCK_CREATE 11
#define SAHNE_SYSCALL_LOCK_ACQUIRE 12
#define SAHNE_SYSCALL_LOCK_RELEASE 13
#define SAHNE_SYSCALL_THREAD_CREATE 14
#define SAHNE_SYSCALL_THREAD_EXIT 15
#define SAHNE_SYSCALL_GET_SYSTEM_TIME 16
#define SAHNE_SYSCALL_SHARED_MEM_CREATE 17
#define SAHNE_SYSCALL_SHARED_MEM_MAP 18
#define SAHNE_SYSCALL_SHARED_MEM_UNMAP 19
#define SAHNE_SYSCALL_MESSAGE_SEND 20
#define SAHNE_SYSCALL_MESSAGE_RECEIVE 21
#define SAHNE_SYSCALL_GET_KERNEL_INFO 100
#define SAHNE_SYSCALL_TASK_YIELD 101
#define SAHNE_SYSCALL_RESOURCE_CONTROL 102

// --- Resource Modes (from Rust resource module) ---
#define SAHNE_MODE_READ (1 << 0)
#define SAHNE_MODE_WRITE (1 << 1)
#define SAHNE_MODE_CREATE (1 << 2)
#define SAHNE_MODE_EXCLUSIVE (1 << 3)
#define SAHNE_MODE_TRUNCATE (1 << 4)

// --- Kernel Info Types (from Rust kernel module) ---
#define SAHNE_KERNEL_INFO_VERSION_MAJOR 1
#define SAHNE_KERNEL_INFO_VERSION_MINOR 2
#define SAHNE_KERNEL_INFO_BUILD_ID 3
#define SAHNE_KERNEL_INFO_UPTIME_SECONDS 4
#define SAHNE_KERNEL_INFO_ARCHITECTURE 5

// --- Low-level Syscall Interface (Optional, usually wrapped) ---
// Raw syscall interface - generally discouraged for direct use by applications
 int64_t sahne_raw_syscall(uint64_t number, uint64_t arg1, uint64_t arg2, uint64_t arg3, uint64_t arg4, uint64_t arg5);

 // --- Memory Management ---
/**
 * Allocates a region of memory.
 * @param size The size of the memory region to allocate.
 * @param out_ptr Output parameter to store the allocated address on success.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_mem_allocate(size_t size, void** out_ptr);

/**
 * Releases a previously allocated memory region.
 * @param ptr The address of the memory region.
 * @param size The size of the memory region.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_mem_release(void* ptr, size_t size);

/**
 * Creates a shared memory area.
 * @param size The size of the shared memory area.
 * @param out_handle Output parameter to store the handle on success.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_mem_create_shared(size_t size, sahne_handle_t* out_handle);

/**
 * Maps a shared memory area into the current task's address space.
 * @param handle The handle of the shared memory area.
 * @param offset The offset within the shared memory area.
 * @param size The size of the mapping.
 * @param out_ptr Output parameter to store the mapped address on success.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_mem_map_shared(sahne_handle_t handle, size_t offset, size_t size, void** out_ptr);

/**
 * Unmaps a shared memory area from the current task's address space.
 * @param addr The mapped address.
 * @param size The size of the mapped area.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_mem_unmap_shared(void* addr, size_t size);


// --- Task Management ---
/**
 * Spawns a new task.
 * @param code_handle Handle of the resource containing the executable code.
 * @param args_ptr Pointer to task argument data.
 * @param args_len Length of task argument data.
 * @param out_task_id Output parameter to store the Task ID of the new task on success.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_task_spawn(sahne_handle_t code_handle, const uint8_t* args_ptr, size_t args_len, sahne_task_id_t* out_task_id);

/**
 * Terminates the current task with the given exit code. Does not return.
 * @param code The exit code.
 */
void sahne_task_exit(int32_t code) __attribute__((noreturn)); // Use __attribute__((noreturn)) or similar for portability

/**
 * Gets the ID of the current task.
 * @param out_task_id Output parameter to store the current Task ID on success.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_task_current_id(sahne_task_id_t* out_task_id);

/**
 * Puts the current task to sleep for a specified duration.
 * @param milliseconds The duration to sleep in milliseconds.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_task_sleep(uint64_t milliseconds);

/**
 * Creates a new thread within the current task.
 * @param entry_point The address of the entry function (uint64_t assuming function pointer cast).
 * @param stack_size The size of the stack for the new thread.
 * @param arg An argument passed to the entry function.
 * @param out_thread_id Output parameter to store the new thread's ID on success.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_thread_create(uint64_t entry_point, size_t stack_size, uint64_t arg, uint64_t* out_thread_id); // Assuming thread ID is u64

/**
 * Terminates the current thread with the given exit code. Does not return.
 * @param code The exit code.
 */
void sahne_thread_exit(int32_t code) __attribute__((noreturn));

/**
 * Voluntarily yields the CPU to another runnable task.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_task_yield();

// --- Resource Management ---
/**
 * Acquires a handle to a resource specified by ID.
 * @param id_ptr Pointer to the resource ID (byte string).
 * @param id_len Length of the resource ID.
 * @param mode Access modes (SAHNE_MODE_* flags).
 * @param out_handle Output parameter to store the acquired handle on success.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_resource_acquire(const char* id_ptr, size_t id_len, uint32_t mode, sahne_handle_t* out_handle);

/**
 * Reads data from a resource using its handle.
 * @param handle The handle of the resource.
 * @param buffer_ptr Pointer to the buffer to read into.
 * @param buffer_len The maximum number of bytes to read (size of the buffer).
 * @param out_bytes_read Output parameter to store the number of bytes actually read on success.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_resource_read(sahne_handle_t handle, uint8_t* buffer_ptr, size_t buffer_len, size_t* out_bytes_read);

/**
 * Writes data to a resource using its handle.
 * @param handle The handle of the resource.
 * @param buffer_ptr Pointer to the data to write.
 * @param buffer_len The number of bytes to write.
 * @param out_bytes_written Output parameter to store the number of bytes actually written on success.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_resource_write(sahne_handle_t handle, const uint8_t* buffer_ptr, size_t buffer_len, size_t* out_bytes_written);

/**
 * Releases a resource handle.
 * @param handle The handle to release.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_resource_release(sahne_handle_t handle);

/**
 * Sends a resource-specific control command.
 * @param handle The handle of the resource.
 * @param request The command code.
 * @param arg An argument for the command.
 * @param out_result Output parameter to store the command's result on success.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_resource_control(sahne_handle_t handle, uint64_t request, uint64_t arg, int64_t* out_result);


// --- Kernel Interaction ---
/**
 * Gets specific kernel information.
 * @param info_type The type of information requested (SAHNE_KERNEL_INFO_*).
 * @param out_value Output parameter to store the requested value on success.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_kernel_get_info(uint32_t info_type, uint64_t* out_value);

/**
 * Gets the current system time.
 * @param out_time Output parameter to store the system time (e.g., in nanos) on success.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_kernel_get_time(uint64_t* out_time);


// --- Synchronization ---
/**
 * Creates a new lock resource.
 * @param out_handle Output parameter to store the lock handle on success.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_sync_lock_create(sahne_handle_t* out_handle);

/**
 * Acquires a lock. Blocks if the lock is held by another thread/task.
 * @param handle The handle of the lock.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_sync_lock_acquire(sahne_handle_t handle);

/**
 * Releases a lock. The caller must hold the lock.
 * @param handle The handle of the lock.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_sync_lock_release(sahne_handle_t handle);


// --- Messaging / IPC ---
/**
 * Sends a message to a target task.
 * @param target_task The ID of the target task.
 * @param message_ptr Pointer to the message data.
 * @param message_len Length of the message data.
 * @return SAHNE_SUCCESS on success, an error code otherwise.
 */
sahne_error_t sahne_msg_send(sahne_task_id_t target_task, const uint8_t* message_ptr, size_t message_len);

/**
 * Receives a message for the current task. Blocks if no message is available.
 * @param buffer_ptr Pointer to the buffer to receive the message into.
 * @param buffer_len The maximum size of the message (size of the buffer).
 * @param out_bytes_received Output parameter to store the number of bytes received on success.
 * @return SAHNE_SUCCESS on success (possibly with 0 bytes received), SAHNE_ERROR_NO_MESSAGE if non-blocking and no message, or another error code.
 */
sahne_error_t sahne_msg_receive(uint8_t* buffer_ptr, size_t buffer_len, size_t* out_bytes_received);


#ifdef __cplusplus
} // extern "C"
#endif