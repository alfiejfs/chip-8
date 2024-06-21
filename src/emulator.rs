use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::time::{Duration, Instant, SystemTime};

use crate::{display::Display, font};

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
        let instruction = instruction_msb | instruction_lsb;

        // Increment program counter
        self.program_counter += 2;

        // Decode
        let first_opcode = instruction & 0xF000;

        let x = ((instruction & 0x0F00) >> 8) as u8;
        let y = ((instruction & 0x00F0) >> 4) as u8;
        let n = (instruction & 0x000F) as u8;
        let nn = (instruction & 0x00FF) as u8;
        let nnn = instruction & 0x0FFF;

        // Execute
        match instruction {
            0x00E0 => self.display.clear(),
            _ => match first_opcode {
                0x1000 => self.program_counter = nnn,
                0x6000 => self.registers[x as usize] = nn,
                0x7000 => self.registers[x as usize] += nn,
                0xA000 => self.index_register = nnn,
                0xD000 => {
                    let x_pos = self.registers[x as usize] % 64;
                    let y_pos = self.registers[y as usize] % 32;

                    let start = self.index_register as usize;
                    let end = start + n as usize;
                    let bytes = if let Some(slice) = self.memory.get(start..end) {
                        slice.to_vec()
                    } else {
                        vec![]
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
                _ => panic!("Unimplemented instruction {:x}", instruction),
            },
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
