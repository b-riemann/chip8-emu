extern crate sdl2;

//use std::env;
use std::io;
use std::io::prelude::*;
use std::fs::File;

// graphics part
use sdl2::{pixels::Color, render::Canvas, video::Window, rect::Point};
use std::time::Duration;

struct Chip8State {
    pc: u16,      // main address register (program counter)
    i: u16,       // additional 16-bit address register
    v: [u8; 16],  // variables v0 -- vF
    stack: Vec<u16>, // stores registers when (possibly multiple enclosed) subroutines are called
    delay: u8,    // delay timer
    keyboard: u8, // hex keyboard
    display: [bool; 2048], // bits of the 32x64 display. the u8s are xor'ed with sprites and thus form a part of the state
    drawop: u16   // custom: indicate if the content needs to be drawn, and what is drawn. see draw method.
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
        Chip8State { pc: 0x200, i: 0, v: [0; 16], stack: Vec::with_capacity(32), delay: 0, keyboard: 0x00, display: [false; 2048], drawop: 0x0000 }
    }

    fn get_pixel(&self, x: u8, y: u8) -> bool {
        if y>32 || x>64 {
            println!("skipdraw");
            return false; 
        }
        let idx = y as usize*64+x as usize; // idx of a boolean array
        //println!("getp x={}, y={}, idx={}", x, y, idx);
        return self.display[idx]; 
    }

    fn set_pixel(&mut self, x: u8, y: u8, tf: bool) {
        if y>32 || x>64 {
            return; 
        }
        let idx = y as usize*64+x as usize; // idx of a boolean array
        self.display[idx] = tf;
    }

    fn draw(&mut self, canvas: &mut Canvas<Window>, memo: &[u8; 0xf00]) {
        let bgcolor = Color::RGB(60,60,60);
        let fgcolor = Color::RGB(240,255, 255);
        match self.drawop & 0xf000 {// different from orig.opcode 00e0 for faster comparison
            0xe000 => {
                self.display = [false; 2048];
                canvas.set_draw_color(bgcolor);
                canvas.clear();
            },
            0xd000 => { // 0xdxyn draw in rectangle (original opcode) 
                let x0 = self.v[get_0x00(self.drawop)] + 8;
                let y0 = self.v[get_00y0(self.drawop)];
                
                let h = (self.drawop & 0x000f) as u8+ 1;
                let mut p: u8=0;
                let mut any_flipped = false;
                while p<h {
                  let bitti = memo[self.i as usize +p as usize];
                  let y = if y0==255 {println!("drawskip");y0} else {y0+p};
                  let mut q=0;
                  while q<8 {
                    let x = x0-q;

                    let flip = (bitti >> q) & 0x01 == 0x01;
                    if flip {
                      any_flipped = true;
                      if self.get_pixel(x,y) {  
                        canvas.set_draw_color(bgcolor);
                        self.set_pixel(x,y, false);
                      } else {
                        canvas.set_draw_color(fgcolor);
                        self.set_pixel(x,y, true);
                      }
                      canvas.draw_point(Point::new(x as i32, y as i32)).unwrap();
                    }

                    q += 1;
                  }
                  p += 1;
                }
                self.v[0xf] = if any_flipped { 1 } else { 0 };
            },
            _ => panic!("unknown draw operation. skip over all entries not 0xe... or 0xd..."),
        }
        self.drawop = 0x0000;
    }

    fn run_address(&mut self, memo: &[u8; 0xf00]) {
        let uaddr = usize::from(self.pc);
        let opcode = get_opcode(memo[uaddr], memo[uaddr+1]);

        match opcode & 0xf000 {
            0x0000 => match opcode {
                         0x00e0 => { 
                             self.drawop = 0xe000;
                             println!("{:#06x} CLEAR DRAW", opcode);
                         },
                         0x00ee => {
                             self.pc = self.stack.pop().unwrap();
                             println!("{:#06x} return from subroutine (lvl {})", opcode, self.stack.len());
                         },
                         _ => panic!("{:#06x} call RCA 1802 routine {:#05x}: not implemented", opcode, opcode & 0x0fff),
                      },
            0x1000 => { 
                         self.pc = opcode & 0x0fff;
                         println!("{:#06x} goto address {:#05x}", opcode, self.pc);
                         return;
                      },
            0x2000 => { 
                         self.stack.push(self.pc);
                         self.pc = opcode & 0x0fff;
                         println!("{:#06x} run subroutine at {:#05x} (lvl {})", opcode, self.pc, self.stack.len());
                      },
            0x3000 => { 
                         let varnum = get_0x00(opcode);
                         let num = get_00nn(opcode);
                         println!("{:#06x} if (v{:#03x} == {:#x}) skip next", opcode, varnum, num);
                         if self.v[varnum] == num { 
                             self.pc += 4;
                             return;
                         }
                      },
            0x4000 => { 
                         let varnum = get_0x00(opcode);
                         let num = get_00nn(opcode);
                         println!("{:#06x} if (v{:#03x} != {:#x}) skip next", opcode, varnum, num);
                         if self.v[varnum] != num { 
                             self.pc += 4;
                             return;
                         }
                      },

            0x6000 => { 
                         let varnum = get_0x00(opcode);
                         let num = get_00nn(opcode);
                         println!("{:#06x} v{:#03x} = {:#04x}", opcode, varnum, num);
                         self.v[varnum] = num;
                      },
            0x7000 => { 
                         let varnum = get_0x00(opcode);
                         let num = get_00nn(opcode);
                         self.v[varnum] = self.v[varnum].wrapping_add(num);
                         println!("{:#06x} v{:#03x} += {:#04x} ignoring carry (now {:#04x})", opcode, varnum, num, self.v[varnum]);
                      },
            0x8000 => {  let varxnum = get_0x00(self.drawop);
                         let varynum = get_00y0(self.drawop);
                         match opcode & 0xf00f {
                           0x8000 => { self.v[varxnum]  = self.v[varynum];
                                       println!("{:#06x} v{:#03x} = v{:#03x}", opcode, varxnum, varynum); },
                           0x8001 => { self.v[varxnum]  = self.v[varxnum] | self.v[varynum];
                                       println!("{:#06x} v{:#03x} = v{:#03x} | v{:#03x}", opcode, varxnum, varxnum, varynum); },
                           0x8002 => { self.v[varxnum]  = self.v[varxnum] & self.v[varynum];
                                       println!("{:#06x} v{:#03x} = v{:#03x} & v{:#03x}", opcode, varxnum, varxnum, varynum); },
                           0x8003 => { self.v[varxnum]  = self.v[varxnum] ^ self.v[varynum];
                                       println!("{:#06x} v{:#03x} = v{:#03x} ^ v{:#03x}", opcode, varxnum, varxnum, varynum); },
                           0x8004 => { self.v[varxnum]  = self.v[varxnum] + self.v[varynum]; // self.v[varxnum].wrapping_add( self.v[varynum] );
                                       println!("{:#06x} v{:#03x} += v{:#03x}", opcode, varxnum, varynum); },
                           0x8005 => { self.v[varxnum]  = self.v[varxnum] - self.v[varynum]; // self.v[varxnum].wrapping_sub( self.v[varynum] );
                                       println!("{:#06x} v{:#03x} -= v{:#03x}", opcode, varxnum, varynum); },
                           // 0x8006 => println!("{:#06x} Vx>>=1", opcode),
                           0x8007 => { self.v[varxnum]  = self.v[varynum] - self.v[varxnum]; //self.v[varynum].wrapping_sub( self.v[varxnum] );
                                       println!("{:#06x} v{:#03x} = v{:#03x} - v{:#03x}", opcode, varxnum, varynum, varxnum); },
                           // 0x800e => println!("{:#06x} Vx<<=1", opcode),
                           _ => panic!("{:06x} unknown opcode!", opcode),
                         }
                      },
            0xa000 => {  self.i = opcode & 0x0fff;
                         println!("{:#06x} i = {:#05x}", opcode, self.i); },
            0xc000 => { let varnum = get_0x00(opcode);
                        let num = get_00nn(opcode);
                        self.v[varnum] = rand::random::<u8>() & num;
                        println!("{:#06x} v{:#03x} = rand() & {:#04x} (now {:#04x})", opcode, varnum, num, self.v[varnum]);
                      },
            0xd000 => { self.drawop = opcode;
                        println!("{:#06x} DRAW", opcode);
                      },
            0xe000 => match opcode  & 0xf0ff {
                         0xe09e => {
                           let varnum = get_0x00(opcode);
                           println!("{:#06x} if (v{:#03x} == keyboard) skip next", opcode, varnum);
                           if self.keyboard == self.v[varnum] {
                             self.pc += 4;
                             return;
                           }
                         },
                         0xe0a1 => {
                           let varnum = get_0x00(opcode);
                           println!("{:#06x} if (v{:#03x} != keyboard) skip next", opcode, varnum);
                           if self.keyboard != self.v[varnum] {
                             self.pc += 4;
                             return;
                           }
                         },
                         _ => panic!("{:06x} unknown opcode!", opcode),
                      },
            0xf000 => match opcode & 0xf0ff {
                         0xf007 => {
                           let varnum = get_0x00(opcode);
                           self.v[varnum] = self.delay; 
                           println!("{:#06x} set v{:03x} to delay", opcode, varnum)
                         },
                         0xf015 => {
                           let varnum = get_0x00(opcode);
                           self.delay = self.v[varnum];
                           println!("{:#06x} set delay to v{:#03x}", opcode, varnum)
                         },
                         0xf01e => {
                           let varnum = get_0x00(opcode);
                           if varnum == 0xf {
                             panic!("VF should not be affected, should this indtruction occur?");
                           }
                           self.i += u16::from(self.v[varnum]);
                           println!("{:#06x} i += v{:#03x}", opcode, varnum)
                         },
                         _ => panic!("{:#06x} not implemented", opcode),
                      },
            _ => panic!("{:#06x} not implemented", opcode),
        }

        self.pc += 2;
    }


}

fn main() -> io::Result<()> {
    let filename = "roms/tetris.rom";

    let mut file = File::open(filename)?;

    let mut memo: [u8; 0xf00] = [0; 0xf00]; 
    
    //file.read(&mut memo)?; // maybe we need an offset here!, 0x200?
    file.read(&mut memo[0x200..])?; // maybe we need an offset here!, 0x200?



    //let mut count = 0;
    //loop
    //{
    //    println!("  instr {} at address {:#03x}", count, address);
    //    address = chip.run_address(&memo, address);

    //    count += 1;
    //    if count > 50
    //    {
    //        break;
    //    }
    //}

    let sdl_context = sdl2::init().unwrap();
    let video = sdl_context.video().unwrap();
    let window = video.window(&filename, 128, 64)
        //.position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    canvas.set_scale(2.0, 2.0).unwrap();

    let mut chip = Chip8State::init();

    chip.drawop = 0xe000; // clear screen drawing command
    chip.draw(&mut canvas, &memo);

    canvas.present();

    let mut count = 0;
    let dur = Duration::new(0, 20_000_000u32); // 50ms

    while count < 0x0ff {
        ::std::thread::sleep(dur); 
    
        if chip.delay > 0 {
          chip.delay -= 1;
          continue;
        }

        while chip.drawop == 0x0000 {
          print!("{:#03x} addr {:#03x} ", count, chip.pc);
          chip.run_address(&memo);
          count += 1;
        }
        
        chip.draw(&mut canvas, &memo);
        canvas.present();
    }
    Ok(())
}

