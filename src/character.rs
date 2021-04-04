use super::*;

#[derive(geng::Assets)]
pub struct Assets {
    run: Animation,
    fall: Animation,
    fall_side: Animation,
}

#[derive(PartialEq, Eq)]
pub enum State {
    Run,
    Fall,
    FallSide,
}

pub struct Character {
    assets: Rc<Assets>,
    pub state: State,
    animation_position: f32,
    pub position: Vec2<f32>,
    pub velocity: Vec2<f32>,
}

impl Character {
    pub fn new(assets: Rc<Assets>, position: Vec2<f32>) -> Self {
        Self {
            assets,
            state: State::Run,
            position,
            animation_position: 0.0,
            velocity: vec2(0.0, 0.0),
        }
    }
    pub fn update(&mut self, delta_time: f32) {
        match self.state {
            State::Run => {
                self.position += self.velocity * delta_time;
                self.animation_position += 3.0 * delta_time * self.velocity.len();
                while self.animation_position >= 1.0 {
                    self.animation_position -= 1.0;
                }
            }
            State::FallSide | State::Fall => self.animation_position += 2.0 * delta_time,
        }
    }
    pub fn draw(&self) -> (&ugli::Texture, Vec3<f32>, Vec2<f32>, Size) {
        let (animation, origin, size): (&Animation, Vec2<f32>, f32) = match self.state {
            State::Fall => (&self.assets.fall, vec2(0.5, 0.5), 0.7),
            State::FallSide => (&self.assets.fall_side, vec2(0.5, 0.0), 2.3),
            State::Run => (&self.assets.run, vec2(0.5, 0.0), 1.0),
        };
        let texture = &animation[((self.animation_position * animation.len() as f32) as usize)
            .min(animation.len() - 1)];
        (
            texture,
            self.position.extend(0.0),
            origin,
            Size::FixedWidth(PLAYER_SIZE * 2.0 * size),
        )
    }
    pub fn check_hit(&mut self, obstacle_position: Vec2<f32>, obstacle_size: f32) -> bool {
        if self.state != State::Run {
            return false;
        }
        let dp = self.position - obstacle_position;
        let size = PLAYER_SIZE + obstacle_size;
        if dp.x.abs() < size && dp.y.abs() < size {
            if dp.x.abs() > dp.y.abs() {
                self.position.x = obstacle_position.x + dp.x.signum() * size;
            } else {
                self.position.y = obstacle_position.y + dp.y.signum() * size;
                if self.position.y < obstacle_position.y {
                    return true;
                }
            }
        }
        false
    }
    pub fn fall(&mut self) {
        if self.state != State::Run {
            return;
        }
        self.velocity = vec2(0.0, 0.0);
        self.animation_position = 0.0;
        self.state = State::Fall;
    }
    pub fn fall_side(&mut self) {
        if self.state != State::Run {
            return;
        }
        self.velocity = vec2(0.0, 0.0);
        self.animation_position = 0.0;
        self.state = State::FallSide;
    }
}
