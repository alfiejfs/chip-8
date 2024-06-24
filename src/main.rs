use std::env;
use std::fs;

mod controller;
mod decoder;
mod display;
mod emulator;
mod font;

fn main() {
    let mut path = env::current_dir().expect("path");
    path.push("programs");
    path.push("c8_test.ch8");

    let program = fs::read(path).unwrap();

    emulator::emulate(program);
}
