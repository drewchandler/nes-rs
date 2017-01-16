use minifb::{Window, WindowOptions};

pub trait VideoDriver {
    fn is_open(&self) -> bool;
    fn output_frame(&mut self, frame: &[u32]);
}

pub struct MiniFBWindow {
    window: Window,
}

impl MiniFBWindow {
    pub fn new() -> MiniFBWindow {
        MiniFBWindow { window: Window::new("NES", 256, 224, WindowOptions::default()).unwrap() }
    }
}

impl VideoDriver for MiniFBWindow {
    fn is_open(&self) -> bool {
        self.window.is_open()
    }

    fn output_frame(&mut self, frame: &[u32]) {
        self.window.update_with_buffer(frame);
    }
}
