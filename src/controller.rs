use sdl2::keyboard::Keycode;

pub struct Controller {
    pub pressed: Option<u8>,
    pub released: Option<u8>,
}

impl Default for Controller {
    fn default() -> Self {
        Controller {
            pressed: None,
            released: None,
        }
    }
}

impl Controller {
    pub fn new() -> Self {
        Self::default()
    }

    // TODO: support various mappings
    pub fn map_to_hex(&self, key: Keycode) -> Option<u8> {
        match key {
            Keycode::NUM_1 => Some(0x1),
            Keycode::NUM_2 => Some(0x2),
            Keycode::NUM_3 => Some(0x3),
            Keycode::NUM_4 => Some(0xC),
            Keycode::Q => Some(0x4),
            Keycode::W => Some(0x5),
            Keycode::E => Some(0x6),
            Keycode::R => Some(0xD),
            Keycode::A => Some(0x7),
            Keycode::S => Some(0x8),
            Keycode::D => Some(0x9),
            Keycode::F => Some(0xE),
            Keycode::Z => Some(0xA),
            Keycode::X => Some(0x0),
            Keycode::C => Some(0xB),
            Keycode::V => Some(0xF),
            _ => None,
        }
    }
}
