use std::{io::Cursor, str::FromStr};

use actix_web::cookie::time::format_description::modifier::TrailingInput;
use image::{
    ImageError, ImageFormat, ImageReader,
    imageops::{FilterType, resize},
};
use num_traits::AsPrimitive;
use serde_json::Value;

use crate::file_handling::{
    self,
    from_json::FromJson,
    video_processing::{
        thumbnail_processor::{DynClone, ThumbnailImageProcessor},
        video_processor::VideoProcessor,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct LocalThumbnailImageProcessor {
    id: u64,
    width: u32,
    height: u32,
    output_format: ImageFormat,
    filter_type: FilterType,
}

impl DynClone for LocalThumbnailImageProcessor {
    fn clone_dyn(self_val: &Self) -> Self
    where
        Self: Sized + DynClone,
    {
        self_val.clone()
    }
}

impl ThumbnailImageProcessor for LocalThumbnailImageProcessor {
    fn new(
        id: u64,
        width: u32,
        height: u32,
        output_format: &ImageFormat,
        filter_type: &FilterType,
    ) -> Self {
        Self {
            id,
            width,
            height,
            output_format: *output_format,
            filter_type: *filter_type,
        }
    }

    fn process(&self, raw_data: &[u8], output_dir: &std::path::Path) -> Result<(), ImageError> {
        //Read image:
        let mut reader = match ImageReader::new(Cursor::new(raw_data)).with_guessed_format() {
            Ok(res) => res,
            Err(e) => return Err(ImageError::IoError(e)),
        };
        let image = match reader.decode() {
            Ok(r) => r,
            Err(e) => return Err(e),
        };

        //Resize:
        let image_processed = resize(
            &image,
            self.get_width(),
            self.get_height(),
            self.get_filter_type(),
        );

        //Save image
        let output_dir_canon = match std::fs::canonicalize(output_dir) {
            Ok(r) => r,
            Err(e) => return Err(ImageError::IoError(e)),
        };
        if !{
            match std::fs::exists(&output_dir_canon) {
                Ok(v) => v,
                Err(_) => false,
            }
        } {
            return Err(ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "The directory \"{}\" does not exist!",
                    output_dir.to_string_lossy()
                ),
            )));
        }
        if !output_dir_canon.is_dir() {
            return Err(ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                format!(
                    "The path \"{}\" is not a directory!",
                    output_dir.to_string_lossy()
                ),
            )));
        }
        match image_processed.save_with_format(
            output_dir_canon.join(format!("./thumbnail.{}", self.get_extension())),
            self.get_output_format(),
        ) {
            Ok(_) => {}
            Err(e) => return Err(e),
        };

        Ok(())
    }

    fn get_id(&self) -> u64 {
        self.id
    }

    fn set_id(&mut self, id: u64) {
        self.id = id;
    }

    fn set_width(&mut self, width: u32) {
        self.width = width;
    }

    fn get_width(&self) -> u32 {
        self.width
    }

    fn set_height(&mut self, height: u32) {
        self.height = height;
    }

    fn get_height(&self) -> u32 {
        self.height
    }

    fn get_output_format(&self) -> ImageFormat {
        self.output_format
    }

    fn set_output_format(&mut self, output_format: &ImageFormat) {
        self.output_format = *output_format;
    }

    fn get_filter_type(&self) -> FilterType {
        self.filter_type
    }

    fn set_filter_type(&mut self, filter_type: &FilterType) {
        self.filter_type = *filter_type;
    }
}

fn string_to_image_format(input: &str) -> ImageFormat {
    match input {
        "png" => ImageFormat::Png,
        "jpg" => ImageFormat::Jpeg,
        "jpeg" => ImageFormat::Jpeg,
        "gif" => ImageFormat::Gif,
        "webp" => ImageFormat::WebP,
        "pnm" => ImageFormat::Pnm,
        "tiff" => ImageFormat::Tiff,
        "tga" => ImageFormat::Tga,
        "dds" => ImageFormat::Dds,
        "bmp" => ImageFormat::Dds,
        "ico" => ImageFormat::Ico,
        "hdr" => ImageFormat::Hdr,
        "exr" => ImageFormat::OpenExr,
        "farbfeld" => ImageFormat::Farbfeld,
        "avif" => ImageFormat::Avif,
        "qoi" => ImageFormat::Qoi,
        _ => unimplemented!("Unsupported file format!"),
    }
}

fn string_to_filter_type(input: &str) -> FilterType {
    match input.to_lowercase().as_str() {
        "nearest" => FilterType::Nearest,
        "triangle" => FilterType::Triangle,
        "catmullrom" => FilterType::CatmullRom,
        "gaussian" => FilterType::Gaussian,
        "lanczos3" => FilterType::Lanczos3,
        _ => unimplemented!("Unsupported filter type!"),
    }
}

impl FromJson for LocalThumbnailImageProcessor {
    ///Convert the given JSON data (json) to an object of the LocalThumbnailImageProcessor type.
    ///
    /// Parameters:
    ///     json: The given JSON data.
    ///
    /// JSON data format:
    /// `{"id": [processor's id as a u64], "width": [width of the processed version of the image], "height": [height of the processed version of the image], "output_format": "file extension of the output format (example "jpeg", "png", .etc)", "filter_type": "the filter type used to scale the image"}`
    fn from_json(json_data: &Value) -> Self {
        let id: u64 = {
            let id_obj = json_data.get("id").unwrap();
            id_obj.as_u64().unwrap()
        };
        let output_format: ImageFormat = {
            let out_obj = json_data.get("output_format").unwrap();
            string_to_image_format(out_obj.as_str().unwrap())
        };
        let filter_type: FilterType = {
            let filter_obj = json_data.get("filter_type").unwrap();
            string_to_filter_type(filter_obj.as_str().unwrap())
        };

        let width: u32 = {
            let input = json_data.get("width").unwrap();
            // input.as_str().unwrap().parse::<u32>().unwrap()
            input.as_u64().unwrap() as u32
        };
        let height: u32 = {
            let input = json_data.get("height").unwrap();
            // input.as_str().unwrap().parse::<u32>().unwrap()
            input.as_u64().unwrap() as u32
        };

        Self::new(id, width, height, &output_format, &filter_type)
    }
}
