pub enum ObjectKind {
    Blob,
    Commit,
    Tag,
    Tree,
}

impl ObjectKind {
    pub fn as_str(&self) -> &str {
        match self {
            ObjectKind::Blob => "blob",
            ObjectKind::Commit => "commit",
            ObjectKind::Tree => "tree",
            ObjectKind::Tag => "tag",
        }
    }
}
