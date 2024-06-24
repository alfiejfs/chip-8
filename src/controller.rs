use sdl2::keyboard::Keycode;

pub struct Controller {
    pressed: [bool; 16],
    pub last_pressed: Option<u8>, // last key pressed that is still pressed. will not go back to keys previously pressed (chip-8 hardware not this advanced).
}

impl Default for Controller {
    fn default() -> Self {
        Controller {
            pressed: [false; 16],
            last_pressed: None,
        }
    }
}

impl Controller {
    pub fn new() -> Self {
        Self::default()
    }

    // TODO: support various mappings
    fn map_to_hex(&self, key: Keycode) -> Option<u8> {
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

    pub fn press_key(&mut self, key: Keycode) {
        if let Some(hex) = self.map_to_hex(key) {
            self.pressed[hex as usize] = true;
            self.last_pressed = Some(hex);
        }
    }

    pub fn release_key(&mut self, key: Keycode) {
        if let Some(hex) = self.map_to_hex(key) {
            self.pressed[hex as usize] = false;
            if Some(hex) == self.last_pressed {
                self.last_pressed = None;
            }
        }
    }

    pub fn is_key_pressed(&self, key: u8) -> bool {
        *self.pressed.get(key as usize).unwrap_or(&false)
    }
}
