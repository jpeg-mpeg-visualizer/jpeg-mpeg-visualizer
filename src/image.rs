use crate::block::BlockMatrix;
use std::rc::Rc;

pub mod pixel {
    pub struct RGB {
        pub r: u8,
        pub g: u8,
        pub b: u8,
    }

    impl RGB {
        pub fn to_flat_data(&self) -> [u8; 4] {
            [self.r, self.g, self.b, 255]
        }

        pub fn to_ycbcr(&self) -> YCbCr {
            let (r, g, b) = (self.r as f32, self.g as f32, self.b as f32);

            let y = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
            let cb = (128.0 - 0.168736 * r - 0.331264 * g + 0.5 * b) as u8;
            let cr = (128.0 + 0.5 * r - 0.418688 * g - 0.081312 * b) as u8;

            YCbCr { y, cb, cr }
        }
    }

    pub struct YCbCr {
        pub y: u8,
        pub cb: u8,
        pub cr: u8,
    }

    impl YCbCr {
        pub fn to_rgb(&self) -> RGB {
            let (y, cb, cr) = (self.y as f32, self.cb as f32, self.cr as f32);

            let r = (y + 1.402 * (cr - 128.0)) as u8;
            let g = (y - 0.344136 * (cb - 128.0) - 0.714136 * (cr - 128.0)) as u8;
            let b = (y + 1.772 * (cb - 128.0)) as u8;

            RGB { r, g, b }
        }
    }
}

#[derive(Default, Clone)]
pub struct RawImage {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl RawImage {
    pub fn new(data: Vec<u8>, width: u32, height: u32) -> RawImage {
        RawImage {
            width,
            height,
            data,
        }
    }

    pub fn to_rgb_image(&self) -> RGBImage {
        let mut rgb = Vec::new();
        for i in (0..self.data.len()).step_by(4) {
            let r = self.data[i];
            let g = self.data[i + 1];
            let b = self.data[i + 2];
            rgb.push(pixel::RGB { r, g, b });
        }
        RGBImage(rgb)
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn width(&self) -> u32 {
        self.width
    }
}

impl AsRef<[u8]> for RawImage {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

impl<Idx> std::ops::Index<Idx> for RawImage
where
    Idx: std::slice::SliceIndex<[u8]>,
{
    type Output = Idx::Output;

    fn index(&self, index: Idx) -> &Self::Output {
        &self.data[index]
    }
}

pub struct RawImageWindow {
    raw_image: Rc<RawImage>,

    pub start_x: u32,
    pub start_y: u32,

    width: u32,
    height: u32,
}

impl RawImageWindow {
    pub fn new(
        raw_image: Rc<RawImage>,
        start_x: u32,
        start_y: u32,
        width: u32,
        height: u32,
    ) -> RawImageWindow {
        RawImageWindow {
            raw_image,
            start_x,
            start_y,
            width,
            height,
        }
    }

    #[allow(dead_code)]
    pub fn height(self) -> u32 {
        self.height
    }

    #[allow(dead_code)]
    pub fn width(self) -> u32 {
        self.width
    }

    pub fn to_rgb_image(&self) -> RGBImage {
        let mut rgb = Vec::new();
        for i in (0..(self.width * self.height * 4) as usize).step_by(4) {
            let r = self[i];
            let g = self[i + 1];
            let b = self[i + 2];
            rgb.push(pixel::RGB { r, g, b });
        }
        RGBImage(rgb)
    }
}

impl std::ops::Index<usize> for RawImageWindow {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        let chunk_index_x: u32 = (index / 4) as u32 % self.width;
        let chunk_index_y: u32 = (index / 4) as u32 / self.width;
        assert!(chunk_index_y <= self.height);
        assert!(chunk_index_y + self.start_y <= self.raw_image.height);
        assert!(chunk_index_x + self.start_x <= self.raw_image.width);

        let y = self.start_y + chunk_index_y;
        let x = self.start_x + chunk_index_x;

        &self.raw_image.data[((x + y * self.raw_image.width) * 4 + (index % 4) as u32) as usize]
    }
}

pub struct RGBImage(pub Vec<pixel::RGB>);

impl RGBImage {
    pub fn to_image_data(&self) -> Vec<u8> {
        self.0
            .iter()
            .map(|rgb| rgb.to_flat_data())
            .flatten()
            .collect::<Vec<u8>>()
    }

    pub fn to_ycbcr_image(&self) -> YCbCrImage {
        YCbCrImage(
            self.0
                .iter()
                .map(|rgb| rgb.to_ycbcr())
                .collect::<Vec<pixel::YCbCr>>(),
        )
    }
}

pub struct YCbCrImage(pub Vec<pixel::YCbCr>);

impl YCbCrImage {
    pub fn from_block_matrices(ys: &BlockMatrix, cb: &BlockMatrix, cr: &BlockMatrix) -> Self {
        let ys_flat = ys.flatten();
        let cb_flat = cb.flatten();
        let cr_flat = cr.flatten();

        let pixels = ys_flat
            .iter()
            .zip(cb_flat.iter())
            .zip(cr_flat.iter())
            .map(|((y, cb), cr)| pixel::YCbCr {
                y: *y,
                cb: *cb,
                cr: *cr,
            })
            .collect::<Vec<pixel::YCbCr>>();
        Self(pixels)
    }

    pub fn to_rgb_image(&self) -> RGBImage {
        RGBImage(
            self.0
                .iter()
                .map(|ycbcr| ycbcr.to_rgb())
                .collect::<Vec<pixel::RGB>>(),
        )
    }

    pub fn to_ys_channel(&self) -> Vec<u8> {
        self.0.iter().map(|x| x.y).collect::<Vec<u8>>()
    }

    pub fn to_cbs_channel(&self) -> Vec<u8> {
        self.0.iter().map(|x| x.cb).collect::<Vec<u8>>()
    }

    pub fn to_crs_channel(&self) -> Vec<u8> {
        self.0.iter().map(|x| x.cr).collect::<Vec<u8>>()
    }
}
