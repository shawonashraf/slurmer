//! Slurm job model, `squeue` output parsing, and the ssh argv builder.

use serde::Serialize;

/// Pipe-delimited `squeue -o` format. Ten fields, one job per line.
pub const SQUEUE_FORMAT: &str = "%i|%P|%j|%T|%M|%D|%R|%l|%C|%m";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Job {
    pub id: String,
    pub partition: String,
    pub name: String,
    pub state: String,
    pub time: String,
    pub nodes: String,
    pub reason: String,
    pub time_limit: String,
    pub cpus: String,
    pub min_memory: String,
}

/// Parses `squeue --noheader -o SQUEUE_FORMAT` output. Lines that do not
/// have exactly ten fields are skipped.
pub fn parse_squeue_output(output: &str) -> Vec<Job> {
    output
        .lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split('|').collect();
            if f.len() != 10 {
                return None;
            }
            Some(Job {
                id: f[0].to_string(),
                partition: f[1].to_string(),
                name: f[2].to_string(),
                state: f[3].to_string(),
                time: f[4].to_string(),
                nodes: f[5].to_string(),
                reason: f[6].to_string(),
                time_limit: f[7].to_string(),
                cpus: f[8].to_string(),
                min_memory: f[9].to_string(),
            })
        })
        .collect()
}

/// Wraps a value in single quotes for the remote POSIX shell, escaping any
/// embedded single quote as `'\''`. Every value that reaches the remote
/// command line must pass through this.
pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Builds the argv passed to the local `ssh` binary (excluding the program
/// name). `host` is used verbatim as the ssh target so `~/.ssh/config`
/// aliases apply. When `user` is empty, the alias's own `User` is relied on
/// and `squeue --me` selects the current remote user.
pub fn build_ssh_args(host: &str, user: &str) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    if !user.is_empty() {
        args.push("-o".into());
        args.push(format!("User={user}"));
    }
    args.push("--".into());
    args.push(host.to_string());
    args.push("squeue".into());
    args.push("--noheader".into());
    if user.is_empty() {
        args.push("--me".into());
    } else {
        args.push("-u".into());
        args.push(shell_quote(user));
    }
    args.push("-o".into());
    args.push(shell_quote(SQUEUE_FORMAT));
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
24611463|gpu_h100|lms-gemma4-train|RUNNING|3:03:26|1|gcn159|5-00:00:00|64|180G
24611653|gpu_h100|a-very-long-job-name-that-squeue-would-truncate|PENDING|0:00|2|(Priority)|1-00:00:00|32|90G
";

    #[test]
    fn parses_multiple_lines_in_field_order() {
        let jobs = parse_squeue_output(SAMPLE);
        assert_eq!(jobs.len(), 2);
        assert_eq!(
            jobs[0],
            Job {
                id: "24611463".into(),
                partition: "gpu_h100".into(),
                name: "lms-gemma4-train".into(),
                state: "RUNNING".into(),
                time: "3:03:26".into(),
                nodes: "1".into(),
                reason: "gcn159".into(),
                time_limit: "5-00:00:00".into(),
                cpus: "64".into(),
                min_memory: "180G".into(),
            }
        );
        assert_eq!(jobs[1].state, "PENDING");
        assert_eq!(jobs[1].reason, "(Priority)");
    }

    #[test]
    fn preserves_untruncated_names() {
        let jobs = parse_squeue_output(SAMPLE);
        assert_eq!(
            jobs[1].name,
            "a-very-long-job-name-that-squeue-would-truncate"
        );
    }

    #[test]
    fn skips_lines_with_wrong_field_count() {
        let input = "1|2|3\n\n24611463|gpu|n|RUNNING|1:00|1|node|2:00|4|8G\ngarbage\n";
        let jobs = parse_squeue_output(input);
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, "24611463");
    }

    #[test]
    fn empty_output_is_no_jobs() {
        assert!(parse_squeue_output("").is_empty());
        assert!(parse_squeue_output("\n\n").is_empty());
    }

    #[test]
    fn shell_quote_wraps_in_single_quotes() {
        assert_eq!(shell_quote("alice"), "'alice'");
    }

    #[test]
    fn shell_quote_escapes_embedded_single_quotes() {
        assert_eq!(shell_quote("o'brien"), "'o'\\''brien'");
    }

    #[test]
    fn build_ssh_args_with_user() {
        let args = build_ssh_args("Snellius-Large", "alice");
        assert_eq!(
            args,
            vec![
                "-o",
                "User=alice",
                "--",
                "Snellius-Large",
                "squeue",
                "--noheader",
                "-u",
                "'alice'",
                "-o",
                "'%i|%P|%j|%T|%M|%D|%R|%l|%C|%m'",
            ]
        );
    }

    #[test]
    fn build_ssh_args_without_user_uses_me() {
        let args = build_ssh_args("login.example.org", "");
        assert_eq!(
            args,
            vec![
                "--",
                "login.example.org",
                "squeue",
                "--noheader",
                "--me",
                "-o",
                "'%i|%P|%j|%T|%M|%D|%R|%l|%C|%m'",
            ]
        );
    }

    #[test]
    fn build_ssh_args_quotes_user_with_quote() {
        let args = build_ssh_args("h", "o'b");
        assert_eq!(args[1], "User=o'b");
        assert_eq!(args[7], "'o'\\''b'");
    }

    #[test]
    fn build_ssh_args_separates_options_from_host() {
        let with_user = build_ssh_args("myhost", "alice");
        assert_eq!(with_user[2], "--");

        let without_user = build_ssh_args("myhost", "");
        assert_eq!(without_user[0], "--");
    }
}
