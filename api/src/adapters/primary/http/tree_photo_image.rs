use std::io::Cursor;

use image::{DynamicImage, ImageDecoder, ImageReader, imageops::FilterType, metadata::Orientation};

pub const MAX_SOURCE_PHOTO_BYTES: usize = 40 * 1024 * 1024;
const MAX_SOURCE_PIXELS: u64 = 80_000_000;

pub struct PreparedTreePhoto {
    pub full_webp: Vec<u8>,
    pub thumbnail_webp: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub enum TreePhotoPreparationError {
    InvalidImage,
    ImageTooLarge,
    CouldNotEncode,
}

pub fn prepare_tree_photo(source: &[u8]) -> Result<PreparedTreePhoto, TreePhotoPreparationError> {
    if source.len() > MAX_SOURCE_PHOTO_BYTES {
        return Err(TreePhotoPreparationError::ImageTooLarge);
    }

    let reader = ImageReader::new(Cursor::new(source))
        .with_guessed_format()
        .map_err(|_| TreePhotoPreparationError::InvalidImage)?;
    let mut decoder = reader
        .into_decoder()
        .map_err(|_| TreePhotoPreparationError::InvalidImage)?;
    let (width, height) = decoder.dimensions();
    if width == 0
        || height == 0
        || u64::from(width).saturating_mul(u64::from(height)) > MAX_SOURCE_PIXELS
    {
        return Err(TreePhotoPreparationError::ImageTooLarge);
    }
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let mut image =
        DynamicImage::from_decoder(decoder).map_err(|_| TreePhotoPreparationError::InvalidImage)?;
    image.apply_orientation(orientation);

    Ok(PreparedTreePhoto {
        full_webp: resize_and_encode(&image, 2400, 86.0)?,
        thumbnail_webp: resize_and_encode(&image, 480, 66.0)?,
    })
}

fn resize_and_encode(
    image: &DynamicImage,
    maximum_dimension: u32,
    quality: f32,
) -> Result<Vec<u8>, TreePhotoPreparationError> {
    let resized = if image.width() <= maximum_dimension && image.height() <= maximum_dimension {
        image.clone()
    } else {
        image.resize(maximum_dimension, maximum_dimension, FilterType::Triangle)
    };
    let rgba = resized.to_rgba8();
    webp::Encoder::from_rgba(rgba.as_raw(), rgba.width(), rgba.height())
        .encode_simple(false, quality)
        .map(|webp| webp.to_vec())
        .map_err(|_| TreePhotoPreparationError::CouldNotEncode)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use image::{DynamicImage, ImageFormat, RgbaImage};

    use super::{TreePhotoPreparationError, prepare_tree_photo};

    #[test]
    fn create_bounded_webp_variants_from_an_original_browser_image() {
        let image = DynamicImage::ImageRgba8(
            RgbaImage::from_raw(1200, 600, vec![127; 1200 * 600 * 4]).unwrap(),
        );
        let mut png = Cursor::new(Vec::new());
        image.write_to(&mut png, ImageFormat::Png).unwrap();

        let prepared = prepare_tree_photo(png.get_ref()).unwrap();

        assert!(is_webp(&prepared.full_webp));
        assert!(is_webp(&prepared.thumbnail_webp));
        let full = image::load_from_memory(&prepared.full_webp).unwrap();
        let thumbnail = image::load_from_memory(&prepared.thumbnail_webp).unwrap();
        assert_eq!((full.width(), full.height()), (1200, 600));
        assert_eq!((thumbnail.width(), thumbnail.height()), (480, 240));
    }

    #[test]
    fn reject_non_images_before_encoding() {
        assert_eq!(
            prepare_tree_photo(b"not an image").err(),
            Some(TreePhotoPreparationError::InvalidImage),
        );
    }

    fn is_webp(bytes: &[u8]) -> bool {
        bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP"
    }
}
