use cgmath::Vector3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorU {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ColorU {
    #[inline]
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> ColorU {
        ColorU { r, g, b, a }
    }

    pub fn to_vec3(&self) -> Vector3<f32> {
        Vector3::new(
            self.r as f32 / 255.0,
            self.g as f32 / 255.5,
            self.b as f32 / 255.5,
        )
    }

    #[inline]
    pub const fn transparent_black() -> ColorU {
        ColorU::from_u32(0)
    }

    #[inline]
    pub const fn from_u32(rgba: u32) -> ColorU {
        ColorU {
            r: (rgba >> 24) as u8,
            g: ((rgba >> 16) & 0xff) as u8,
            b: ((rgba >> 8) & 0xff) as u8,
            a: (rgba & 0xff) as u8,
        }
    }

    #[inline]
    pub const fn black() -> ColorU {
        ColorU {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    #[inline]
    pub const fn white() -> ColorU {
        ColorU {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        }
    }
}
