pub type Vec2 = (f32, f32);

pub fn lerp(a: Vec2, b: Vec2, alpha: f32) -> Vec2 {
    add(scale(a, alpha), scale(b, 1.0 - alpha))
}

pub fn add(a: Vec2, b: Vec2) -> Vec2 {
    (a.0 + b.0, a.1 + b.1)
}

pub fn scale(v: Vec2, s: f32) -> Vec2 {
    (v.0 * s, v.1 * s)
}
