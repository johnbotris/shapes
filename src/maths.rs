use core::f32::consts::PI;

// thanks paul bourke
pub fn lerp(y0: f32, y1: f32, mu: f32) -> f32 {
    y0 * (1.0 - mu) + y1 * mu
}

pub fn cosine_lerp(y0: f32, y1: f32, mu: f32) -> f32 {
    let mu2 = (1.0 - f32::cos(mu * PI)) / 2.0;
    lerp(y0, y1, mu2)
}

pub fn cubic_interp(y0: f32, y1: f32, y2: f32, y3: f32, mu: f32) -> f32 {
    let mu2 = mu * mu;
    let a0 = y3 - y2 - y0 - y1;
    let a1 = y0 - y1 - a0;
    let a2 = y2 - y0;
    let a3 = y1;
    a0 * mu * mu2 + a1 * mu2 + a2 * mu + a3
}

pub fn catmull_rom_interp(y0: f32, y1: f32, y2: f32, y3: f32, mu: f32) -> f32 {
    let mu2 = mu * mu;
    let a0 = -0.5 * y0 + 1.5 * y1 - 1.5 * y2 + 0.5 * y3;
    let a1 = y0 - 2.5 * y1 + 2.0 * y2 - 0.5 * y3;
    let a2 = -0.5 * y0 + 0.5 * y2;
    let a3 = y1;
    a0 * mu * mu2 + a1 * mu2 + a2 * mu + a3
}

pub fn hermite_interp(y0: f32, y1: f32, y2: f32, y3: f32, mu: f32, tension: f32, bias: f32) -> f32 {
    let mu2 = mu * mu;
    let mu3 = mu2 * mu;
    let m0 = ((y1 - y0) * (1.0 + bias) * (1.0 - tension) / 2.0)
        + ((y2 - y1) * (1.0 - bias) * (1.0 - tension) / 2.0);
    let m1 = ((y2 - y1) * (1.0 + bias) * (1.0 - tension) / 2.0)
        + ((y3 - y2) * (1.0 - bias) * (1.0 - tension) / 2.0);
    let a0 = 2.0 * mu3 - 3.0 * mu2 + 1.0;
    let a1 = mu3 - 2.0 * mu2 + mu;
    let a2 = mu3 - mu2;
    let a3 = -2.0 * mu3 + 3.0 * mu2;

    a0 * y1 + a1 * m0 + a2 * m1 + a3 * y2
}

pub mod vec2 {
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
}
