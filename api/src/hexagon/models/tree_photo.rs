#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TreePhotoVariant {
    Full,
    Thumbnail,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreePhoto {
    pub full_webp: Vec<u8>,
    pub thumbnail_webp: Vec<u8>,
}
