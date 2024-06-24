#[derive(Debug)]
pub enum Instruction {
    Clear,
    PopStack,
    SetProgramCounter,
    PushStackSetProgramCounter,
    SkipIfEqualImmediate,
    SkipIfNotEqualImmediate,
    SkipIfEqualRegister,
    SkipIfNotEqualRegister,
    SetRegister,
    AddToRegister,
    CopyFromRegisterToRegister,
    LogicalOr,
    LogicalAnd,
    LogicalXor,
    Addition,
    Subtraction,
    RightShift,
    FlippedSubtraction,
    LeftShift,
    SetIndexRegister,
    SetProgramCounterOffset,
    RandomNumber,
    Draw,
    KeyDown,
    KeyNotDown,
    CopyDelayTimer,
    SetDelayTimer,
    SetSoundTimer,
    AddToIndexRegister,
    WaitForKeyPress,
    SetIndexRegisterToFontCharacter,
    ConvertToDecimal,
    WriteToMemory,
    ReadFromMemory,
}

#[derive(Debug)]
pub struct ParsedInstruction {
    pub raw_instruction: u16,
    pub instruction: Instruction,
    pub x: usize,
    pub y: usize,
    pub n: u8,
    pub nn: u8,
    pub nnn: u16,
}

impl ParsedInstruction {
    pub fn parse(raw_instruction: u16) -> Self {
        let first_nibble: u8 = ((raw_instruction & 0xF000) >> 12) as u8;
        let n = (raw_instruction & 0x000F) as u8;
        let nn = (raw_instruction & 0x00FF) as u8;
        let instruction = match raw_instruction {
            0x00E0 => Instruction::Clear,
            0x00EE => Instruction::PopStack,
            _ => match first_nibble {
                0x1 => Instruction::SetProgramCounter,
                0x2 => Instruction::PushStackSetProgramCounter,
                0x3 => Instruction::SkipIfEqualImmediate,
                0x4 => Instruction::SkipIfNotEqualImmediate,
                0x5 => Instruction::SkipIfEqualRegister,
                0x6 => Instruction::SetRegister,
                0x7 => Instruction::AddToRegister,
                0x8 => match n {
                    0x0 => Instruction::CopyFromRegisterToRegister,
                    0x1 => Instruction::LogicalOr,
                    0x2 => Instruction::LogicalAnd,
                    0x3 => Instruction::LogicalXor,
                    0x4 => Instruction::Addition,
                    0x5 => Instruction::Subtraction,
                    0x6 => Instruction::RightShift,
                    0x7 => Instruction::FlippedSubtraction,
                    0xE => Instruction::LeftShift,
                    _ => panic!("Invalid instruction {:x}", raw_instruction),
                },
                0x9 => Instruction::SkipIfNotEqualRegister,
                0xA => Instruction::SetIndexRegister,
                0xB => Instruction::SetProgramCounterOffset,
                0xC => Instruction::RandomNumber,
                0xD => Instruction::Draw,
                0xE => match nn {
                    0x9E => Instruction::KeyDown,
                    0xA1 => Instruction::KeyNotDown,
                    _ => panic!("Invalid instruction {:x}", raw_instruction),
                },
                0xF => match nn {
                    0x07 => Instruction::CopyDelayTimer,
                    0x0A => Instruction::WaitForKeyPress,
                    0x15 => Instruction::SetDelayTimer,
                    0x18 => Instruction::SetSoundTimer,
                    0x1E => Instruction::AddToIndexRegister,
                    0x29 => Instruction::SetIndexRegisterToFontCharacter,
                    0x33 => Instruction::ConvertToDecimal,
                    0x55 => Instruction::WriteToMemory,
                    0x65 => Instruction::ReadFromMemory,
                    _ => panic!("Invalid instruction {:x}", raw_instruction),
                },
                _ => panic!("Invalid instruction {:x}", raw_instruction),
            },
        };

        ParsedInstruction {
            raw_instruction,
            instruction,
            x: ((raw_instruction & 0x0F00) >> 8) as usize,
            y: ((raw_instruction & 0x00F0) >> 4) as usize,
            n,
            nn,
            nnn: raw_instruction & 0x0FFF,
        }
    }
}
