use crate::repository::GitRepository;
use std::collections::BTreeSet;
use std::fs;
use std::io;

#[derive(Debug, Clone)]
pub struct IgnorePattern {
    components: Vec<Vec<u8>>,
    negated: bool,
    directory_only: bool,
    anchored: bool,
}

impl IgnorePattern {
    pub fn new(pattern: &str) -> Option<Self> {
        let pattern = pattern.trim();

        if pattern.is_empty() || pattern.starts_with('#') {
            return None;
        }

        let negated = pattern.starts_with('!');
        let pattern = if negated { &pattern[1..] } else { pattern };

        if pattern.is_empty() {
            return None;
        }

        let pattern = pattern.trim();
        if pattern.is_empty() {
            return None;
        }

        let directory_only = pattern.ends_with('/') && pattern != "/";
        let pattern = if directory_only {
            &pattern[..pattern.len() - 1]
        } else {
            pattern
        };

        let anchored = pattern.contains('/');

        let components: Vec<Vec<u8>> = pattern
            .split('/')
            .filter(|c| !c.is_empty())
            .map(|c| c.as_bytes().to_vec())
            .collect();

        if components.is_empty() {
            return None;
        }

        Some(IgnorePattern {
            components,
            negated,
            directory_only,
            anchored,
        })
    }

    pub fn matches(&self, path: &[u8], is_dir: bool) -> bool {
        if self.directory_only && !is_dir {
            return false;
        }

        let path_components: Vec<&[u8]> = path
            .split(|&b| b == b'/')
            .filter(|c| !c.is_empty())
            .collect();

        if path_components.is_empty() {
            return false;
        }

        if self.anchored {
            self.match_anchored(&path_components)
        } else {
            self.match_unanchored(&path_components)
        }
    }

    fn match_anchored(&self, path: &[&[u8]]) -> bool {
        component_match_list(&self.components, path)
    }

    fn match_unanchored(&self, path: &[&[u8]]) -> bool {
        for i in 0..path.len() {
            if component_match_list(&self.components, &path[i..]) {
                return true;
            }
        }
        false
    }
}

fn component_match_list(patterns: &[Vec<u8>], names: &[&[u8]]) -> bool {
    let mut pi = 0;
    let mut ni = 0;

    while pi < patterns.len() {
        if patterns[pi] == b"**" {
            pi += 1;
            if pi == patterns.len() {
                return true;
            }
            while ni < names.len() {
                if component_match_list(&patterns[pi..], &names[ni..]) {
                    return true;
                }
                ni += 1;
            }
            return false;
        }

        if ni >= names.len() {
            return false;
        }

        if !component_matches(&patterns[pi], names[ni]) {
            return false;
        }

        pi += 1;
        ni += 1;
    }

    ni == names.len()
}

fn component_matches(pattern: &[u8], name: &[u8]) -> bool {
    glob_match(pattern, name)
}

fn glob_match(pattern: &[u8], name: &[u8]) -> bool {
    let mut pi: isize = 0;
    let mut ni: isize = 0;
    let p_len = pattern.len() as isize;
    let n_len = name.len() as isize;

    while pi < p_len || ni < n_len {
        if pi < p_len && pattern[pi as usize] == b'*' {
            pi += 1;
            if pi == p_len {
                return true;
            }
            let ch = pattern[pi as usize];
            if ch == b'*' || ch == b'?' || ch == b'[' {
                continue;
            }
            while ni < n_len {
                if name[ni as usize] == ch
                    && glob_match(&pattern[pi as usize..], &name[ni as usize..])
                {
                    return true;
                }
                ni += 1;
            }
            return false;
        } else if pi < p_len && pattern[pi as usize] == b'?' {
            if ni >= n_len {
                return false;
            }
            pi += 1;
            ni += 1;
        } else if pi < p_len && pattern[pi as usize] == b'[' {
            if ni >= n_len {
                return false;
            }
            let end = match pattern[pi as usize..].iter().position(|&b| b == b']') {
                Some(p) => pi as usize + p,
                None => return false,
            };
            let class = &pattern[pi as usize + 1..end];
            let matched = char_class_match(class, name[ni as usize]);
            if !matched {
                return false;
            }
            pi = (end + 1) as isize;
            ni += 1;
        } else {
            if ni >= n_len || pattern[pi as usize] != name[ni as usize] {
                return false;
            }
            pi += 1;
            ni += 1;
        }
    }

    true
}

fn char_class_match(class: &[u8], ch: u8) -> bool {
    if class.is_empty() {
        return false;
    }

    let negated = class[0] == b'!';
    let class = if negated { &class[1..] } else { class };

    let mut i = 0;
    while i < class.len() {
        if i + 2 < class.len() && class[i + 1] == b'-' {
            if ch >= class[i] && ch <= class[i + 2] {
                return !negated;
            }
            i += 3;
        } else {
            if ch == class[i] {
                return !negated;
            }
            i += 1;
        }
    }

    negated
}

pub struct IgnoreRules {
    patterns: Vec<IgnorePattern>,
}

impl IgnoreRules {
    pub fn new() -> Self {
        IgnoreRules {
            patterns: Vec::new(),
        }
    }

    pub fn load(repo: &GitRepository) -> io::Result<Self> {
        let mut rules = IgnoreRules::new();
        let git_dir_path = repo.get_git_dir();
        let worktree = std::path::PathBuf::from(
            git_dir_path.parent().unwrap_or_else(|| std::path::Path::new(".")),
        );

        rules.load_gitignore(&worktree, "");

        let exclude_path = git_dir_path.join("info").join("exclude");
        if let Ok(content) = fs::read_to_string(&exclude_path) {
            for line in content.lines() {
                if let Some(p) = IgnorePattern::new(line) {
                    rules.patterns.push(p);
                }
            }
        }

        Ok(rules)
    }

    fn load_gitignore(&mut self, base: &std::path::Path, prefix: &str) {
        let dir = if prefix.is_empty() {
            base.to_path_buf()
        } else {
            base.join(prefix)
        };

        let gitignore_path = dir.join(".gitignore");
        if let Ok(content) = fs::read_to_string(&gitignore_path) {
            for line in content.lines() {
                if let Some(p) = IgnorePattern::new(line) {
                    self.patterns.push(p);
                }
            }
        }

        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if name == ".git" {
                    continue;
                }
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let sub_prefix = if prefix.is_empty() {
                        name.to_string_lossy().into_owned()
                    } else {
                        format!("{}/{}", prefix, name.to_string_lossy())
                    };
                    self.load_gitignore(base, &sub_prefix);
                }
            }
        }
    }

    pub fn is_ignored(&self, path: &[u8], is_dir: bool) -> bool {
        if self.match_path(path, is_dir) {
            return true;
        }

        if !is_dir {
            if self.match_path(path, true) {
                return true;
            }

            let mut pos = 0;
            while pos < path.len() {
                if path[pos] == b'/'
                    && self.match_path(&path[..pos], true) {
                        return true;
                    }
                pos += 1;
            }
        }

        false
    }

    fn match_path(&self, path: &[u8], is_dir: bool) -> bool {
        let mut ignored = false;
        for pattern in &self.patterns {
            if pattern.matches(path, is_dir) {
                ignored = !pattern.negated;
            }
        }
        ignored
    }

    #[allow(dead_code)]
    pub fn filter_untracked(&self, files: &BTreeSet<Vec<u8>>) -> Vec<Vec<u8>> {
        files
            .iter()
            .filter(|p| !self.is_ignored(p, false))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> IgnorePattern {
        IgnorePattern::new(s).unwrap()
    }

    #[test]
    fn simple_filename() {
        let pat = p("*.o");
        assert!(pat.matches(b"file.o", false));
        assert!(!pat.matches(b"file.c", false));
        assert!(pat.matches(b"src/file.o", false));
    }

    #[test]
    fn anchored_pattern() {
        let pat = p("src/*.o");
        assert!(pat.matches(b"src/file.o", false));
        assert!(!pat.matches(b"file.o", false));
    }

    #[test]
    fn directory_only() {
        let pat = p("target/");
        assert!(pat.matches(b"target", true));
        assert!(!pat.matches(b"target", false));
    }

    #[test]
    fn negation() {
        let pat = p("!important.txt");
        assert!(pat.matches(b"important.txt", false));
    }

    #[test]
    fn double_star() {
        let pat = p("a/**/b");
        assert!(pat.matches(b"a/b", false));
        assert!(pat.matches(b"a/x/b", false));
        assert!(pat.matches(b"a/x/y/b", false));
        assert!(!pat.matches(b"a/x/y/z", false));
    }

    #[test]
    fn leading_double_star() {
        let pat = p("**/foo");
        assert!(pat.matches(b"foo", false));
        assert!(pat.matches(b"a/foo", false));
        assert!(pat.matches(b"a/b/foo", false));
    }

    #[test]
    fn trailing_slash_star_star() {
        let pat = p("target/**");
        assert!(pat.matches(b"target", true));
        assert!(pat.matches(b"target/file.o", false));
        assert!(pat.matches(b"target/debug/file.o", false));
    }

    #[test]
    fn char_class() {
        let pat = p("[abc].txt");
        assert!(pat.matches(b"a.txt", false));
        assert!(pat.matches(b"b.txt", false));
        assert!(!pat.matches(b"d.txt", false));
    }

    #[test]
    fn char_class_range() {
        let pat = p("[0-9].txt");
        assert!(pat.matches(b"5.txt", false));
        assert!(!pat.matches(b"x.txt", false));
    }

    #[test]
    fn comment_and_blank() {
        assert!(IgnorePattern::new("# comment").is_none());
        assert!(IgnorePattern::new("").is_none());
        assert!(IgnorePattern::new("  ").is_none());
    }

    #[test]
    fn ignore_rules_simple() {
        let mut rules = IgnoreRules::new();
        rules.patterns.push(p("*.o"));
        rules.patterns.push(p("target/"));
        rules.patterns.push(p("!important.o"));

        assert!(rules.is_ignored(b"file.o", false));
        assert!(!rules.is_ignored(b"important.o", false));
        assert!(rules.is_ignored(b"target", true));
        assert!(!rules.is_ignored(b"src/main.rs", false));
    }
}
