use crate::{
    cli,
    repo::{self, GitRepository},
};

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use indexmap::IndexMap;
use sha1::{Digest, Sha1};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::PathBuf,
    process::exit,
};

pub enum GitObject {
    Blob(BlobObject),
    Commit(CommitObject),
    Tag(Vec<u8>),
    Tree(Vec<u8>),
}

pub struct BlobObject {
    data: Vec<u8>,
}

pub struct CommitObject {
    kvlm: IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>>,
}

impl GitObject {
    pub fn write(&self, repo: Option<&repo::GitRepository>) -> io::Result<String> {
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

    pub fn serialize(&self) -> io::Result<Vec<u8>> {
        match self {
            GitObject::Blob(blob) => Ok(blob.data.clone()),
            GitObject::Commit(commit) => Ok(kvlm_serialize(&commit.kvlm)),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unimplemented/Not existing object type",
            )),
        }
    }
}

impl BlobObject {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl CommitObject {
    pub fn new(kvlm: IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>>) -> Self {
        Self { kvlm }
    }
}

pub fn read_object(repo: &repo::GitRepository, sha: &str) -> io::Result<GitObject> {
    let path: PathBuf = repo::repo_file(repo, &["objects", &sha[0..2], &sha[2..]], None)?;
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
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Malformed object {}: bad length", sha),
        ));
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

        // Unimplemented object reading
        b"tree" => exit(1),
        b"tag" => exit(1),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Non-existing object type",
            ));
        }
    }
}

pub fn find_object(
    repo: &GitRepository,
    name: &String,
    fmt: Option<cli::CatFileMode>,
    follow: Option<bool>,
) -> String {
    name.clone()
}

fn kvlm_parse(
    raw: &Vec<u8>,
    start: Option<usize>,
    dct: Option<IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>>>,
) -> io::Result<IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>>> {
    let start = start.unwrap_or(0);
    let mut dct = dct.unwrap_or(IndexMap::new());

    let spc: Option<usize> = raw[start..]
        .iter()
        .position(|&b| b == b' ')
        .map(|p| p + start);

    let nl = raw[start..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|p| p + start);

    let is_message_body = match (spc, nl) {
        (None, _) => true,
        (Some(s), Some(n)) if n < s => true,
        _ => false,
    };

    if is_message_body {
        let nl_pos =
            nl.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing newline"))?;

        if nl_pos != start {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Malformed header: newline not at start",
            ));
        }

        let mut msg: Vec<Vec<u8>> = Vec::new();

        msg.push(raw[start + 1..].to_vec());

        dct.insert(None, msg);
        return Ok(dct);
    }

    let spc = spc.unwrap();

    let key = raw[start..spc].to_vec();

    let end = raw[start..]
        .windows(2)
        .position(|w| w[0] == b'\n' && w[1] != b' ')
        .map(|p| p + start)
        .unwrap();

    let value = raw[spc + 1..end].to_vec();

    dct.entry(Some(key)).or_insert_with(Vec::new).push(value);

    return kvlm_parse(raw, Some(end + 1), Some(dct));
}

fn kvlm_serialize(kvlm: &IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>>) -> Vec<u8> {
    let mut ret = Vec::new();

    for (key, value) in kvlm {
        let k = match key {
            Some(k) => k,
            None => continue,
        };

        for v in value {
            let v_processed: Vec<u8> = v
                .iter()
                .flat_map(|&b| {
                    if b == b'\n' {
                        vec![b'\n', b' ']
                    } else {
                        vec![b]
                    }
                })
                .collect();
            ret.extend_from_slice(k);
            ret.push(b' ');
            ret.extend_from_slice(&v_processed);
            ret.push(b'\n');
        }
    }
    ret.push(b'\n');
    if let Some(msg) = kvlm.get(&None) {
        ret.extend_from_slice(&msg[0]);
    }
    ret
}
