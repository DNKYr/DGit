enum GitObject {
    Blob(BlobObject),
}

struct BlobObject {
    data: Vec<u8>,
}

impl BlobObject {
    fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

