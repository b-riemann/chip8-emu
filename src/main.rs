use std::env;
use std::io::Write;
use std::fs::OpenOptions;
use std::time::{Duration,Instant};

mod instruction;
use instruction::from_opcode;

mod cartridge;
use cartridge::Cartridge;

mod state;
use state::{Chip8State, TermDisplay};

mod window;
use window::Chip8Window;

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
    _ => {
      let frame_duration = Duration::from_micros(16_666);
      let cpu_cycles_per_frame = 29_333u16; // ~1.76 MHz, cosmac vp

      let pix_height: usize = 14;
      let pix_width: usize = 16;
      let color = 93000u32;

      let mut cas = Chip8State::new(cartridge);

      let mut cwin = Chip8Window::new((TermDisplay::WIDTH_PX as usize)*pix_width, (TermDisplay::HEIGHT_PX as usize)*pix_height);
      let mut cycle = 0u16;
      let mut monitor_now = Instant::now();
      let mut monitor_remaining = frame_duration;

      while cwin.is_active() {
        cycle += 1;
        loop {
          match cas.display.flips.pop_front() {
            Some(p) => if p.clear_all {cwin.clear()} else { cwin.draw_rectangle((p.x as usize)*pix_width,  (p.y as usize)*pix_height, pix_width, pix_height, if p.on { color } else {0u32} ) },
            None => break
          }
        }

        let opcode = cas.cartridge.get_opcode_from(cas.pc).unwrap();
        let instruction = from_opcode( opcode );

        if cycle >= cpu_cycles_per_frame {
          loop {
            let monitor_elapsed = monitor_now.elapsed();
            if monitor_elapsed < monitor_remaining {
              std::thread::sleep(monitor_remaining - monitor_elapsed)
            } else {
              let tau = monitor_elapsed - monitor_remaining;
              monitor_remaining = if frame_duration > tau { frame_duration - tau } else { Duration::from_millis(0) };
              monitor_now = Instant::now();
              break
            }
          }
          cycle = 0;

          match cwin.check_keypress() {
            Some(k) => cas.keyboard.push(k),
            None => {}
          }
          // ToDo: impl a new trait for window that uses frame instead?
          cwin.update_window();
          cas.register.tick();
        }

        cas.run_instruction(instruction);
      }
    }
  }

}
