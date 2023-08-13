use minifb::{Window, WindowOptions, Key};

use crate::state::PixelEvent;

pub struct Chip8Window {
    pixels: Vec<u32>,
    width: usize,
    height: usize,
    w: Window
}
  
impl Chip8Window {
    const BG_COLOR : u32 =  29 << 8 |  31 << 4 |  38; //R,G,B
    const FG_COLOR : u32 = 240 << 8 | 255 << 4 | 255;
    const PIX_HEIGHT: usize = 14;
    const PIX_WIDTH: usize = 16;

    pub fn new(width_px: usize, height_px: usize) -> Self {
        let width = width_px * Self::PIX_WIDTH;
        let height = height_px * Self::PIX_HEIGHT; 
        Self {
            pixels: vec![Self::BG_COLOR; width*height],
            width,
            height,
            w: Window::new("chip8-emu", width, height, WindowOptions::default()).expect("window created")
         }
    }

    fn get_idx(&self, x: usize, y: usize) -> usize {
        x + y*self.width
    }

    fn draw_rectangle(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        for l in y..y+h {
        let start_idx = self.get_idx(x, l);
        for i in start_idx..start_idx+w {
            self.pixels[i] = color;
        }
        }
    }

    pub fn draw_pixel(&mut self, p: &PixelEvent) {
        if p.clear_all {
            self.clear()
        } else {
            self.draw_rectangle((p.x as usize)*Self::PIX_WIDTH,  (p.y as usize)*Self::PIX_HEIGHT, Self::PIX_WIDTH, Self::PIX_HEIGHT, if p.on { Self::FG_COLOR } else { Self::BG_COLOR } )
        }
    }

    fn clear(&mut self) {
        self.pixels.fill(Self::BG_COLOR);
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
