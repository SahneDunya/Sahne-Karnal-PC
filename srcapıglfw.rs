extern crate glfw;

use glfw::{Action, Context, Key, Window};

fn main() {
    // GLFW'yi başlat ve hataları işle
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).expect("GLFW başlatılamadı!");

    // Pencere ipuçlarını ayarla (OpenGL 3.3 çekirdek profilini kullan)
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3)); // OpenGL 3.3'ü iste
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core)); // Çekirdek profilini kullan
    #[cfg(target_os = "macos")]
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true)); // macOS için ileriye dönük uyumluluk

    // Yeniden boyutlandırılabilir pencere oluştur
    let (mut window, events) = glfw.create_window(640, 480, "Geliştirilmiş GLFW Örneği", glfw::Windowed)
        .expect("Pencere oluşturulamadı!");

    // Pencereyi mevcut OpenGL bağlamına yap
    window.make_current();

    // GLFW olaylarını dinlemek için olay alıcısını ayarla
    window.set_key_polling(true);
    window.set_framebuffer_size_polling(true); // Çerçeve arabelleği boyutu değişikliklerini dinle

    // Çerçeve arabelleği boyutunu başlangıçta al ve OpenGL viewport'u ayarla
    let (width, height) = window.get_framebuffer_size();
    unsafe {
        gl::Viewport(0, 0, width, height);
    }

    // OpenGL yükleme fonksiyonlarını yükle (gl crate ile)
    gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

    // Olay döngüsü: Pencere kapanana kadar çalışır
    while !window.should_close() {
        // Olayları işle (tuş basmaları, pencere yeniden boyutlandırmaları vb.)
        glfw.poll_events();

        // Olay alıcısından gelen olayları işle
        for (_, event) in glfw::flush_messages(&events) {
            match event {
                glfw::WindowEvent::FramebufferSize(width, height) => {
                    // Çerçeve arabelleği boyutu değiştiğinde viewport'u ayarla
                    unsafe {
                        gl::Viewport(0, 0, width, height);
                    }
                }
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    // ESC tuşuna basıldığında pencereyi kapat
                    window.set_should_close(true);
                }
                _ => {} // Diğer olayları şimdilik işlemliyoruz
            }
        }

        // Çizim komutları (arka planı temizle)
        unsafe {
            gl::ClearColor(0.3, 0.3, 0.5, 1.0); // Arka plan rengini ayarla (açık mavi-gri)
            gl::Clear(gl::COLOR_BUFFER_BIT); // Renk arabelleğini temizle
        }

        // Ön ve arka tamponları takas et (çizilenleri ekrana yansıt)
        window.swap_buffers();
    }
}