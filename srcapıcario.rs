use core::ptr::NonNull;

// Surface creation function
pub fn create_surface(width: i32, height: i32) -> Result<NonNull<cairo::cairo_surface_t>, cairo::Status> {
    unsafe {
        let surface = cairo::cairo_image_surface_create(cairo::Format::Rgb24, width, height);
        if surface.is_null() {
            return Err(cairo::Status::NoMemory);
        }
        Ok(NonNull::new_unchecked(surface))
    }
}

// Context creation function
pub fn create_context(surface: NonNull<cairo::cairo_surface_t>) -> Result<NonNull<cairo::Context>, cairo::Status> {
    unsafe {
        let cr = cairo::cairo_create(surface.as_ptr());
        if cr.is_null() {
            return Err(cairo::Status::NoMemory);
        }
        Ok(NonNull::new_unchecked(cr))
    }
}

// Example drawing function
pub fn draw_example(cr: NonNull<cairo::Context>) -> Result<(), cairo::Status> {
    unsafe {
        cairo::cairo_set_source_rgb(cr.as_ptr(), 1.0, 0.0, 0.0); // Red color
        cairo::cairo_rectangle(cr.as_ptr(), 10.0, 10.0, 100.0, 50.0); // Rectangle
        cairo::cairo_fill(cr.as_ptr())?; // Fill rectangle, propagate error with '?'

        Ok(())
    }
}

// Save surface function (e.g., to a framebuffer)
pub fn save_surface(surface: NonNull<cairo::cairo_surface_t>, data: *mut u8, stride: i32) -> Result<(), cairo::Status> {
    unsafe {
        let surface_data = cairo::cairo_image_surface_get_data(surface.as_ptr());
        if surface_data.is_null() {
            return Err(cairo::Status::NoMemory);
        }

        let surface_height = cairo::cairo_image_surface_get_height(surface.as_ptr());

        // Calculate the size of the surface data in bytes
        let surface_stride = cairo::cairo_image_surface_get_stride(surface.as_ptr());
        let data_size = surface_stride * surface_height;

        // Check if the provided stride is at least as large as the surface stride
        if stride < surface_stride {
            return Err(cairo::Status::InvalidStride); // Or a more appropriate error
        }
        // It's crucial to ensure 'data' points to a buffer large enough to hold 'stride * surface_height' bytes.
        // However, we cannot safely check the size of the buffer pointed to by raw pointer 'data' in Rust.
        // The caller must guarantee that 'data' is valid and large enough.

        // Copy data, being careful with sizes. We use surface_stride for source and provided stride for destination row width.
        for row in 0..surface_height {
            let src_ptr = surface_data.add(row * surface_stride);
            let dest_ptr = data.add(row * stride);
            core::ptr::copy_nonoverlapping(src_ptr, dest_ptr, surface_stride as usize);
        }


        Ok(())
    }
}

fn main() -> Result<(), cairo::Status> {
    let width = 200;
    let height = 100;

    // 1. Create surface
    let surface = create_surface(width, height)?;

    // 2. Create context
    let cr = create_context(surface)?;

    // 3. Draw example
    draw_example(cr)?;

    // 4. Save surface to a buffer
    let surface_ptr = surface.as_ptr();
    unsafe {
        let stride = cairo::cairo_image_surface_get_stride(surface_ptr);
        let height = cairo::cairo_image_surface_get_height(surface_ptr);
        let buffer_size = stride * height;
        let mut buffer: Vec<u8> = vec![0; buffer_size as usize]; // Create a buffer to hold image data


        save_surface(surface, buffer.as_mut_ptr(), stride)?;

        // Example: print first few bytes of the buffer (for demonstration)
        println!("First few bytes of the buffer: {:?}", &buffer[0..24]);

        // In a real framebuffer scenario, 'buffer.as_mut_ptr()' would be replaced with the actual framebuffer pointer.
        // And 'stride' would be the framebuffer's line stride.
    }

    println!("Drawing and saving successful!");
    Ok(())
}