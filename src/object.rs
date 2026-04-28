use crate::repo;

use compress::zlib;
use flate2::Compression;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::PathBuf,
    process::exit,
};

enum GitObject {
    Blob(BlobObject),
    Commit(Vec<u8>),
    Tag(Vec<u8>),
    Tree(Vec<u8>),
}

struct BlobObject {
    data: Vec<u8>,
}

impl GitObject {
    fn write(&self, repo: Option<&repo::GitRepository>) -> io::Result<String> {
        // serialize the data
        let data = self.serialize();

        // Handle fmt and size
        let fmt_bytes = self.get_format().as_bytes();
        let size_bytes = data.len().to_string().into_bytes();

        // Construct the <fmt> <size>\0<data>
        let mut result = Vec::new();
        result.extend_from_slice(fmt_bytes);
        result.push(b' ');
        result.extend_from_slice(&size_bytes);
        result.push(0);
        result.extend_from_slice(&data);

        // Calculate the hash
        let mut hasher = Sha1::new();
        hasher.update(&result);
        let sha = hex::encode(hasher.finalize());

        if let Some(r) = repo {
            let path =
                repo::repo_file(&r, &["objects", &sha[0..2], &sha[2..]], Option::from(true))?;

            if !path.exists() {
                let file = fs::File::create(path)?;
                let mut encoder = ZlibEncoder::new(file, Compression::default());
                encoder.write_all(&result)?;
                encoder.finish()?;
            }
        }

        Ok(sha)
    }

    fn get_format(&self) -> &str {
        match self {
            GitObject::Blob(_) => "blob",
            GitObject::Commit(_) => "commit",
            GitObject::Tag(_) => "tag",
            GitObject::Tree(_) => "tree",
        }
    }

    fn serialize(&self) -> Vec<u8> {
        match self {
            GitObject::Blob(blob) => blob.data.clone(),
            GitObject::Commit(data) => data.clone(),
            _ => {
                println!("Unimplemented ");
                exit(1);
            }
        }
    }
}

impl BlobObject {
    fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

pub fn read_object(repo: &repo::GitRepository, sha: &str) -> io::Result<GitObject> {
    let path: PathBuf = repo::repo_file(repo, &["objects", &sha[0..2], &sha[2..]], None)?;
    if !path.is_file() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Object not found"));
    }

    let stream = File::open(&path)?;
    let mut raw = Vec::new();
    zlib::Decoder::new(stream).read_to_end(&mut raw)?;

    // 1. Find the first space (type delimiter)
    let x = raw
        .iter()
        .position(|&b| b == b' ')
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing Space"))?;

    let fmt = &raw[0..x];

    // 2. Find the null byte afte index x (size delimiter)
    let y = raw[x..]
        .iter()
        .position(|&b| b == 0)
        .map(|i| i + x)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing null byte"))?;

    // 3. Parse and validate size
    let size_str = std::str::from_utf8(&raw[x + 1..y])
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid size encoder"))?;

    let size: usize = size_str
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid size header"))?;

    if size != raw.len() - y - 1 {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Malformed object {}: bad length", sha),
        ));
    }

    // 4 match the type and handle data
    let data = &raw[y + 1..];

    match fmt {
        b"blob" => {
            let blob = BlobObject::new(raw[y + 1..].to_vec());
            Ok(GitObject::Blob(blob))
        }

        // Unimplemented object reading
        b"tree" => exit(1),
        b"tag" => exit(1),
        b"commit" => exit(1),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Non-existing object type",
            ));
        }
    }
}
