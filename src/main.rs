use std::env;
use std::fs;

mod display;
mod emulator;
mod font;

fn main() {
    let mut path = env::current_dir().expect("path");
    path.push("programs");
    path.push("ibm.ch8");

    let program = fs::read(path).unwrap();

    emulator::emulate(program);
}
