use crate::platform::types::*; // Platforma özgü tipler
use core::ptr::NonNull;

// OpenGL fonksiyon işaretçileri
type GlClearColorFunc = unsafe extern "C" fn(red: GLfloat, green: GLfloat, blue: GLfloat, alpha: GLfloat);
type GlClearFunc = unsafe extern "C" fn(mask: GLbitfield);

// OpenGL fonksiyonlarını yükle
pub struct OpenGL {
    gl_clear_color: GlClearColorFunc,
    gl_clear: GlClearFunc,
}

impl OpenGL {
    pub unsafe fn new(get_proc_address: unsafe extern "C" fn(name: *const c_char) -> *const c_void) -> Option<Self> {
        let gl_clear_color = core::mem::transmute(get_proc_address(b"glClearColor\0".as_ptr() as *const c_char));
        let gl_clear = core::mem::transmute(get_proc_address(b"glClear\0".as_ptr() as *const c_char));

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

// Örnek kullanım (iyileştirilmiş - init_opengl fonksiyonu kaldırıldı)
pub unsafe fn main_example(get_proc_address: unsafe extern "C" fn(name: *const c_char) -> *const c_void) -> Option<OpenGL> {
    OpenGL::new(get_proc_address)
}