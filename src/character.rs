use super::*;

pub struct Character {
    animation: Rc<Animation>,
    animation_position: f32,
    pub position: Vec2<f32>,
    pub velocity: Vec2<f32>,
}

impl Character {
    pub fn new(animation: Rc<Animation>, position: Vec2<f32>) -> Self {
        Self {
            animation,
            position,
            animation_position: 0.0,
            velocity: vec2(0.0, 0.0),
        }
    }
    pub fn update(&mut self, delta_time: f32) {
        self.position += self.velocity * delta_time;
        self.animation_position += 3.0 * delta_time;
        while self.animation_position >= 1.0 {
            self.animation_position -= 1.0;
        }
    }
    pub fn draw(&self) -> (&ugli::Texture, Vec3<f32>, Vec2<f32>, f32) {
        let texture =
            &self.animation[(self.animation_position * self.animation.len() as f32) as usize];
        (texture, self.position.extend(0.0), vec2(0.5, 0.0), 0.3)
    }
    pub fn check_hit(&mut self, obstacle_position: Vec2<f32>, obstacle_size: f32) {
        let dp = self.position - obstacle_position;
        let size = PLAYER_SIZE + obstacle_size;
        if dp.x.abs() < size && dp.y.abs() < size {
            if dp.x.abs() > dp.y.abs() {
                self.position.x = obstacle_position.x + dp.x.signum() * size;
            } else {
                self.position.y = obstacle_position.y + dp.y.signum() * size;
            }
        }
    }
}
