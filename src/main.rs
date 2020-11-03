extern crate sdl2;


use std::env;
use std::io;
use std::io::prelude::*;
use std::fs::File;

use rand::prelude::*;

// graphics part
use sdl2::{pixels::Color, render::Canvas, video::Window, rect::Point, event::Event, keyboard::Keycode};
use std::time::{Duration, Instant};

struct Chip8State {
    pc: u16,      // main address register (program counter)
    i: u16,       // additional 16-bit address register
    v: [u8; 16],  // variables v0 -- vF
    stack: Vec<u16>, // stores registers when (possibly multiple enclosed) subroutines are called
    delay: u8,    // delay timer
    keyboard: u8, // hex keyboard
    display: [bool; 2048], // bits of the 32x64 display. the u8s are xor'ed with sprites and thus form a part of the state
    drawop: u16,     // custom: indicate if the content needs to be drawn, and what is drawn. see draw method.
    rng: ThreadRng,  // custom: random number generator
    show_instr: bool // custom: show instructions
}


fn get_opcode(first: u8, second: u8) -> u16 {
    u16::from_be_bytes([first,second])
}

fn get_0x00(opcode: u16) -> usize {
    usize::from((opcode & 0x0f00) >> 8)
}

fn get_00y0(opcode: u16) -> usize {
    usize::from((opcode & 0x00f0) >> 4)
}

fn get_00nn(opcode: u16) -> u8 {
   (opcode & 0x00ff) as u8
}


impl Chip8State {
    fn init() -> Chip8State {
        Chip8State { pc: 0x200, i: 0, v: [0; 16], stack: Vec::with_capacity(32),
                     delay: 0, keyboard: 0x00, display: [false; 2048],
                     drawop: 0x0000, rng: rand::thread_rng(), show_instr: false }
    }

    fn get_pixel(&self, x: u8, y: u8) -> bool {
        if y>31 || x>63 {
            return false; // never drawn 
        }
        let idx = y as usize*64+x as usize; // idx of a boolean array
        return self.display[idx]; 
    }

    fn set_pixel(&mut self, x: u8, y: u8, tf: bool) {
        if y>31 || x>63 {
            panic!("pixel out of range (x={},y={})", x, y); 
        }
        let idx = y as usize*64+x as usize; // idx of a boolean array
        self.display[idx] = tf;
    }

    fn draw(&mut self, canvas: &mut Canvas<Window>, memo: &[u8; 0xf00]) {
        let bgcolor = Color::RGB(29, 31, 33);
        let fgcolor = Color::RGB(240, 255, 255); //Color::RGB(95, 215, 255);
        match self.drawop & 0xf000 {// different from orig.opcode 00e0 for faster comparison
            0xe000 => {
                self.display = [false; 2048];
                canvas.set_draw_color(bgcolor);
                canvas.clear();
            },
            0xd000 => { // 0xdxyn draw in rectangle (original opcode) 
                let x0 = self.v[get_0x00(self.drawop)] + 7;
                let y0 = self.v[get_00y0(self.drawop)];
                
                let h = (self.drawop & 0x000f) as u8;
                let mut p: u8=0;
                let mut any_flipped_off = false;
                while p<h {
                  let bitti = memo[self.i as usize +p as usize];
                  
                  let y = y0+p;
                  let mut q=0;
                  while q<8 {
                    let x = x0-q;

                    let flip = (bitti >> q) & 0x01 == 0x01;
                    let oldstate = self.get_pixel(x,y);

                    
                    if flip {
                      canvas.set_draw_color(if oldstate {bgcolor} else {fgcolor});
                      self.set_pixel(x,y, !oldstate);
                      if oldstate {
                          any_flipped_off = true;
                      }
                      canvas.draw_point(Point::new(x as i32, y as i32)).unwrap();
                    }

                    q += 1;
                  }
                  p += 1;
                }
                self.v[0xf] = if any_flipped_off { 1 } else { 0 };
            },
            _ => panic!("unknown draw operation. skip over all entries not 0xe... or 0xd..."),
        }
        self.drawop = 0x0000;
    }

    fn run_address(&mut self, memo: &mut [u8]) {
        let uaddr = usize::from(self.pc);
        let opcode = get_opcode(memo[uaddr], memo[uaddr+1]);

        let mut stri = String::with_capacity(64);
        for _ in 0..self.stack.len() {
          stri.push_str("  ");
        }
        stri.push_str(&format!("adr {:#03x} opc {:#06x} ", self.pc, opcode));

        match opcode & 0xf000 {
            0x0000 => match opcode {
                         0x00e0 => { 
                             self.drawop = 0xe000;
                             stri.push_str("CLEAR DRAW");
                         },
                         0x00ee => {
                             self.pc = self.stack.pop().unwrap();
                             stri.push_str("return from subroutine");
                         },
                         _ => panic!("{:#06x} call RCA 1802 routine {:#05x}: not implemented", opcode, opcode & 0x0fff),
                      },
            0x1000 => { 
                         self.pc = opcode & 0x0fff;
                         stri.push_str(&format!("goto address {:#05x}", self.pc));
                         if self.show_instr { println!("{}", stri) }
                         return;
                      },
            0x2000 => { 
                         self.stack.push(self.pc);
                         self.pc = opcode & 0x0fff;
                         stri.push_str(&format!("run subroutine at {:#05x}", self.pc));
                         if self.show_instr { println!("{}", stri) }
                         return;
                      },
            0x3000 => { 
                         let varnum = get_0x00(opcode);
                         let num = get_00nn(opcode);
                         stri.push_str(&format!("if (v{:#03x} == {:#04x}) skip next (is {:#04x})", varnum, num, self.v[varnum]));
                         if self.v[varnum] == num { 
                             self.pc += 4;
                             if self.show_instr { println!("{}", stri) }
                             return;
                         }
                      },
            0x4000 => { 
                         let varnum = get_0x00(opcode);
                         let num = get_00nn(opcode);
                         stri.push_str(&format!("if (v{:#03x} != {:#04x}) skip next (is {:#04x})", varnum, num, self.v[varnum]));
                         if self.v[varnum] != num { 
                             self.pc += 4;
                             if self.show_instr { println!("{}", stri) }
                             return;
                         }
                      },

            0x6000 => { 
                         let varnum = get_0x00(opcode);
                         let num = get_00nn(opcode);
                         stri.push_str(&format!("v{:#03x} = {:#04x}", varnum, num));
                         self.v[varnum] = num;
                      },
            0x7000 => { 
                         let varnum = get_0x00(opcode);
                         let num = get_00nn(opcode);
                         self.v[varnum] = self.v[varnum].wrapping_add(num);
                         stri.push_str(&format!("v{:#03x} += {:#04x} ignoring carry (now {:#04x})", varnum, num, self.v[varnum]));
                      },
            0x8000 => {  let varxnum = get_0x00(opcode);
                         let varynum = get_00y0(opcode);
                         match opcode & 0xf00f {
                           0x8000 => { self.v[varxnum]  = self.v[varynum];
                                       stri.push_str(&format!("v{:#03x} = v{:#03x}", varxnum, varynum)); },
                           0x8001 => { self.v[varxnum]  = self.v[varxnum] | self.v[varynum];
                                       stri.push_str(&format!("v{:#03x} = v{:#03x} | v{:#03x}", varxnum, varxnum, varynum)); },
                           0x8002 => { self.v[varxnum]  = self.v[varxnum] & self.v[varynum];
                                       stri.push_str(&format!("v{:#03x} = v{:#03x} & v{:#03x}", varxnum, varxnum, varynum)); },
                           0x8003 => { self.v[varxnum]  = self.v[varxnum] ^ self.v[varynum];
                                       stri.push_str(&format!("v{:#03x} = v{:#03x} ^ v{:#03x}", varxnum, varxnum, varynum)); },
                           0x8004 => { let newnum = self.v[varxnum] as u16 + self.v[varynum] as u16;
                                       let carry = (newnum & 0x0f00) > 0;
                                       self.v[0xf] = if carry { 1 } else {0};
                                       self.v[varxnum] = (newnum & 0x00ff) as u8;
                                       stri.push_str(&format!("v{:#03x} += v{:#03x} with carry in v0xf", varxnum, varynum)); 
                                     },
                           0x8005 => { let newnum = self.v[varxnum] as u16 - self.v[varynum] as u16;
                                       let borrow = (newnum & 0xff00) > 0;
                                       self.v[0xf] = if borrow { 1 } else {0};
                                       self.v[varxnum] = (newnum & 0x00ff) as u8;
                                       stri.push_str(&format!("v{:#03x} -= v{:#03x}", varxnum, varynum)); },
                           // 0x8006 => stri = pref + format!("Vx>>=1"),
                           0x8007 => { self.v[varxnum]  = self.v[varynum] - self.v[varxnum]; //self.v[varynum].wrapping_sub( self.v[varxnum] );
                                       stri.push_str(&format!("v{:#03x} = v{:#03x} - v{:#03x}", varxnum, varynum, varxnum)); },
                           // 0x800e => stri = pref + format!("Vx<<=1"),
                           _ => panic!("{:#06x} unknown opcode!", opcode),
                         }
                      },
            0x9000 => {  let varxnum = get_0x00(opcode);
                         let varynum = get_00y0(opcode);
                         stri.push_str(&format!("if v{:#03x} != v{:#03x} skip next", varxnum, varynum));
                         if self.v[varxnum] != self.v[varynum] { 
                             self.pc += 4;
                             if self.show_instr { println!("{}", stri) }
                             return;
                         }
                      },                         
            0xa000 => {  self.i = opcode & 0x0fff;
                         stri.push_str(&format!("i = {:#05x}", self.i)); },
            0xc000 => { let varnum = get_0x00(opcode);
                        let num = get_00nn(opcode);
                        let ran: u8 = self.rng.gen();
                        self.v[varnum] = ran & num;
                        stri.push_str(&format!("v{:#03x} = rand() & {:#04x} (now {:#04x})", varnum, num, self.v[varnum]));
                      },
            0xd000 => { self.drawop = opcode;
                        stri.push_str(&format!("DRAW"));
                      },
            0xe000 => match opcode  & 0xf0ff {
                         0xe09e => {
                           let varnum = get_0x00(opcode);
                           stri.push_str(&format!("if (v{:#03x} == keyboard) skip next (is {:#04x})", varnum, self.v[varnum]));
                           if self.keyboard == self.v[varnum] {
                             self.pc += 4;
                             if self.show_instr { println!("{}", stri) }
                             return;
                           }
                         },
                         0xe0a1 => {
                           let varnum = get_0x00(opcode);
                           stri.push_str(&format!("if (v{:#03x} != keyboard) skip next (is {:#04x})", varnum, self.v[varnum]));
                           if self.keyboard != self.v[varnum] {
                             self.pc += 4;
                             if self.show_instr { println!("{}", stri) }
                             return;
                           }
                         },
                         _ => panic!("{:06x} unknown opcode!", opcode),
                      },
            0xf000 => match opcode & 0xf0ff {
                         0xf007 => {
                           let varnum = get_0x00(opcode);
                           self.v[varnum] = self.delay; 
                           stri.push_str(&format!("set v{:03x} to delay", varnum))
                         },
                         0xf015 => {
                           let varnum = get_0x00(opcode);
                           self.delay = self.v[varnum];
                           stri.push_str(&format!("set delay to v{:#03x}", varnum))
                         },
                         0xf01e => {
                           let varnum = get_0x00(opcode);
                           if varnum == 0xf {
                             panic!("VF should not be affected, should this instruction occur?");
                           }
                           self.i += u16::from(self.v[varnum]);
                           stri.push_str(&format!("i += v{:#03x} (now {:#05x})", varnum, self.i))
                         },
                         0xf029 => {
                           let varnum = get_0x00(opcode);
                           self.i = (self.v[varnum]*5) as u16;
                           stri.push_str(&format!("set i to sprite for character v{:#03x} not implemented", varnum));
                         },
                         0xf033 => {
                           let varnum = get_0x00(opcode);
                           let ui = self.i as usize;
                           let hun = self.v[varnum] / 100;
                           memo[ui] = hun;
                           let dec = self.v[varnum] / 10 - hun*10;
                           memo[ui+1] = dec;
                           let uno = self.v[varnum] - dec*10 - hun*100;
                           memo[ui+2] = uno;
                           stri.push_str(&format!("store v{:#03x}={:#05x} to i={:#05x} as bin.coded.dec.{}{}{})",
                               varnum, self.v[varnum], self.i, hun, dec, uno));
                         },
                         0xf055 => {
                           let n_to_store = get_0x00(opcode) as usize +1;
                           let ui = self.i as usize;
                           for offset in 0..n_to_store {
                               memo[ui + offset] = self.v[offset];
                           }
                           stri.push_str(&format!("dump v0x0,...,v{:#03x} to i={:#05x})", n_to_store-1, self.i))
                         },
                         0xf065 => {
                           let n_to_load = get_0x00(opcode) as usize +1;
                           let ui = self.i as usize;
                           for offset in 0..n_to_load {
                               self.v[offset] = memo[ui + offset];
                           }
                           stri.push_str(&format!("load v0x0,...,v{:#03x} from i={:#05x})", n_to_load-1, self.i))
                         },
                         _ => panic!("opcode {:#06x} not implemented", opcode),
                      },
            _ => panic!("{:#06x} not implemented", opcode),
        }
        if self.show_instr { println!("{}", stri) }
        self.pc += 2;
    }
}

fn main() -> io::Result<()> {
    let filename = env::args().nth(1).unwrap(); //"roms/tetris.rom";

    let mut file = File::open(&filename)?;

    let mut memo: [u8; 0xf00] = [0; 0xf00]; 

    let mut fonts = File::open("fonts")?;
    fonts.read(&mut memo)?;

 
    file.read(&mut memo[0x200..])?;

    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();
    let window = video.window(&filename, 512, 256)
        //.position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    canvas.set_scale(8.0, 8.0).unwrap();

    let mut chip = Chip8State::init();

    chip.drawop = 0xe000; // clear screen drawing command
    chip.draw(&mut canvas, &memo);

    canvas.present();

    let dur = Duration::new(0, 15_000_000u32);
    let longdur = Duration::new(0, 100_000_000u32);
    let microdur = Duration::new(0, 100_000u32);
    let mut event_pump = sdl_context.event_pump().unwrap();

    'maincontrolloop: loop {
        let now = Instant::now();

        if chip.delay > 0 {
          chip.delay -= 1;
        }

        for event in event_pump.poll_iter() {
          match event {
              Event::KeyDown { keycode: Some(Keycode::Escape), .. }=> break 'maincontrolloop,
              Event::KeyDown { keycode: Some(Keycode::W), .. } => chip.keyboard = 0x04,
              Event::KeyDown { keycode: Some(Keycode::A), .. } => chip.keyboard = 0x05,
              Event::KeyDown { keycode: Some(Keycode::D), .. } => chip.keyboard = 0x06,
              Event::KeyDown { keycode: Some(Keycode::S), .. } => chip.keyboard = 0x07,
              Event::KeyDown { keycode: Some(Keycode::X), .. } => chip.show_instr = !chip.show_instr,
              Event::KeyUp { keycode: Some(Keycode::W), .. } => chip.keyboard = 0,
              Event::KeyUp { keycode: Some(Keycode::A), .. } => chip.keyboard = 0,
              Event::KeyUp { keycode: Some(Keycode::D), .. } => chip.keyboard = 0,
              Event::KeyUp { keycode: Some(Keycode::S), .. } => chip.keyboard = 0,
              _ => (),
          }
        }

        for _ in 0..8 {
          chip.run_address(&mut memo);
          if chip.drawop != 0x0000 {
            chip.draw(&mut canvas, &memo);
            canvas.present();
            break;
          }
          if chip.show_instr { std::thread::sleep(longdur) };
        }
        
        while now.elapsed() < dur {
          std::thread::sleep(microdur);
        }
    }
    Ok(())
}
