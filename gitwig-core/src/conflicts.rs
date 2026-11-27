use std::path::Path;

use crate::{DiffLine, DiffLineKind, git_command, stage_file};

pub fn is_merging(repo_path: &Path) -> bool {
    repo_path.join(".git/MERGE_HEAD").exists()
}

/// Returns the conflict-marker diff for a conflicted file by parsing the file on disk.
/// Colorizes conflict blocks using DiffLineKind variants.
pub fn get_conflict_markers_diff(repo_path: &Path, file_path: &str) -> Vec<DiffLine> {
    let full_path = repo_path.join(file_path);
    let content = match std::fs::read_to_string(&full_path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut lines = Vec::new();
    let mut in_ours = false;
    let mut in_theirs = false;

    for line in content.lines() {
        if line.starts_with("<<<<<<<") {
            in_ours = true;
            in_theirs = false;
            lines.push(DiffLine {
                kind: DiffLineKind::ConflictSeparator,
                content: line.to_string(),
                old_lineno: None,
                new_lineno: None,
                hunk_idx: None,
            });
        } else if line.starts_with("=======") {
            in_ours = false;
            in_theirs = true;
            lines.push(DiffLine {
                kind: DiffLineKind::ConflictSeparator,
                content: line.to_string(),
                old_lineno: None,
                new_lineno: None,
                hunk_idx: None,
            });
        } else if line.starts_with(">>>>>>>") {
            in_ours = false;
            in_theirs = false;
            lines.push(DiffLine {
                kind: DiffLineKind::ConflictSeparator,
                content: line.to_string(),
                old_lineno: None,
                new_lineno: None,
                hunk_idx: None,
            });
        } else if in_ours {
            lines.push(DiffLine {
                kind: DiffLineKind::ConflictOurs,
                content: line.to_string(),
                old_lineno: None,
                new_lineno: None,
                hunk_idx: None,
            });
        } else if in_theirs {
            lines.push(DiffLine {
                kind: DiffLineKind::ConflictTheirs,
                content: line.to_string(),
                old_lineno: None,
                new_lineno: None,
                hunk_idx: None,
            });
        } else {
            lines.push(DiffLine {
                kind: DiffLineKind::Context,
                content: line.to_string(),
                old_lineno: None,
                new_lineno: None,
                hunk_idx: None,
            });
        }
    }
    lines
}

/// Accept the OURS (HEAD) version of a conflicted file.
/// Equivalent to: git checkout --ours <file> && git add <file>
pub fn resolve_ours(repo_path: &Path, file_path: &str) -> Result<(), String> {
    let output1 = git_command()
        .args(["checkout", "--ours", file_path])
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;
    if !output1.status.success() {
        return Err(String::from_utf8_lossy(&output1.stderr).to_string());
    }
    stage_file(repo_path, file_path)?;
    Ok(())
}

/// Accept the THEIRS (incoming) version of a conflicted file.
/// Equivalent to: git checkout --theirs <file> && git add <file>
pub fn resolve_theirs(repo_path: &Path, file_path: &str) -> Result<(), String> {
    let output1 = git_command()
        .args(["checkout", "--theirs", file_path])
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;
    if !output1.status.success() {
        return Err(String::from_utf8_lossy(&output1.stderr).to_string());
    }
    stage_file(repo_path, file_path)?;
    Ok(())
}

/// Mark the file as resolved (stage it) after manual edits.
pub fn mark_resolved(repo_path: &Path, file_path: &str) -> Result<(), String> {
    stage_file(repo_path, file_path)
}

/// Resolve a specific conflict hunk inside a file (Ours vs Theirs).
/// Replaces the hunk at index `hunk_idx` in the file on disk.
/// If no more conflicts remain in the file, it automatically stages the file.
pub fn resolve_conflict_hunk(
    repo_path: &Path,
    file_path: &str,
    hunk_idx: usize,
    accept_ours: bool,
) -> Result<(), String> {
    let full_path = repo_path.join(file_path);
    let content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;

    let mut new_lines = Vec::new();
    let mut lines_iter = content.lines().peekable();
    let mut current_hunk_idx = 0;

    while let Some(line) = lines_iter.next() {
        if line.starts_with("<<<<<<<") {
            let mut ours_block = Vec::new();
            let mut theirs_block = Vec::new();

            // Read ours block (until =======)
            let mut found_separator = false;
            while let Some(&next_line) = lines_iter.peek() {
                if next_line.starts_with("=======") {
                    lines_iter.next(); // consume =======
                    found_separator = true;
                    break;
                }
                if let Some(line) = lines_iter.next() {
                    ours_block.push(line.to_string());
                }
            }

            // Read theirs block (until >>>>>>>)
            let mut found_end = false;
            let mut end_line_marker = ">>>>>>>".to_string();
            while let Some(&next_line) = lines_iter.peek() {
                if next_line.starts_with(">>>>>>>") {
                    if let Some(marker) = lines_iter.next() {
                        end_line_marker = marker.to_string(); // consume >>>>>>>
                    }
                    found_end = true;
                    break;
                }
                if let Some(line) = lines_iter.next() {
                    theirs_block.push(line.to_string());
                }
            }

            if current_hunk_idx == hunk_idx {
                if accept_ours {
                    new_lines.extend(ours_block);
                } else {
                    new_lines.extend(theirs_block);
                }
            } else {
                new_lines.push(line.to_string());
                new_lines.extend(ours_block);
                if found_separator {
                    new_lines.push("=======".to_string());
                }
                new_lines.extend(theirs_block);
                if found_end {
                    new_lines.push(end_line_marker);
                }
            }

            current_hunk_idx += 1;
        } else {
            new_lines.push(line.to_string());
        }
    }

    let mut new_content = new_lines.join("\n");
    if content.ends_with('\n') && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    std::fs::write(&full_path, new_content).map_err(|e| e.to_string())?;

    // Check if any conflict markers remain in the file
    let updated_content = std::fs::read_to_string(&full_path).map_err(|e| e.to_string())?;
    let has_conflict_markers = updated_content
        .lines()
        .any(|l| l.starts_with("<<<<<<<") || l.starts_with("=======") || l.starts_with(">>>>>>>"));

    if !has_conflict_markers {
        stage_file(repo_path, file_path)?;
    }

    Ok(())
}

/// Abort the in-progress merge.
pub fn abort_merge(repo_path: &Path) -> Result<(), String> {
    let output = git_command()
        .args(["merge", "--abort"])
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}

/// Continue the merge after conflicts are resolved.
pub fn continue_merge(repo_path: &Path) -> Result<(), String> {
    let output = git_command()
        .args(["merge", "--continue"])
        .env("GIT_EDITOR", "true")
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(())
}
