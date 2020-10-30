use std::io;
use std::io::prelude::*;
use std::fs::File;

//struct Chip8State {
//    n: usize,    // main address regiter ("pointer"). memsize in chip-8 are 16 bit, but we use usize
//    i: usize, // additional 16-bit address register
//    v: [u8; 16] // variables v0 -- vF
//}
//
//impl Chip {
//    fn run_opcode(opcode: u16) {
//
//    }
//}

fn main() -> io::Result<()> {
    println!("Hello, world!");

    let mut file = File::open("roms/tetris.rom")?;

    let mut memo: [u8; 4096] = [0; 4096];
    
    //let bu = file.read(&mut memo)?; // maybe we need an offset here!, 0x200?
    let bu = file.read(&mut memo[0x200..])?; // maybe we need an offset here!, 0x200?

    println!("{}\n", bu);

    let mut n: usize = 0x200;

    let mut i: usize = 0; // additional 16-bit address register
    let mut v: [u8; 16] = [0; 16]; // variables v0 -- vF

    let mut count = 0;

    loop
    {
        let opcode = u16::from_be_bytes([memo[n],memo[n+1]]);

        //println!( "{}: {x:03} {:#04x}, next opcode {opc:#06x}", n, x=memo[n], opc=opcode);     

        println!{"n {0:#05x}:", n}
        let nold = n;

        match opcode & 0xf000 {
            0x0000 => match opcode {
                         0x00e0 => println!("{:#06x} clear screen", opcode),
                         0x00ee => println!("{:#06x} return", opcode),
                         _ => println!("{:#06x} call machine routine {:#05x}", opcode, opcode & 0x0fff),
                      },
            0x1000 => { n = usize::from(opcode & 0x0fff); println!("{:#06x} goto n={:#05x}", opcode, n); },
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
            0xa000 => { i = usize::from(opcode & 0x0fff); println!("{:#06x} i = {:#05x}", opcode, i); },
            0xf000 => match opcode & 0xf0ff {
                         0xf01e => {
                           let varnum = usize::from((opcode & 0x0f00)>> 2);
                           i += usize::from(v[varnum]);
                           println!("{:#06x} i += v{:#03x} variable assignment", opcode, varnum)
                         },
                         _ => println!("{:#06x} not implemented", opcode),
                      },

            _ => println!("{:#06x} not implemented", opcode),
        }

        //match memo[n] {
        //    0x25 => println!("this is hex code 0x25"),
        //    _    => (),
        //}

        if n==nold {n += 2;}

        count += 1;
        if count > 20
        {
            break;
        }
    }
    //for element in memo.iter() {
    //    println!( "{}", element);
    //}
    v[2] = 7;
    Ok(())
}

