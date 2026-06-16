use crate::cli;

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
