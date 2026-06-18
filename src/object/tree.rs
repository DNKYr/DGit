use std::io;
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct TreeObject {
    pub items: Vec<TreeLeaf>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct TreeLeaf {
    pub mode: Vec<u8>,
    pub path: Vec<u8>,
    pub sha: Vec<u8>,
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
        if a.mode.starts_with(b"4") {
            name_a.push(b'/');
        }

        let mut name_b = b.path.to_vec();
        if b.mode.starts_with(b"4") {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_regular_file_tree_entry() {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"100644 hello.txt\0");
        raw.extend_from_slice(&[1u8; 20]);

        let entries = tree_parse(&raw).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0],
            TreeLeaf::new(b"100644".to_vec(), b"hello.txt".to_vec(), vec![1u8; 20])
        );
    }

    #[test]
    fn parse_directory_tree_entry() {
        let mut raw: Vec<u8> = Vec::new();
        raw.extend_from_slice(b"40000 src\0");
        raw.extend_from_slice(&[9u8; 20]);

        let entries = tree_parse(&raw).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0],
            TreeLeaf::new(b"40000".to_vec(), b"src".to_vec(), vec![9u8; 20])
        );
    }

    #[test]
    fn parse_multiple_tree_entry() {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"100644 hello.txt\0");
        raw.extend_from_slice(&[1u8; 20]);

        raw.extend_from_slice(b"100644 hello.cpp\0");
        raw.extend_from_slice(&[2u8; 20]);

        raw.extend_from_slice(b"40000 hello_src\0");
        raw.extend_from_slice(&[4u8; 20]);

        let entries = tree_parse(&raw).unwrap();

        assert_eq!(entries.len(), 3);
        assert_eq!(
            entries[0],
            TreeLeaf::new(b"100644".to_vec(), b"hello.txt".to_vec(), vec![1u8; 20])
        );
        assert_eq!(
            entries[1],
            TreeLeaf::new(b"100644".to_vec(), b"hello.cpp".to_vec(), vec![2u8; 20])
        );
        assert_eq!(
            entries[2],
            TreeLeaf::new(b"40000".to_vec(), b"hello_src".to_vec(), vec![4u8; 20])
        );
    }

    #[test]
    fn parse_tree_entry_error_catching() {
        // Test case with invalid mode
        let mut panic_raw = Vec::new();
        panic_raw.extend_from_slice(b"1111 wrong_mode.err\0");
        panic_raw.extend_from_slice(&[1u8; 20]);

        let err = tree_parse(&panic_raw).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);

        // Test case without the zero bytes
        let mut panic_raw = Vec::new();
        panic_raw.extend_from_slice(b"100644 hello.txt");
        panic_raw.extend_from_slice(&[1u8; 20]);

        let err = tree_parse(&panic_raw).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_truncated_sha() {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"100644 hello.txt\0");
        raw.extend_from_slice(&[1u8; 19]);

        let err = tree_parse(&raw).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn serializes_tree_entry() {
        let entry = TreeLeaf::new(b"100644".to_vec(), b"hello.txt".to_vec(), vec![1u8; 20]);

        let raw = tree_serialize(&[entry]);

        let mut expected = Vec::new();
        expected.extend_from_slice(b"100644 hello.txt\0");
        expected.extend_from_slice(&[1u8; 20]);

        assert_eq!(raw, expected);
    }

    #[test]
    fn sorting_in_tree_serialize() {
        let entries = [
            TreeLeaf::new(b"100644".to_vec(), b"hello.txt".to_vec(), vec![1u8; 20]),
            TreeLeaf::new(b"100644".to_vec(), b"hello.cpp".to_vec(), vec![2u8; 20]),
            TreeLeaf::new(b"40000".to_vec(), b"hello_src".to_vec(), vec![4u8; 20]),
        ];

        let raw = tree_serialize(&entries);

        let mut expected = Vec::new();
        expected.extend_from_slice(b"100644 hello.cpp\0");
        expected.extend_from_slice(&[2u8; 20]);
        expected.extend_from_slice(b"100644 hello.txt\0");
        expected.extend_from_slice(&[1u8; 20]);
        expected.extend_from_slice(b"40000 hello_src\0");
        expected.extend_from_slice(&[4u8; 20]);

        assert_eq!(raw, expected);
    }

    #[test]
    fn sorts_directories_with_trailing_slash_semantics() {
        let entries = [
            TreeLeaf::new(b"100644".to_vec(), b"foo.txt".to_vec(), vec![1u8; 20]),
            TreeLeaf::new(b"40000".to_vec(), b"foo".to_vec(), vec![2u8; 20]),
            TreeLeaf::new(b"100644".to_vec(), b"foo".to_vec(), vec![3u8; 20]),
        ];

        let raw = tree_serialize(&entries);

        let mut expected = Vec::new();
        expected.extend_from_slice(b"100644 foo\0");
        expected.extend_from_slice(&[3u8; 20]);
        expected.extend_from_slice(b"100644 foo.txt\0");
        expected.extend_from_slice(&[1u8; 20]);
        expected.extend_from_slice(b"40000 foo\0");
        expected.extend_from_slice(&[2u8; 20]);

        assert_eq!(raw, expected);
    }

    #[test]
    fn parse_then_serialize_round_trip_tree() {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"100644 MAKEFILE\0");
        raw.extend_from_slice(&[5u8; 20]);
        raw.extend_from_slice(b"100644 hello.py\0");
        raw.extend_from_slice(&[3u8; 20]);
        raw.extend_from_slice(b"100644 hello.rs\0");
        raw.extend_from_slice(&[2u8; 20]);
        raw.extend_from_slice(b"100644 hello.txt\0");
        raw.extend_from_slice(&[1u8; 20]);
        raw.extend_from_slice(b"100644 src/hello.s\0");
        raw.extend_from_slice(&[6u8; 20]);
        raw.extend_from_slice(b"100644 target/hello\0");
        raw.extend_from_slice(&[4u8; 20]);

        let entries = tree_parse(&raw).unwrap();
        let serialized = tree_serialize(&entries);

        assert_eq!(raw, serialized);
    }

    #[test]
    fn reject_missing_mode_separator() {
        let mut raw = Vec::new();
        raw.extend_from_slice(b"100644hello.txt\0");
        raw.extend_from_slice(&[1u8; 20]);

        let err = tree_parse(&raw).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }
}
