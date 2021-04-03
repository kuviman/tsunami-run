use geng::prelude::*;

#[derive(derive_more::Deref)]
struct Animation {
    #[deref]
    frames: Vec<ugli::Texture>,
}

impl geng::LoadAsset for Animation {
    fn load(geng: &Rc<Geng>, path: &str) -> geng::AssetFuture<Self> {
        let data = <Vec<u8> as geng::LoadAsset>::load(geng, path);
        let geng = geng.clone();
        async move {
            let data = data.await?;
            use image::AnimationDecoder;
            Ok(Self {
                frames: image::codecs::png::PngDecoder::new(data.as_slice())
                    .unwrap()
                    .apng()
                    .into_frames()
                    .map(|frame| {
                        let frame = frame.unwrap();
                        ugli::Texture::from_image_image(geng.ugli(), frame.into_buffer())
                    })
                    .collect(),
            })
        }
        .boxed_local()
    }
    const DEFAULT_EXT: Option<&'static str> = Some("png");
}

#[derive(geng::Assets)]
struct Assets {
    character: Animation,
    #[asset(path = "house*.png", range = "1..=3")]
    houses: Vec<Rc<ugli::Texture>>,
    car: Rc<ugli::Texture>,
    tsunami: ugli::Texture,
}

struct GameState {
    geng: Rc<Geng>,
    assets: Assets,
    far_distance: f32,
    near_distance: f32,
    camera_near: f32,
    road_ratio: f32,
    position: Vec2<f32>,
    tsunami_position: f32,
    character_animation: f32,
    next_house: f32,
    next_obstacle: f32,
    houses: Vec<(Vec2<f32>, Rc<ugli::Texture>)>,
    obstacles: Vec<(Vec2<f32>, Rc<ugli::Texture>)>,
}

impl GameState {
    pub fn new(geng: &Rc<Geng>, assets: Assets) -> Self {
        Self {
            geng: geng.clone(),
            assets,
            houses: Vec::new(),
            obstacles: Vec::new(),
            far_distance: 0.0,
            near_distance: 10.0,
            camera_near: 1.0,
            road_ratio: 0.5,
            position: vec2(0.0, 5.0),
            tsunami_position: 0.0,
            character_animation: 0.0,
            next_house: 0.0,
            next_obstacle: 10.0,
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
    pub fn draw_texture(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        texture: &ugli::Texture,
        position: Vec3<f32>,
        origin: Vec2<f32>,
        height: f32,
    ) {
        if position.y > self.near_distance + self.camera_near {
            return;
        }
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
    fn look_at(&mut self, position: f32) {
        self.near_distance = position + 2.0;
        self.far_distance = position - 10.0;
    }
    fn random_house(&self) -> Rc<ugli::Texture> {
        self.assets
            .houses
            .choose(&mut rand::thread_rng())
            .unwrap()
            .clone()
    }
}

impl geng::State for GameState {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        let framebuffer_size = framebuffer.size();
        ugli::clear(framebuffer, Some(Color::rgb(0.0, 1.0, 0.0)), None);
        let road = [
            self.to_screen(framebuffer, vec3(-self.road_ratio, self.far_distance, 0.0))
                .0,
            self.to_screen(framebuffer, vec3(self.road_ratio, self.far_distance, 0.0))
                .0,
            self.to_screen(framebuffer, vec3(self.road_ratio, self.near_distance, 0.0))
                .0,
            self.to_screen(framebuffer, vec3(-self.road_ratio, self.near_distance, 0.0))
                .0,
        ];
        self.geng.draw_2d().quad(
            framebuffer,
            AABB::pos_size(
                vec2(0.0, framebuffer_size.y as f32 * 0.8),
                framebuffer_size.map(|x| x as f32),
            ),
            Color::rgb(0.8, 0.8, 1.0),
        );
        self.geng.draw_2d().draw(
            framebuffer,
            &road,
            Color::rgb(0.7, 0.7, 0.7),
            ugli::DrawMode::TriangleFan,
        );
        let mut sprites: Vec<(&ugli::Texture, Vec3<f32>, Vec2<f32>, f32)> = Vec::new();
        for (position, texture) in &self.houses {
            sprites.push((texture, position.extend(0.0), vec2(0.5, 0.0), 1.5));
        }
        for (position, texture) in &self.obstacles {
            sprites.push((texture, position.extend(0.0), vec2(0.5, 0.0), 0.2));
        }
        let character_texture = &self.assets.character
            [(self.character_animation * self.assets.character.len() as f32) as usize];
        sprites.push((
            character_texture,
            self.position.extend(0.0),
            vec2(0.5, 0.0),
            0.3,
        ));
        sprites.push((
            &self.assets.tsunami,
            vec3(0.0, self.tsunami_position, 0.0),
            vec2(0.5, 0.2),
            2.0,
        ));
        sprites.sort_by_key(|&(_, pos, _, _)| r32(pos.y));
        for (texture, position, origin, height) in sprites {
            self.draw_texture(framebuffer, texture, position, origin, height);
        }
    }
    fn update(&mut self, delta_time: f64) {
        let delta_time = delta_time as f32;
        let mut velocity = vec2(0.0, 1.0);
        if self.geng.window().is_key_pressed(geng::Key::Left) {
            velocity.x -= 1.0;
        }
        if self.geng.window().is_key_pressed(geng::Key::Right) {
            velocity.x += 1.0;
        }
        if self.geng.window().is_key_pressed(geng::Key::Up) {
            velocity.y = -1.0;
        }
        self.position += velocity * delta_time;
        const PLAYER_SIZE: f32 = 0.1;
        self.position.x = clamp(
            self.position.x,
            -self.road_ratio + PLAYER_SIZE..=self.road_ratio - PLAYER_SIZE,
        );
        for &(position, _) in &self.obstacles {
            let dp = self.position - position;
            const SIZE_X: f32 = 0.23 + PLAYER_SIZE;
            const SIZE_Y: f32 = 0.23 + PLAYER_SIZE;
            if dp.x.abs() < SIZE_X && dp.y.abs() < SIZE_Y {
                if dp.x.abs() > dp.y.abs() {
                    self.position.x = position.x + dp.x.signum() * SIZE_X;
                } else {
                    self.position.y = position.y + dp.y.signum() * SIZE_Y;
                }
            }
        }
        self.tsunami_position += delta_time;
        self.look_at(self.position.y);
        while self.near_distance + self.camera_near > self.next_house {
            self.houses
                .push((vec2(1.5, self.next_house), self.random_house()));
            self.houses
                .push((vec2(-1.5, self.next_house), self.random_house()));
            self.next_house += 1.0;
        }
        while self.near_distance + self.camera_near > self.next_obstacle {
            self.obstacles.push((
                vec2(
                    if rand::thread_rng().gen_bool(0.5) {
                        1.0
                    } else {
                        -1.0
                    } * 0.25,
                    self.next_obstacle,
                ),
                self.assets.car.clone(),
            ));
            self.next_obstacle += 2.0;
        }
        self.character_animation += 3.0 * delta_time;
        while self.character_animation >= 1.0 {
            self.character_animation -= 1.0;
        }
        let near_distance = self.near_distance;
        let far_distance = self.far_distance;
        let camera_near = self.camera_near;
        self.houses.retain(|&(position, _)| {
            far_distance <= position.y && position.y <= near_distance + camera_near
        });
        self.obstacles.retain(|&(position, _)| {
            far_distance <= position.y && position.y <= near_distance + camera_near
        });
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
