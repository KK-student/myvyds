use std::{
    any::Any,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use image::{ImageError, ImageFormat, imageops::FilterType};

use crate::file_handling::{
    from_json::FromJson, video_processing::video_processor::VideoProcessor,
};

pub struct DynCloneBox<T: DynClone + ?Sized> {
    value: Box<T>,
}

impl<T: DynClone> DynCloneBox<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Box::new(value),
        }
    }

    // pub fn get(&self) -> &<Self as Deref>::Target {
    //     self.value.deref()
    // }

    pub fn get_box(&self) -> &Box<T> {
        &self.value
    }

    pub fn get_box_mut(&mut self) -> &mut Box<T> {
        &mut self.value
    }
}

impl<T: DynClone> Clone for DynCloneBox<T> {
    fn clone(&self) -> Self {
        let data = self.value.deref();
        Self {
            value: Box::new(DynClone::clone_dyn(data)),
        }
    }
}

impl<T: DynClone> DerefMut for DynCloneBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value.deref_mut()
    }
}

impl<T: DynClone> Deref for DynCloneBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value.deref()
    }
}

pub trait DynClone {
    fn clone_dyn(self_value: &Self) -> Self
    where
        Self: Sized + DynClone;
}

pub trait ThumbnailVideoProcessor: VideoProcessor + Debug {}

pub trait ThumbnailImageProcessor: Debug + Any + DynClone + FromJson {
    fn new(
        id: u64,
        width: u32,
        height: u32,
        output_format: &ImageFormat,
        filter_type: &FilterType,
    ) -> Self
    where
        Self: Sized;

    fn process(&self, raw_data: &[u8], output_dir: &std::path::Path) -> Result<(), ImageError>;

    fn get_id(&self) -> u64;

    fn set_id(&mut self, id: u64);

    fn set_width(&mut self, width: u32);

    fn get_width(&self) -> u32;

    fn set_height(&mut self, height: u32);

    fn get_height(&self) -> u32;

    fn get_output_format(&self) -> ImageFormat;

    fn set_output_format(&mut self, output_format: &ImageFormat);

    fn get_filter_type(&self) -> FilterType;

    fn set_filter_type(&mut self, filter_type: &FilterType);

    fn get_extension(&self) -> String {
        match self.get_output_format() {
            ImageFormat::Png => "png".to_string(),
            ImageFormat::Jpeg => "jpg".to_string(),
            ImageFormat::Gif => "gif".to_string(),
            ImageFormat::WebP => "webp".to_string(),
            ImageFormat::Pnm => "pnm".to_string(),
            ImageFormat::Tiff => "tiff".to_string(),
            ImageFormat::Tga => "tga".to_string(),
            ImageFormat::Dds => "dds".to_string(),
            ImageFormat::Bmp => "bmp".to_string(),
            ImageFormat::Ico => "ico".to_string(),
            ImageFormat::Hdr => "hdr".to_string(),
            ImageFormat::OpenExr => "exr".to_string(),
            ImageFormat::Farbfeld => "farbfeld".to_string(),
            ImageFormat::Avif => "avif".to_string(),
            ImageFormat::Qoi => "qoi".to_string(),
            _ => "bin".to_string(),
        }
    }
}
