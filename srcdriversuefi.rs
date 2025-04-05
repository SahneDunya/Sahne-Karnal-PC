use core::fmt::Write;
use uefi::{prelude::*, proto::console::gop::GraphicsOutput, graphics::Color};

pub struct UefiDriver {
    st: SystemTable,
}

impl UefiDriver {
    pub fn new(st: SystemTable) -> Self {
        UefiDriver { st }
    }

    pub fn initialize(&mut self) -> Result<(), uefi::Error> {
        // UEFI sistem tablosundan temel servisleri alıyoruz
        let _boot_services = self.st.BootServices(); // Artık alt çizgi ile 시작하지 않음, 명시적으로 사용하지 않음을 나타냄
        let _runtime_services = self.st.RuntimeServices(); // Artículos y 밑줄 제거, 명시적으로 사용하지 않음을 나타냄

        // 화면을 검정색으로 지웁니다. 오류 처리를 명시적으로 합니다.
        self.clear_screen_black()?;

        // 콘솔에 시작 메시지를 출력합니다. 오류 처리를 명시적으로 합니다.
        self.output_start_message()?;

        Ok(())
    }

    fn clear_screen_black(&mut self) -> Result<(), uefi::Error> {
        let gop = self.get_graphics_output_protocol()?;

        // 가장 일반적인 비디오 모드를 설정합니다 (일반적으로 첫 번째 모드).
        let mode_info = gop.QueryMode(None)?; // 오류 처리 추가
        let best_mode = mode_info.1[0]; // 첫 번째 모드를 기본값으로 사용
        gop.SetMode(best_mode)?; // 오류 처리 추가

        // 화면을 검정색으로 채웁니다. 오류 처리 추가
        gop.Clear(Color::Black)?;
        Ok(())
    }

    fn output_start_message(&mut self) -> Result<(), uefi::Error> {
        let mut con_out = self.st.ConOut();
        // 출력 작업에서 발생할 수 있는 오류를 처리합니다.
        con_out.OutputString("UEFI sürücüsü başlatılıyor...\r\n")?;
        Ok(())
    }


    fn get_graphics_output_protocol(&self) -> Result<&GraphicsOutput, uefi::Error> {
        let handle = self.st.BootServices().LocateProtocol(
            &GraphicsOutput::GUID,
        )?;
        // 안전하지 않은 포인터 역참조를 캡슐화합니다.
        let gop = unsafe { &mut *handle.cast() };
        Ok(gop)
    }
}