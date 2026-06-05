pub mod uinput;

pub trait VirtualKeyboard: Send + Sync {
    fn send_key(&self, code: u8, state: i32);
    fn mouse_move(&self, x: i32, y: i32);
    fn mouse_scroll(&self, x: i32, y: i32);
    fn mouse_move_abs(&self, x: i32, y: i32);
}
