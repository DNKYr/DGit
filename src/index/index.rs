use super::entry::{IndexEntry, ENTRY_FIXED_SIZE, FLAG_NAME_MASK};
use crate::repository::{GitRepository, repo_file, repo_path};
use sha1::{Digest, Sha1};
use std::fs;
use std::io::{self, Write};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Index {
    pub version: u32,
    pub entries: Vec<IndexEntry>,
}

impl Default for Index {
    fn default() -> Self {
        Index {
            version: 2,
            entries: Vec::new(),
        }
    }
}

impl Index {
    pub fn add_entry(&mut self, entry: IndexEntry) {
        self.entries.retain(|e| e.path != entry.path);
        let pos = self
            .entries
            .binary_search_by(|e| {
                e.path
                    .cmp(&entry.path)
                    .then(e.stage().cmp(&entry.stage()))
            })
            .unwrap_or_else(|i| i);
        self.entries.insert(pos, entry);
    }

    pub fn remove_entries_for_path(&mut self, path: &[u8]) {
        self.entries.retain(|e| e.path != path);
    }
}

fn u32_from_be(buf: &[u8], offset: usize) -> io::Result<u32> {
    if offset + 4 > buf.len() {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Truncated data"));
    }
    Ok(u32::from_be_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ]))
}

fn u16_from_be(buf: &[u8], offset: usize) -> io::Result<u16> {
    if offset + 2 > buf.len() {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Truncated data"));
    }
    Ok(u16::from_be_bytes([buf[offset], buf[offset + 1]]))
}

fn write_u32_be(buf: &mut Vec<u8>, val: u32) {
    buf.extend_from_slice(&val.to_be_bytes());
}

fn write_u16_be(buf: &mut Vec<u8>, val: u16) {
    buf.extend_from_slice(&val.to_be_bytes());
}

fn parse_entry(data: &[u8], offset: &mut usize) -> io::Result<IndexEntry> {
    let entry_start = *offset;

    if *offset + ENTRY_FIXED_SIZE > data.len() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Truncated index entry",
        ));
    }

    let ctime_sec = u32_from_be(data, *offset)?;
    *offset += 4;
    let ctime_ns = u32_from_be(data, *offset)?;
    *offset += 4;
    let mtime_sec = u32_from_be(data, *offset)?;
    *offset += 4;
    let mtime_ns = u32_from_be(data, *offset)?;
    *offset += 4;
    let dev = u32_from_be(data, *offset)?;
    *offset += 4;
    let ino = u32_from_be(data, *offset)?;
    *offset += 4;
    let mode = u32_from_be(data, *offset)?;
    *offset += 4;
    let uid = u32_from_be(data, *offset)?;
    *offset += 4;
    let gid = u32_from_be(data, *offset)?;
    *offset += 4;
    let file_size = u32_from_be(data, *offset)?;
    *offset += 4;

    if *offset + 20 > data.len() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Truncated SHA in index entry",
        ));
    }
    let sha: [u8; 20] = data[*offset..*offset + 20].try_into().unwrap();
    *offset += 20;

    let flags = u16_from_be(data, *offset)?;
    *offset += 2;

    let name_len = (flags & FLAG_NAME_MASK) as usize;
    if *offset + name_len > data.len() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Truncated path in index entry",
        ));
    }
    let path = data[*offset..*offset + name_len].to_vec();
    *offset += name_len;

    let entry_size = *offset - entry_start;
    let padding = if entry_size % 8 == 0 {
        8
    } else {
        8 - (entry_size % 8)
    };
    if *offset + padding > data.len() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Truncated padding in index entry",
        ));
    }
    *offset += padding;

    Ok(IndexEntry {
        ctime_sec,
        ctime_ns,
        mtime_sec,
        mtime_ns,
        dev,
        ino,
        mode,
        uid,
        gid,
        file_size,
        sha,
        flags,
        path,
    })
}

fn serialize_entry(entry: &IndexEntry) -> Vec<u8> {
    let mut buf = Vec::with_capacity(ENTRY_FIXED_SIZE + entry.path.len() + 8);

    write_u32_be(&mut buf, entry.ctime_sec);
    write_u32_be(&mut buf, entry.ctime_ns);
    write_u32_be(&mut buf, entry.mtime_sec);
    write_u32_be(&mut buf, entry.mtime_ns);
    write_u32_be(&mut buf, entry.dev);
    write_u32_be(&mut buf, entry.ino);
    write_u32_be(&mut buf, entry.mode);
    write_u32_be(&mut buf, entry.uid);
    write_u32_be(&mut buf, entry.gid);
    write_u32_be(&mut buf, entry.file_size);
    buf.extend_from_slice(&entry.sha);
    write_u16_be(&mut buf, entry.flags);
    buf.extend_from_slice(&entry.path);

    let current_len = buf.len();
    let padding = if current_len % 8 == 0 {
        8
    } else {
        8 - (current_len % 8)
    };
    buf.resize(current_len + padding, 0);

    buf
}

fn serialize_index_content(index: &Index) -> Vec<u8> {
    let mut buf = Vec::new();

    buf.extend_from_slice(b"DIRC");
    write_u32_be(&mut buf, index.version);
    write_u32_be(&mut buf, index.entries.len() as u32);

    for entry in &index.entries {
        buf.extend_from_slice(&serialize_entry(entry));
    }

    buf
}

fn compute_checksum(data: &[u8]) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(data);
    hasher
        .finalize()
        .as_slice()
        .try_into()
        .expect("SHA1 is 20 bytes")
}

pub fn read_index(repo: &GitRepository) -> io::Result<Index> {
    let path = repo_path(repo, &["index"]);
    let data = match fs::read(&path) {
        Ok(d) => d,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Ok(Index::default());
        }
        Err(e) => return Err(e),
    };

    if data.len() < 12 + 20 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Index file too short",
        ));
    }

    let signature = &data[0..4];
    if signature != b"DIRC" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Bad index signature: {:?}",
                String::from_utf8_lossy(signature)
            ),
        ));
    }

    let version = u32_from_be(&data, 4)?;
    if version != 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unsupported index version: {}", version),
        ));
    }

    let entry_count = u32_from_be(&data, 8)? as usize;
    let mut offset: usize = 12;

    let mut entries = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        let entry = parse_entry(&data, &mut offset)?;
        entries.push(entry);
    }

    let content_end = offset;
    if content_end + 20 > data.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Missing checksum in index file",
        ));
    }

    let content = &data[..content_end];
    let stored_checksum = &data[content_end..content_end + 20];
    let computed_checksum = compute_checksum(content);

    if computed_checksum.as_slice() != stored_checksum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Index checksum mismatch",
        ));
    }

    Ok(Index {
        version,
        entries,
    })
}

pub fn write_index(repo: &GitRepository, index: &Index) -> io::Result<()> {
    let mut sorted_entries = index.entries.clone();
    sorted_entries.sort_by(|a, b| a.path.cmp(&b.path).then(a.stage().cmp(&b.stage())));

    let sorted_index = Index {
        version: index.version,
        entries: sorted_entries,
    };

    let content = serialize_index_content(&sorted_index);
    let checksum = compute_checksum(&content);

    let path = repo_file(repo, &["index"], true)?;
    let mut file = fs::File::create(path)?;
    file.write_all(&content)?;
    file.write_all(&checksum)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::entry as index_entry;
    use crate::repository::GitRepository;
    use std::path::PathBuf;

    fn test_repo(name: &str) -> (PathBuf, GitRepository) {
        let root = std::env::temp_dir().join(name);
        let git_dir = root.join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();
        let repo = GitRepository::new(&root);
        (root, repo)
    }

    fn make_entry(path: &str, sha: [u8; 20], mode: u32) -> IndexEntry {
        let path_bytes = path.as_bytes().to_vec();
        let name_len = path_bytes.len();
        let flags = IndexEntry::make_flags(name_len, 0);
        IndexEntry::new(path_bytes, sha, mode, flags)
    }

    fn make_test_sha(val: u8) -> [u8; 20] {
        let mut sha = [0u8; 20];
        sha[0] = val;
        sha
    }

    #[test]
    fn empty_index_roundtrip() {
        let (_, repo) = test_repo("dgit-index-empty-roundtrip");
        let index = Index::default();
        write_index(&repo, &index).unwrap();
        let read = read_index(&repo).unwrap();
        assert!(read.entries.is_empty());
        assert_eq!(read.version, 2);
    }

    #[test]
    fn single_entry_roundtrip() {
        let (_, repo) = test_repo("dgit-index-single-roundtrip");
        let sha = make_test_sha(42);
        let path = "src/main.rs";
        let entry = make_entry(path, sha, index_entry::MODE_REGULAR);

        let mut index = Index::default();
        index.add_entry(entry);

        write_index(&repo, &index).unwrap();
        let read = read_index(&repo).unwrap();

        assert_eq!(read.entries.len(), 1);
        assert_eq!(read.entries[0].path, path.as_bytes());
        assert_eq!(read.entries[0].sha, sha);
    }

    #[test]
    fn multiple_entries_roundtrip() {
        let (_, repo) = test_repo("dgit-index-multi-roundtrip");
        let mut index = Index::default();
        index.add_entry(make_entry("b.txt", make_test_sha(1), index_entry::MODE_REGULAR));
        index.add_entry(make_entry("a.txt", make_test_sha(2), index_entry::MODE_REGULAR));
        index.add_entry(make_entry("c.txt", make_test_sha(3), index_entry::MODE_REGULAR));

        write_index(&repo, &index).unwrap();
        let read = read_index(&repo).unwrap();

        assert_eq!(read.entries.len(), 3);
        let paths: Vec<&[u8]> = read.entries.iter().map(|e| e.path.as_slice()).collect();
        assert_eq!(paths, vec![b"a.txt", b"b.txt", b"c.txt"]);
    }

    #[test]
    fn add_entry_replaces_same_path() {
        let (_, repo) = test_repo("dgit-index-replace-path");
        let mut index = Index::default();
        let sha1 = make_test_sha(1);
        let sha2 = make_test_sha(2);
        index.add_entry(make_entry("file.txt", sha1, index_entry::MODE_REGULAR));
        index.add_entry(make_entry("file.txt", sha2, index_entry::MODE_REGULAR));

        write_index(&repo, &index).unwrap();
        let read = read_index(&repo).unwrap();

        assert_eq!(read.entries.len(), 1);
        assert_eq!(read.entries[0].sha, sha2);
    }

    #[test]
    fn entries_sorted_by_path_then_stage() {
        let (_, repo) = test_repo("dgit-index-sort-order");
        let mut index = Index::default();
        let name_len_a = b"a.txt".len();
        let name_len_b = b"b.txt".len();
        let flags_a0 = IndexEntry::make_flags(name_len_a, 0);
        let flags_a1 = IndexEntry::make_flags(name_len_a, 1);
        let flags_b0 = IndexEntry::make_flags(name_len_b, 0);

        index.entries.push(IndexEntry::new(
            b"b.txt".to_vec(),
            make_test_sha(3),
            index_entry::MODE_REGULAR,
            flags_b0,
        ));
        index.entries.push(IndexEntry::new(
            b"a.txt".to_vec(),
            make_test_sha(1),
            index_entry::MODE_REGULAR,
            flags_a1,
        ));
        index.entries.push(IndexEntry::new(
            b"a.txt".to_vec(),
            make_test_sha(0),
            index_entry::MODE_REGULAR,
            flags_a0,
        ));

        write_index(&repo, &index).unwrap();
        let read = read_index(&repo).unwrap();

        assert_eq!(read.entries.len(), 3);
        let entries: Vec<(&[u8], u16)> =
            read.entries.iter().map(|e| (e.path.as_slice(), e.stage())).collect();
        assert_eq!(
            entries,
            vec![
                (b"a.txt".as_slice(), 0u16),
                (b"a.txt".as_slice(), 1u16),
                (b"b.txt".as_slice(), 0u16),
            ]
        );
    }

    #[test]
    fn read_missing_index_returns_empty() {
        let (_, repo) = test_repo("dgit-index-missing");
        let index = read_index(&repo).unwrap();
        assert!(index.entries.is_empty());
        assert_eq!(index.version, 2);
    }

    #[test]
    fn rejects_bad_signature() {
        let (_, repo) = test_repo("dgit-index-bad-signature");
        let path = repo_file(&repo, &["index"], true).unwrap();
        std::fs::write(&path, b"XXXX").unwrap();
        let err = read_index(&repo).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_unsupported_version() {
        let (_, repo) = test_repo("dgit-index-bad-version");
        let path = repo_file(&repo, &["index"], true).unwrap();
        let mut data = Vec::new();
        data.extend_from_slice(b"DIRC");
        write_u32_be(&mut data, 99);
        write_u32_be(&mut data, 0);
        let checksum = compute_checksum(&data);
        data.extend_from_slice(&checksum);
        std::fs::write(&path, data).unwrap();
        let err = read_index(&repo).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_checksum_mismatch() {
        let (_, repo) = test_repo("dgit-index-bad-checksum");
        let path = repo_file(&repo, &["index"], true).unwrap();
        let mut data = Vec::new();
        data.extend_from_slice(b"DIRC");
        write_u32_be(&mut data, 2);
        write_u32_be(&mut data, 0);
        data.extend_from_slice(&[0u8; 20]);
        std::fs::write(&path, data).unwrap();
        let err = read_index(&repo).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn preserves_stat_metadata_roundtrip() {
        let (_, repo) = test_repo("dgit-index-stat-roundtrip");
        let mut index = Index::default();
        let mut entry = make_entry("main.rs", make_test_sha(7), index_entry::MODE_EXECUTABLE);
        entry.ctime_sec = 1234567890;
        entry.ctime_ns = 500;
        entry.mtime_sec = 1234567899;
        entry.mtime_ns = 700;
        entry.dev = 2050;
        entry.ino = 12345;
        entry.uid = 1000;
        entry.gid = 1000;
        entry.file_size = 4096;
        index.add_entry(entry);

        write_index(&repo, &index).unwrap();
        let read = read_index(&repo).unwrap();

        assert_eq!(read.entries.len(), 1);
        let e = &read.entries[0];
        assert_eq!(e.ctime_sec, 1234567890);
        assert_eq!(e.ctime_ns, 500);
        assert_eq!(e.mtime_sec, 1234567899);
        assert_eq!(e.mtime_ns, 700);
        assert_eq!(e.dev, 2050);
        assert_eq!(e.ino, 12345);
        assert_eq!(e.mode, index_entry::MODE_EXECUTABLE);
        assert_eq!(e.uid, 1000);
        assert_eq!(e.gid, 1000);
        assert_eq!(e.file_size, 4096);
    }
}
