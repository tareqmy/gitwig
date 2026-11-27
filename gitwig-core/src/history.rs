use std::path::Path;

use git2::Repository;

use crate::{
    FileEntry, ReflogEntry, commit_changed_files, format_relative_time, format_utc_date,
    git_command, sanitize_text,
};

#[derive(Debug, Clone, Default)]
pub struct FileRevision {
    pub commit_oid: String,
    pub author: String,
    pub date: String,
    pub when: String,
    pub summary: String,
    /// Path of the file in that revision (differs from current path before renames).
    pub path: Option<String>,
}

fn parse_file_history_output(raw: &str) -> Vec<FileRevision> {
    let mut revisions = Vec::new();
    for record in raw.split('\x1e') {
        let record = record.trim_start_matches(['\r', '\n']);
        if record.is_empty() {
            continue;
        }
        let mut lines = record.lines();
        let Some(header) = lines.next() else {
            continue;
        };
        let mut fields = header.splitn(4, '\x1f');
        let Some(oid) = fields.next().map(str::trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        let author_raw = fields.next().unwrap_or("? <?>");
        let ts_secs = fields.next().and_then(|s| s.trim().parse::<i64>().ok()).unwrap_or(0);
        let summary_raw = fields.next().unwrap_or("").trim();

        let path = lines.map(str::trim).find(|l| !l.is_empty()).map(str::to_string);
        let author = sanitize_text(author_raw);
        let when = format_relative_time(ts_secs);
        let date = format_utc_date(ts_secs);
        let summary = if summary_raw.is_empty() {
            "(no commit message)".to_string()
        } else {
            sanitize_text(summary_raw)
        };

        revisions.push(FileRevision {
            commit_oid: oid.to_string(),
            author,
            date,
            when,
            summary,
            path,
        });
    }
    revisions
}

pub fn get_file_history(repo_path: &Path, file_path: &str) -> Result<Vec<FileRevision>, String> {
    let mut cmd = git_command();
    cmd.args([
        "-c",
        "core.quotepath=false",
        "log",
        "--follow",
        "--name-only",
        "--format=%x1e%H%x1f%an <%ae>%x1f%at%x1f%s",
        "--",
        file_path,
    ])
    .env("GIT_LITERAL_PATHSPECS", "1")
    .current_dir(repo_path);

    let output = cmd.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("does not have any commits yet")
            || stderr.contains("bad default revision 'HEAD'")
        {
            return Ok(Vec::new());
        }
        return Err(stderr.trim().to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_file_history_output(&stdout))
}

#[derive(Debug, Clone)]
pub struct BlameEntry {
    pub line_no: usize,
    pub commit_id: String,
    pub author: String,
    pub date: String,
}

pub fn get_file_blame(repo_path: &Path, file_path: &str) -> Result<Vec<BlameEntry>, String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let mut opts = git2::BlameOptions::new();
    let blame = repo
        .blame_file(std::path::Path::new(file_path), Some(&mut opts))
        .map_err(|e| e.to_string())?;

    let mut entries = Vec::new();
    for i in 0..blame.len() {
        if let Some(hunk) = blame.get_index(i) {
            let commit_id = hunk.final_commit_id().to_string();
            let short_id = if commit_id.len() >= 7 { &commit_id[..7] } else { &commit_id };

            let mut author_name = "Unknown".to_string();
            let mut date_str = "unknown".to_string();
            if let Ok(commit) = repo.find_commit(hunk.final_commit_id()) {
                author_name = commit.author().name().unwrap_or("Unknown").to_string();
                date_str = format_utc_date(commit.time().seconds());
            }

            let start = hunk.final_start_line();
            let count = hunk.lines_in_hunk();
            for line_idx in 0..count {
                entries.push(BlameEntry {
                    line_no: start + line_idx,
                    commit_id: short_id.to_string(),
                    author: author_name.clone(),
                    date: date_str.clone(),
                });
            }
        }
    }

    entries.sort_by_key(|e| e.line_no);
    Ok(entries)
}

pub fn get_commit_files(repo_path: &Path, oid: &str) -> Result<Vec<FileEntry>, String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let oid = git2::Oid::from_str(oid).map_err(|e| e.to_string())?;
    let commit = repo.find_commit(oid).map_err(|e| e.to_string())?;
    Ok(commit_changed_files(&repo, &commit))
}

pub fn load_tab_reflog(repo_path: &Path) -> Result<Vec<ReflogEntry>, String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let reflog = repo.reflog("HEAD").map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    for (i, entry) in reflog.iter().enumerate() {
        let target_oid = format!("{}", entry.id_new());
        let selector = format!("HEAD@{{{}}}", i);
        let msg = entry.message().ok().flatten().unwrap_or("").to_string();

        let (command, message) = if let Some(pos) = msg.find(':') {
            (msg[..pos].trim().to_string(), msg[pos + 1..].trim().to_string())
        } else {
            (String::new(), msg)
        };

        let sig = entry.committer();
        let when_secs = sig.when().seconds();
        let when = format_relative_time(when_secs);
        let date = format_utc_date(when_secs);

        entries.push(ReflogEntry { index: i, target_oid, selector, command, message, when, date });
    }
    Ok(entries)
}

pub fn checkout_commit(repo_path: &Path, commit_oid: &str) -> Result<(), git2::Error> {
    let output = git_command()
        .arg("checkout")
        .arg(commit_oid)
        .current_dir(repo_path)
        .output()
        .map_err(|e| git2::Error::from_str(&e.to_string()))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(git2::Error::from_str(&err));
    }
    Ok(())
}
