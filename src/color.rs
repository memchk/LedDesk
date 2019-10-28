use std::ops::Mul;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct RGB([u8; 3]);

impl RGB {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        RGB([r, g, b])
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl Mul<u8> for RGB {
    type Output = RGB;
    fn mul(self, rhs: u8) -> Self::Output {
        RGB(
            [self.0[0] * rhs, self.0[1] * rhs, self.0[2] * rhs]
        )
    }
}

impl Mul<f32> for RGB {
    type Output = RGB;
    fn mul(self, rhs: f32) -> Self::Output {
        RGB(
            [(self.0[0] as f32 * rhs) as u8, (self.0[1] as f32 * rhs) as u8, (self.0[2] as f32 * rhs) as u8]
        )
    }
}