//! Single-file patches built from a rendered diff hunk, and the `git apply`
//! that feeds them to the index, the working tree, or a scratch index.
//!
//! Hunk and line staging, discarding and stashing all assemble their patch
//! here, so the same selection produces the same patch whichever of them
//! asked for it.

use std::io::Write;
use std::path::{Component, Path};
use std::process::Stdio;

use crate::{DiffLine, DiffLineKind, git_command};

/// Parses a `@@ -old_start,old_count +new_start,new_count @@` header.
pub(crate) fn parse_hunk_header(header: &str) -> Option<(usize, usize, usize, usize)> {
    if !header.starts_with("@@") {
        return None;
    }
    let parts: Vec<&str> = header.split("@@").collect();
    if parts.len() < 3 {
        return None;
    }
    let meta = parts[1].trim();
    let subparts: Vec<&str> = meta.split_whitespace().collect();
    if subparts.len() < 2 {
        return None;
    }

    let parse_part = |p: &str| -> (usize, usize) {
        let s = p.trim_start_matches(['-', '+']);
        let comps: Vec<&str> = s.split(',').collect();
        let start = comps[0].parse::<usize>().unwrap_or(0);
        let count = if comps.len() > 1 { comps[1].parse::<usize>().unwrap_or(1) } else { 1 };
        (start, count)
    };

    let (old_start, old_count) = parse_part(subparts[0]);
    let (new_start, new_count) = parse_part(subparts[1]);
    Some((old_start, old_count, new_start, new_count))
}

/// Refuses a path that would let `git apply` write outside the repository.
fn check_repo_relative(file_path: &str) -> Result<(), String> {
    let path = Path::new(file_path);
    if path.is_absolute() || path.components().any(|c| c == Component::ParentDir) {
        return Err("Path escapes repository root".to_string());
    }
    Ok(())
}

fn file_header(file_path: &str) -> String {
    format!("diff --git a/{0} b/{0}\n--- a/{0}\n+++ b/{0}\n", file_path)
}

/// Appends one patch line: the `+` / `-` / ` ` prefix for `kind`, then `content`.
/// A header line's content is the whole `@@ … @@` line, so it has no prefix.
fn push_line(patch: &mut String, kind: &DiffLineKind, content: &str) {
    let prefix = match kind {
        DiffLineKind::Added => "+",
        DiffLineKind::Removed => "-",
        DiffLineKind::Context => " ",
        _ => "",
    };
    patch.push_str(prefix);
    patch.push_str(content);
    patch.push('\n');
}

/// The patch for one whole hunk exactly as the diff shows it (`hunk[0]` is
/// its `@@` header).
pub(crate) fn hunk_patch(file_path: &str, hunk: &[DiffLine]) -> Result<String, String> {
    check_repo_relative(file_path)?;
    let mut patch = file_header(file_path);
    for line in hunk {
        push_line(&mut patch, &line.kind, &line.content);
    }
    Ok(patch)
}

/// The patch for the single `+` / `-` line at `selected` in `hunk`, with the
/// hunk's other changes rewritten so it applies to a side that holds only
/// some of them.
///
/// - `revert` inverts the selected line (`+` ↔ `-`), to take the change out.
/// - `target_has_modification` says the side the patch applies to already
///   holds the hunk's other changes (the working tree, for an unstaged hunk):
///   their `+` lines become context and their `-` lines are dropped. When it
///   does not (the index), the `-` lines become context and the `+` lines are
///   dropped.
pub(crate) fn line_patch(
    file_path: &str,
    hunk: &[DiffLine],
    selected: usize,
    revert: bool,
    target_has_modification: bool,
) -> Result<String, String> {
    check_repo_relative(file_path)?;

    if hunk.is_empty() {
        return Err("Empty hunk".to_string());
    }
    let selected_line = hunk.get(selected).ok_or_else(|| "Invalid line index".to_string())?;
    if selected_line.kind != DiffLineKind::Added && selected_line.kind != DiffLineKind::Removed {
        return Err("Selected line is not a modification (must be + or -)".to_string());
    }

    let (old_start, _, new_start, _) = parse_hunk_header(&hunk[0].content)
        .ok_or_else(|| format!("Invalid hunk header: {}", hunk[0].content))?;

    let mut body = String::new();
    let mut old_count = 0;
    let mut new_count = 0;
    for (i, line) in hunk.iter().enumerate().skip(1) {
        let kind = if i == selected {
            if (line.kind == DiffLineKind::Added) != revert {
                DiffLineKind::Added
            } else {
                DiffLineKind::Removed
            }
        } else {
            match line.kind {
                DiffLineKind::Context => DiffLineKind::Context,
                DiffLineKind::Added if target_has_modification => DiffLineKind::Context,
                DiffLineKind::Removed if !target_has_modification => DiffLineKind::Context,
                _ => continue,
            }
        };
        match kind {
            DiffLineKind::Added => new_count += 1,
            DiffLineKind::Removed => old_count += 1,
            _ => {
                old_count += 1;
                new_count += 1;
            }
        }
        push_line(&mut body, &kind, &line.content);
    }

    let mut patch = file_header(file_path);
    patch.push_str(&format!("@@ -{},{} +{},{} @@\n", old_start, old_count, new_start, new_count));
    patch.push_str(&body);
    Ok(patch)
}

/// Feeds `patch` to `git apply <args> -` in `repo_path`. `index_file` points
/// git at an index other than the repository's own (`GIT_INDEX_FILE`).
pub(crate) fn git_apply(
    repo_path: &Path,
    patch: &str,
    args: &[&str],
    index_file: Option<&Path>,
) -> Result<(), String> {
    let mut cmd = git_command();
    cmd.arg("apply").args(args).arg("-").current_dir(repo_path).stdin(Stdio::piped());
    if let Some(index) = index_file {
        cmd.env("GIT_INDEX_FILE", index);
    }
    let mut child = cmd.spawn().map_err(|e| format!("Failed to spawn git apply: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(patch.as_bytes())
            .map_err(|e| format!("Failed to write patch to stdin: {}", e))?;
    }

    let output =
        child.wait_with_output().map_err(|e| format!("Failed to wait for git apply: {}", e))?;
    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(format!("git apply failed: {}", err_msg.trim()));
    }
    Ok(())
}
