#include "sahne.h"
#include <stdio.h> // For printf
#include <string.h> // For strlen

int main() {
    sahne_task_id_t task_id;
    sahne_error_t err;

    printf("Sahne64 C Program Starting...\n");

    // Get current task ID
    err = sahne_task_current_id(&task_id);
    if (err == SAHNE_SUCCESS) {
        printf("Current Task ID: %llu\n", (unsigned long long)task_id);
    } else {
        fprintf(stderr, "Failed to get Task ID, error: %d\n", err);
    }

    // Memory allocation
    void* allocated_mem = NULL;
    size_t mem_size = 1024;
    err = sahne_mem_allocate(mem_size, &allocated_mem);
    if (err == SAHNE_SUCCESS) {
        printf("Allocated %zu bytes at %p\n", mem_size, allocated_mem);
        // Use memory...
        if (allocated_mem != NULL) {
             ((uint8_t*)allocated_mem)[0] = 42; // Example use
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

    // Resource acquire/read (assuming "sahne://test/file" exists or can be created)
    sahne_handle_t file_handle = 0; // Initialize handle
    const char* res_name = "sahne://test/file";
    uint32_t mode = SAHNE_MODE_READ | SAHNE_MODE_WRITE | SAHNE_MODE_CREATE; // Read, Write, Create if not exists

    err = sahne_resource_acquire(res_name, strlen(res_name), mode, &file_handle);
    if (err == SAHNE_SUCCESS) {
        printf("Acquired resource '%s', Handle: %llu\n", res_name, (unsigned long long)file_handle);

        char buffer[256];
        size_t bytes_read = 0;
        err = sahne_resource_read(file_handle, (uint8_t*)buffer, sizeof(buffer), &bytes_read);
        if (err == SAHNE_SUCCESS) {
            printf("Read %zu bytes from resource.\n", bytes_read);
            // Process data in buffer...
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
        fprintf(stderr, "Failed to acquire resource '%s', error: %d\n", res_name, err);
    }


    printf("Sahne64 C Program Exiting.\n");
    sahne_task_exit(0); // Exit the task
    return 0; // Should not be reached
}