use crate::{display::Display, font};
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
}

#[derive(Debug)]
struct Instruction {
    raw_instruction: u16,
    first_opcode: u16,
    x: usize,
    y: usize,
    n: u8,
    nn: u8,
    nnn: u16,
}

impl Instruction {
    fn new(raw_instruction: u16) -> Self {
        Instruction {
            raw_instruction,
            first_opcode: raw_instruction & 0xF000 as u16,
            x: ((raw_instruction & 0x0F00) >> 8) as usize,
            y: ((raw_instruction & 0x00F0) >> 4) as usize,
            n: (raw_instruction & 0x000F) as u8,
            nn: (raw_instruction & 0x00FF) as u8,
            nnn: raw_instruction & 0x0FFF,
        }
    }
}

impl Emulator {
    fn new(program: Vec<u8>) -> Self {
        let mut memory = [0; 4096];

        memory[80..80 + font::FONT.len()].copy_from_slice(&font::FONT);
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
        let instruction = Instruction::new(raw_instruction);
        self.execute_instruction(instruction);
    }

    fn execute_instruction(&mut self, instruction: Instruction) {
        // Execute
        match instruction.raw_instruction {
            0x00E0 => self.display.clear(),
            0x00EE => {
                self.program_counter = self.stack.pop().expect("No value to pop off the stack")
            }
            _ => match instruction.first_opcode {
                0x1000 => self.program_counter = instruction.nnn,
                0x2000 => {
                    self.stack.push(self.program_counter);
                    self.program_counter = instruction.nnn;
                }
                0x3000 => {
                    if self.registers[instruction.x] == instruction.nn {
                        self.program_counter += 2;
                    }
                }
                0x4000 => {
                    if self.registers[instruction.x] != instruction.nn {
                        self.program_counter += 2;
                    }
                }
                0x5000 => {
                    if self.registers[instruction.x] == self.registers[instruction.y] {
                        self.program_counter += 2;
                    }
                }
                0x6000 => self.registers[instruction.x] = instruction.nn,
                0x7000 => self.registers[instruction.x] += instruction.nn,
                0x8000 => {
                    let x_register = self.registers[instruction.x];
                    let y_register = self.registers[instruction.y];
                    self.registers[instruction.x] = match instruction.n {
                        0x0 => y_register,
                        0x1 => x_register | y_register,
                        0x2 => x_register & y_register,
                        0x3 => x_register ^ y_register,
                        0x4 => {
                            let (result, overflow) = x_register.overflowing_add(y_register);
                            if overflow {
                                self.registers[0xF] = 1;
                            }
                            result
                        }
                        0x5 => {
                            let (result, underflow) = x_register.overflowing_sub(y_register);
                            if underflow {
                                self.registers[0xF] = 0;
                            } else {
                                self.registers[0xF] = 1;
                            }
                            result
                        }
                        0x6 => {
                            if x_register & (0x1 << 7) == 1 {
                                // VF = 1 iff the bit shifted was 1
                                self.registers[0xF] = 1;
                            } else {
                                self.registers[0xF] = 0;
                            }
                            x_register << 1
                        }
                        0x7 => {
                            let (result, underflow) = y_register.overflowing_sub(x_register);
                            if underflow {
                                self.registers[0xF] = 0;
                            } else {
                                self.registers[0xF] = 1;
                            }
                            result
                        }
                        0x8 => {
                            if x_register & 0x1 == 1 {
                                // VF = 1 iff the bit shifted was 1
                                self.registers[0xF] = 1;
                            } else {
                                self.registers[0xF] = 0;
                            }
                            x_register >> 1
                        }
                        _ => panic!("Invalid instruction {}", instruction.raw_instruction),
                    }
                }
                0x9000 => {
                    if self.registers[instruction.x] != self.registers[instruction.y] {
                        self.program_counter += 2;
                    }
                }
                0xA000 => self.index_register = instruction.nnn,
                0xB000 => self.program_counter = instruction.nnn + self.registers[0x0] as u16,
                0xC000 => self.registers[instruction.x] = rand::thread_rng().gen::<u8>() & instruction.nn,
                0xD000 => self.execute_draw_instruction(&instruction),
                _ => panic!(
                    "Unimplemented instruction {:x}",
                    instruction.raw_instruction
                ),
            },
        }
    }

    fn execute_draw_instruction(&mut self, instruction: &Instruction) {
        let x_pos = self.registers[instruction.x] % 64;
        let y_pos = self.registers[instruction.y] % 32;

        let start = self.index_register as usize;
        let end = start + instruction.n as usize;
        let bytes = if let Some(slice) = self.memory.get(start..end) {
            slice.to_vec()
        } else {
            panic!(
                "Bad draw instruction (memory not found) {}",
                instruction.raw_instruction
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
                _ => {}
            }
        }

        // Check if it's time to execute the next instruction
        if last_instruction_time.elapsed() >= Duration::from_millis(1_000 / 700) {
            emulator.perform_fde_cycle();

            // Rerender if necessary
            if emulator.display.draw {
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();
                canvas.set_draw_color(Color::WHITE);

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

        ::std::thread::sleep(Duration::from_millis(1));
    }
}
