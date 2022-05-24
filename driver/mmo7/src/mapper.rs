use enigo::{Enigo, MouseButton, MouseControllable};

struct ClickState {
    left: bool,
    right: bool,
    middle: bool,
}

pub struct Mapper {
    enigo: Enigo,
    pub mode: Option<u8>,
    pub shift_mode: Option<u8>,
    click_state: ClickState,
}

impl Mapper {
    pub fn new() -> Self {
        Self {
            enigo: Enigo::new(),
            mode: Some(0),
            shift_mode: None,
            click_state: ClickState {
                left: false,
                right: false,
                middle: false,
            },
        }
    }

    pub fn emulate(&mut self, buffer: &[u8]) {
        self.update_mode(buffer);
        self.basic_emulation(buffer);
        self.mapped_emulation(buffer);
    }

    fn update_mode(&mut self, buffer: &[u8]) {
        self.mode = match buffer[2] {
            0 | 1 | 2 => Some(buffer[2]),
            _ => None,
        };
        self.shift_mode = match buffer[2] {
            4 | 5 | 6 => Some(buffer[2] - 0b100),
            _ => None,
        };
    }

    fn basic_emulation(&mut self, buffer: &[u8]) {
        // button emulation
        let click_state = ClickState {
            left: (buffer[0] & 1) > 0,
            right: (buffer[0] & 2) > 0,
            middle: (buffer[0] & 4) > 0,
        };

        if click_state.left != self.click_state.left {
            self.click_state.left = click_state.left;

            if click_state.left {
                self.enigo.mouse_down(MouseButton::Left);
            } else {
                self.enigo.mouse_up(MouseButton::Left);
            }
        }
        if click_state.right != self.click_state.right {
            self.click_state.right = click_state.right;

            if click_state.right {
                self.enigo.mouse_down(MouseButton::Right);
            } else {
                self.enigo.mouse_up(MouseButton::Right);
            }
        }
        if click_state.middle != self.click_state.middle {
            self.click_state.middle = click_state.middle;

            if click_state.middle {
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
