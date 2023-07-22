use std::env;
use std::io;
use std::io::Write;
use std::fs::OpenOptions;

mod instruction;
use instruction::from_opcode;

mod cartridge;
use cartridge::Cartridge;

mod state;
use state::Chip8State;

fn main() -> Result<(), io::Error> {
  let mode = "run";

  let filename = env::args().nth(1).expect("insert cartridge (.rom file)"); 

  let cartridge = Cartridge::new(filename);

  let mut outfile = OpenOptions::new().write(true).truncate(true).open(format!("{}.txt", mode)).unwrap();

  writeln!(outfile, "-----| {} |------", mode.to_ascii_uppercase()).unwrap();
  match mode {
    "listing" => {
      for addr in (cartridge.start()..cartridge.len()).step_by(2) {
        let opcode = cartridge.get_opcode_from(addr).unwrap();
        writeln!(outfile, "{:#06x}  {:#06x}  {}", addr, opcode, from_opcode(opcode)).unwrap();
      }
    }
    _ => {
      let mut cas = Chip8State::new(cartridge);
      for cycle in 0..5000 {
        let opcode = cas.cartridge.get_opcode_from(cas.pc).unwrap();
        let instr = from_opcode( opcode );

        if opcode & 0xf000 == 0xd000 {
          write!(outfile, "{}", cas.display).unwrap();
        } else {
          writeln!(outfile, "{0:4}  {1:#06x}  {2:#06x}  {3:♥<4$}{5}", cycle, cas.pc, opcode, "", cas.stack.len(), from_opcode(opcode)).unwrap();
        }

        cas.run_instruction(instr);
        if cycle % 4 == 0 {
          cas.register.tick()
        }
      }
    }
  }

  Ok(())
}
