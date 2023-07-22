use std::fmt;

#[derive(PartialEq,Debug,Clone)]
pub enum Varset {
    V(u8),
    Keyboard,
    DelayTimer,
    SoundTimer
}

impl fmt::Display for Varset {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Varset::V(vnum) => write!(f, "V{:1X}", vnum),
      _ => write!(f, "{:?}", self)
    }
  }
}

#[derive(Debug)]
pub enum Operation {
    Set,
    IncrementNoCarry,
    IncrementWithCarry,
    //DecrementNoBorrow,
    DecrementWithBorrow,
    Randomize,
    BitOr,
    BitAnd,
    BitXor,
    BitshiftAndStore,
    DecrementAndFlip,
    SpriteMultiply
}

pub enum Instruction {
    RCARoutine(u16),

    ClearDraw,
    DrawSpriteXYH(Varset,Varset,u8), //ClearDraw = DrawXYH(0,0,0)

    GotoAdress(u16),
    RunSubroutineAtAdress(u16),
    ReturnFromSubroutine,

    SkipNextIfVarsEq(Varset, Varset),
    SkipNextIfVarEq(Varset, u8),
    SkipNextIfVarsNeq(Varset, Varset),
    SkipNextIfVarNeq(Varset, u8),

    VariableOnValue(Varset, u8, Operation),
    VariableOnVariable(Varset, Varset, Operation),
    SetITo(u16),
    IOnVariable(Varset, Operation),
    StoreVarAsDecimalInPositionI(Varset),
    DumpVariablesUptoInPositionI(Varset),
    LoadVariablesUptoFromPositionI(Varset),
}

fn get_0x00(opcode: u16) -> Varset {
    Varset::V( ((opcode & 0x0f00) >> 8) as u8 )
}

fn get_00y0(opcode: u16) -> Varset {
    Varset::V( ((opcode & 0x00f0) >> 4) as u8 )
}

fn get_000n(opcode: u16) -> u8 {
    (opcode & 0x000f) as u8
}

fn get_00nn(opcode: u16) -> u8 {
    (opcode & 0x00ff) as u8
}

fn get_0nnn(opcode: u16) -> u16 {
    opcode & 0x0fff
}

impl fmt::Display for Instruction {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Instruction::RCARoutine(routine) => write!(f, "RCA routine {:#05x}", routine),
      Instruction::ClearDraw => write!(f, "clear display"),
      Instruction::ReturnFromSubroutine => write!(f, "return from subroutine"),
      Instruction::GotoAdress(address) => write!(f, "goto address {:#05x}", address),
      Instruction::RunSubroutineAtAdress(address) => write!(f, "run subroutine at {:#05x}", address),
      Instruction::SkipNextIfVarEq(var, val) => write!(f, "if ({} == {:#04x}) skip next", var, val),
      Instruction::SkipNextIfVarNeq(var, val) => write!(f, "if ({} != {:#04x}) skip next", var, val),
      Instruction::VariableOnValue(var, val, op) => match op {
          Operation::Set => write!(f, "set {} to {:#04x}", var, val),
          Operation::IncrementNoCarry => write!(f, "increment {} by {:#04x} ignoring carry", var, val),
          Operation::Randomize => write!(f, "randomize {} using {:#04x}", var, val),
          _ => panic!("display of operation not supported")
      },
      Instruction::SkipNextIfVarsEq(varx, vary) => write!(f, "if ({} == {}) skip next", varx, vary),
      Instruction::SkipNextIfVarsNeq(varx, vary) => write!(f, "if ({} != {}) skip next", varx, vary),
      Instruction::VariableOnVariable(varx, vary, op) => match op {
          Operation::Set => write!(f, "set {} to {}", varx, vary),
          Operation::IncrementNoCarry => write!(f, "increment {} by {} ignoring carry", varx, vary),
          Operation::IncrementWithCarry => write!(f, "increment {} by {} using carry", varx, vary),
          Operation::DecrementAndFlip => write!(f, "decrement {} by {} ignoring carry, then flip its sign", varx, vary),
          Operation::DecrementWithBorrow => write!(f, "decrement {} by {} using borrow", varx, vary),
          Operation::Randomize => write!(f, "randomize {} using {}", varx, vary),
          Operation::BitOr => write!(f, "bitwise Or on {} using {} as second input", varx, vary),
          Operation::BitXor => write!(f, "bitwise XOr on {} using {} as second input", varx, vary),
          Operation::BitAnd => write!(f, "bitwise And on {} using {} as second input", varx, vary),
          Operation::BitshiftAndStore => write!(f, "bitshift {} and store carry on {}", varx, vary),
          _ => panic!("display of operation not supported")
      },
      Instruction::DrawSpriteXYH(varx,vary, h) => write!(f, "draw sprite at {},{} with height {}", varx, vary, h),
      Instruction::SetITo(num) => write!(f,"set I to {}", num),
      Instruction::IOnVariable(vs, op) => match op {
          Operation::SpriteMultiply => write!(f,"set I to address for sprite {}", vs),
          Operation::IncrementNoCarry => write!(f, "increment I by {}", vs),
          _ => panic!("display of operation not supported")
      },
      Instruction::StoreVarAsDecimalInPositionI(vs) => write!(f, "store {} at decimal starting from memory position I", vs),
      Instruction::DumpVariablesUptoInPositionI(vs) => write!(f, "dump {}--{} to memory position I", Varset::V(0), vs),
      Instruction::LoadVariablesUptoFromPositionI(vs ) => write!(f, "load {}--{} from memory position I", Varset::V(0), vs)
    }
  }
}

pub fn from_opcode(opcode: u16) -> Instruction {
  match opcode {
    0x00e0 => Instruction::ClearDraw,
    0x00ee => Instruction::ReturnFromSubroutine,
    _ => match opcode & 0xf000 {
      0x0000 => Instruction::RCARoutine(get_0nnn(opcode)), // this is usually just empty space in the ROM file, 0 maps to idle
      0x1000 => Instruction::GotoAdress(get_0nnn(opcode)),
      0x2000 => Instruction::RunSubroutineAtAdress(get_0nnn(opcode)),
      0x3000 => Instruction::SkipNextIfVarEq(get_0x00(opcode), get_00nn(opcode)),
      0x4000 => Instruction::SkipNextIfVarNeq(get_0x00(opcode), get_00nn(opcode)),
      0x6000 => Instruction::VariableOnValue(get_0x00(opcode), get_00nn(opcode), Operation::Set),
      0x7000 => Instruction::VariableOnValue(get_0x00(opcode), get_00nn(opcode), Operation::IncrementNoCarry),
      0x8000 => Instruction::VariableOnVariable(get_0x00(opcode), get_00y0(opcode), match opcode & 0x000f {
           0 => Operation::Set,
           1 => Operation::BitOr,
           2 => Operation::BitAnd,
           3 => Operation::BitXor,
           4 => Operation::IncrementWithCarry,
           5 => Operation::DecrementWithBorrow,
           6 => Operation::BitshiftAndStore,
           7 => Operation::DecrementAndFlip,
           _ => panic!("var-on-var opcode {:#06x} not implemented", opcode)
           }),
      0x9000 => Instruction::SkipNextIfVarsNeq(get_0x00(opcode), get_00y0(opcode)),
      0xa000 => Instruction::SetITo(get_0nnn(opcode)),
      0xc000 => Instruction::VariableOnValue(get_0x00(opcode), get_00nn(opcode), Operation::Randomize),
      0xd000 => Instruction::DrawSpriteXYH(get_0x00(opcode), get_00y0(opcode), get_000n(opcode)),
      0xe000 => match opcode & 0x00ff {
        0x9e => Instruction::SkipNextIfVarsEq(get_0x00(opcode), Varset::Keyboard),
        0xa1 => Instruction::SkipNextIfVarsNeq(get_0x00(opcode), Varset::Keyboard),
           _ => panic!("keyboard detector opcode {:#06x} not implemented", opcode)
           }
      0xf000 => match opcode & 0x00ff {
        0x07 => Instruction::VariableOnVariable(get_0x00(opcode), Varset::DelayTimer, Operation::Set),
        0x15 => Instruction::VariableOnVariable(Varset::DelayTimer, get_0x00(opcode), Operation::Set),
        0x18 => Instruction::VariableOnVariable(Varset::SoundTimer, get_0x00(opcode), Operation::Set),
        0x1e => {let vx = get_0x00(opcode); if vx == Varset::V(0xf) { panic!("vf not allowed here") }; Instruction::IOnVariable(get_0x00(opcode), Operation::IncrementNoCarry) },
        0x29 => Instruction::IOnVariable(get_0x00(opcode), Operation::SpriteMultiply),
        0x33 => Instruction::StoreVarAsDecimalInPositionI(get_0x00(opcode)),
        0x55 => Instruction::DumpVariablesUptoInPositionI(get_0x00(opcode)),
        0x65 => Instruction::LoadVariablesUptoFromPositionI(get_0x00(opcode)),
           _ => panic!("special-function opcode {:#06x} not implemented", opcode)
           }
      _ => panic!("opcode {:#06x} not implemented", opcode)
    }
  }
}
