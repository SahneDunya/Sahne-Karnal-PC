#[cfg(feature = "sdl1")]
extern crate sdl1;

#[cfg(feature = "sdl2")]
extern crate sdl2;

// --- SDL1 Driver Module ---
#[cfg(feature = "sdl1")]
pub mod sdl_driver {
    use sdl1::video;
    use sdl1::event;
    use sdl1::surface::Surface;
    use sdl1::event::Event;

    pub struct SDLDriver {
        screen: Surface,
    }

    impl SDLDriver {
        pub fn new() -> Result<SDLDriver, String> {
            if sdl1::init(sdl1::INIT_VIDEO) < 0 {
                return Err(sdl1::get_error());
            }

            let screen = match video::set_video_mode(640, 480, 32, video::SWSURFACE) {
                Some(screen) => screen,
                None => return Err(sdl1::get_error()),
            };

            Ok(SDLDriver { screen })
        }

        pub fn handle_events(&mut self) -> bool {
            while let Some(event) = event::poll_event() {
                match event {
                    Event::Quit { .. } => return false,
                    _ => {}
                }
            }
            true
        }

        pub fn clear_screen(&mut self, color: u32) {
            video::fill_rect(&mut self.screen, None, color);
            video::flip(&mut self.screen);
        }
    }
}

// --- SDL2 Driver Module ---
#[cfg(feature = "sdl2")]
pub mod sdl_driver {
    use sdl2::pixels::Color;
    use sdl2::event::Event;
    use sdl2::keyboard::Keycode;
    use sdl2::video::Window;
    use sdl2::render::Canvas;
    use sdl2::EventPump;

    pub struct SDLDriver {
        canvas: Canvas<Window>,
        event_pump: EventPump,
    }

    impl SDLDriver {
        pub fn new() -> Result<SDLDriver, String> {
            let sdl_context = sdl2::init(sdl2::INIT_VIDEO)?;
            let video_subsystem = sdl_context.video()?;

            let window = video_subsystem
                .window("SDL Example", 640, 480)
                .position_centered()
                .build()
                .map_err(|e| e.to_string())?;

            let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
            let event_pump = sdl_context.event_pump()?;

            Ok(SDLDriver { canvas, event_pump })
        }

        pub fn handle_events(&mut self) -> bool {
            for event in self.event_pump.poll_iter() {
                match event {
                    Event::Quit { .. }
                    | Event::KeyDown {
                        keycode: Some(Keycode::Escape),
                        ..
                    } => return false,
                    _ => {}
                }
            }
            true
        }

        pub fn clear_screen(&mut self, color: Color) {
            self.canvas.set_draw_color(color);
            self.canvas.clear();
            self.canvas.present();
        }
    }
}

// --- Common SDL Start Function ---
#[cfg(any(feature = "sdl1", feature = "sdl2"))]
pub fn start_sdl() -> Result<(), String> {
    // --- Initialize SDL Driver based on feature flag ---
    #[cfg(feature = "sdl1")]
    let mut sdl_driver = sdl_driver::SDLDriver::new()?; // SDL1 driver is used if 'sdl1' feature is enabled

    #[cfg(feature = "sdl2")]
    let mut sdl_driver = sdl_driver::SDLDriver::new()?; // SDL2 driver is used if 'sdl2' feature is enabled (or if neither 'sdl1' nor 'sdl2' is specified, and 'default-features = true' in Cargo.toml includes 'sdl2')


    let mut running = true;
    while running {
        running = sdl_driver.handle_events();

        // --- Clear screen with black color based on feature flag ---
        #[cfg(feature = "sdl1")]
        sdl_driver.clear_screen(0); // Black screen for SDL1 (u32 color)

        #[cfg(feature = "sdl2")]
        sdl_driver.clear_screen(Color::RGB(0, 0, 0)); // Black screen for SDL2 (Color struct)
    }

    Ok(())
}

// --- No SDL Feature Enabled Error Function ---
#[cfg(not(any(feature = "sdl1", feature = "sdl2")))]
pub fn start_sdl() -> Result<(), String> {
    Err("SDL sürücüsü etkinleştirilmedi. Lütfen 'sdl1' veya 'sdl2' özelliklerinden birini etkinleştirin.".to_string())
}

// --- Main function (for demonstration purposes) ---
fn main() -> Result<(), String> {
    start_sdl()?; // Start SDL with the selected/default feature (sdl2 in this example)
    Ok(())
}