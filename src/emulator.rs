use crate::{
    controller::Controller, decoder::Instruction, decoder::ParsedInstruction, display::Display,
    font,
};
use rand::Rng;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::time::{Duration, Instant};

struct Emulator {
    memory: [u8; 4096],
    display: Display,
    program_counter: u16, // most games require only u12, but u16 is used
    index_register: u16,  // most games require only u12, but u16 is used
    stack: Vec<u16>,
    delay_timer: u8,
    sound_timer: u8,
    registers: [u8; 16],
    controller: Controller,
}

impl Emulator {
    fn new(program: Vec<u8>) -> Self {
        let mut memory = [0; 4096];

        memory[font::FONT_OFFSET..font::FONT_OFFSET + font::FONT.len()]
            .copy_from_slice(&font::FONT);
        memory[512..512 + program.len()].copy_from_slice(&program);

        Self {
            memory,
            display: Display::new(),
            program_counter: 512,
            index_register: 0,
            stack: Vec::new(),
            delay_timer: 0,
            sound_timer: 0,
            registers: [0; 16],
            controller: Controller::new(),
        }
    }

    fn perform_fde_cycle(&mut self) {
        // Fetch
        let instruction_msb =
            (*self.memory.get(self.program_counter as usize).unwrap() as u16) << 8;
        let instruction_lsb = *self.memory.get(self.program_counter as usize + 1).unwrap() as u16;
        let raw_instruction = instruction_msb | instruction_lsb;

        // Increment program counter
        self.program_counter += 2;

        // Decode & Execute
        let instruction = ParsedInstruction::parse(raw_instruction);
        self.execute_instruction(instruction);
    }

    fn execute_instruction(&mut self, parsed_instruction: ParsedInstruction) {
        match parsed_instruction.instruction {
            Instruction::Clear => self.display.clear(),
            Instruction::PopStack => {
                self.program_counter = self.stack.pop().expect("No value to pop off the stack")
            }
            Instruction::SetProgramCounter => self.program_counter = parsed_instruction.nnn,
            Instruction::PushStackSetProgramCounter => {
                self.stack.push(self.program_counter);
                self.program_counter = parsed_instruction.nnn;
            }
            Instruction::SkipIfEqualImmediate => {
                if self.registers[parsed_instruction.x] == parsed_instruction.nn {
                    self.program_counter += 2;
                }
            }
            Instruction::SkipIfNotEqualImmediate => {
                if self.registers[parsed_instruction.x] != parsed_instruction.nn {
                    self.program_counter += 2;
                }
            }
            Instruction::SkipIfEqualRegister => {
                if self.registers[parsed_instruction.x] == self.registers[parsed_instruction.y] {
                    self.program_counter += 2;
                }
            }
            Instruction::SetRegister => {
                self.registers[parsed_instruction.x] = parsed_instruction.nn
            }
            Instruction::AddToRegister => {
                self.registers[parsed_instruction.x] =
                    self.registers[parsed_instruction.x].wrapping_add(parsed_instruction.nn)
            }
            Instruction::CopyFromRegisterToRegister => {
                self.registers[parsed_instruction.x] = self.registers[parsed_instruction.y]
            }
            Instruction::LogicalOr => {
                self.registers[parsed_instruction.x] =
                    self.registers[parsed_instruction.x] | self.registers[parsed_instruction.y]
            }
            Instruction::LogicalAnd => {
                self.registers[parsed_instruction.x] =
                    self.registers[parsed_instruction.x] & self.registers[parsed_instruction.y]
            }
            Instruction::LogicalXor => {
                self.registers[parsed_instruction.x] =
                    self.registers[parsed_instruction.x] ^ self.registers[parsed_instruction.y]
            }
            Instruction::Addition => {
                let (result, overflow) = self.registers[parsed_instruction.x]
                    .overflowing_add(self.registers[parsed_instruction.y]);
                self.registers[parsed_instruction.x] = result;
                if overflow {
                    self.registers[0xF] = 1;
                } else {
                    self.registers[0xF] = 0;
                }
            }
            Instruction::Subtraction => {
                let (result, underflow) = self.registers[parsed_instruction.x]
                    .overflowing_sub(self.registers[parsed_instruction.y]);
                self.registers[parsed_instruction.x] = result;
                if underflow {
                    self.registers[0xF] = 0;
                } else {
                    self.registers[0xF] = 1;
                }
            }
            Instruction::RightShift => {
                let (result, overflow) = (
                    self.registers[parsed_instruction.x] >> 1,
                    self.registers[parsed_instruction.x] & 1,
                );
                self.registers[parsed_instruction.x] = result;
                self.registers[0xF] = overflow;
            }
            Instruction::FlippedSubtraction => {
                let (result, underflow) = self.registers[parsed_instruction.y]
                    .overflowing_sub(self.registers[parsed_instruction.x]);
                self.registers[parsed_instruction.x] = result;
                if underflow {
                    self.registers[0xF] = 0;
                } else {
                    self.registers[0xF] = 1;
                }
            }
            Instruction::LeftShift => {
                let (result, overflow) = (
                    self.registers[parsed_instruction.x] << 1,
                    self.registers[parsed_instruction.x] & (1 << 7),
                );
                self.registers[parsed_instruction.x] = result;
                self.registers[0xF] = overflow >> 7;
            }
            Instruction::SkipIfNotEqualRegister => {
                if self.registers[parsed_instruction.x] != self.registers[parsed_instruction.y] {
                    self.program_counter += 2;
                }
            }
            Instruction::SetIndexRegister => self.index_register = parsed_instruction.nnn,
            Instruction::SetProgramCounterOffset => {
                self.program_counter = parsed_instruction.nnn + self.registers[0x0] as u16
            }
            Instruction::RandomNumber => {
                self.registers[parsed_instruction.x] =
                    rand::thread_rng().gen::<u8>() & parsed_instruction.nn
            }
            Instruction::Draw => self.execute_draw_instruction(&parsed_instruction),
            Instruction::KeyDown => {
                if self
                    .controller
                    .is_key_pressed(self.registers[parsed_instruction.x])
                {
                    self.program_counter += 2
                }
            }
            Instruction::KeyNotDown => {
                if !self
                    .controller
                    .is_key_pressed(self.registers[parsed_instruction.x])
                {
                    self.program_counter += 2
                }
            }
            Instruction::CopyDelayTimer => self.registers[parsed_instruction.x] = self.delay_timer,
            Instruction::SetDelayTimer => self.delay_timer = self.registers[parsed_instruction.x],
            Instruction::SetSoundTimer => self.sound_timer = self.registers[parsed_instruction.x],
            Instruction::AddToIndexRegister => {
                let (result, overflow) = self
                    .index_register
                    .overflowing_add(self.registers[parsed_instruction.x].into());
                if overflow || result > 0x0FFF {
                    self.registers[0xF] = 1;
                }

                self.index_register = result % 0x0FFF;
            }
            Instruction::WaitForKeyPress => {
                if let Some(key) = self.controller.last_pressed {
                    self.registers[parsed_instruction.x] = key;
                } else {
                    self.program_counter -= 2;
                }
            }
            Instruction::SetIndexRegisterToFontCharacter => {
                self.index_register = (font::FONT_OFFSET as u8
                    + (self.registers[parsed_instruction.x] & 0x0F))
                    .into();
            }
            Instruction::ConvertToDecimal => {
                let mut x_register = self.registers[parsed_instruction.x];
                for i in (0..=2).rev() {
                    self.memory[(self.index_register + i) as usize] = x_register % 10;
                    x_register /= 10;
                }
            }
            Instruction::WriteToMemory => {
                for i in 0..=parsed_instruction.x {
                    self.memory[(self.index_register + i as u16) as usize] = self.registers[i];
                }
            }
            Instruction::ReadFromMemory => {
                for i in 0..=parsed_instruction.x {
                    self.registers[i] = self.memory[(self.index_register + i as u16) as usize];
                }
            }
        }
    }

    fn execute_draw_instruction(&mut self, parsed_instruction: &ParsedInstruction) {
        let x_pos = self.registers[parsed_instruction.x] % 64;
        let y_pos = self.registers[parsed_instruction.y] % 32;

        let start = self.index_register as usize;
        let end = start + parsed_instruction.n as usize;
        let bytes = if let Some(slice) = self.memory.get(start..end) {
            slice.to_vec()
        } else {
            panic!(
                "Bad draw instruction (memory not found) {}",
                parsed_instruction.raw_instruction
            );
        };

        self.registers[0xF] = 0;

        for (pos, &byte) in bytes.iter().enumerate() {
            let draw_y_pos = (y_pos + pos as u8) as usize;
            if draw_y_pos >= 32 {
                break;
            }

            for i in 0..8 {
                if (byte >> (7 - i)) & 0x01 == 0 {
                    continue;
                }

                let draw_x_pos = (x_pos + i) as usize;

                if draw_x_pos >= 64 {
                    break;
                }

                if self.display.buffer[draw_y_pos][draw_x_pos] {
                    self.registers[0xF] = 1;
                }

                self.display.buffer[draw_y_pos][draw_x_pos] ^= true;
                self.display.draw = true;
            }
        }
    }
}

pub fn emulate(program: Vec<u8>) {
    let mut emulator = Emulator::new(program);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let scale_factor = (20, 20);
    let scale_factor_32 = (scale_factor.0 as u32, scale_factor.1 as u32);

    let width: u16 = 64 * scale_factor.0;
    let height: u16 = 32 * scale_factor.1;

    let window = video_subsystem
        .window("CHIP-8 Emulator", width as u32, height as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    canvas.set_draw_color(Color::BLACK);
    canvas.clear();
    canvas.present();

    let mut last_timer_update = Instant::now();
    let mut last_instruction_time = Instant::now();

    'running: loop {
        let elapsed = last_timer_update.elapsed();
        if elapsed >= Duration::from_millis(16) {
            if emulator.delay_timer > 0 {
                emulator.delay_timer -= 1;
            }

            if emulator.sound_timer > 0 {
                emulator.sound_timer -= 1;
            }
            last_timer_update = Instant::now();
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(key), ..
                } => emulator.controller.press_key(key),
                Event::KeyUp {
                    keycode: Some(key), ..
                } => emulator.controller.release_key(key),
                _ => {}
            }
        }

        // Check if it's time to execute the next instruction
        if last_instruction_time.elapsed() >= Duration::from_millis(1) {
            emulator.perform_fde_cycle();

            // Rerender if necessary
            if emulator.display.draw {
                canvas.set_draw_color(Color::BLUE);
                canvas.clear();
                canvas.set_draw_color(Color::YELLOW);

                emulator
                    .display
                    .buffer
                    .iter()
                    .enumerate()
                    .for_each(|(col_num, col)| {
                        col.iter().enumerate().for_each(|(row_num, &val)| {
                            if val {
                                let row_num = row_num as i32;
                                let col_num = col_num as i32;

                                let rect = Rect::new(
                                    row_num * scale_factor.0 as i32,
                                    col_num * scale_factor.1 as i32,
                                    scale_factor_32.0,
                                    scale_factor_32.1,
                                );

                                canvas.fill_rect(rect).unwrap();
                            }
                        });
                    });

                // Update the canvas
                canvas.present();
                emulator.display.draw = false;
            }

            last_instruction_time = Instant::now();
        }
    }
}
