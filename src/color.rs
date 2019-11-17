use std::ops::Mul;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(C)]
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RGB {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        RGB { r, g, b }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(self as *const _ as *const _, 3)
        }
    }

    pub fn from_hex(hex: &str) -> Self {
        let tmp = u32::from_str_radix(hex, 16).unwrap();
        let r: u8 = ((tmp >> 16) & 0xFF) as u8;
        let g: u8 = ((tmp >> 8) & 0xFF) as u8;
        let b: u8 = (tmp & 0xFF) as u8;

        RGB {
            r, g, b
        }
    }

    pub fn saturate(self) -> RGB {
        let max = self.as_slice().iter().max().unwrap();
        let ratio = 255.0 / *max as f32;
        ratio * self
    }
}

impl Mul<RGB> for f32 {
    type Output = RGB;
    fn mul(self, rhs: RGB) -> Self::Output {
        RGB {
            r: (rhs.r as f32 * self).min(255.0) as u8,
            g: (rhs.g as f32 * self).min(255.0) as u8,
            b: (rhs.b as f32 * self).min(255.0) as u8
        }
    }
}
