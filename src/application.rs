pub enum MouseButton {
    Left,
    Right,
    Middle,
}

pub enum MouseButton {
    Left,
    Right,
    Middle,
}

pub enum MouseAction {
    Up,
    Down,
    Motion { x1: i32, y1: i32, x2: i32, y2: i32 },
}

pub struct MouseEvent {
    pub button: MouseButton,
        Left,
        Right,
        Middle,
    },
    pub action: MouseAction,
}

pub trait Mode {
    fn handle_key(kstr: &str) -> bool,
    fn handle_mouse(event: MouseEvent
}
