mod blob;
mod commit;
mod kind;
mod lookup;
mod store;
mod tree;

pub use blob::BlobObject;
pub use commit::{CommitObject, TagObject, kvlm_parse, kvlm_serialize};
pub use kind::ObjectKind;
pub use lookup::find_object;
use std::io;
pub use store::{read_object, write_object};
pub use tree::{TreeObject, tree_parse, tree_serialize};

pub enum GitObject {
    Blob(BlobObject),
    Commit(CommitObject),
    Tag(TagObject),
    Tree(TreeObject),
}

impl GitObject {
    fn kind(&self) -> &ObjectKind {
        match self {
            GitObject::Blob(_) => &ObjectKind::Blob,
            GitObject::Commit(_) => &ObjectKind::Commit,
            GitObject::Tag(_) => &ObjectKind::Tag,
            GitObject::Tree(_) => &ObjectKind::Tree,
        }
    }

    pub fn serialize(&self) -> io::Result<Vec<u8>> {
        match self {
            GitObject::Blob(blob) => Ok(blob.data.clone()),
            GitObject::Commit(commit) => Ok(kvlm_serialize(&commit.kvlm)),
            GitObject::Tree(tree) => Ok(tree_serialize(&tree.items)),
            GitObject::Tag(tag) => Ok(kvlm_serialize(&tag.kvlm)),
        }
    }
}
