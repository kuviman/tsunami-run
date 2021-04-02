use geng::prelude::*;

#[derive(geng::Assets)]
struct Assets {
    #[asset(path = "character/*.png", range = "1..=4")]
    character: Vec<ugli::Texture>,
    house: ugli::Texture,
}

struct GameState {
    geng: Rc<Geng>,
    assets: Assets,
    far_distance: f32,
    near_distance: f32,
    camera_near: f32,
    road_ratio: f32,
    position: Vec2<f32>,
    character_animation: f32,
}

impl GameState {
    pub fn new(geng: &Rc<Geng>, assets: Assets) -> Self {
        Self {
            geng: geng.clone(),
            assets,
            far_distance: 0.0,
            near_distance: 10.0,
            camera_near: 1.0,
            road_ratio: 0.5,
            position: vec2(0.0, 5.0),
            character_animation: 0.0,
        }
    }
    fn to_screen(&self, framebuffer: &ugli::Framebuffer, position: Vec3<f32>) -> (Vec2<f32>, f32) {
        let framebuffer_size = framebuffer.size();
        let scale = self.camera_near / (self.near_distance + self.camera_near - position.y);
        let screen_position = vec2(
            position.x * scale * self.road_ratio,
            scale - position.z * scale,
        );
        (
            vec2(
                screen_position.x * framebuffer_size.y as f32 + framebuffer_size.x as f32 / 2.0,
                framebuffer_size.y as f32 * 0.8 * (1.0 - screen_position.y),
            ),
            scale,
        )
    }
    pub fn draw_circle(&self, framebuffer: &mut ugli::Framebuffer, position: Vec3<f32>) {
        let (screen_position, scale) = self.to_screen(framebuffer, position);
        self.geng.draw_2d().circle(
            framebuffer,
            screen_position,
            10.0 * scale as f32,
            Color::WHITE,
        );
    }
    pub fn draw_texture(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        texture: &ugli::Texture,
        position: Vec3<f32>,
        origin: Vec2<f32>,
        height: f32,
    ) {
        let (screen_position, scale) = self.to_screen(framebuffer, position);
        let height = framebuffer.size().y as f32 * 0.8 * height * scale;
        let size = texture.size().map(|x| x as f32);
        let size = vec2(height * size.x / size.y, height);
        let aabb = AABB::pos_size(
            screen_position - vec2(size.x * origin.x, size.y * origin.y),
            size,
        );
        self.geng
            .draw_2d()
            .textured_quad(framebuffer, aabb, texture, Color::WHITE);
    }
}

impl geng::State for GameState {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        ugli::clear(framebuffer, Some(Color::BLACK), None);
        for y in self.far_distance.ceil() as u32..=self.near_distance.floor() as u32 {
            self.draw_circle(framebuffer, vec3(self.road_ratio, y as f32, 0.0));
            self.draw_circle(framebuffer, vec3(-self.road_ratio, y as f32, 0.0));
        }
        for y in (self.far_distance.ceil() as u32..=self.near_distance.floor() as u32).step_by(2) {
            self.draw_texture(
                framebuffer,
                &self.assets.house,
                vec3(self.road_ratio + 1.5, y as f32, 0.0),
                vec2(0.5, 0.0),
                1.5,
            );
        }
        let character_texture = &self.assets.character
            [(self.character_animation * self.assets.character.len() as f32) as usize];
        self.draw_texture(
            framebuffer,
            character_texture,
            self.position.extend(0.0),
            vec2(0.5, 0.0),
            0.5,
        );
    }
    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
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
        self.character_animation += 3.0 * delta_time;
        while self.character_animation >= 1.0 {
            self.character_animation -= 1.0;
        }
    }
}

fn main() {
    geng::setup_panic_handler();
    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        std::env::set_current_dir(std::path::Path::new(&dir).join("static")).unwrap();
    }
    let geng = Rc::new(Geng::new(default()));
    let assets = <Assets as geng::LoadAsset>::load(&geng, ".");
    geng::run(
        geng.clone(),
        geng::LoadingScreen::new(&geng, geng::EmptyLoadingScreen, assets, {
            let geng = geng.clone();
            move |assets| GameState::new(&geng, assets.unwrap())
        }),
    )
}
