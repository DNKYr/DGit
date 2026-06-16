use super::ObjectKind;
use crate::repository::GitRepository;

pub fn find_object(
    repo: &GitRepository,
    name: &str,
    fmt: Option<ObjectKind>,
    follow: bool,
) -> String {
    name.to_string()
}
