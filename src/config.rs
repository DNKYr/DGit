use crate::repository::GitRepository;
use std::collections::BTreeMap;
use std::fs;
use std::io;

#[derive(Debug, Default)]
pub struct Config {
    sections: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

impl Config {
    pub fn read(repo: &GitRepository) -> io::Result<Self> {
        let config_path = repo.get_git_dir().join("config");
        let content = match fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Config::default()),
            Err(e) => return Err(e),
        };
        Ok(parse_config(&content))
    }

    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        self.sections
            .get(section)
            .and_then(|keys| keys.get(key))
            .and_then(|vals| vals.last())
            .map(|v| v.as_str())
    }
}

fn parse_config(content: &str) -> Config {
    let mut sections: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    let mut current_section = String::new();
    let mut current_key: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }

        if trimmed.starts_with('[') {
            current_key = None;
            if let Some(end) = trimmed.find(']') {
                let header = &trimmed[1..end];
                current_section = section_name(header);
            }
            continue;
        }

        if let Some(continued_key) = &current_key {
            if line.starts_with(' ') || line.starts_with('\t') {
                let stripped = trimmed;
                if !stripped.contains('=') {
                    if let Some(entry) = sections
                        .get_mut(&current_section)
                        .and_then(|keys| keys.get_mut(continued_key))
                        .and_then(|vals| vals.last_mut())
                    {
                        entry.push('\n');
                        entry.push_str(stripped);
                    }
                    continue;
                }
            }
            current_key = None;
        }

        if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim().to_string();
            let val = trimmed[eq_pos + 1..].trim().to_string();
            let val = strip_quotes(&val);

            let section = sections.entry(current_section.clone()).or_default();
            section.entry(key.clone()).or_default().push(val);
            current_key = Some(key);
        }
    }

    Config { sections }
}

fn section_name(header: &str) -> String {
    if let Some(quote_start) = header.find('"') {
        let rest = &header[quote_start + 1..];
        if let Some(quote_end) = rest.find('"') {
            let subsection = &rest[..quote_end];
            if quote_start > 0 {
                let section_name = header[..quote_start].trim_end();
                let trimmed = format!("{}.{}", section_name, subsection.trim());
                let trimmed = trimmed.trim().to_string();
                return trimmed;
            }
            return subsection.trim().to_string();
        }
    }
    header.trim().to_string()
}

fn strip_quotes(val: &str) -> String {
    let val = val.trim();
    if (val.starts_with('"') && val.ends_with('"')) || (val.starts_with('\'') && val.ends_with('\''))
    {
        val[1..val.len() - 1].to_string()
    } else {
        val.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_config() {
        let content = "[user]\n\tname = John Doe\n\temail = john@example.com\n";
        let config = parse_config(content);

        assert_eq!(config.get("user", "name"), Some("John Doe"));
        assert_eq!(config.get("user", "email"), Some("john@example.com"));
    }

    #[test]
    fn parse_multiple_sections() {
        let content = "[core]\n\ta = 1\n[user]\n\tname = Jane\n";
        let config = parse_config(content);

        assert_eq!(config.get("core", "a"), Some("1"));
        assert_eq!(config.get("user", "name"), Some("Jane"));
    }

    #[test]
    fn parse_quoted_value() {
        let content = "[user]\n\tname = \"John Doe\"\n";
        let config = parse_config(content);
        assert_eq!(config.get("user", "name"), Some("John Doe"));
    }

    #[test]
    fn parse_subsection() {
        let content = "[branch \"main\"]\n\tremote = origin\n";
        let config = parse_config(content);
        assert_eq!(config.get("branch.main", "remote"), Some("origin"));
    }

    #[test]
    fn parse_comments_and_blanks() {
        let content = "# comment\n; another\n\n[user]\n\tname = X\n";
        let config = parse_config(content);
        assert_eq!(config.get("user", "name"), Some("X"));
    }

    #[test]
    fn read_missing_config() {
        let root = std::env::temp_dir().join("dgit-config-missing-test");
        std::fs::create_dir_all(root.join(".git")).unwrap();
        let repo = GitRepository::new(&root);

        let config = Config::read(&repo).unwrap();
        assert_eq!(config.get("user", "name"), None);
    }

    #[test]
    fn read_existing_config() {
        let root = std::env::temp_dir().join("dgit-config-read-test");
        let git_dir = root.join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("config"),
            "[user]\n\tname = Test User\n\temail = test@test.com\n",
        )
        .unwrap();
        let repo = GitRepository::new(&root);

        let config = Config::read(&repo).unwrap();
        assert_eq!(config.get("user", "name"), Some("Test User"));
        assert_eq!(config.get("user", "email"), Some("test@test.com"));
    }
}
