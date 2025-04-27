#include "sahne.h"
#include <iostream>
#include <vector>
#include <string>

int main() {
    sahne_task_id_t task_id;
    sahne_error_t err;

    std::cout << "Sahne64 C++ Program Starting..." << std::endl;

    // Get current task ID
    err = sahne_task_current_id(&task_id);
    if (err == SAHNE_SUCCESS) {
        std::cout << "Current Task ID: " << task_id << std::endl;
    } else {
        std::cerr << "Failed to get Task ID, error: " << err << std::endl;
    }

    // Memory allocation
    void* allocated_mem = nullptr;
    size_t mem_size = 1024;
    err = sahne_mem_allocate(mem_size, &allocated_mem);
    if (err == SAHNE_SUCCESS) {
        std::cout << "Allocated " << mem_size << " bytes at " << allocated_mem << std::endl;
        // Use memory...
        if (allocated_mem != nullptr) {
            static_cast<uint8_t*>(allocated_mem)[0] = 42; // Example use
        }

        // Release memory
        err = sahne_mem_release(allocated_mem, mem_size);
        if (err == SAHNE_SUCCESS) {
            std::cout << "Released allocated memory." << std::endl;
        } else {
            std::cerr << "Failed to release memory, error: " << err << std::endl;
        }
    } else {
        std::cerr << "Memory allocation failed, error: " << err << std::endl;
    }

    // Resource acquire/read
    sahne_handle_t file_handle = 0;
    std::string res_name = "sahne://test/file";
    uint32_t mode = SAHNE_MODE_READ | SAHNE_MODE_WRITE | SAHNE_MODE_CREATE;

    err = sahne_resource_acquire(res_name.c_str(), res_name.length(), mode, &file_handle);
    if (err == SAHNE_SUCCESS) {
        std::cout << "Acquired resource '" << res_name << "', Handle: " << file_handle << std::endl;

        std::vector<uint8_t> buffer(256);
        size_t bytes_read = 0;
        err = sahne_resource_read(file_handle, buffer.data(), buffer.size(), &bytes_read);
        if (err == SAHNE_SUCCESS) {
            std::cout << "Read " << bytes_read << " bytes from resource." << std::endl;
            // buffer now contains the data
        } else {
             std::cerr << "Failed to read from resource, error: " << err << std::endl;
        }

        // Release resource handle
        err = sahne_resource_release(file_handle);
        if (err == SAHNE_SUCCESS) {
             std::cout << "Released resource handle." << std::endl;
        } else {
             std::cerr << "Failed to release resource handle, error: " << err << std::endl;
        }

    } else {
        std::cerr << "Failed to acquire resource '" << res_name << "', error: " << err << std::endl;
    }


    std::cout << "Sahne64 C++ Program Exiting." << std::endl;
    sahne_task_exit(0); // Exit the task
    return 0; // Should not be reached
}