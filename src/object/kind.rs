pub enum ObjectKind {
    Blob,
    Commit,
    Tag,
    Tree,
}

impl ObjectKind {
    pub fn as_str(&self) -> &str {
        match self {
            ObjectKind::Blob => "Blob",
            ObjectKind::Commit => "Commit",
            ObjectKind::Tree => "Tree",
            ObjectKind::Tag => "Tag",
        }
    }
}
