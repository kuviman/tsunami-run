use geng::prelude::*;

mod character;

use character::Character;

const PLAYER_SIZE: f32 = 0.1;
const OBSTACLE_SIZE: f32 = 0.23;

#[derive(derive_more::Deref)]
pub struct Animation {
    #[deref]
    frames: Vec<ugli::Texture>,
}

pub enum Size {
    Fixed(f32, f32),
    FixedWidth(f32),
    FixedHeight(f32),
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
    character: Rc<character::Assets>,
    #[asset(path = "house*.png", range = "1..=5")]
    houses: Vec<Rc<ugli::Texture>>,
    #[asset(path = "beach_house*.png", range = "1..=2")]
    beach_houses: Vec<Rc<ugli::Texture>>,
    #[asset(path = "car*.png", range = "1..=2")]
    cars: Vec<Rc<ugli::Texture>>,
    tsunami: Animation,
    road: ugli::Texture,
    sand_road: ugli::Texture,
    pierce: ugli::Texture,
}

struct GameState {
    geng: Rc<Geng>,
    assets: Rc<Assets>,
    far_distance: f32,
    near_distance: f32,
    camera_near: f32,
    road_ratio: f32,
    player: Character,
    tsunami_position: f32,
    next_house: f32,
    next_obstacle: f32,
    houses: Vec<(Vec2<f32>, Rc<ugli::Texture>)>,
    obstacles: Vec<(Vec2<f32>, Rc<ugli::Texture>)>,
    characters: Vec<Character>,
    game_speed: f32,
    transition: Option<geng::Transition>,
    font: geng::Font,
    time: Option<f32>,
    pressed_location: Option<f32>,
    tsunami_animation: f32,
}

impl GameState {
    pub fn new(geng: &Rc<Geng>, assets: Rc<Assets>, skip_intro: bool) -> Self {
        let player = Character::new(assets.character.clone(), vec2(0.0, 1.0));
        Self {
            geng: geng.clone(),
            assets,
            houses: Vec::new(),
            obstacles: Vec::new(),
            far_distance: 0.0,
            near_distance: 10.0,
            camera_near: 1.0,
            road_ratio: 0.5,
            player,
            characters: Vec::new(),
            tsunami_position: -500.0,
            next_house: BEACH_START + 1.0,
            next_obstacle: 10.0,
            game_speed: 1.0,
            transition: None,
            font: geng::Font::new(geng, include_bytes!("../static/virilica.otf").to_vec()).unwrap(),
            time: if skip_intro { Some(0.0) } else { None },
            pressed_location: None,
            tsunami_animation: 0.0,
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
        size: Size,
    ) {
        if position.y > self.near_distance + self.camera_near {
            return;
        }
        let (screen_position, scale) = self.to_screen(framebuffer, position);
        let size = match size {
            Size::Fixed(width, height) => vec2(width, height),
            _ => {
                let height = match size {
                    Size::FixedHeight(height) => framebuffer.size().y as f32 * 0.8 * height * scale,
                    Size::FixedWidth(width) => {
                        let height = width * texture.size().y as f32 / texture.size().x as f32;
                        framebuffer.size().y as f32 * 0.8 * height * scale
                    }
                    _ => unreachable!(),
                };
                let size = texture.size().map(|x| x as f32);
                vec2(height * size.x / size.y, height)
            }
        };
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
        if self.next_house > BEACH_END {
            self.assets
                .houses
                .choose(&mut rand::thread_rng())
                .unwrap()
                .clone()
        } else {
            self.assets
                .beach_houses
                .choose(&mut rand::thread_rng())
                .unwrap()
                .clone()
        }
    }
    fn game_finished(&self) -> bool {
        self.tsunami_position > self.near_distance + self.camera_near
    }
    fn draw_road(
        &self,
        framebuffer: &mut ugli::Framebuffer,
        near_pos: f32,
        far_pos: f32,
        texture: &ugli::Texture,
    ) {
        let near_pos = near_pos.max(self.far_distance);
        let far_pos = far_pos.min(self.near_distance);
        if far_pos < near_pos {
            return;
        }
        let mut road = Vec::new();
        const N: usize = 1000; // DIRTY HACK KEK
        for i in 0..N {
            let near = near_pos + (far_pos - near_pos) * i as f32 / N as f32;
            let far = near_pos + (far_pos - near_pos) * (i + 1) as f32 / N as f32;
            road.push(geng::draw_2d::TexturedVertex {
                a_pos: self
                    .to_screen(framebuffer, vec3(-self.road_ratio, far, 0.0))
                    .0,
                a_color: Color::WHITE,
                a_vt: vec2(0.0, far),
            });
            road.push(geng::draw_2d::TexturedVertex {
                a_pos: self
                    .to_screen(framebuffer, vec3(self.road_ratio, far, 0.0))
                    .0,
                a_color: Color::WHITE,
                a_vt: vec2(1.0, far),
            });
            road.push(geng::draw_2d::TexturedVertex {
                a_pos: self
                    .to_screen(framebuffer, vec3(self.road_ratio, near, 0.0))
                    .0,
                a_color: Color::WHITE,
                a_vt: vec2(1.0, near),
            });
            road.push(geng::draw_2d::TexturedVertex {
                a_pos: self
                    .to_screen(framebuffer, vec3(-self.road_ratio, far, 0.0))
                    .0,
                a_color: Color::WHITE,
                a_vt: vec2(0.0, far),
            });
            road.push(geng::draw_2d::TexturedVertex {
                a_pos: self
                    .to_screen(framebuffer, vec3(self.road_ratio, near, 0.0))
                    .0,
                a_color: Color::WHITE,
                a_vt: vec2(1.0, near),
            });
            road.push(geng::draw_2d::TexturedVertex {
                a_pos: self
                    .to_screen(framebuffer, vec3(-self.road_ratio, near, 0.0))
                    .0,
                a_color: Color::WHITE,
                a_vt: vec2(0.0, near),
            });
        }
        for v in &mut road {
            v.a_vt.y *= 0.1;
        }
        self.geng.draw_2d().draw_textured(
            framebuffer,
            &road,
            texture,
            Color::WHITE,
            ugli::DrawMode::Triangles,
        );
    }
}

const BEACH_START: f32 = 2.0;
const BEACH_END: f32 = 20.0;

impl geng::State for GameState {
    fn draw(&mut self, framebuffer: &mut ugli::Framebuffer) {
        let framebuffer_size = framebuffer.size();
        ugli::clear(framebuffer, Some(Color::rgb(0.8, 0.8, 1.0)), None);
        let beach_start = self
            .to_screen(
                framebuffer,
                vec3(0.0, BEACH_START.min(self.near_distance), 0.0),
            )
            .0
            .y;
        let beach_end = self
            .to_screen(
                &framebuffer,
                vec3(0.0, BEACH_END.min(self.near_distance), 0.0),
            )
            .0
            .y;
        self.geng.draw_2d().quad(
            framebuffer,
            AABB::pos_size(
                vec2(0.0, beach_start),
                vec2(
                    framebuffer_size.x as f32,
                    framebuffer_size.y as f32 * 0.8 - beach_start,
                ),
            ),
            Color::rgb(0.0, 0.0, 1.0),
        );
        self.geng.draw_2d().quad(
            framebuffer,
            AABB::pos_size(
                vec2(0.0, beach_start),
                vec2(framebuffer_size.x as f32, beach_end - beach_start),
            ),
            Color::rgb(1.0, 1.0, 0.0),
        );
        self.geng.draw_2d().quad(
            framebuffer,
            AABB::pos_size(vec2(0.0, 0.0), vec2(framebuffer_size.x as f32, beach_end)),
            Color::rgb(0.0, 0.7, 0.0),
        );
        self.draw_road(
            framebuffer,
            BEACH_END,
            self.near_distance,
            &self.assets.road,
        );
        self.draw_road(
            framebuffer,
            BEACH_START.min(self.near_distance),
            BEACH_END.min(self.near_distance),
            &self.assets.sand_road,
        );
        self.draw_road(
            framebuffer,
            0.0,
            BEACH_START.min(self.near_distance),
            &self.assets.pierce,
        );
        let mut sprites: Vec<(&ugli::Texture, Vec3<f32>, Vec2<f32>, Size)> = Vec::new();
        for (position, texture) in &self.houses {
            sprites.push((
                texture,
                position.extend(0.0),
                vec2(0.5, 0.0),
                Size::FixedWidth(1.0),
            ));
        }
        if !self.game_finished() {
            for (position, texture) in &self.obstacles {
                sprites.push((
                    texture,
                    position.extend(0.0),
                    vec2(0.5, 0.0),
                    Size::FixedWidth(0.28),
                ));
            }
            sprites.push(self.player.draw());
            for character in &self.characters {
                sprites.push(character.draw());
            }
            sprites.push((
                &self.assets.tsunami
                    [(self.tsunami_animation * self.assets.tsunami.len() as f32) as usize],
                vec3(0.0, self.tsunami_position, 0.0),
                vec2(0.5, 0.2),
                Size::Fixed(1000.0, 2.0),
            ));
        }
        sprites.sort_by_key(|&(_, pos, _, _)| r32(pos.y));
        for (texture, position, origin, size) in sprites {
            if let Size::Fixed(_, height) = size {
                let (pos, scale) = self.to_screen(framebuffer, position);
                let size = height * scale * framebuffer_size.y as f32 * 0.8;
                let y = pos.y - size * origin.y;
                let texture_width = framebuffer_size.x as f32
                    / (size * texture.size().x as f32 / texture.size().y as f32);
                let vt1 = -texture_width / 2.0 + 0.5;
                let vt2 = texture_width / 2.0 + 0.5;
                let y1 = y;
                let y2 = y1 + size;
                self.geng.draw_2d().draw_textured(
                    framebuffer,
                    &[
                        geng::draw_2d::TexturedVertex {
                            a_color: Color::WHITE,
                            a_pos: vec2(0.0, y1),
                            a_vt: vec2(vt1, 0.0),
                        },
                        geng::draw_2d::TexturedVertex {
                            a_color: Color::WHITE,
                            a_pos: vec2(framebuffer_size.x as f32, y1),
                            a_vt: vec2(vt2, 0.0),
                        },
                        geng::draw_2d::TexturedVertex {
                            a_color: Color::WHITE,
                            a_pos: vec2(framebuffer_size.x as f32, y2),
                            a_vt: vec2(vt2, 1.0),
                        },
                        geng::draw_2d::TexturedVertex {
                            a_color: Color::WHITE,
                            a_pos: vec2(0.0, y2),
                            a_vt: vec2(vt1, 1.0),
                        },
                    ],
                    texture,
                    Color::WHITE,
                    ugli::DrawMode::TriangleFan,
                );
            } else {
                self.draw_texture(framebuffer, texture, position, origin, size);
            }
        }
        if self.game_finished() {
            self.geng.draw_2d().quad(
                framebuffer,
                AABB::pos_size(
                    vec2(0.0, 0.0),
                    vec2(framebuffer_size.x as f32, framebuffer_size.y as f32 * 0.8),
                ),
                Color::rgba(0.0, 0.5, 1.0, 0.5),
            );
        }
        let font_size = framebuffer_size.y as f32 * 0.05;
        if let Some(time) = self.time {
            if self.tsunami_position < self.near_distance + self.camera_near {
                self.font.draw_aligned(
                    framebuffer,
                    &format!("{:.1}", time),
                    vec2(
                        framebuffer_size.x as f32 / 2.0,
                        framebuffer_size.y as f32 - font_size - 10.0,
                    ),
                    0.5,
                    font_size,
                    Color::BLACK,
                );
            } else {
                self.geng.draw_2d().quad(
                    framebuffer,
                    AABB::pos_size(vec2(0.0, 0.0), framebuffer_size.map(|x| x as f32)),
                    Color::rgba(0.8, 0.8, 1.0, 0.5),
                );
                let mut y = framebuffer_size.y as f32 * 0.8;
                y -= font_size * 2.0;
                self.font.draw_aligned(
                    framebuffer,
                    "ГОРОД под ВОДОЙ",
                    vec2(framebuffer_size.x as f32 / 2.0, y),
                    0.5,
                    font_size * 2.0,
                    Color::BLACK,
                );
                y -= font_size;
                self.font.draw_aligned(
                    framebuffer,
                    "И ты тоже!",
                    vec2(framebuffer_size.x as f32 / 2.0, y),
                    0.5,
                    font_size,
                    Color::rgb(0.1, 0.1, 0.1),
                );
                y -= 2.0 * font_size;
                self.font.draw_aligned(
                    framebuffer,
                    "Ты продержался",
                    vec2(framebuffer_size.x as f32 / 2.0, y),
                    0.5,
                    font_size * 2.0,
                    Color::rgb(0.1, 0.1, 0.1),
                );
                y -= 2.0 * font_size;
                self.font.draw_aligned(
                    framebuffer,
                    &format!("целых {:.1} секунд!", time),
                    vec2(framebuffer_size.x as f32 / 2.0, y),
                    0.5,
                    font_size * 2.0,
                    Color::rgb(0.1, 0.1, 0.1),
                );
                y -= font_size;
                self.font.draw_aligned(
                    framebuffer,
                    "Вот это да!",
                    vec2(framebuffer_size.x as f32 / 2.0, y),
                    0.5,
                    font_size,
                    Color::rgb(0.1, 0.1, 0.1),
                );
                y -= font_size;
                self.font.draw_aligned(
                    framebuffer,
                    "Поздравляю!",
                    vec2(framebuffer_size.x as f32 / 2.0, y),
                    0.5,
                    font_size,
                    Color::rgb(0.1, 0.1, 0.1),
                );
                y -= font_size;
                self.font.draw_aligned(
                    framebuffer,
                    "Ты молодец!",
                    vec2(framebuffer_size.x as f32 / 2.0, y),
                    0.5,
                    font_size,
                    Color::rgb(0.1, 0.1, 0.1),
                );
                y -= 3.0 * font_size;
                self.font.draw_aligned(
                    framebuffer,
                    "Любой клик - рестарт",
                    vec2(framebuffer_size.x as f32 / 2.0, y),
                    0.5,
                    font_size,
                    Color::rgb(0.1, 0.1, 0.1),
                );
            }
        } else {
            self.geng.draw_2d().quad(
                framebuffer,
                AABB::pos_size(vec2(0.0, 0.0), framebuffer_size.map(|x| x as f32)),
                Color::rgba(0.8, 0.8, 1.0, 0.5),
            );
            let mut y = framebuffer_size.y as f32 * 0.8;
            y -= font_size * 2.0;
            self.font.draw_aligned(
                framebuffer,
                "ПОБЕГ от ЦУНАМИ",
                vec2(framebuffer_size.x as f32 / 2.0, y),
                0.5,
                font_size * 2.0,
                Color::BLACK,
            );
            y -= font_size;
            self.font.draw_aligned(
                framebuffer,
                "кодирование - kuviman",
                vec2(framebuffer_size.x as f32 / 2.0, y),
                0.5,
                font_size,
                Color::rgb(0.1, 0.1, 0.1),
            );
            y -= font_size;
            self.font.draw_aligned(
                framebuffer,
                "рисование - mikky_ti",
                vec2(framebuffer_size.x as f32 / 2.0, y),
                0.5,
                font_size,
                Color::rgb(0.1, 0.1, 0.1),
            );
            y -= font_size;
            y -= font_size;
            self.font.draw_aligned(
                framebuffer,
                "Надвигается цунами",
                vec2(framebuffer_size.x as f32 / 2.0, y),
                0.5,
                font_size,
                Color::rgb(0.1, 0.1, 0.1),
            );
            y -= font_size;
            self.font.draw_aligned(
                framebuffer,
                "Город скоро окажется под водой",
                vec2(framebuffer_size.x as f32 / 2.0, y),
                0.5,
                font_size,
                Color::rgb(0.1, 0.1, 0.1),
            );
            y -= font_size;
            self.font.draw_aligned(
                framebuffer,
                "Сможешь ли ты его спасти? Нет!",
                vec2(framebuffer_size.x as f32 / 2.0, y),
                0.5,
                font_size,
                Color::rgb(0.1, 0.1, 0.1),
            );
            y -= font_size;
            self.font.draw_aligned(
                framebuffer,
                "Сможешь ли ты спасти себя? Нет!",
                vec2(framebuffer_size.x as f32 / 2.0, y),
                0.5,
                font_size,
                Color::rgb(0.1, 0.1, 0.1),
            );
            y -= font_size;
            self.font.draw_aligned(
                framebuffer,
                "Сколько сможешь продержаться? Да!",
                vec2(framebuffer_size.x as f32 / 2.0, y),
                0.5,
                font_size,
                Color::rgb(0.1, 0.1, 0.1),
            );
            y -= 3.0 * font_size;
            self.font.draw_aligned(
                framebuffer,
                "Любой клик - старт",
                vec2(framebuffer_size.x as f32 / 2.0, y),
                0.5,
                font_size,
                Color::rgb(0.1, 0.1, 0.1),
            );
        }
    }
    fn update(&mut self, delta_time: f64) {
        let mut delta_time = delta_time as f32;
        if self.time.is_none() {
            delta_time = 0.0;
        } else if self.tsunami_position < self.near_distance + self.camera_near {
            self.time = Some(self.time.unwrap() + delta_time);
        }
        self.tsunami_animation += 6.0 * delta_time;
        while self.tsunami_animation >= 1.0 {
            self.tsunami_animation -= 1.0;
        }
        if self.tsunami_position < -4.0 {
            delta_time *= -self.tsunami_position;
        } else if self.player.state == character::State::Run {
            self.game_speed += 0.05 * delta_time;
        } else {
            self.game_speed = 2.0;
        }
        let delta_time = delta_time * self.game_speed;
        if self.player.state == character::State::Run && self.tsunami_position > -4.0 {
            let mut velocity = vec2(0.0, 1.0);
            if self.geng.window().is_key_pressed(geng::Key::Left)
                || self.geng.window().is_key_pressed(geng::Key::A)
            {
                velocity.x -= 1.0;
                self.pressed_location = None;
            }
            if self.geng.window().is_key_pressed(geng::Key::Right)
                || self.geng.window().is_key_pressed(geng::Key::D)
            {
                velocity.x += 1.0;
                self.pressed_location = None;
            }
            self.player.velocity.x = velocity.x;
            if let Some(location) = self.pressed_location {
                let window_size = self.geng.window().size();
                let target = (location - window_size.x as f32 / 2.0)
                    / (min(window_size.x, window_size.y) as f32 / 2.0);
                self.player.velocity.x = clamp_abs(
                    (target * self.road_ratio - self.player.position.x) * 10.0,
                    1.0,
                );
            }
            self.player.velocity.y +=
                clamp_abs(velocity.y - self.player.velocity.y, delta_time * 5.0);
        }
        self.player.update(delta_time);
        self.player.position.x = clamp(
            self.player.position.x,
            -self.road_ratio + PLAYER_SIZE..=self.road_ratio - PLAYER_SIZE,
        );
        for &(position, _) in &self.obstacles {
            for character in self
                .characters
                .iter_mut()
                .chain(std::iter::once(&mut self.player))
            {
                if character.check_hit(position, OBSTACLE_SIZE) {
                    character.fall_side();
                }
            }
        }
        for character in &mut self.characters {
            if character.position.y < self.tsunami_position + 1.0 {
                if rand::thread_rng().gen_bool(0.5) {
                    character.fall();
                } else {
                    character.fall_side();
                }
            }
            if self.player.check_hit(character.position, PLAYER_SIZE) {
                self.player.fall();
                character.fall_side();
            }
        }
        self.tsunami_position += delta_time;
        self.look_at(self.player.position.y);
        while self.near_distance + self.camera_near > self.next_house {
            if self.next_house > BEACH_END {
                self.houses
                    .push((vec2(1.3, self.next_house), self.random_house()));
                self.houses
                    .push((vec2(-1.3, self.next_house), self.random_house()));
            } else {
                if rand::thread_rng().gen_bool(0.5) {
                    self.houses
                        .push((vec2(1.3, self.next_house), self.random_house()));
                } else {
                    self.houses
                        .push((vec2(-1.3, self.next_house), self.random_house()));
                }
            }
            self.next_house += 1.0;
        }
        while self.near_distance + self.camera_near > self.next_obstacle {
            if rand::thread_rng().gen_bool(0.7) {
                let mut character = Character::new(
                    self.assets.character.clone(),
                    vec2(
                        rand::thread_rng().gen_range(
                            -self.road_ratio + PLAYER_SIZE..=self.road_ratio - PLAYER_SIZE,
                        ),
                        self.next_obstacle,
                    ),
                );
                character.velocity = vec2(0.0, rand::thread_rng().gen_range(0.3..0.7));
                self.characters.push(character);
            } else {
                self.obstacles.push((
                    vec2(
                        if rand::thread_rng().gen_bool(0.5) {
                            1.0
                        } else {
                            -1.0
                        } * 0.25,
                        self.next_obstacle,
                    ),
                    self.assets
                        .cars
                        .choose(&mut rand::thread_rng())
                        .unwrap()
                        .clone(),
                ));
            }
            self.next_obstacle += 2.0;
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
        self.characters.retain(|&Character { position, .. }| {
            far_distance <= position.y && position.y <= near_distance + camera_near
        });
        for character in &mut self.characters {
            character.update(delta_time);
        }
    }
    fn handle_event(&mut self, event: geng::Event) {
        match event {
            geng::Event::KeyDown { .. }
            | geng::Event::MouseDown { .. }
            | geng::Event::TouchStart { .. } => {
                if self.time.is_none() {
                    self.time = Some(0.0);
                } else if self.tsunami_position > self.camera_near + self.near_distance {
                    self.transition = Some(geng::Transition::Switch(Box::new(GameState::new(
                        &self.geng,
                        self.assets.clone(),
                        true,
                    ))));
                }
            }
            _ => {}
        }
        match event {
            geng::Event::MouseDown { position, .. } => {
                self.pressed_location = Some(position.x as f32);
            }
            geng::Event::MouseMove { position, .. } if self.pressed_location.is_some() => {
                self.pressed_location = Some(position.x as f32);
            }
            geng::Event::MouseUp { .. } => {
                self.pressed_location = None;
            }
            geng::Event::TouchStart { ref touches, .. } => {
                self.pressed_location = Some(touches[0].position.x as f32);
            }
            geng::Event::TouchMove { ref touches, .. } if self.pressed_location.is_some() => {
                self.pressed_location = Some(touches[0].position.x as f32);
            }
            geng::Event::TouchEnd { .. } => {
                self.pressed_location = None;
            }
            _ => {}
        }
        if let geng::Event::KeyDown { key: geng::Key::R } = event {
            self.transition = Some(geng::Transition::Switch(Box::new(GameState::new(
                &self.geng,
                self.assets.clone(),
                false,
            ))));
        }
    }
    fn transition(&mut self) -> Option<geng::Transition> {
        self.transition.take()
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
            move |assets| {
                let mut assets = assets.unwrap();
                assets.road.set_wrap_mode(ugli::WrapMode::Repeat);
                assets.sand_road.set_wrap_mode(ugli::WrapMode::Repeat);
                assets.pierce.set_wrap_mode(ugli::WrapMode::Repeat);
                fn prev_pot(n: usize) -> usize {
                    let mut x = 1;
                    while x * 2 <= n {
                        x *= 2;
                    }
                    x
                }
                for frame in &mut assets.tsunami.frames {
                    let mut texture = ugli::Texture::new_uninitialized(
                        geng.ugli(),
                        vec2(prev_pot(frame.size().x), prev_pot(frame.size().y)),
                    );
                    let texture_size = texture.size();
                    let mut framebuffer = ugli::Framebuffer::new_color(
                        geng.ugli(),
                        ugli::ColorAttachment::Texture(&mut texture),
                    );
                    ugli::clear(&mut framebuffer, Some(Color::TRANSPARENT_BLACK), None);
                    geng.draw_2d().textured_quad(
                        &mut framebuffer,
                        AABB::pos_size(
                            vec2(0.0, texture_size.y as f32),
                            vec2(texture_size.x as f32, -(texture_size.y as f32)),
                        ),
                        frame,
                        Color::WHITE,
                    );
                    texture.set_wrap_mode(ugli::WrapMode::Repeat);
                    *frame = texture;
                }
                GameState::new(&geng, Rc::new(assets), false)
            }
        }),
    )
}
