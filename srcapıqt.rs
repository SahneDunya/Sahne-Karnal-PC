#[cfg(any(
    all(feature = "qt4"),
    all(feature = "qt5"),
    all(feature = "qt6")
))]
mod qt {
    #[cfg(feature = "qt4")]
    use qt4 as qt;

    #[cfg(feature = "qt5")]
    use qt5 as qt;

    #[cfg(feature = "qt6")]
    use qt6 as qt;

    // Temel bir Qt sınıfı örneği (örneğin, QLabel)
    pub struct QtLabel {
        #[cfg(feature = "qt4")]
        inner: *mut qt::QWidget,

        #[cfg(feature = "qt5")]
        inner: *mut qt::QWidget,

        #[cfg(feature = "qt6")]
        inner: *mut qt::QWidget,
    }

    impl QtLabel {
        pub fn new(text: &str) -> Self {
            let label = unsafe {
                #[cfg(feature = "qt4")]
                {
                    qt::new_qlabel(text.as_ptr() as *mut i8, std::ptr::null_mut())
                }
                #[cfg(feature = "qt5")]
                {
                    qt::new_qlabel(text.as_ptr() as *mut i8, std::ptr::null_mut())
                }
                #[cfg(feature = "qt6")]
                {
                    qt::new_qlabel(text.as_ptr() as *mut i8, std::ptr::null_mut())
                }
            };

            Self { inner: label }
        }

        pub fn set_text(&mut self, text: &str) {
            unsafe {
                #[cfg(feature = "qt4")]
                {
                    qt::qlabel_set_text(self.inner, text.as_ptr() as *mut i8);
                }
                #[cfg(feature = "qt5")]
                {
                    qt::qlabel_set_text(self.inner, text.as_ptr() as *mut i8);
                }
                #[cfg(feature = "qt6")]
                {
                    qt::qlabel_set_text(self.inner, text.as_ptr() as *mut i8);
                }
            }
        }

        // ... diğer Qt metotları ...
    }

    // ... diğer Qt sınıfları ve fonksiyonları ...
}