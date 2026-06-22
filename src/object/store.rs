use super::{BlobObject, CommitObject, GitObject, TreeObject, kvlm_parse, tree_parse};
use crate::{
    object::TagObject,
    repository::{GitRepository, repo_file},
};
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha1::{Digest, Sha1};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::PathBuf,
};
pub fn read_object(repo: &GitRepository, sha: &str) -> io::Result<GitObject> {
    let path: PathBuf = repo_file(repo, &["objects", &sha[0..2], &sha[2..]], false)?;
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
        b"tag" => {
            let kvlm = kvlm_parse(&data, None, None)?;
            let tag = TagObject::new(kvlm);
            Ok(GitObject::Tag(tag))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Non-existing object type",
        )),
    }
}

pub fn write_object(object: &GitObject, repo: Option<&GitRepository>) -> io::Result<String> {
    // serialize the data
    let data = object.serialize()?;

    // Handle fmt and size
    let fmt_bytes = object.kind().as_str().as_bytes();
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
        let path = repo_file(r, &["objects", &sha[0..2], &sha[2..]], true)?;

        if !path.exists() {
            let file = fs::File::create(path)?;
            let mut encoder = ZlibEncoder::new(file, Compression::default());
            encoder.write_all(&result)?;
            encoder.finish()?;
        }
    }

    Ok(sha)
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_repo(name: &str) -> (PathBuf, GitRepository) {
        let root = std::env::temp_dir().join(name);
        let git_dir = root.join(".git");

        std::fs::create_dir_all(git_dir).unwrap();

        let repo = GitRepository::new(&root);
        (root, repo)
    }

    fn write_object_blob_test(blob_content: &[u8], repo: Option<&GitRepository>) -> String {
        let object = GitObject::Blob(BlobObject::new(blob_content.to_vec()));
        write_object(&object, repo).unwrap()
    }

    fn write_raw_object(repo: &GitRepository, sha: &str, raw: &[u8]) {
        let path = repo_file(repo, &["objects", &sha[0..2], &sha[2..]], true).unwrap();
        let file = fs::File::create(path).unwrap();
        let mut encoder = ZlibEncoder::new(file, Compression::default());

        encoder.write_all(raw).unwrap();
        encoder.finish().unwrap();
    }

    fn test_expected_vs_actual(blob_content: Vec<u8>, actual_sha: &str) {
        let format_bytes = b"blob";
        let size_bytes = blob_content.len().to_string().into_bytes();
        let mut hasher = Sha1::new();

        let mut result = Vec::new();
        result.extend_from_slice(format_bytes);
        result.push(b' ');
        result.extend_from_slice(&size_bytes);
        result.push(0);
        result.extend_from_slice(blob_content.as_slice());
        hasher.update(&result);
        let expected_sha = hex::encode(hasher.finalize());
        assert_eq!(actual_sha, expected_sha);
    }

    #[test]
    fn write_object_repo_is_none_blob() {
        let blob_content = b"hello world".to_vec();
        let actual_sha = write_object_blob_test(&blob_content, None);
        test_expected_vs_actual(blob_content, &actual_sha);
    }

    #[test]
    fn write_object_and_write_blob() {
        let (_, repo) = test_repo("dgit-store-test-write-blob");
        let blob_content = b"This is writing blob test".to_vec();
        let actual_sha = write_object_blob_test(&blob_content, Some(&repo));

        // first test computed_sha vs actual_sha for accuracy
        test_expected_vs_actual(blob_content, &actual_sha);

        // Then check if the sha actually exists in the test repo
        repo_file(
            &repo,
            &["objects", &actual_sha[0..2], &actual_sha[2..]],
            false,
        )
        .unwrap();
    }

    #[test]
    fn write_then_read_blob_roundtrip() {
        let (_, repo) = test_repo("dgit-store-roundtrip-test");
        let blob_content = b"roundtrip test".to_vec();
        let actual_sha = write_object_blob_test(&blob_content, Some(&repo));

        test_expected_vs_actual(blob_content.clone(), &actual_sha);

        let read_blob_content = match read_object(&repo, &actual_sha).unwrap() {
            GitObject::Blob(b) => b.data,
            _ => panic!("The return git object type is not blob"),
        };

        assert_eq!(read_blob_content, blob_content);
    }

    #[test]
    fn read_object_rejects_non_existing_object() {
        let (_, repo) = test_repo("dgit-store-read-non-existing-object-test");
        let err = read_object(&repo, "1234567890abcdefghij").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn read_object_rejects_missing_space_in_header() {
        let (_, repo) = test_repo("dgit-store-malformed-missing-space");
        let sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        write_raw_object(&repo, sha, b"blob5\0hello");

        let err = read_object(&repo, sha).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn read_object_rejects_missing_null_byte_in_header() {
        let (_, repo) = test_repo("dgit-store-malformed-missing-null");
        let sha = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        write_raw_object(&repo, sha, b"blob 5hello");

        let err = read_object(&repo, sha).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn read_object_rejects_invalid_size_header() {
        let (_, repo) = test_repo("dgit-store-malformed-invalid-size");
        let sha = "cccccccccccccccccccccccccccccccccccccccc";
        write_raw_object(&repo, sha, b"blob five\0hello");

        let err = read_object(&repo, sha).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn read_object_rejects_bad_length() {
        let (_, repo) = test_repo("dgit-store-malformed-bad-length");
        let sha = "dddddddddddddddddddddddddddddddddddddddd";
        write_raw_object(&repo, sha, b"blob 10\0hello");

        let err = read_object(&repo, sha).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::Other);
    }
}
