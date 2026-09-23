#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TreePhotoVariant {
    Full,
    Thumbnail,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TreePhotoId(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreePhotoSummary {
    pub id: TreePhotoId,
    pub created_at_unix_seconds: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreePhoto {
    pub full_webp: Vec<u8>,
    pub thumbnail_webp: Vec<u8>,
}
