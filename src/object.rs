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

impl BlobObject {
    fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

pub fn read_object(repo: repo::GitRepository, sha: &str) -> io::Result<GitObject> {
    let path: PathBuf = repo::repo_file(&repo, &["objects", &sha[0..2], &sha[2..]], None)?;
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
            format!("Malformed object { }: bad length", sha),
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
