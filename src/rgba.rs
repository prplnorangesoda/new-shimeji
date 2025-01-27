use std::fmt::Debug;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl Debug for Rgba {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "(r: {:3}, g: {:3}, b: {:3}, a: {:3})",
            self.red, self.green, self.blue, self.alpha
        )
    }
}
impl Rgba {
    pub fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub fn from_tuple<T>(tuple: T) -> Self
    where
        T: Into<(u8, u8, u8, u8)>,
    {
        let rgba = tuple.into();
        Self {
            red: rgba.0,
            green: rgba.1,
            blue: rgba.2,
            alpha: rgba.3,
        }
    }

    /// --------
    ///
    /// Pixel format (`u32`):
    ///
    /// 00000000RRRRRRRRGGGGGGGGBBBBBBBB
    ///
    /// 0: Bit is 0
    /// R: Red channel
    /// G: Green channel
    /// B: Blue channel
    pub fn to_softbuf_u32(self) -> u32 {
        if self.alpha == 0 {
            return 0;
        }
        // println!("self.alpha as f32: {}", self.alpha as f32);
        let alpha = self.alpha as f32 / 255.0;
        let red = (self.red as f32 * alpha).floor() as u32;
        let blue = (self.blue as f32 * alpha).floor() as u32;
        let green = (self.green as f32 * alpha).floor() as u32;
        // println!("red: {}", red);
        // println!("green: {}", green);
        // println!("blue: {}", blue);
        // println!("alpha: {}", alpha);

        let ret = (red << 16) | (blue << 8) | green;

        // println!("ret: {:#034b}", ret);
        ret

        // (self.alpha as u32) << 24
        //     | (self.red as u32) << 16
        //     | (self.green as u32) << 8
        //     | self.blue as u32
    }
}
