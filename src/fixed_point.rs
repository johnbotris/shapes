pub struct FixedPoint<const points: u32> {
    value: u32,
    scale: u32,
}

impl<const points: u32> FixedPoint<points> {
    pub fn from_f32(float: f32) -> Self {
        Self {
            value: (float * Self::scale_f32()) as u32,
            scale: Self::scale_u32(),
        }
    }

    pub fn from_u32(int: u32) -> Self {
        Self {
            value: int,
            scale: Self::scale_u32(),
        }
    }

    pub fn get_f32(&self) -> f32 {
        self.value as f32 / self.scale as f32
    }

    pub fn get_u32(&self) -> u32 {
        self.value
    }

    pub const fn scale_u32() -> u32 {
        10u32.pow(points)
    }

    pub const fn scale_f32() -> f32 {
        Self::scale_u32() as f32
    }
}

impl<const points: u32> From<f32> for FixedPoint<points> {
    fn from(float: f32) -> Self {
        Self::from_f32(float)
    }
}

impl<const points: u32> From<u32> for FixedPoint<points> {
    fn from(int: u32) -> Self {
        Self::from_u32(int)
    }
}
