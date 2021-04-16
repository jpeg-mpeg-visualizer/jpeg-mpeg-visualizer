use crate::pixel::{RGB, YCbCr};

#[derive(Default)]
pub struct RawImage(pub Vec<u8>);

impl RawImage {
    pub fn to_rgb_image(&self) -> RGBImage {
        let mut rgb = Vec::new();
        for i in (0..self.0.len()).step_by(4) {
            let r = self.0[i];
            let g = self.0[i + 1];
            let b = self.0[i + 2];
            rgb.push(RGB((r, g, b)));
        }
        RGBImage(rgb)
    }
}

pub struct RGBImage(pub Vec<RGB>);

impl RGBImage {
    pub fn to_ycbcr_image(&self) -> YCbCrImage {
        YCbCrImage(
            self.0.iter()
                .map(|rgb| rgb.to_ycbcr())
                .collect::<Vec<YCbCr>>()
        )
    }
}

pub struct YCbCrImage(pub Vec<YCbCr>);

impl YCbCrImage {
    pub fn to_rgb_image(&self) -> RGBImage {
        RGBImage(
            self.0.iter()
                .map(|ycbcr| ycbcr.to_rgb())
                .collect::<Vec<RGB>>()
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
