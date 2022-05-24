use enigo::{Enigo, MouseButton, MouseControllable};

#[derive(Debug)]
struct ClickState {
    left: bool,
    right: bool,
    middle: bool,
}

#[derive(Debug)]
struct ButtonState {
    scroll_button: bool,
    left_actionlock: bool,
    right_actionlock: bool,
    forwards_button: bool,
    back_button: bool,
    thumb_anticlockwise: bool,
    thumb_clockwise: bool,
    hat_top: bool,
    hat_left: bool,
    hat_right: bool,
    hat_bottom: bool,
    button_1: bool,
    precision_aim: bool,
    button_2: bool,
    button_3: bool,
}

pub struct Mapper {
    enigo: Enigo,
    pub mode: Option<u8>,
    pub shift_mode: Option<u8>,
    click_state: ClickState,
    button_state: ButtonState,
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
            button_state: ButtonState {
                back_button: false,
                forwards_button: false,
                button_1: false,
                button_2: false,
                button_3: false,
                hat_top: false,
                hat_bottom: false,
                hat_left: false,
                hat_right: false,
                precision_aim: false,
                thumb_clockwise: false,
                thumb_anticlockwise: false,
                scroll_button: false,
                left_actionlock: false,
                right_actionlock: false,
            },
        }
    }

    pub fn emulate(&mut self, buffer: &[u8]) {
        self.update_mode(buffer);
        self.basic_emulation(buffer);
        self.mapped_emulation(buffer);
    }

    fn update_mode(&mut self, buffer: &[u8]) {
        let modes = buffer[2] & 0b111;

        self.mode = match modes {
            0 | 1 | 2 => Some(modes),
            _ => None,
        };
        self.shift_mode = match modes {
            4 | 5 | 6 => Some(modes - 0b100),
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
        if click_state.middle != self.click_state.middle {
            self.click_state.middle = click_state.middle;

            if click_state.middle {
                self.enigo.mouse_down(MouseButton::Middle);
            } else {
                self.enigo.mouse_up(MouseButton::Middle);
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

    fn mapped_emulation(&mut self, buffer: &[u8]) {
        let mut button_state = ButtonState {
            back_button: (buffer[0] & 8) > 0,
            forwards_button: (buffer[0] & 16) > 0,
            button_1: (buffer[0] & 32) > 0,
            button_2: (buffer[0] & 64) > 0,
            button_3: (buffer[0] & 128) > 0,
            hat_top: (buffer[1] & 1) > 0,
            hat_bottom: (buffer[1] & 2) > 0,
            hat_left: (buffer[1] & 4) > 0,
            hat_right: (buffer[1] & 8) > 0,
            precision_aim: (buffer[1] & 16) > 0,
            thumb_clockwise: (buffer[1] & 32) > 0,
            thumb_anticlockwise: (buffer[1] & 64) > 0,
            scroll_button: (buffer[2] & 8) > 0,
            left_actionlock: (buffer[2] & 16) > 0,
            right_actionlock: (buffer[2] & 32) > 0,
        };
    }
}
