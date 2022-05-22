use enigo::{Enigo, MouseButton, MouseControllable};

pub struct Mapper {
    enigo: Enigo,
    pub mode: Option<u8>,
    pub shift_mode: Option<u8>,
    button_state: [bool; 3],
}

impl Mapper {
    pub fn new() -> Self {
        Self {
            enigo: Enigo::new(),
            mode: Some(0),
            shift_mode: None,
            button_state: [false, false, false],
        }
    }

    pub fn emulate(&mut self, buffer: &[u8]) {
        self.basic_emulation(buffer);
        self.mapped_emulation(buffer);
    }

    fn basic_emulation(&mut self, buffer: &[u8]) {
        // button emulation
        let button_state = [
            (buffer[0] & 1) > 0,
            (buffer[0] & 2) > 0,
            (buffer[0] & 4) > 0,
        ];

        if button_state[0] != self.button_state[0] {
            self.button_state[0] = button_state[0];

            if button_state[0] {
                self.enigo.mouse_down(MouseButton::Left);
            } else {
                self.enigo.mouse_up(MouseButton::Left);
            }
        }
        if button_state[1] != self.button_state[1] {
            self.button_state[1] = button_state[1];

            if button_state[1] {
                self.enigo.mouse_down(MouseButton::Right);
            } else {
                self.enigo.mouse_up(MouseButton::Right);
            }
        }
        if button_state[2] != self.button_state[2] {
            self.button_state[2] = button_state[2];

            if button_state[2] {
                self.enigo.mouse_down(MouseButton::Middle);
            } else {
                self.enigo.mouse_up(MouseButton::Middle);
            }
        }

        // movement emulation
        self.enigo.mouse_move_relative(
            if buffer[3] < 128 {
                buffer[3] as i32
            } else {
                buffer[3] as i32 - 256
            },
            if buffer[5] < 128 {
                buffer[5] as i32
            } else {
                buffer[5] as i32 - 256
            },
        );

        // wheel emulation
        if buffer[7] == 1 {
            self.enigo.mouse_scroll_y(-1);
        }
        if buffer[7] == 255 {
            self.enigo.mouse_scroll_y(1);
        }
    }

    fn mapped_emulation(&mut self, buffer: &[u8]) {}
}
