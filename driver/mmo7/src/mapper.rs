use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};

use enigo::{Enigo, MouseButton, MouseControllable};
use util::{
    config::ConfigManager,
    thread::MutexTrait,
    tokenizer::{tokenize, StateToken, Token},
};

use crate::{ButtonConfig, ButtonConfigs, MousesConfig};

type ButtonConfigToken = [[StateToken; 3]; 2];

pub struct ButtonConfigsToken {
    scroll_button: ButtonConfigToken,
    left_actionlock: ButtonConfigToken,
    right_actionlock: ButtonConfigToken,
    forwards_button: ButtonConfigToken,
    back_button: ButtonConfigToken,
    thumb_anticlockwise: ButtonConfigToken,
    thumb_clockwise: ButtonConfigToken,
    hat_top: ButtonConfigToken,
    hat_left: ButtonConfigToken,
    hat_right: ButtonConfigToken,
    hat_bottom: ButtonConfigToken,
    button_1: ButtonConfigToken,
    precision_aim: ButtonConfigToken,
    button_2: ButtonConfigToken,
    button_3: ButtonConfigToken,
}

impl ButtonConfigsToken {
    fn from_config(button_configs: ButtonConfigs) -> Self {
        Self {
            scroll_button: button_configs.scroll_button.tokenize(),
            left_actionlock: button_configs.left_actionlock.tokenize(),
            right_actionlock: button_configs.right_actionlock.tokenize(),
            forwards_button: button_configs.forwards_button.tokenize(),
            back_button: button_configs.back_button.tokenize(),
            thumb_anticlockwise: button_configs.thumb_anticlockwise.tokenize(),
            thumb_clockwise: button_configs.thumb_clockwise.tokenize(),
            hat_top: button_configs.hat_top.tokenize(),
            hat_left: button_configs.hat_left.tokenize(),
            hat_right: button_configs.hat_right.tokenize(),
            hat_bottom: button_configs.hat_bottom.tokenize(),
            button_1: button_configs.button_1.tokenize(),
            precision_aim: button_configs.precision_aim.tokenize(),
            button_2: button_configs.button_2.tokenize(),
            button_3: button_configs.button_3.tokenize(),
        }
    }
}

struct ClickState {
    left: bool,
    right: bool,
    middle: bool,
}

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

enum Mode {
    Normal(u8),
    Shift(u8),
}

pub struct Mapper {
    enigo: Enigo,
    mode: Mode,
    click_state: ClickState,
    button_state: ButtonState,
    button_configs_token: ButtonConfigsToken,
    mouses_config_mutex: Arc<Mutex<ConfigManager<MousesConfig>>>,
    mouses_config_state_id: Arc<AtomicU32>,
    last_mouses_config_state_id: u32,
    serial_number: String,
}

impl Mapper {
    pub fn new(
        mouses_config_mutex: Arc<Mutex<ConfigManager<MousesConfig>>>,
        mouses_config_state_id: Arc<AtomicU32>,
        serial_number: String,
    ) -> Self {
        let last_mouses_config_state_id = mouses_config_state_id.load(Ordering::SeqCst);
        let button_configs = mouses_config_mutex.lock_safe().config[&serial_number].clone();

        Self {
            enigo: Enigo::new(),
            mode: Mode::Normal(0),
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
            button_configs_token: ButtonConfigsToken::from_config(button_configs),
            mouses_config_mutex,
            mouses_config_state_id,
            last_mouses_config_state_id,
            serial_number,
        }
    }

    pub fn emulate(&mut self, buffer: &[u8]) {
        if self.config_has_change() {
            self.button_configs_token = ButtonConfigsToken::from_config(
                self.mouses_config_mutex.lock_safe().config[&self.serial_number].clone(),
            );
        }

        self.update_mode(buffer);
        self.basic_emulation(buffer);
        self.mapped_emulation(buffer);
    }

    fn update_mode(&mut self, buffer: &[u8]) {
        let modes = buffer[2] & 0b111;

        self.mode = match modes {
            0 | 1 | 2 => Mode::Normal(modes),
            4 | 5 | 6 => Mode::Shift(modes - 0b100),
            _ => Mode::Normal(0),
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
        let button_state = ButtonState {
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

        if button_state.back_button != self.button_state.back_button && button_state.back_button {
            println!(
                "back button : {:?}",
                self.button_configs_token.back_button.get(self)
            );
        }

        self.button_state = button_state;
    }

    fn is_shift_mode(&self) -> bool {
        match self.mode {
            Mode::Normal(_) => false,
            Mode::Shift(_) => true,
        }
    }

    fn absolute_mode(&self) -> u8 {
        match self.mode {
            Mode::Normal(mode) => mode,
            Mode::Shift(mode) => mode,
        }
    }

    fn config_has_change(&mut self) -> bool {
        let mouses_config_state_id = self.mouses_config_state_id.load(Ordering::SeqCst);

        if self.last_mouses_config_state_id != mouses_config_state_id {
            self.last_mouses_config_state_id = mouses_config_state_id;

            true
        } else {
            false
        }
    }
}

trait ButtonConfigExt {
    fn tokenize(&self) -> ButtonConfigToken;
}

impl ButtonConfigExt for ButtonConfig {
    fn tokenize(&self) -> ButtonConfigToken {
        let mut button_config_token = [
            [
                StateToken::default(),
                StateToken::default(),
                StateToken::default(),
            ],
            [
                StateToken::default(),
                StateToken::default(),
                StateToken::default(),
            ],
        ];

        for mode_type_index in 0..2 {
            for mode_index in 0..3 {
                if let Some(config) = self[mode_type_index].get(mode_index) {
                    button_config_token[mode_type_index][mode_index] = tokenize(config.clone());
                }
            }
        }

        button_config_token
    }
}

trait ButtonConfigTokenExt {
    fn get(&self, mapper: &Mapper) -> StateToken;
}

impl ButtonConfigTokenExt for ButtonConfigToken {
    fn get(&self, mapper: &Mapper) -> StateToken {
        self[mapper.is_shift_mode() as usize][mapper.absolute_mode() as usize].clone()
    }
}
