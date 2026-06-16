mod blob;
mod commit;
mod kind;
mod tree;
use crate::repository::{self, GitRepository};

pub use blob::BlobObject;
pub use commit::{CommitObject, kvlm_parse, kvlm_serialize};
pub use kind::ObjectKind;
pub use tree::{TreeObject, tree_parse, tree_serialize};

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::PathBuf,
};

pub enum GitObject {
    Blob(BlobObject),
    Commit(CommitObject),
    Tag(CommitObject),
    Tree(TreeObject),
}

impl GitObject {
    pub fn write(&self, repo: Option<&repository::GitRepository>) -> io::Result<String> {
        // serialize the data
        let data = self.serialize()?;

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
            let path = repository::repo_file(&r, &["objects", &sha[0..2], &sha[2..]], true)?;

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

    pub fn serialize(&self) -> io::Result<Vec<u8>> {
        match self {
            GitObject::Blob(blob) => Ok(blob.data.clone()),
            GitObject::Commit(commit) => Ok(kvlm_serialize(&commit.kvlm)),
            GitObject::Tree(tree) => Ok(tree_serialize(&tree.items)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unimplemented/Not existing object type",
            )),
        }
    }
}

pub fn read_object(repo: &repository::GitRepository, sha: &str) -> io::Result<GitObject> {
    let path: PathBuf = repository::repo_file(repo, &["objects", &sha[0..2], &sha[2..]], false)?;
    if !path.is_file() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Object not found"));
    }

    let stream = File::open(&path)?;
    let mut raw = Vec::new();
    ZlibDecoder::new(stream).read_to_end(&mut raw)?;

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
        return Err(io::Error::other(format!(
            "Malformed object {}: bad length",
            sha
        )));
    }

    // 4 match the type and handle data
    let data = raw[y + 1..].to_vec();

    match fmt {
        b"blob" => {
            let blob = BlobObject::new(data);
            Ok(GitObject::Blob(blob))
        }
        b"commit" => {
            let kvlm = kvlm_parse(&data, None, None)?;
            let commit = CommitObject::new(kvlm);
            Ok(GitObject::Commit(commit))
        }

        b"tree" => {
            let items = tree_parse(&data)?;
            let tree = TreeObject::new(items);
            Ok(GitObject::Tree(tree))
        }
        // Unimplemented object reading
        b"tag" => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "tag object isn't supported yet to read",
        )),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Non-existing object type",
        )),
    }
}

pub fn find_object(
    repo: &GitRepository,
    name: &str,
    fmt: Option<ObjectKind>,
    follow: bool,
) -> String {
    name.to_string()
}
