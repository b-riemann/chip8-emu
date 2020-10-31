use std::io;
use std::io::prelude::*;
use std::fs::File;

// graphics part
use pixel_canvas::{Canvas, Color, input::MouseState, image::Image};

fn block_draw(im: &mut Image, x0: u8, y0: u8, w: u8, h: u8) {
    let wd = im.width() as usize;
    let ys = y0 as usize;
    let xs = x0 as usize;
    for (y, row) in im.chunks_mut(wd).enumerate() {
        if y >= ys && y < ys+(h as usize) {
            for (x, pxl) in row.iter_mut().enumerate() {
                if x >= xs && x < xs+(w as usize) {
                    *pxl = Color { r: 255, g: 255, b: 255 };
                }
            }
        }
    }
}
//

struct Chip8State {
    // ToDo: change usize to u16 for n and i
    i: u16, // additional 16-bit address register
    v: [u8; 16] // variables v0 -- vF
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
        Chip8State { i: 0, v: [0; 16] }
    }

    fn run_address(&mut self, memo: &[u8; 4096], address: u16, im: &mut Image) -> u16 {
        let uaddr = usize::from(address);
        let opcode = get_opcode(memo[uaddr], memo[uaddr+1]);

        match opcode & 0xf000 {
            0x0000 => match opcode {
                         0x00e0 => println!("{:#06x} clear screen", opcode),
                         0x00ee => println!("{:#06x} return", opcode),
                         _ => panic!("{:#06x} call RCA 1802 routine {:#05x}: not implemented", opcode, opcode & 0x0fff),
                      },
            0x1000 => { 
                         let next_addr = opcode & 0x0fff;
                         println!("{:#06x} goto address {:#05x}", opcode, next_addr);
                         return next_addr;
                      },
            0x2000 => { 
                         let nother = opcode & 0x0fff;
                         println!("{:#06x} run subroutine at {:#05x}", opcode, nother);
                         if self.run_address(memo, nother, im) != nother+2 {
                             panic!("no recursion support.");
                         }
                      },
            0x3000 => { 
                         let varnum = get_0x00(opcode);
                         let num = get_00nn(opcode);
                         println!("{:#06x} if (v{:#03x} == {:#x}) skip next", opcode, varnum, num);
                         if self.v[varnum] == num { 
                             return address+4;
                         }
                      },
            0x4000 => { 
                         let varnum = get_0x00(opcode);
                         let num = get_00nn(opcode);
                         println!("{:#06x} if (v{:#03x} != {:#x}) skip next", opcode, varnum, num);
                         if self.v[varnum] != num { 
                             return address+4;
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
                         println!("{:#06x} v{:#03x} += {:#04x} (now {:#04x})", opcode, varnum, num, self.v[varnum]);
                      },
            0x8000 => match opcode & 0xf00f {
                         0x8000 => println!("{:#06x} variable assignment", opcode),
                         0x8001 => println!("{:#06x} bitwise |", opcode),
                         0x8002 => println!("{:#06x} bitwise &", opcode),
                         0x8003 => println!("{:#06x} bitwise ^(xor)", opcode),
                         0x8004 => println!("{:#06x} +=", opcode),
                         0x8005 => println!("{:#06x} -=", opcode),
                         0x8006 => println!("{:#06x} Vx>>=1", opcode),
                         0x8007 => println!("{:#06x} Vx=Vy-Vx", opcode),
                         0x800e => println!("{:#06x} Vx<<=1", opcode),
                         _ => panic!("{:06x} unknown opcode!", opcode),
                      },
            0xa000 => { self.i = opcode & 0x0fff; println!("{:#06x} i = {:#05x}", opcode, self.i); },
            0xc000 => { 
                        let varnum = get_0x00(opcode);
                        let num = get_00nn(opcode);
                        self.v[varnum] = rand::random::<u8>() & num;
                        println!("{:#06x} v{:#03x} = rand() & {:#04x} (now {:#04x})", opcode, varnum, num, self.v[varnum]);
                      },
            0xd000 => { 
                        let varxnum = get_0x00(opcode);
                        let varynum = get_00y0(opcode);
                        let height = (opcode & 0x000f) as u8;
                        block_draw( im,  self.v[varxnum], self.v[varynum], 8, height);
                        println!("{:#06x} DRAW", opcode); },
            0xf000 => match opcode & 0xf0ff {
                         0xf01e => {
                           let varnum = get_0x00(opcode);
                           self.i += u16::from(self.v[varnum]);
                           println!("{:#06x} i += v{:#03x}", opcode, varnum)
                         },
                         _ => println!("{:#06x} not implemented", opcode),
                      },
            _ => panic!("{:#06x} not implemented", opcode),
        }

        return address+2
    }


}

fn main() -> io::Result<()> {
    let filename = "roms/tetris.rom";

    let mut file = File::open(filename)?;

    let mut memo: [u8; 4096] = [0; 4096];
    
    //let bu = file.read(&mut memo)?; // maybe we need an offset here!, 0x200?
    file.read(&mut memo[0x200..])?; // maybe we need an offset here!, 0x200?

    let mut chip = Chip8State::init();
    let mut address: u16 = 0x200;

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

    let canvas = Canvas::new(512, 512)
        .title(filename)
        .show_ms(true)
        .state(MouseState::new());
        //.input(MouseState::handle_input);

    let mut c = 0;

    canvas.render ( move |mouse, image| {
      //if c % 2 == 0 {
        print!("  address {:#03x} {}", address, mouse.x);
        address = chip.run_address(&memo, address, image);
      //}
      if c % 256 == 0 {
        println!("processed {} instructions", c);
      }
      c += 1;
    } );


    Ok(())
}

