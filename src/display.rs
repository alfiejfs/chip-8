pub struct Display {
    pub buffer: [[bool; 64]; 32],
    pub draw: bool,
}

impl Display {
    pub fn new() -> Self {
        Display {
            buffer: [[false; 64]; 32],
            draw: false,
        }
    }

    pub fn clear(&mut self) {
        for row in self.buffer.iter_mut() {
            for elem in row.iter_mut() {
                *elem = false;
            }
        }
        self.draw = true;
    }
}

impl Default for Display {
    fn default() -> Self {
        Display::new()
    }
}
