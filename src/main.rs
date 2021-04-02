use geng::prelude::*;

struct GameState {}

impl GameState {
    pub fn new() -> Self {
        Self {}
    }
}

impl geng::State for GameState {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Color::BLACK), None);
    }
}

fn main() {
    geng::setup_panic_handler();
    geng::run(Rc::new(Geng::new(default())), GameState::new())
}
