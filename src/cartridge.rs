
use std::io::{prelude::*, Error, ErrorKind};
use std::fs::File;

pub struct Cartridge {
    pub memory: [u8; 0xf00],
    fin: u16
}

impl Cartridge {
    const CARTRIDGE_START: u16 = 0x0200;

    pub fn new(filename: String) -> Self {
        let mut file = File::open(&filename).expect("file opened");

        let mut x = Self { memory: [0; 0xf00], fin: 0 };
        let mut fonts = File::open("fonts").expect("font file opened");
        fonts.read(&mut x.memory).expect("font file read");
        x.fin = Self::CARTRIDGE_START + file.read(&mut x.memory[0x200..]).expect("file read") as u16;
        x
    }

    pub fn start(&self) -> u16 {
        Self::CARTRIDGE_START
    }

    pub fn len(&self) -> u16 {
      self.fin
    }

    pub fn set_memory(&mut self, address: u16, val: u8) {
        self.memory[address as usize] = val
    }

    pub fn get_memory(&mut self, address: u16) -> u8 {
        self.memory[address as usize]
    }

    pub fn get_opcode_from(&self, address: u16) -> Result<u16, Error> {
        if address > self.len() {
            Err(Error::new(ErrorKind::UnexpectedEof, "end of file"))
        } else {
            let au = address as usize;
            Ok( u16::from_be_bytes([self.memory[au], self.memory[au+1]]) )
        }
    }
}