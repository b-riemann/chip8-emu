use std::env;
use std::io::Write;
use std::fs::OpenOptions;
use std::time::{Duration,Instant};

mod instruction;
use instruction::from_opcode;

mod cartridge;
use cartridge::Cartridge;

mod state;
use minifb::{Window, WindowOptions,Key};
use state::{Chip8State, TermDisplay};



fn check_keypress(w: &Window) -> Option<u8> {
  let mut okey = 0u8;
  w.get_keys().iter().for_each(|key|
    okey = match key { // only the last one counts
        Key::Q => 1,
        Key::W => 2,
        Key::E => 3,
        Key::R => 4,
        Key::A => 5, 
        Key::S => 6,
        Key::D => 7,
        Key::F => 8,
        Key::Y => 9, 
        Key::X => 0xA,
        Key::C => 0xB,
        Key::V => 0xC,
        Key::T => 0xD, 
        Key::Z => 0xE,
        Key::U => 0xF,
        _ => 0,
    }
  );
  if okey==0u8 {None} else {Some(okey)} 
}

struct Frame {
  pixels: Vec<u32>,
  width: usize,
  height: usize
}

impl Frame {
  pub fn new(width: usize, height: usize) -> Self {
    Self { pixels: vec![0u32; width*height], width, height }
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
}


fn main() {
  let mode = "run";

  let filename = env::args().nth(1).expect("insert cartridge (.rom file)"); 

  let cartridge = Cartridge::new(filename);

  let mut outfile = OpenOptions::new().create(true).write(true).truncate(true).open(format!("{}.txt", mode)).unwrap();

  writeln!(outfile, "-----| {} |------", mode.to_ascii_uppercase()).unwrap();
  match mode {
    "listing" => {
      for addr in (cartridge.start()..cartridge.len()).step_by(2) {
        let opcode = cartridge.get_opcode_from(addr).unwrap();
        writeln!(outfile, "{:#06x}  {:#06x}  {}", addr, opcode, from_opcode(opcode)).unwrap();
      }
    },
    "idle-run" => {
      let mut cas = Chip8State::new(cartridge);
      for cycle in 0..5000 {
        let opcode = cas.cartridge.get_opcode_from(cas.pc).unwrap();
        let instr = from_opcode( opcode );

        if opcode & 0xf000 == 0xd000 {
          write!(outfile, "{}", cas.display).unwrap();
        } else {
          writeln!(outfile, "{0:4}  {1:#06x}  {2:#06x}  {3:♥<4$}{5}", cycle, cas.pc, opcode, "", cas.stack.len(), instr).unwrap();
        }

        cas.run_instruction(instr);
        if cycle % 4 == 0 {
          cas.register.tick()
        }
      }
    },
    "keyboard-test" => {
      loop {
        let now = Instant::now();
        while now.elapsed() < Duration::from_millis(500) {

        }
        
      }
    },
    _ => {
      //let min_cpu_cycle = Duration::from_nanos(588); // ~1.76 MHz, cosmac vp

      let frame_duration = Duration::from_millis(17); //~ 60Hz
      let cpu_cycles_per_monitor = 28_911;

      let pix_height: usize = 14;
      let pix_width: usize = 16;
      let color = 93000u32;

      let mut cas = Chip8State::new(cartridge);

      let mut frame = Frame::new((TermDisplay::WIDTH_PX as usize)*pix_width, (TermDisplay::HEIGHT_PX as usize)*pix_height);
      let mut window = Window::new("chip8-emu", frame.width, frame.height, WindowOptions::default()).expect("window created");
      
      let mut cycle=0;
      let mut monitor_now = Instant::now();
      let mut monitor_remaining = frame_duration;

      while window.is_open() {
        loop {
          match cas.display.flips.pop_front() {
            Some(p) => if p.clear_all {frame.clear()} else { frame.draw_rectangle((p.x as usize)*pix_width,  (p.y as usize)*pix_height, pix_width, pix_height, if p.on { color } else {0u32} ) },
            None => break
          }
        }
        cycle += 1;

        let opcode = cas.cartridge.get_opcode_from(cas.pc).unwrap();
        let instruction = from_opcode( opcode );

        if cycle >= cpu_cycles_per_monitor {
          loop {
            let monitor_elapsed = monitor_now.elapsed();
            if monitor_elapsed < monitor_remaining {
              std::thread::sleep(monitor_remaining-monitor_elapsed)
            } else {
              let tau = monitor_elapsed-monitor_remaining;
              monitor_remaining = if frame_duration > tau { frame_duration-tau } else { Duration::from_millis(0) };
              monitor_now = Instant::now();
              break
            }
          }
          cycle = 0;

          match check_keypress(&window) {
            Some(k) => cas.keyboard.push(k),
            None => {}
          }
          window.update_with_buffer(&frame.pixels, frame.width, frame.height).expect("buffer was updated");
          cas.register.tick();

          //println!("{:6.2} ms", Duration::as_micros(&monitor_remaining) as f64 / 1000.0);
        }

        cas.run_instruction(instruction);
      }
    }
  }

}
