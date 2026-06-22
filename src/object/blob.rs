#[derive(Debug)]
pub struct BlobObject {
    pub data: Vec<u8>,
}

impl BlobObject {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}
