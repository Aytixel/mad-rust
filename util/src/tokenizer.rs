/*

    Modified version of the enigo crate tokenizer

*/
#[derive(Debug, Clone, Copy)]
pub enum Key {
    Shift,
    Control,
    Alt,
    Command,
}

#[derive(Debug, Clone, Copy)]
pub enum Button {
    Left,
    Middle,
    Right,
    ScrollUp,
    ScrollDown,
}

#[derive(Debug, Clone)]
pub enum Token {
    Sequence(String),
    Unicode(String),
    KeyUp(Key),
    KeyDown(Key),
    MouseUp(Button),
    MouseDown(Button),
    Click(Button),
    WaitUp,
    Repeat,
}

pub fn tokenize(input: String) -> Vec<Token> {
    let mut is_unicode = false;
    let mut token_vec = Vec::new();
    let mut buffer = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '{' => match chars.next() {
                Some('{') => buffer.push('{'),
                Some(mut c) => {
                    flush(&mut token_vec, &mut buffer, is_unicode);

                    let mut tag = String::new();

                    loop {
                        tag.push(c);
                        match chars.next() {
                            Some('{') => match chars.peek() {
                                Some(&'{') => {
                                    chars.next();
                                    c = '{'
                                }
                                _ => {}
                            },
                            Some('}') => match chars.peek() {
                                Some(&'}') => {
                                    chars.next();
                                    c = '}'
                                }
                                _ => break,
                            },
                            Some(new) => c = new,
                            None => {}
                        }
                    }

                    match &*tag {
                        "REPEAT" => token_vec.push(Token::Repeat),
                        "WAIT_UP" => token_vec.push(Token::WaitUp),
                        "+UNICODE" => is_unicode = true,
                        "-UNICODE" => is_unicode = false,
                        "+SHIFT" => token_vec.push(Token::KeyDown(Key::Shift)),
                        "-SHIFT" => token_vec.push(Token::KeyUp(Key::Shift)),
                        "+CTRL" => token_vec.push(Token::KeyDown(Key::Control)),
                        "-CTRL" => token_vec.push(Token::KeyUp(Key::Control)),
                        "+META" => token_vec.push(Token::KeyDown(Key::Command)),
                        "-META" => token_vec.push(Token::KeyUp(Key::Command)),
                        "+ALT" => token_vec.push(Token::KeyDown(Key::Alt)),
                        "-ALT" => token_vec.push(Token::KeyUp(Key::Alt)),
                        "+LEFT" => token_vec.push(Token::MouseUp(Button::Left)),
                        "-LEFT" => token_vec.push(Token::MouseDown(Button::Left)),
                        "+MIDDLE" => token_vec.push(Token::MouseUp(Button::Middle)),
                        "-MIDDLE" => token_vec.push(Token::MouseDown(Button::Middle)),
                        "+RIGHT" => token_vec.push(Token::MouseUp(Button::Right)),
                        "-RIGHT" => token_vec.push(Token::MouseDown(Button::Right)),
                        "SCROLL_UP" => token_vec.push(Token::Click(Button::ScrollUp)),
                        "SCROLL_DOWN" => token_vec.push(Token::Click(Button::ScrollDown)),
                        _ => {}
                    }
                }
                None => {}
            },
            '}' => {
                if let Some('}') = chars.next() {
                    buffer.push('}')
                }
            }
            _ => buffer.push(c),
        }
    }

    token_vec
}

fn flush(tokens: &mut Vec<Token>, buffer: &mut String, is_unicode: bool) {
    if !buffer.is_empty() {
        if is_unicode {
            tokens.push(Token::Unicode(buffer.clone()));
        } else {
            tokens.push(Token::Sequence(buffer.clone()));
        }
    }

    buffer.clear();
}
