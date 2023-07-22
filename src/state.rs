use rand::Rng;
use rand::rngs::ThreadRng;

use crate::instruction::{Varset, Instruction, Operation};
use crate::cartridge::Cartridge;
use std::fmt;


pub struct TermDisplay {
  display: [bool; 64*32]
}

impl TermDisplay {
  const WIDTH_PX : u8 = 64;
  const HEIGHT_PX : u8 = 32;
}

impl TermDisplay {
  fn new() -> Self {
    Self { display: [false; 64*32] }
  }

  fn clear(&mut self) {
    self.display.fill( false );
  }

  fn get_idx(x: u8, y: u8) -> usize {
    let y_rollaround = (y & 0x1f) as usize; // mod 32 
    let x_rollaround = (x & 0x3f) as usize; // mod 64

    y_rollaround*(Self::WIDTH_PX as usize) + x_rollaround
  }

  fn get_pixel(&self, x: u8, y: u8) -> bool {
    let idx = Self::get_idx(x,y);
    self.display[idx]   
  }

  fn get_character(&self, x: u8, y: u8) -> &str {
    if self.get_pixel(x, y) { "██" } else { "  " }
  }

  fn set_pixel(&mut self, x: u8, y: u8, tf: bool) {
    let idx = Self::get_idx(x,y);
    self.display[idx] = tf
  }
}

impl fmt::Display for TermDisplay {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    for y in 0..Self::HEIGHT_PX {
      for x in 0..Self::WIDTH_PX {
        write!(f, "{}", self.get_character(x, y)).unwrap();
      }
      write!(f, "\n").unwrap();
    }
    write!(f, "\n")
  }

}


pub struct Register {
    v: [u8; 16],  // variables v0 -- vF
    delay: u8,    // delay timer
    sound: u8,
    keyboard: u8, // hex keyboard
}

impl Register {
  fn new() -> Self {
    Self { v: [0; 16], keyboard: 0, delay: 0, sound: 0 }
  }

  pub fn get(&self, vs: Varset) -> u8 {
    match vs {
      Varset::V(vnum) => self.v[vnum as usize],
      Varset::Keyboard => self.keyboard,
      Varset::DelayTimer => self.delay,
      Varset::SoundTimer => self.sound
    }
  }

  pub fn tick(&mut self) {
    if self.delay > 0 { self.delay -= 1 }
  }

  fn vars_are_equal(&self, va: Varset, vb: Varset) -> bool {
    self.get(va) == self.get(vb)
  }

  fn var_equals_val(&self, vs: Varset, val: u8) -> bool {
    self.get(vs) == val
  }

  fn set(&mut self, vs: Varset, val: u8) {
    match vs {
      Varset::V(vnum) => self.v[vnum as usize] = val,
      Varset::Keyboard => self.keyboard = val,
      Varset::DelayTimer => self.delay = val,
      Varset::SoundTimer => self.sound = val
    }
  }

  fn inc_nocarry(&mut self, vs: Varset, val: u8) {
    let new_val = self.get(vs.clone()).wrapping_add(val);
    self.set(vs, new_val)
  }

  fn set_to_var(&mut self, vs: Varset, vi: Varset) {
    self.set(vs, self.get(vi));
  }

  fn decrement_and_flip(&mut self, vs: Varset, vi: Varset) {
    let x = self.get(vs.clone());
    let y = self.get(vi);
    self.set(vs, y-x)
  }

  fn decrement_with_borrow(&mut self, vs: Varset, vi: Varset) {
    let val = self.get(vs.clone()) as u16 - self.get(vi) as u16;
    let borrow = (val & 0xff00) > 0;
    self.set(Varset::V(0xf),  if borrow { 1 } else { 0 });
    self.set(vs, (val & 0x00ff) as u8)
  }

  fn bitshift_and_store(&mut self, vs: Varset) {
    let val = self.get(vs.clone());
    self.set(Varset::V(0xf), val & 0x01);
    self.set(vs, val >> 1)
  }

}

pub struct Chip8State {
  pub pc: u16,      // main address register (program counter)
  i: u16,       // additional 16-bit address register
  pub stack: Vec<u16>, // stores registers when (possibly multiple enclosed) subroutines are called
  pub register: Register,
  pub display: TermDisplay, // bits of the 32x64 display. the u8s are xor'ed with sprites and thus form a part of the state
  pub cartridge: Cartridge,
  rng: ThreadRng,  // custom: random number generator
}


impl Chip8State {
  pub fn new(cartridge: Cartridge) -> Self {
      Self { pc: 0x200, i: 0, stack: Vec::with_capacity(32),
        register: Register::new(), display: TermDisplay::new(),
        cartridge, rng: rand::thread_rng() }
  }

  pub fn run_instruction(&mut self, instruction: Instruction) {
    self.pc += 2;
    match instruction {
      Instruction::RCARoutine(_r) => panic!("RCA routines are not implemented"),
      
      Instruction::GotoAdress(address) => self.pc = address,
      Instruction::RunSubroutineAtAdress(address) => {self.stack.push(self.pc); self.pc = address},
      Instruction::ReturnFromSubroutine => self.pc = self.stack.pop().unwrap(),
      
      Instruction::SkipNextIfVarEq(vs, val) => if self.register.var_equals_val(vs, val) { self.pc += 2 },
      Instruction::SkipNextIfVarNeq(vs, val) => if !self.register.var_equals_val(vs, val) { self.pc += 2 },
      Instruction::SkipNextIfVarsEq(va, vb) => if self.register.vars_are_equal(va, vb) { self.pc += 2 },
      Instruction::SkipNextIfVarsNeq(va, vb) => if !self.register.vars_are_equal(va, vb) { self.pc += 2 },

      Instruction::VariableOnValue(vs, val, op) => match op {
        Operation::Set => self.register.set(vs, val),
        Operation::IncrementNoCarry => self.register.inc_nocarry(vs, val),
        Operation::Randomize => {
          let random_number: u8 = self.rng.gen();
          self.register.set(vs, random_number & val)
        },
        _ => panic!("operation not implemented")
      },
      Instruction::VariableOnVariable(vs, vi, op) => match op {
        Operation::Set => self.register.set_to_var(vs, vi),
        Operation::IncrementNoCarry => self.register.inc_nocarry(vs, self.register.get(vi)),
        Operation::DecrementAndFlip => self.register.decrement_and_flip(vs, vi),
        Operation::DecrementWithBorrow => self.register.decrement_with_borrow(vs, vi),
        Operation::BitOr => self.register.set(vs.clone(), self.register.get(vs) | self.register.get(vi)),
        Operation::BitAnd => self.register.set(vs.clone(), self.register.get(vs) & self.register.get(vi)),
        Operation::BitXor => self.register.set(vs.clone(), self.register.get(vs) ^ self.register.get(vi)),
        Operation::BitshiftAndStore => self.register.bitshift_and_store(vs),
        _ => panic!("{:?} not implemented", op)
      },


      Instruction::SetITo(num) => self.i = num,
      Instruction::IOnVariable(vs, op) => match op {
        Operation::Set => self.i = self.register.get(vs) as u16,
        Operation::IncrementNoCarry => self.i += self.register.get(vs) as u16,
        Operation::SpriteMultiply => self.i = 5*(self.register.get(vs) as u16),
        _ => panic!("{:?} not implemented", op)
      },
      Instruction::StoreVarAsDecimalInPositionI(vs) => {
        let val = self.register.get(vs);
        let hun = val / 100;
        self.cartridge.set_memory(self.i, hun);
        let dec = val / 10 - hun*10;
        self.cartridge.set_memory(self.i+1, dec);
        let uno = val - dec*10 - hun*100;
        self.cartridge.set_memory(self.i+2, uno);
      },
      Instruction::DumpVariablesUptoInPositionI(vs) => {
        match vs {
          Varset::V(vmax) => {
            for vnum in 0..=vmax {
              self.cartridge.set_memory(self.i + vnum as u16, self.register.get(Varset::V(vnum)))
            }
          },
          _ => panic!("dump requires a V* variable as input")
        }
      },
      Instruction::LoadVariablesUptoFromPositionI(vs) => {
        match vs {
          Varset::V(vmax) => {
            for vnum in 0..=vmax {
              self.register.set(Varset::V(vnum), self.cartridge.get_memory(self.i + vnum as u16))
            }
          },
          _ => panic!("dump requires a V* variable as input")
        }
      },

      Instruction::ClearDraw => self.display.clear(),
      Instruction::DrawSpriteXYH(vx, vy, h) => {
        let x0 = self.register.get(vx) + 7;
        let y0 = self.register.get(vy);
        
        let mut any_flipped_off = false;
        for p in 0..h {
          let bitti = self.cartridge.get_memory(self.i + p as u16);
          let y = y0+p;

          for q in 0..8 {
            let x = x0-q;

            let flip = (bitti >> q) & 0x01 == 0x01;

            if flip {
              let oldstate = self.display.get_pixel(x,y);
              self.display.set_pixel(x,y, !oldstate);
              if oldstate {
                  any_flipped_off = true;
              }
            }
          }
        }
        self.register.set(Varset::V(0xf), if any_flipped_off { 1 } else { 0 });
      }
    }
  }
}


