//! Minimal `~/.ssh/config` reader: lists single-name `Host` aliases with
//! their `HostName` and `User`, so the cluster form can offer them.

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SshHost {
    pub alias: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
}

/// Splits an ssh config line into (key, value). Accepts both `Key value`
/// and `Key=value`. Returns `None` if either side is empty.
fn split_key_value(line: &str) -> Option<(&str, &str)> {
    let idx = line.find(|c: char| c.is_whitespace() || c == '=')?;
    let key = &line[..idx];
    let value = line[idx..]
        .trim_start_matches(|c: char| c.is_whitespace() || c == '=')
        .trim();
    if key.is_empty() || value.is_empty() {
        return None;
    }
    Some((key, value))
}

/// True when a `Host` value names exactly one concrete alias: no spaces,
/// no glob characters, not negated.
fn is_single_alias(value: &str) -> bool {
    !value.contains(char::is_whitespace)
        && !value.contains('*')
        && !value.contains('?')
        && !value.starts_with('!')
}

/// Pushes `host`, or, if a host with the same alias was already collected,
/// merges into that existing entry instead: the existing entry keeps its
/// position and only its still-`None` fields are filled in, so the first
/// block's values win, matching ssh's first-obtained-value rule.
fn push_or_merge(hosts: &mut Vec<SshHost>, host: SshHost) {
    if let Some(existing) = hosts.iter_mut().find(|h| h.alias == host.alias) {
        if existing.hostname.is_none() {
            existing.hostname = host.hostname;
        }
        if existing.user.is_none() {
            existing.user = host.user;
        }
    } else {
        hosts.push(host);
    }
}

pub fn parse_ssh_config(contents: &str) -> Vec<SshHost> {
    let mut hosts: Vec<SshHost> = Vec::new();
    let mut current: Option<SshHost> = None;

    for raw in contents.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = split_key_value(line) else {
            continue;
        };
        let key = key.to_ascii_lowercase();

        match key.as_str() {
            "host" => {
                if let Some(done) = current.take() {
                    push_or_merge(&mut hosts, done);
                }
                if is_single_alias(value) {
                    current = Some(SshHost {
                        alias: value.to_string(),
                        hostname: None,
                        user: None,
                    });
                }
            }
            "match" => {
                if let Some(done) = current.take() {
                    push_or_merge(&mut hosts, done);
                }
            }
            "hostname" => {
                if let Some(h) = current.as_mut() {
                    h.hostname = Some(value.to_string());
                }
            }
            "user" => {
                if let Some(h) = current.as_mut() {
                    h.user = Some(value.to_string());
                }
            }
            _ => {}
        }
    }
    if let Some(done) = current.take() {
        push_or_merge(&mut hosts, done);
    }
    hosts
}

/// Reads and parses the file. A missing or unreadable file is not an
/// error: the import list is simply empty.
pub fn load_ssh_hosts(path: &Path) -> Vec<SshHost> {
    match std::fs::read_to_string(path) {
        Ok(contents) => parse_ssh_config(&contents),
        Err(_) => Vec::new(),
    }
}

/// `$HOME/.ssh/config` (or `%USERPROFILE%\.ssh\config` on Windows).
pub fn default_config_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|home| PathBuf::from(home).join(".ssh").join("config"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# personal clusters
Host snellius
  HostName snellius.surf.nl
  User snellius_user
  ControlMaster auto

Host lisa
    hostname lisa.example.org

Host *.internal
  User svc

Host a b
  User shared

Match host foo
  User matched

Host=eqform
  User=eq_user
";

    #[test]
    fn parses_multiple_host_blocks() {
        let hosts = parse_ssh_config(SAMPLE);
        let aliases: Vec<&str> = hosts.iter().map(|h| h.alias.as_str()).collect();
        assert_eq!(aliases, vec!["snellius", "lisa", "eqform"]);
    }

    #[test]
    fn reads_hostname_and_user() {
        let hosts = parse_ssh_config(SAMPLE);
        assert_eq!(
            hosts[0],
            SshHost {
                alias: "snellius".into(),
                hostname: Some("snellius.surf.nl".into()),
                user: Some("snellius_user".into()),
            }
        );
    }

    #[test]
    fn keys_are_case_insensitive_and_user_may_be_missing() {
        let hosts = parse_ssh_config(SAMPLE);
        assert_eq!(hosts[1].hostname.as_deref(), Some("lisa.example.org"));
        assert_eq!(hosts[1].user, None);
    }

    #[test]
    fn skips_wildcard_and_multi_pattern_hosts_and_match_blocks() {
        let hosts = parse_ssh_config(SAMPLE);
        assert!(hosts.iter().all(|h| h.alias != "*.internal"));
        assert!(hosts.iter().all(|h| h.alias != "a b" && h.alias != "a"));
        assert!(hosts.iter().all(|h| h.user.as_deref() != Some("matched")));
    }

    #[test]
    fn supports_key_equals_value_form() {
        let hosts = parse_ssh_config(SAMPLE);
        assert_eq!(hosts[2].user.as_deref(), Some("eq_user"));
    }

    #[test]
    fn empty_and_comment_only_input_yields_nothing() {
        assert!(parse_ssh_config("").is_empty());
        assert!(parse_ssh_config("# nothing\n\n   \n").is_empty());
    }

    #[test]
    fn load_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load_ssh_hosts(&dir.path().join("config")).is_empty());
    }

    #[test]
    fn load_reads_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config");
        std::fs::write(&path, "Host x\n User y\n").unwrap();
        let hosts = load_ssh_hosts(&path);
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].alias, "x");
    }

    #[test]
    fn duplicate_host_blocks_are_merged_first_wins() {
        let hosts = parse_ssh_config(
            "\
Host foo
  User first
Host foo
  HostName foo.example.org
  User second
",
        );
        let foos: Vec<&SshHost> = hosts.iter().filter(|h| h.alias == "foo").collect();
        assert_eq!(foos.len(), 1);
        assert_eq!(foos[0].user.as_deref(), Some("first"));
        assert_eq!(foos[0].hostname.as_deref(), Some("foo.example.org"));
    }
}
