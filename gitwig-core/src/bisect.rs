//! Git bisect inspection and subprocess actions (`git bisect start/good/bad/skip/reset`).

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::Duration;

use git2::Repository;

use crate::process::{git_command, run_git_with_timeout, summarize_stderr};
use crate::{invalidate_ref_map_cache, resolve_git_dirs, sanitize_text};

const BISECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Single decision recorded in `.git/BISECT_LOG`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BisectLogEntry {
    pub oid: String,
    pub short_oid: String,
    pub verdict: String,
    pub summary: String,
}

/// Snapshot of an in-progress `git bisect` session (read-only).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BisectStatus {
    pub active: bool,
    pub start_ref: Option<String>,
    pub current_oid: Option<String>,
    pub current_short_oid: Option<String>,
    pub current_summary: Option<String>,
    pub bad_oid: Option<String>,
    pub good_oids: Vec<String>,
    pub skip_oids: Vec<String>,
    pub remaining_revisions: Option<usize>,
    pub remaining_steps: Option<usize>,
    pub culprit: Option<String>,
    /// Recorded verdicts from `BISECT_LOG`, newest first.
    pub log: Vec<BisectLogEntry>,
}

/// Summary returned after starting or stepping a bisect session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BisectStepOutcome {
    pub summary: String,
    pub culprit_oid: Option<String>,
    pub details: String,
}

fn validate_rev(rev: &str) -> Result<&str, String> {
    let trimmed = rev.trim();
    if trimmed.is_empty() {
        return Err("Revision cannot be empty".to_string());
    }
    if trimmed.starts_with('-') {
        return Err(format!("Invalid revision name: '{}'", trimmed));
    }
    Ok(trimmed)
}

fn commit_summary(repo: &Repository, oid: git2::Oid) -> String {
    repo.find_commit(oid)
        .ok()
        .and_then(|c| c.summary().ok().flatten().map(|s| sanitize_text(s.trim())))
        .unwrap_or_else(|| "(unknown)".to_string())
}

fn estimate_remaining(
    repo: &Repository,
    bad_oid_str: &str,
    good_oid_strs: &[String],
    skip_oid_strs: &[String],
) -> Option<(usize, usize)> {
    let bad_oid = git2::Oid::from_str(bad_oid_str).ok()?;
    let mut walk = repo.revwalk().ok()?;
    walk.push(bad_oid).ok()?;
    for g in good_oid_strs {
        if let Ok(oid) = git2::Oid::from_str(g) {
            let _ = walk.hide(oid);
        }
    }
    let skipped: HashSet<git2::Oid> =
        skip_oid_strs.iter().filter_map(|s| git2::Oid::from_str(s).ok()).collect();
    let remaining = walk.flatten().filter(|oid| *oid != bad_oid && !skipped.contains(oid)).count();
    let steps = if remaining == 0 { 0 } else { (usize::BITS - remaining.leading_zeros()) as usize };
    Some((remaining, steps))
}

fn parse_bisect_log(log_path: &Path, repo: &Repository) -> Vec<BisectLogEntry> {
    let Ok(contents) = fs::read_to_string(log_path) else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let (Some("git"), Some("bisect"), Some(verdict), Some(oid_str)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if !matches!(verdict, "bad" | "good" | "skip") {
            continue;
        }
        if let Ok(oid) = git2::Oid::from_str(oid_str) {
            let oid_full = oid.to_string();
            let short_oid = oid_full.chars().take(7).collect();
            let summary = commit_summary(repo, oid);
            entries.push(BisectLogEntry {
                oid: oid_full,
                short_oid,
                verdict: verdict.to_string(),
                summary,
            });
        }
    }
    entries.reverse();
    entries
}

/// Inspects whether `repo_path` currently has a `git bisect` session in
/// progress (supporting linked worktrees via `resolve_git_dirs`).
pub fn get_bisect_status(repo_path: &Path) -> BisectStatus {
    let Some(dirs) = resolve_git_dirs(repo_path) else {
        return BisectStatus::default();
    };
    let start_path = dirs.git_dir.join("BISECT_START");
    if !start_path.exists() {
        return BisectStatus::default();
    }

    let mut status = BisectStatus { active: true, ..BisectStatus::default() };
    if let Ok(raw) = fs::read_to_string(&start_path) {
        let trimmed = sanitize_text(raw.trim());
        if !trimmed.is_empty() {
            status.start_ref = Some(trimmed);
        }
    }

    let Ok(repo) = Repository::open(repo_path) else {
        return status;
    };

    if let Ok(head) = repo.head() {
        if let Ok(commit) = head.peel_to_commit() {
            let oid_str = commit.id().to_string();
            status.current_short_oid = Some(oid_str.chars().take(7).collect());
            status.current_oid = Some(oid_str);
            status.current_summary = Some(sanitize_text(
                commit.summary().ok().flatten().unwrap_or("(no commit message)"),
            ));
        }
    }

    if let Ok(refs) = repo.references() {
        for reference in refs.flatten() {
            let Ok(name) = reference.name() else { continue };
            let Ok(commit) = reference.peel_to_commit() else { continue };
            let oid = commit.id().to_string();
            if name == "refs/bisect/bad" {
                status.bad_oid = Some(oid);
            } else if name.starts_with("refs/bisect/good-") {
                status.good_oids.push(oid);
            } else if name.starts_with("refs/bisect/skip-") {
                status.skip_oids.push(oid);
            }
        }
    }

    if let Some(ref bad_oid) = status.bad_oid {
        if !status.good_oids.is_empty() {
            if let Some((remaining, steps)) =
                estimate_remaining(&repo, bad_oid, &status.good_oids, &status.skip_oids)
            {
                status.remaining_revisions = Some(remaining);
                status.remaining_steps = Some(steps);
                if remaining == 0 && status.skip_oids.is_empty() {
                    if let Ok(oid) = git2::Oid::from_str(bad_oid) {
                        let short: String = bad_oid.chars().take(7).collect();
                        status.culprit = Some(format!("{} {}", short, commit_summary(&repo, oid)));
                    }
                }
            }
        }
    }

    status.log = parse_bisect_log(&dirs.git_dir.join("BISECT_LOG"), &repo);
    status
}

fn parse_bisect_output(stdout: &str) -> BisectStepOutcome {
    let clean = sanitize_text(stdout);
    let lines: Vec<&str> = clean.lines().map(str::trim).filter(|line| !line.is_empty()).collect();

    let mut culprit_oid = None;
    let mut summary = None;
    for line in &lines {
        if let Some(prefix) = line
            .strip_suffix("is the first 'bad' commit")
            .or_else(|| line.strip_suffix("is the first bad commit"))
        {
            let oid = prefix.trim();
            if !oid.is_empty() && oid.chars().all(|c| c.is_ascii_hexdigit()) {
                culprit_oid = Some(oid.to_string());
                let short: String = oid.chars().take(7).collect();
                summary = Some(format!("First bad commit: {}", short));
                break;
            }
        }
    }

    if summary.is_none() {
        for line in &lines {
            if line.starts_with("Bisecting:")
                || line.starts_with("status: waiting for")
                || line.starts_with("There are only 'skip'ped commits")
            {
                summary = Some((*line).to_string());
                break;
            }
        }
    }

    let summary = summary.unwrap_or_else(|| {
        lines.first().map(|s| (*s).to_string()).unwrap_or_else(|| "Bisect updated".to_string())
    });

    BisectStepOutcome { summary, culprit_oid, details: clean.trim().to_string() }
}

fn run_bisect_command(repo_path: &Path, args: &[&str]) -> Result<BisectStepOutcome, String> {
    let mut cmd = git_command();
    cmd.current_dir(repo_path).args(args);
    let output = run_git_with_timeout(cmd, BISECT_TIMEOUT)
        .map_err(|e| format!("git {} failed: {}", args.join(" "), e))?;
    if !output.status.success() {
        let reason = if let Some(stderr) = summarize_stderr(&output.stderr) {
            stderr
        } else {
            let stdout = sanitize_text(&String::from_utf8_lossy(&output.stdout));
            if !stdout.trim().is_empty() {
                stdout.trim().to_string()
            } else {
                format!("exit status {}", output.status)
            }
        };
        return Err(reason);
    }
    invalidate_ref_map_cache(repo_path);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_bisect_output(&stdout))
}

/// Starts a `git bisect` session, optionally marking `bad_rev` and `good_rev`.
pub fn bisect_start(
    repo_path: &Path,
    bad_rev: Option<&str>,
    good_rev: Option<&str>,
) -> Result<BisectStepOutcome, String> {
    let mut args = vec!["bisect", "start"];
    if let Some(good) = good_rev {
        let bad = validate_rev(bad_rev.unwrap_or("HEAD"))?;
        let good = validate_rev(good)?;
        args.push(bad);
        args.push(good);
    } else if let Some(bad) = bad_rev {
        args.push(validate_rev(bad)?);
    }
    let outcome = run_bisect_command(repo_path, &args)?;
    if args.len() == 2 && outcome.summary == "Bisect updated" {
        return Ok(BisectStepOutcome {
            summary: "Bisect started — mark commits as bad and good".to_string(),
            ..outcome
        });
    }
    Ok(outcome)
}

/// Marks `rev` (or `HEAD` when `None`) as good during an active bisect.
pub fn bisect_good(repo_path: &Path, rev: Option<&str>) -> Result<BisectStepOutcome, String> {
    match rev {
        Some(r) => run_bisect_command(repo_path, &["bisect", "good", validate_rev(r)?]),
        None => run_bisect_command(repo_path, &["bisect", "good"]),
    }
}

/// Marks `rev` (or `HEAD` when `None`) as bad during an active bisect.
pub fn bisect_bad(repo_path: &Path, rev: Option<&str>) -> Result<BisectStepOutcome, String> {
    match rev {
        Some(r) => run_bisect_command(repo_path, &["bisect", "bad", validate_rev(r)?]),
        None => run_bisect_command(repo_path, &["bisect", "bad"]),
    }
}

/// Skips `rev` (or `HEAD` when `None`) during an active bisect.
pub fn bisect_skip(repo_path: &Path, rev: Option<&str>) -> Result<BisectStepOutcome, String> {
    match rev {
        Some(r) => run_bisect_command(repo_path, &["bisect", "skip", validate_rev(r)?]),
        None => run_bisect_command(repo_path, &["bisect", "skip"]),
    }
}

/// Ends the active bisect session and restores the starting branch (`git bisect reset`).
pub fn bisect_reset(repo_path: &Path) -> Result<String, String> {
    let outcome = run_bisect_command(repo_path, &["bisect", "reset"])?;
    Ok(if outcome.summary == "Bisect updated" {
        "Bisect reset — returned to original HEAD".to_string()
    } else {
        outcome.summary
    })
}
