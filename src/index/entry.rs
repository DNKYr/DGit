pub const ENTRY_FIXED_SIZE: usize = 62;

pub const FLAG_STAGE_MASK: u16 = 0x3000;
pub const FLAG_NAME_MASK: u16 = 0x0FFF;
pub const FLAG_ASSUME_VALID: u16 = 0x8000;
pub const FLAG_EXTENDED: u16 = 0x4000;

pub const MODE_REGULAR: u32 = 0o100644;
pub const MODE_EXECUTABLE: u32 = 0o100755;
pub const MODE_SYMLINK: u32 = 0o120000;
pub const MODE_GITLINK: u32 = 0o160000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexEntry {
    pub ctime_sec: u32,
    pub ctime_ns: u32,
    pub mtime_sec: u32,
    pub mtime_ns: u32,
    pub dev: u32,
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub file_size: u32,
    pub sha: [u8; 20],
    pub flags: u16,
    pub path: Vec<u8>,
}

impl IndexEntry {
    pub fn new(path: Vec<u8>, sha: [u8; 20], mode: u32, flags: u16) -> Self {
        IndexEntry {
            ctime_sec: 0,
            ctime_ns: 0,
            mtime_sec: 0,
            mtime_ns: 0,
            dev: 0,
            ino: 0,
            mode,
            uid: 0,
            gid: 0,
            file_size: 0,
            sha,
            flags,
            path,
        }
    }

    pub fn stage(&self) -> u16 {
        (self.flags & FLAG_STAGE_MASK) >> 12
    }

    pub fn name_len(&self) -> usize {
        (self.flags & FLAG_NAME_MASK) as usize
    }

    pub fn set_name_len(&mut self, len: usize) {
        self.flags = (self.flags & !FLAG_NAME_MASK) | (len.min(0xFFF) as u16);
    }

    pub fn set_stage(&mut self, stage: u16) {
        self.flags = (self.flags & !FLAG_STAGE_MASK) | ((stage & 0x3) << 12);
    }

    pub fn make_flags(name_len: usize, stage: u16) -> u16 {
        let len_bits = if name_len < 0xFFF {
            name_len as u16
        } else {
            0xFFF
        };
        ((stage & 0x3) << 12) | len_bits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_flags_encodes_stage_and_length() {
        let flags = IndexEntry::make_flags(42, 0);
        assert_eq!(flags, 42);
        let flags = IndexEntry::make_flags(42, 1);
        assert_eq!(flags, (1 << 12) | 42);
        let flags = IndexEntry::make_flags(42, 3);
        assert_eq!(flags, (3 << 12) | 42);
    }

    #[test]
    fn make_flags_clamps_long_names() {
        let flags = IndexEntry::make_flags(0xFFFF, 0);
        assert_eq!(flags & FLAG_NAME_MASK, 0xFFF);
    }

    #[test]
    fn stage_extracts_correctly() {
        let mut entry = IndexEntry::new(vec![], [0u8; 20], MODE_REGULAR, 0);
        entry.set_stage(0);
        assert_eq!(entry.stage(), 0);
        entry.set_stage(2);
        assert_eq!(entry.stage(), 2);
        entry.set_stage(4);
        assert_eq!(entry.stage(), 0);
    }

    #[test]
    fn name_len_extracts_correctly() {
        let flags = IndexEntry::make_flags(100, 0);
        let entry = IndexEntry::new(vec![], [0u8; 20], MODE_REGULAR, flags);
        assert_eq!(entry.name_len(), 100);
    }
}
