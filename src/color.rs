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

    pub fn from_hex(hex: &str) -> Self {
        let tmp = u32::from_str_radix(hex, 16).unwrap();
        let r: u8 = ((tmp >> 16) & 0xFF) as u8;
        let g: u8 = ((tmp >> 8) & 0xFF) as u8;
        let b: u8 = (tmp & 0xFF) as u8;

        RGB([r, g, b])
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