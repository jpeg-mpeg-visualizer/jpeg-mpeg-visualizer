pub mod pixel {
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

}

#[derive(Default, Clone)]
pub struct RawImage{
    pub all_data: Vec<u8>,
    pub height: u32,
    pub width: u32,
    pub data: Vec<u8>
}

impl RawImage {
    pub fn new(all_data: Vec<u8>, height: u32, width: u32) -> Self {
        let mut viewed_data: Vec<u8> = Vec::with_capacity(500*500*4);
        for i in 0..500 {
            for j in 0..2000 {
                viewed_data.push(all_data[(i * width * 4 + j) as usize]);
            }
        }

        return RawImage {
            all_data,
            height,
            width,
            data: viewed_data
        }
    }
}

impl RawImage {
    pub fn move_viewed(&mut self, start_x: u32, start_y: u32) {
        for i in 0..500 {
            for j in 0..2000 {
                self.data[(i*500*4 + j) as usize] = self.all_data[((i+start_y) * self.width * 4 + (j+start_x*4)) as usize];
            }
        }
    }
}

impl RawImage {
    pub fn to_rgb_image(&self) -> RGBImage {
        let mut rgb = Vec::new();
        for i in (0..self.data.len()).step_by(4) {
            let r = self.data[i];
            let g = self.data[i + 1];
            let b = self.data[i + 2];
            rgb.push(pixel::RGB((r, g, b)));
        }
        RGBImage(rgb)
    }
}

pub struct RGBImage(pub Vec<pixel::RGB>);

impl RGBImage {
    pub fn to_ycbcr_image(&self) -> YCbCrImage {
        YCbCrImage(
            self.0.iter()
                .map(|rgb| rgb.to_ycbcr())
                .collect::<Vec<pixel::YCbCr>>()
        )
    }
}

pub struct YCbCrImage(pub Vec<pixel::YCbCr>);

impl YCbCrImage {
    pub fn to_rgb_image(&self) -> RGBImage {
        RGBImage(
            self.0.iter()
                .map(|ycbcr| ycbcr.to_rgb())
                .collect::<Vec<pixel::RGB>>()
        )
    }

    pub fn to_ys_channel(&self) -> Vec<u8> {
        self.0.iter().map(|x| x.0 .0).collect::<Vec<u8>>()
    }

    pub fn to_cbs_channel(&self) -> Vec<u8> {
        self.0.iter().map(|x| x.0 .1).collect::<Vec<u8>>()
    }

    pub fn to_crs_channel(&self) -> Vec<u8> {
        self.0.iter().map(|x| x.0 .2).collect::<Vec<u8>>()
    }
}
