use crate::{
    cli,
    repository::{self, GitRepository},
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
};

pub enum ObjectKind {
    Blob,
    Commit,
    Tag,
    Tree,
}

impl ObjectKind {
    pub fn new(types: cli::ObjectMode) -> Self {
        match types {
            cli::ObjectMode::Blob => ObjectKind::Blob,
            cli::ObjectMode::Commit => ObjectKind::Commit,
            cli::ObjectMode::Tree => ObjectKind::Tree,
            cli::ObjectMode::Tag => ObjectKind::Tag,
        }
    }
}

pub enum GitObject {
    Blob(BlobObject),
    Commit(CommitObject),
    Tag(CommitObject),
    Tree(TreeObject),
}

pub struct BlobObject {
    pub data: Vec<u8>,
}

pub struct CommitObject {
    kvlm: IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>>,
}

pub struct TreeObject {
    items: Vec<TreeLeaf>,
}

#[derive(Clone)]
pub struct TreeLeaf {
    pub mode: Vec<u8>,
    pub path: Vec<u8>,
    pub sha: Vec<u8>,
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

impl BlobObject {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl CommitObject {
    pub fn new(kvlm: IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>>) -> Self {
        Self { kvlm }
    }

    pub fn get_kvlm(&self) -> &IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>> {
        &self.kvlm
    }
}

impl TreeObject {
    pub fn new(items: Vec<TreeLeaf>) -> Self {
        Self { items }
    }

    pub fn get_items(&self) -> &[TreeLeaf] {
        &self.items
    }
}

impl TreeLeaf {
    pub fn new(mode: Vec<u8>, path: Vec<u8>, sha: Vec<u8>) -> Self {
        Self { mode, path, sha }
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

fn kvlm_parse(
    raw: &[u8],
    start: Option<usize>,
    dct: Option<IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>>>,
) -> io::Result<IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>>> {
    let start = start.unwrap_or(0);
    let mut dct = dct.unwrap_or_default();

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

    kvlm_parse(raw, Some(end + 1), Some(dct))
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

fn tree_parse_one(raw: &[u8], start: Option<usize>) -> io::Result<(usize, TreeLeaf)> {
    // GitTree format: <mode> space <path> 0x00 <sha1>

    let start = start.unwrap_or(0);

    // find the index for the space
    let space_rel: usize = raw[start..]
        .iter()
        .position(|&b| b == b' ')
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Tree mode is null/empty. Doesn't contains a space in tree objects file",
            )
        })?;

    let space_index = start + space_rel;
    let mode_raw = &raw[start..space_index];

    // Get the tree mode
    let mode: Vec<u8> = match mode_raw.len() {
        // add zero at the beginning for 5 characters long mode
        5 | 6 => mode_raw.to_vec(),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unknown tree mode size: Tree mode should be 6 bytes long",
            ));
        }
    };

    // Find the index for null byte
    let null_rel: usize = raw[space_index..]
        .iter()
        .position(|&b| b == b'\x00')
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing null bytes"))?;

    let null_index = space_index + null_rel;

    // Get the tree path
    let path = raw[space_index + 1..null_index].to_vec();

    // Get the sha
    let sha_start = null_index + 1;
    let sha_end = null_index + 21;

    if sha_end > raw.len() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Truncated SHA",
        ));
    }
    let sha: Vec<u8> = raw[sha_start..sha_end].to_vec();

    Ok((sha_end, TreeLeaf::new(mode, path, sha)))
}

pub fn tree_parse(raw: &[u8]) -> io::Result<Vec<TreeLeaf>> {
    let len = raw.len();
    let mut pos = 0;
    let mut ret: Vec<TreeLeaf> = Vec::new();

    // Loops through the tree object to parse
    while pos < len {
        let t: TreeLeaf;
        (pos, t) = tree_parse_one(raw, Some(pos))?;
        ret.push(t);
    }
    Ok(ret)
}

pub fn tree_serialize(obj: &[TreeLeaf]) -> Vec<u8> {
    let mut sorted_obj: Vec<TreeLeaf> = obj.to_vec();

    sorted_obj.sort_by(|a: &TreeLeaf, b: &TreeLeaf| {
        let mut name_a = a.path.to_vec();
        if a.mode.starts_with(&[b'4']) {
            name_a.push(b'/');
        }

        let mut name_b = b.path.to_vec();
        if b.mode.starts_with(&[b'4']) {
            name_b.push(b'/');
        }

        name_a.cmp(&name_b)
    });

    let mut ret: Vec<u8> = Vec::new();

    for i in sorted_obj {
        ret.extend(i.mode);
        ret.push(b' ');
        ret.extend(i.path);
        ret.push(b'\x00');
        ret.extend(i.sha);
    }
    ret
}
