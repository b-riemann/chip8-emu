use std::env;
use std::io;
use std::io::prelude::*;
use std::fs::File;

use crossterm::event::KeyEventKind;
use crossterm::terminal::enable_raw_mode;
use rand::prelude::*;

use std::time::{Duration, Instant};

use crossterm::event::{poll,read,Event,KeyEvent,KeyCode};

struct TermDisplay {
  display: [u8; 2080]
}

impl TermDisplay {
  const OFF_PIXEL : u8 = b'.';
  const ON_PIXEL : u8 = b'O';
  const WIDTH_PX : usize = 64;
  const HEIGHT_PX : usize = 32;

  fn set_newlines(&mut self) {
    for n in 1..Self::HEIGHT_PX+1 {
      self.display[n*(Self::WIDTH_PX+1)-1] = b'\n';
    }   
  }

  fn clear(&mut self) {
    self.display.fill( Self::OFF_PIXEL );
    self.set_newlines();
  }

  fn new() -> Self {
    let mut x = Self { display: [Self::OFF_PIXEL; 2080] };
    x.set_newlines();
    x
  }

  fn get_idx(x: u8, y: u8) -> usize {
    let yu = y as usize;
    let xu = x as usize;
    if yu>=Self::HEIGHT_PX || xu>=Self::WIDTH_PX {
      panic!("pixel out of range (x={},y={})", x, y);
    }
    yu*(Self::WIDTH_PX+1)+xu
  }

  fn get_pixel(&self, x: u8, y: u8) -> bool {
      self.display[Self::get_idx(x,y)] == Self::ON_PIXEL 
  }

  fn set_pixel(&mut self, x: u8, y: u8, tf: bool) {
      self.display[Self::get_idx(x, y)] = if tf { Self::ON_PIXEL } else { Self::OFF_PIXEL };
  }
}


struct Chip8State {
    pc: u16,      // main address register (program counter)
    i: u16,       // additional 16-bit address register
    v: [u8; 16],  // variables v0 -- vF
    stack: Vec<u16>, // stores registers when (possibly multiple enclosed) subroutines are called
    delay: u8,    // delay timer
    keyboard: u8, // hex keyboard
    display: TermDisplay, // bits of the 32x64 display. the u8s are xor'ed with sprites and thus form a part of the state
    drawop: u16,     // custom: indicate if the content needs to be drawn, and what is drawn. see draw method.
    rng: ThreadRng,  // custom: random number generator
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
                     delay: 0, keyboard: 0x00, display: TermDisplay::new(),
                     drawop: 0x0000, rng: rand::thread_rng() }
    }

    fn draw(&mut self, memo: &[u8; 0xf00]) {
        match self.drawop & 0xf000 {// different from orig.opcode 00e0 for faster comparison
            0xe000 => {
                self.display.clear();
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
                    let oldstate = self.display.get_pixel(x,y);

                    
                    if flip {
                      self.display.set_pixel(x,y, !oldstate);
                      if oldstate {
                          any_flipped_off = true;
                      }
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

    fn run_opcode(&mut self, opcode: u16, memo: &mut [u8]) -> String {
      match opcode & 0xf000 {
        0x0000 => match opcode {
                     0x00e0 => { 
                         self.drawop = 0xe000;
                         "CLEAR DRAW".to_string()
                     },
                     0x00ee => {
                         self.pc = self.stack.pop().unwrap();
                         "return from subroutine".to_string()
                     },
                     _ => panic!("{:#06x} call RCA 1802 routine {:#05x}: not implemented", opcode, opcode & 0x0fff),
                  },
        0x1000 => { 
                     let target = opcode & 0x0fff;
                     self.pc = target - 2; //-2: correct next increment
                     format!("goto address {:#05x}", target)
                  },
        0x2000 => { 
                     self.stack.push(self.pc);
                     let target = opcode & 0x0fff;
                     self.pc = target - 2;
                     format!("run subroutine at {:#05x}", target)
                  },
        0x3000 => { 
                     let varnum = get_0x00(opcode);
                     let num = get_00nn(opcode);
                     if self.v[varnum] == num { 
                         self.pc += 2;
                     }
                     format!("if (v{:#03x} == {:#04x}) skip next (is {:#04x})", varnum, num, self.v[varnum])
                  },
        0x4000 => { 
                     let varnum = get_0x00(opcode);
                     let num = get_00nn(opcode);
                     if self.v[varnum] != num { 
                         self.pc += 2;
                     }
                     format!("if (v{:#03x} != {:#04x}) skip next (is {:#04x})", varnum, num, self.v[varnum])
                  },

        0x6000 => { 
                     let varnum = get_0x00(opcode);
                     let num = get_00nn(opcode);
                     self.v[varnum] = num;
                     format!("v{:#03x} = {:#04x}", varnum, num)
                  },
        0x7000 => { 
                     let varnum = get_0x00(opcode);
                     let num = get_00nn(opcode);
                     self.v[varnum] = self.v[varnum].wrapping_add(num);
                     format!("v{:#03x} += {:#04x} ignoring carry (now {:#04x})", varnum, num, self.v[varnum])
                  },
        0x8000 => {  let varxnum = get_0x00(opcode);
                     let varynum = get_00y0(opcode);
                     match opcode & 0xf00f {
                       0x8000 => { self.v[varxnum]  = self.v[varynum];
                                   format!("v{:#03x} = v{:#03x}", varxnum, varynum)
                                  },
                       0x8001 => { self.v[varxnum]  = self.v[varxnum] | self.v[varynum];
                                   format!("v{:#03x} = v{:#03x} | v{:#03x}", varxnum, varxnum, varynum)
                                  },
                       0x8002 => { self.v[varxnum]  = self.v[varxnum] & self.v[varynum];
                                   format!("v{:#03x} = v{:#03x} & v{:#03x}", varxnum, varxnum, varynum)
                                  },
                       0x8003 => { self.v[varxnum]  = self.v[varxnum] ^ self.v[varynum];
                                   format!("v{:#03x} = v{:#03x} ^ v{:#03x}", varxnum, varxnum, varynum)
                                  },
                       0x8004 => { let newnum = self.v[varxnum] as u16 + self.v[varynum] as u16;
                                   let carry = (newnum & 0x0f00) > 0;
                                   self.v[0xf] = if carry { 1 } else {0};
                                   self.v[varxnum] = (newnum & 0x00ff) as u8;
                                   format!("v{:#03x} += v{:#03x} with carry in v0xf", varxnum, varynum)
                                 },
                       0x8005 => { let newnum = self.v[varxnum] as u16 - self.v[varynum] as u16;
                                   let borrow = (newnum & 0xff00) > 0;
                                   self.v[0xf] = if borrow { 1 } else {0};
                                   self.v[varxnum] = (newnum & 0x00ff) as u8;
                                   format!("v{:#03x} -= v{:#03x}", varxnum, varynum)
                                 },
                       0x8006 => { self.v[0xf] = self.v[varxnum] & 0x01;
                                   self.v[varxnum] = self.v[varxnum] >> 1;
                                   format!("v{:#03x} >> 1 (store lost bit in v0xf)", varxnum)
                                 },
                       0x8007 => { self.v[varxnum]  = self.v[varynum] - self.v[varxnum]; //self.v[varynum].wrapping_sub( self.v[varxnum] );
                                   format!("v{:#03x} = v{:#03x} - v{:#03x}", varxnum, varynum, varxnum)
                                 },
                       // 0x800e => stri = pref + format!("Vx<<=1"),
                       _ => panic!("{:#06x} unknown opcode!", opcode),
                     }
                  },
        0x9000 => {  let varxnum = get_0x00(opcode);
                     let varynum = get_00y0(opcode);
                     if self.v[varxnum] != self.v[varynum] { 
                         self.pc += 4;
                     }
                     format!("if v{:#03x} != v{:#03x} skip next", varxnum, varynum)
                  },                         
        0xa000 => {  self.i = opcode & 0x0fff;
                     format!("i = {:#05x}", self.i)
                    },
        0xc000 => { let varnum = get_0x00(opcode);
                    let num = get_00nn(opcode);
                    let ran: u8 = self.rng.gen();
                    self.v[varnum] = ran & num;
                    format!("v{:#03x} = rand() & {:#04x} (now {:#04x})", varnum, num, self.v[varnum])
                  },
        0xd000 => { self.drawop = opcode;
                    "DRAW".to_string()
                  },
        0xe000 => match opcode  & 0xf0ff {
                     0xe09e => {
                       let varnum = get_0x00(opcode);
                       if self.keyboard == self.v[varnum] {
                         self.pc += 4;
                       }
                       format!("if (v{:#03x} == keyboard) skip next (is {:#04x})", varnum, self.v[varnum])
                     },
                     0xe0a1 => {
                       let varnum = get_0x00(opcode);
                       if self.keyboard != self.v[varnum] {
                         self.pc += 4;
                       }
                       format!("if (v{:#03x} != keyboard) skip next (is {:#04x})", varnum, self.v[varnum])
                     },
                     _ => panic!("{:06x} unknown opcode!", opcode),
                  },
        0xf000 => match opcode & 0xf0ff {
                     0xf007 => {
                       let varnum = get_0x00(opcode);
                       self.v[varnum] = self.delay; 
                       format!("set v{:03x} to delay", varnum)
                     },
                     0xf015 => {
                       let varnum = get_0x00(opcode);
                       self.delay = self.v[varnum];
                       format!("set delay to v{:#03x}", varnum)
                     },
                     0xf018 => { "SOUND timer=v0x. (not implemented)".to_string() },
                     0xf01e => {
                       let varnum = get_0x00(opcode);
                       if varnum == 0xf {
                         panic!("VF should not be affected, should this instruction occur?");
                       }
                       self.i += u16::from(self.v[varnum]);
                       format!("i += v{:#03x} (now {:#05x})", varnum, self.i)
                     },
                     0xf029 => {
                       let varnum = get_0x00(opcode);
                       self.i = (self.v[varnum]*5) as u16;
                       format!("set i to sprite for character v{:#03x} not implemented", varnum)
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
                       format!("store v{:#03x}={:#05x} to i={:#05x} as bin.coded.dec.{}{}{})",
                           varnum, self.v[varnum], self.i, hun, dec, uno)
                     },
                     0xf055 => {
                       let n_to_store = get_0x00(opcode) as usize +1;
                       let ui = self.i as usize;
                       for offset in 0..n_to_store {
                           memo[ui + offset] = self.v[offset];
                       }
                       format!("dump v0x0,...,v{:#03x} to i={:#05x})", n_to_store-1, self.i)
                     },
                     0xf065 => {
                       let n_to_load = get_0x00(opcode) as usize +1;
                       let ui = self.i as usize;
                       for offset in 0..n_to_load {
                           self.v[offset] = memo[ui + offset];
                       }
                       format!("load v0x0,...,v{:#03x} from i={:#05x})", n_to_load-1, self.i)
                     },
                     _ => panic!("opcode {:#06x} not implemented", opcode),
                  },
        _ => panic!("{:#06x} not implemented", opcode),
    }
    }

}

fn main() -> Result<(), io::Error> {
    enable_raw_mode().expect("can run in raw mode");

    let filename = env::args().nth(1).expect("insert cartridge"); //"roms/tetris.rom";
    let mut file = File::open(&filename).expect("file opened");

    let mut memo: [u8; 0xf00] = [0; 0xf00]; 
    let mut fonts = File::open("fonts")?;
    fonts.read(&mut memo)?;
    file.read(&mut memo[0x200..])?;

    //let stdout = io::stdout();
    //let backend = CrosstermBackend::new(stdout);
    //let mut terminal: Terminal<CrosstermBackend<io::Stdout>> = Terminal::new(backend).expect("terminal created");

    let mut chip = Chip8State::init();

    chip.drawop = 0xe000; // clear screen drawing command
    chip.draw(&memo);

    let dur = Duration::from_millis(50); //Duration::new(0, 15_000_000u32);
    let microdur = Duration::from_millis(10); //Duration::new(0, 100_000u32);

    'maincontrolloop: loop {
        let now = Instant::now();

        if chip.delay > 0 {
          chip.delay -= 1;
        }

        if poll(microdur)? {
          match read()? {
              Event::Key(KeyEvent { code: KeyCode::Esc, kind: KeyEventKind::Press, .. }) => break 'maincontrolloop,
              Event::Key(KeyEvent { code: KeyCode::Char('w'), kind: KeyEventKind::Press, .. }) => chip.keyboard = 0x04,
              Event::Key(KeyEvent { code: KeyCode::Char('a'), kind: KeyEventKind::Press, .. }) => chip.keyboard = 0x05,
              Event::Key(KeyEvent { code: KeyCode::Char('d'), kind: KeyEventKind::Press, .. }) => chip.keyboard = 0x06,
              Event::Key(KeyEvent { code: KeyCode::Char('s'), kind: KeyEventKind::Press, .. }) => chip.keyboard = 0x07,
              Event::Key(KeyEvent { code: KeyCode::Char('w'), kind: KeyEventKind::Release, .. }) => chip.keyboard = 0,
              Event::Key(KeyEvent { code: KeyCode::Char('a'), kind: KeyEventKind::Release, .. }) => chip.keyboard = 0,
              Event::Key(KeyEvent { code: KeyCode::Char('d'), kind: KeyEventKind::Release, .. }) => chip.keyboard = 0,
              Event::Key(KeyEvent { code: KeyCode::Char('s'), kind: KeyEventKind::Release, .. }) => chip.keyboard = 0,
              _ => (),
          }
        }

        let uaddr = usize::from(chip.pc);
        let opcode = get_opcode(memo[uaddr], memo[uaddr+1]);

        let pc_in = chip.pc;
        let msg = chip.run_opcode(opcode, &mut memo);
        chip.pc += 2;

        if chip.drawop != 0x0000 {
          chip.draw(&memo);
          println!("{}", std::str::from_utf8(&chip.display.display).unwrap());
        }

        println!("{:♥<1$} pc {2:#03x} opc {3:#06x} :: {4}", "", chip.stack.len(), pc_in,  opcode, msg);
        
        while now.elapsed() < dur {
          std::thread::sleep(microdur);
        }
    }
    Ok(())
  }
