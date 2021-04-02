use geng::prelude::*;

struct GameState {
    geng: Rc<Geng>,
    far_distance: f64,
    near_distance: f64,
    camera_near: f64,
    road_ratio: f64,
    position: Vec2<f64>,
}

impl GameState {
    pub fn new(geng: &Rc<Geng>) -> Self {
        Self {
            geng: geng.clone(),
            far_distance: 0.0,
            near_distance: 10.0,
            camera_near: 1.0,
            road_ratio: 0.5,
            position: vec2(0.0, 5.0),
        }
    }
    fn to_screen(&self, framebuffer: &ugli::Framebuffer, position: Vec3<f64>) -> (Vec2<f32>, f32) {
        let framebuffer_size = framebuffer.size();
        let scale = self.camera_near / (self.near_distance + self.camera_near - position.y);
        let screen_position = vec2(position.x * scale * self.road_ratio, scale);
        let screen_position = screen_position.map(|x| x as f32);
        (
            vec2(
                screen_position.x * framebuffer_size.y as f32 + framebuffer_size.x as f32 / 2.0,
                framebuffer_size.y as f32 * 0.8 * (1.0 - screen_position.y),
            ),
            scale as f32,
        )
    }
    pub fn draw_circle(&self, framebuffer: &mut ugli::Framebuffer, position: Vec3<f64>) {
        let (screen_position, scale) = self.to_screen(framebuffer, position);
        self.geng.draw_2d().circle(
            framebuffer,
            screen_position,
            10.0 * scale as f32,
            Color::WHITE,
        );
    }
}

impl geng::State for GameState {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Color::BLACK), None);
        for y in self.far_distance.ceil() as u32..=self.near_distance.floor() as u32 {
            self.draw_circle(framebuffer, vec3(self.road_ratio, y as f64, 0.0));
            self.draw_circle(framebuffer, vec3(-self.road_ratio, y as f64, 0.0));
        }
        self.draw_circle(framebuffer, self.position.extend(0.0));
    }
    fn update(&mut self, delta_time: f64) {
        let mut velocity = vec2(0.0, 0.0);
        if self.geng.window().is_key_pressed(geng::Key::Left) {
            velocity.x -= 1.0;
        }
        if self.geng.window().is_key_pressed(geng::Key::Right) {
            velocity.x += 1.0;
        }
        if self.geng.window().is_key_pressed(geng::Key::Up) {
            velocity.y -= 1.0;
        }
        if self.geng.window().is_key_pressed(geng::Key::Down) {
            velocity.y += 1.0;
        }
        self.position += velocity * delta_time;
        let mut camera_speed = 0.0;
        if self.geng.window().is_key_pressed(geng::Key::PageUp) {
            camera_speed += 1.0;
        }
        if self.geng.window().is_key_pressed(geng::Key::PageDown) {
            camera_speed -= 1.0;
        }
        self.near_distance += camera_speed * delta_time;
        self.far_distance += camera_speed * delta_time;
    }
}

fn main() {
    geng::setup_panic_handler();
    let geng = Rc::new(Geng::new(default()));
    let game_state = GameState::new(&geng);
    geng::run(geng, game_state)
}
