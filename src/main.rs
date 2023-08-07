use std::env;
use std::io;
use std::io::Write;
use std::fs::OpenOptions;
use std::time::Instant;

mod instruction;
use crossterm::QueueableCommand;
use crossterm::cursor::MoveTo;
use crossterm::style;
use crossterm::terminal::Clear;
use crossterm::terminal::ClearType;
use crossterm::Result;
use crossterm::terminal::SetSize;
use instruction::from_opcode;

mod cartridge;
use cartridge::Cartridge;

mod state;
use state::Chip8State;

use crossterm::terminal::enable_raw_mode;
use crossterm::execute;
use std::time::Duration;
use crossterm::event::{poll,read,Event,KeyEvent,KeyEventKind,KeyCode};  

// Unfortunately, in crossterm for Windows API, key releases cannot be detected
fn check_keypress(mut keyboard: u8, timeout: std::time::Duration) -> u8 {
  if poll(timeout).unwrap() {
    match read().unwrap() {
        Event::Key(KeyEvent { code: KeyCode::Char('w'), kind: KeyEventKind::Press, .. }) => keyboard = 0x04,
        Event::Key(KeyEvent { code: KeyCode::Char('a'), kind: KeyEventKind::Press, .. }) => keyboard = 0x05,
        Event::Key(KeyEvent { code: KeyCode::Char('d'), kind: KeyEventKind::Press, .. }) => keyboard = 0x06,
        Event::Key(KeyEvent { code: KeyCode::Char('s'), kind: KeyEventKind::Press, .. }) => keyboard = 0x07,
        _ => (),
    }
  }
  keyboard
}


fn main() -> Result<()> {
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
    "keyboard-test" => {
      enable_raw_mode().expect("can run in raw mode");
      loop {
        if poll(Duration::from_millis(500)).unwrap() {
          let event = read().unwrap();
          println!("{:?}", event);
          match event {
            Event::Key(KeyEvent { code: KeyCode::Char('q'), kind: KeyEventKind::Press, .. }) => break,
            _ => {}
          }
        }
      }
    },
    _ => {
      enable_raw_mode().expect("can run in raw mode");
      let mut stdout = io::stdout();
      // let backend = CrosstermBackend::new(stdout);
      // let mut terminal: Terminal<CrosstermBackend<io::Stdout>> = Terminal::new(backend).expect("terminal created");

      execute!(
        stdout,
        SetSize(64*2+2+5, 32+2+5),
        Clear(ClearType::All)
      ).unwrap();


      let min_cpu_cycle = Duration::from_micros(10); // 1 MHz..
      let monitor_cycle = Duration::from_millis(50); //20Hz for now

      let mut cas = Chip8State::new(cartridge);

      let mut monitor_now = Instant::now();
      let runtime_now = Instant::now();

      //let mut cycle: u32 = 0;
      
      while runtime_now.elapsed() < Duration::from_secs(300) {

        cas.register.keyboard = check_keypress(cas.register.keyboard, min_cpu_cycle);

        let opcode = cas.cartridge.get_opcode_from(cas.pc).unwrap();
        let instruction = from_opcode( opcode );

        if monitor_now.elapsed() > monitor_cycle { //changes_display(instruction.clone()) {
          //execute!(stdout, ScrollDown(32+2)).unwrap();
          //writeln!(stdout, "{6}{0:4}  {1:#06x}  {2:#06x}  {3:♥<4$}{5}", cycle, cas.pc, opcode, "", cas.stack.len(), instruction, cas.display).unwrap();
          stdout.queue(MoveTo(0,0))?.queue(style::Print(format!("{}",cas.display)))?;

          cas.register.keyboard = 0;
          monitor_now = Instant::now();
          cas.register.tick()
        } //else {
          //execute!(stdout, ScrollDown(1)).unwrap();
          //writeln!(stdout, "{0:4}  {1:#06x}  {2:#06x}  {3:♥<4$}{5}", cycle, cas.pc, opcode, "", cas.stack.len(), instruction).unwrap();
        //}

        cas.run_instruction(instruction);

        //cycle += 1;
        std::thread::sleep(min_cpu_cycle)
      }
    }
  }

  Ok(())
}
