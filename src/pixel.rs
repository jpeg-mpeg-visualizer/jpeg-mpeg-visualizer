pub struct RGB(pub (u8, u8, u8));

impl RGB {
    pub fn to_ycbcr(&self) -> YCbCr {
        let (r, g, b) = self.0;
        let (r, g, b) = (r as f32, g as f32, b as f32);

        let y = 0.299 * r + 0.587 * g + 0.114 * b;
        let cb = 128.0 - 0.168736 * r - 0.331264 * g + 0.5 * b;
        let cr = 128.0 + 0.5 * r - 0.418688 * g - 0.081312 * b;

        YCbCr((y as u8, cb as u8, cr as u8))
    }
}

pub struct YCbCr(pub (u8, u8, u8));

impl YCbCr {
    pub fn to_rgb(&self) -> RGB {
        let (y, cb, cr) = self.0;
        let (y, cb, cr) = (y as f32, cb as f32, cr as f32);

        let r = y + 1.402 * (cr - 128.0);
        let g = y - 0.344136 * (cb - 128.0) - 0.714136 * (cr - 128.0);
        let b = y + 1.772 * (cb - 128.0);

        RGB((r as u8, g as u8, b as u8))
    }

}