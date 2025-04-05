use crate::platform::types::*; // Platforma özgü tipler
use std::ffi::CString;

// OpenGL ES fonksiyon işaretçileri
type GlClearColorFunc = unsafe extern "C" fn(red: GLfloat, green: GLfloat, blue: GLfloat, alpha: GLfloat);
type GlClearFunc = unsafe extern "C" fn(mask: GLbitfield);

// OpenGL ES yapılandırması
pub struct OpenGLES {
    gl_clear_color: GlClearColorFunc,
    gl_clear: GlClearFunc,
}

impl OpenGLES {
    pub unsafe fn new(get_proc_address: unsafe extern "C" fn(name: *const c_char) -> *const c_void) -> Option<Self> {
        let clear_color_name = CString::new("glClearColor").unwrap();
        let clear_name = CString::new("glClear").unwrap();

        let gl_clear_color = core::mem::transmute(get_proc_address(clear_color_name.as_ptr()));
        let gl_clear = core::mem::transmute(get_proc_address(clear_name.as_ptr()));

        if gl_clear_color.is_null() || gl_clear.is_null() {
            return None;
        }

        Some(Self {
            gl_clear_color,
            gl_clear,
        })
    }

    pub unsafe fn clear_color(&self, red: GLfloat, green: GLfloat, blue: GLfloat, alpha: GLfloat) {
        (self.gl_clear_color)(red, green, blue, alpha);
    }

    pub unsafe fn clear(&self, mask: GLbitfield) {
        (self.gl_clear)(mask);
    }
}

// Örnek Kullanım
pub unsafe fn main_example(get_proc_address: unsafe extern "C" fn(name: *const c_char) -> *const c_void) -> Option<OpenGLES> {
    OpenGLES::new(get_proc_address)
}