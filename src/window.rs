use minifb::{Window, WindowOptions, Key};

pub struct Chip8Window {
    pixels: Vec<u32>,
    width: usize,
    height: usize,
    w: Window
}
  
impl Chip8Window {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            pixels: vec![0u32; width*height],
            width,
            height,
            w: Window::new("chip8-emu", width, height, WindowOptions::default()).expect("window created")
         }
    }

    fn get_idx(&self, x: usize, y: usize) -> usize {
        x + y*self.width
    }

    pub fn draw_rectangle(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for l in y..y+h {
        let start_idx = self.get_idx(x, l);
        for i in start_idx..start_idx+w {
            self.pixels[i] = color;
        }
        }
    }

    pub fn clear(&mut self) {
        self.pixels.fill(0);
    }

    pub fn update_window(&mut self) {
        self.w.update_with_buffer(&self.pixels, self.width, self.height).expect("buffer was updated");
    }

    pub fn is_active(&self) -> bool {
        self.w.is_open()
    }

    pub fn check_keypress(&self) -> Option<u8> {
        let mut okey = 0u8;
        self.w.get_keys().iter().for_each(|key|
          okey = match key { // only the last one counts
              Key::X => 0x1,
              Key::C => 0x2,
              Key::V => 0x3,
              Key::A => 0x4,
              Key::S => 0x5, 
              Key::D => 0x6,
              Key::F => 0x7,
              Key::Q => 0x8,
              Key::W => 0x9, 
              Key::E => 0xA,
              Key::R => 0xB,
              Key::Key1 => 0xC,
              Key::Key2 => 0xD, 
              Key::Key3 => 0xE,
              Key::Key4 => 0xF,
              _ => 0,
          }
        );
        if okey==0u8 {None} else {Some(okey)} 
    }

}
