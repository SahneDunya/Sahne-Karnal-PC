import core.stdc.stdint;
import core.stdc.stddef;
import core.stdc.stdio;
import core.string; // For toStringz

// Import the C functions and types
extern(C) {
    // Include all defines and function declarations from sahne.h here,
    // or import the C header directly if your D compiler supports it.
    // For simplicity, let's redefine necessary parts:

    // --- Basic Types ---
    alias uint64_t sahne_handle_t;
    alias uint64_t sahne_task_id_t;
    alias int32_t sahne_error_t;

    // --- Success/Error Codes ---
    enum SAHNE_SUCCESS = 0;
    // ... other error codes from sahne.h ...
    enum SAHNE_ERROR_OUT_OF_MEMORY = 1;
    enum SAHNE_ERROR_INVALID_PARAMETER = 3;
    enum SAHNE_ERROR_INVALID_HANDLE = 13;

    // --- Resource Modes ---
    enum SAHNE_MODE_READ = (1 << 0);
    // ... other modes ...

    // --- Function Declarations ---
    sahne_error_t sahne_mem_allocate(size_t size, void** out_ptr);
    sahne_error_t sahne_mem_release(void* ptr, size_t size);

    sahne_error_t sahne_task_current_id(sahne_task_id_t* out_task_id);
    void sahne_task_exit(int32_t code);

    sahne_error_t sahne_resource_acquire(const(char)* id_ptr, size_t id_len, uint32_t mode, sahne_handle_t* out_handle);
    sahne_error_t sahne_resource_read(uint8_t* buffer_ptr, size_t buffer_len, size_t* out_bytes_read); // Note: Read handle missing in C sig above, will add
    sahne_error_t sahne_resource_release(sahne_handle_t handle);

    // ... declare all other functions from sahne.h ...
}

// Corrected sahne_resource_read signature based on Rust API:
extern(C) {
    sahne_error_t sahne_resource_read(sahne_handle_t handle, uint8_t* buffer_ptr, size_t buffer_len, size_t* out_bytes_read);
}


void main() {
    sahne_task_id_t task_id;
    sahne_error_t err;

    printf("Sahne64 D Program Starting...\n");

    // Get current task ID
    err = sahne_task_current_id(&task_id);
    if (err == SAHNE_SUCCESS) {
        printf("Current Task ID: %llu\n", task_id); // D uses %llu for uint64_t
    } else {
        fprintf(stderr, "Failed to get Task ID, error: %d\n", err);
    }

    // Memory allocation
    void* allocated_mem = null; // Use D's null
    size_t mem_size = 1024;
    err = sahne_mem_allocate(mem_size, &allocated_mem);
    if (err == SAHNE_SUCCESS) {
        printf("Allocated %zu bytes at %p\n", mem_size, allocated_mem);
        // Use memory...
        if (allocated_mem !is null) {
            (cast(ubyte*)allocated_mem)[0] = 42; // Example use (ubyte = uint8_t)
        }

        // Release memory
        err = sahne_mem_release(allocated_mem, mem_size);
        if (err == SAHNE_SUCCESS) {
            printf("Released allocated memory.\n");
        } else {
            fprintf(stderr, "Failed to release memory, error: %d\n", err);
        }
    } else {
        fprintf(stderr, "Memory allocation failed, error: %d\n", err);
    }

    // Resource acquire/read
    sahne_handle_t file_handle = 0;
    string res_name = "sahne://test/file";
    uint32_t mode = SAHNE_MODE_READ; // Only read for simplicity

    // Convert D string to C-style const char* and length
    const(char)* res_ptr = res_name.ptr;
    size_t res_len = res_name.length;

    err = sahne_resource_acquire(res_ptr, res_len, mode, &file_handle);
    if (err == SAHNE_SUCCESS) {
        printf("Acquired resource '%s', Handle: %llu\n", toStringz(res_name), file_handle);

        ubyte[256] buffer; // D static array
        size_t bytes_read = 0;
        err = sahne_resource_read(file_handle, buffer.ptr, buffer.length, &bytes_read);
        if (err == SAHNE_SUCCESS) {
            printf("Read %zu bytes from resource.\n", bytes_read);
            // buffer[0 .. bytes_read] contains the data
        } else {
             fprintf(stderr, "Failed to read from resource, error: %d\n", err);
        }

        // Release resource handle
        err = sahne_resource_release(file_handle);
        if (err == SAHNE_SUCCESS) {
             printf("Released resource handle.\n");
        } else {
             fprintf(stderr, "Failed to release resource handle, error: %d\n", err);
        }

    } else {
        fprintf(stderr, "Failed to acquire resource '%s', error: %d\n", toStringz(res_name), err);
    }


    printf("Sahne64 D Program Exiting.\n");
    sahne_task_exit(0); // Exit the task
    // No return needed as exit is noreturn
}