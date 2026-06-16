mod blob;
mod commit;
mod kind;
mod lookup;
mod store;
mod tree;

pub use blob::BlobObject;
pub use commit::{CommitObject, kvlm_parse, kvlm_serialize};
pub use kind::ObjectKind;
pub use lookup::find_object;
use std::io;
pub use store::{read_object, write_object};
pub use tree::{TreeObject, tree_parse, tree_serialize};

pub enum GitObject {
    Blob(BlobObject),
    Commit(CommitObject),
    Tag(CommitObject),
    Tree(TreeObject),
}

impl GitObject {
    fn format(&self) -> &str {
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
