use lyon::path::Path;

/// An vector analogy to ImageTexture.
pub struct VectorTexture {
    pub size: (f32, f32),
    pub paths: Vec<Path>,
}

pub struct VectorServer {}
