use std::path::Path;
use std::time::Duration;

use git2::Repository;

use crate::{
    GitRunError, detached_command, git_command, load_tab_remotes, run_git_with_timeout,
    tool_command,
};

fn run_gh_with_timeout(
    cmd: std::process::Command,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    run_git_with_timeout(cmd, timeout).map_err(|e| match e {
        GitRunError::Timeout(d) => {
            format!("GitHub CLI ('gh') did not respond within {}s and was cancelled", d.as_secs())
        }
        GitRunError::Spawn(msg) => format!(
            "GitHub CLI ('gh') not found or failed to execute: {}. Ensure 'gh' is installed and in your PATH.",
            msg
        ),
    })
}

pub fn load_tab_forge_issues(
    repo_path: &Path,
    assigned_only: bool,
    timeout: Duration,
) -> Result<Vec<ForgeIssue>, String> {
    let mut cmd = tool_command("gh");
    cmd.arg("issue").arg("list");
    if assigned_only {
        cmd.arg("--assignee").arg("@me");
    }
    cmd.arg("--limit")
        .arg("50")
        .arg("--json")
        .arg("number,title,state,author,assignees,url")
        .current_dir(repo_path);

    let output = run_gh_with_timeout(cmd, timeout)?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "GitHub CLI error: {}. Make sure you are authenticated (run 'gh auth login') and this is a GitHub repository.",
            err_msg
        ));
    }

    #[derive(serde::Deserialize)]
    struct GhAuthor {
        login: String,
    }

    #[derive(serde::Deserialize)]
    struct GhAssignee {
        login: String,
    }

    #[derive(serde::Deserialize)]
    struct GhIssue {
        number: u32,
        title: String,
        state: String,
        author: Option<GhAuthor>,
        assignees: Option<Vec<GhAssignee>>,
        url: String,
    }

    let raw_issues: Vec<GhIssue> = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse GitHub CLI response: {}", e))?;

    let issues = raw_issues
        .into_iter()
        .map(|item| {
            let author = item.author.map(|a| a.login).unwrap_or_else(|| "none".to_string());
            let assignees =
                item.assignees.unwrap_or_default().into_iter().map(|a| a.login).collect();
            ForgeIssue {
                number: item.number,
                title: item.title,
                state: item.state,
                author,
                assignees,
                url: item.url,
            }
        })
        .collect();

    Ok(issues)
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct ForgeIssue {
    pub number: u32,
    pub title: String,
    pub state: String,
    pub author: String,
    pub assignees: Vec<String>,
    pub url: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct ForgePR {
    pub number: u32,
    pub title: String,
    pub state: String,
    pub author: String,
    pub assignees: Vec<String>,
    pub url: String,
    pub head_ref: String,
    pub head_ref_oid: String,
    pub body: String,
    pub status_checks: Vec<CIStatusCheck>,
    pub reviews: Vec<PRReview>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct ForgePRComment {
    pub path: String,
    pub line: Option<u32>,
    pub body: String,
    pub author: String,
    pub commit_id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct CIStatusCheck {
    pub name: String,
    pub state: Option<String>,
    pub status: Option<String>,
    pub conclusion: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq)]
pub struct PRReview {
    pub author: String,
    pub body: String,
    pub state: String,
}

pub fn load_tab_forge_prs(repo_path: &Path, timeout: Duration) -> Result<Vec<ForgePR>, String> {
    let mut cmd = tool_command("gh");
    cmd.arg("pr")
        .arg("list")
        .arg("--limit")
        .arg("50")
        .arg("--json")
        .arg("number,title,state,author,assignees,url,headRefName,headRefOid,statusCheckRollup,body,reviews")
        .current_dir(repo_path);

    let output = run_gh_with_timeout(cmd, timeout)?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!(
            "GitHub CLI error: {}. Make sure you are authenticated (run 'gh auth login') and this is a GitHub repository.",
            err_msg
        ));
    }

    parse_forge_prs(&output.stdout)
}

/// Parses the JSON `gh pr list --json ...` prints (see `load_tab_forge_prs`).
pub(crate) fn parse_forge_prs(json: &[u8]) -> Result<Vec<ForgePR>, String> {
    #[derive(serde::Deserialize)]
    struct GhAuthor {
        login: String,
    }

    #[derive(serde::Deserialize)]
    struct GhAssignee {
        login: String,
    }

    #[derive(serde::Deserialize)]
    struct GhReview {
        author: Option<GhAuthor>,
        body: String,
        state: String,
    }

    // `statusCheckRollup` mixes two kinds of entry: check runs (GitHub
    // Actions and other apps) carry `name`, `status` and `conclusion`, while
    // commit statuses (`StatusContext`, e.g. an external CI) carry `context`
    // and `state` and no `name`. A required `name` failed the whole list.
    #[derive(serde::Deserialize)]
    struct GhStatusCheck {
        name: Option<String>,
        context: Option<String>,
        state: Option<String>,
        status: Option<String>,
        conclusion: Option<String>,
    }

    #[derive(serde::Deserialize)]
    struct GhPR {
        number: u32,
        title: String,
        state: String,
        author: Option<GhAuthor>,
        assignees: Option<Vec<GhAssignee>>,
        url: String,
        #[serde(rename = "headRefName")]
        head_ref_name: String,
        #[serde(rename = "headRefOid")]
        head_ref_oid: String,
        body: String,
        #[serde(rename = "statusCheckRollup")]
        status_check_rollup: Option<Vec<GhStatusCheck>>,
        reviews: Option<Vec<GhReview>>,
    }

    let raw_prs: Vec<GhPR> = serde_json::from_slice(json)
        .map_err(|e| format!("Failed to parse GitHub CLI response: {}", e))?;

    let prs = raw_prs
        .into_iter()
        .map(|item| {
            let author = item.author.map(|a| a.login).unwrap_or_else(|| "none".to_string());
            let assignees =
                item.assignees.unwrap_or_default().into_iter().map(|a| a.login).collect();
            let status_checks = item
                .status_check_rollup
                .unwrap_or_default()
                .into_iter()
                .map(|c| {
                    // gh reports a check run that is still going as `conclusion: ""`.
                    let non_empty = |v: Option<String>| v.filter(|s| !s.is_empty());
                    CIStatusCheck {
                        name: c.name.or(c.context).unwrap_or_else(|| "unnamed check".to_string()),
                        state: non_empty(c.state),
                        status: non_empty(c.status),
                        conclusion: non_empty(c.conclusion),
                    }
                })
                .collect();
            let reviews = item
                .reviews
                .unwrap_or_default()
                .into_iter()
                .map(|r| PRReview {
                    author: r.author.map(|a| a.login).unwrap_or_else(|| "none".to_string()),
                    body: r.body,
                    state: r.state,
                })
                .collect();

            ForgePR {
                number: item.number,
                title: item.title,
                state: item.state,
                author,
                assignees,
                url: item.url,
                head_ref: item.head_ref_name,
                head_ref_oid: item.head_ref_oid,
                body: item.body,
                status_checks,
                reviews,
            }
        })
        .collect();

    Ok(prs)
}

pub fn load_pr_comments(
    repo_path: &Path,
    pr_number: u32,
    timeout: Duration,
) -> Result<Vec<ForgePRComment>, String> {
    // GitHub pages review comments 30 at a time; ask for the largest page and
    // let gh follow the rest.
    let mut cmd = tool_command("gh");
    cmd.arg("api")
        .arg(format!("repos/:owner/:repo/pulls/{}/comments?per_page=100", pr_number))
        .arg("--paginate")
        .current_dir(repo_path);

    let output = run_gh_with_timeout(cmd, timeout)?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("Failed to load PR comments: {}", err));
    }

    parse_pr_comments(&output.stdout)
}

/// Parses `gh api --paginate` output for PR review comments: one JSON array
/// per page, printed back to back (`[...][...]`).
pub(crate) fn parse_pr_comments(json: &[u8]) -> Result<Vec<ForgePRComment>, String> {
    #[derive(serde::Deserialize)]
    struct GhUser {
        login: String,
    }

    #[derive(serde::Deserialize)]
    struct GhPRComment {
        path: String,
        line: Option<u32>,
        body: String,
        user: Option<GhUser>,
        #[serde(rename = "commit_id")]
        commit_id: String,
    }

    let mut comments = Vec::new();
    for page in serde_json::Deserializer::from_slice(json).into_iter::<Vec<GhPRComment>>() {
        let page = page.map_err(|e| format!("Failed to parse PR comments: {}", e))?;
        comments.extend(page.into_iter().map(|c| ForgePRComment {
            path: c.path,
            line: c.line,
            body: c.body,
            author: c.user.map(|u| u.login).unwrap_or_else(|| "none".to_string()),
            commit_id: c.commit_id,
        }));
    }
    Ok(comments)
}

/// The `gh api` arguments that post `body` as a review comment on `line` of
/// `path` at `commit_id`. Text fields use `-f` (sent verbatim): `-F` reads a
/// value starting with `@` as a file name, sends `123` or `true` as a number or
/// boolean, and fills `{owner}`-style placeholders, so a comment such as
/// "@alice please look" failed. Only `line` uses `-F`, to be sent as a number.
pub(crate) fn pr_line_comment_args(
    pr_number: u32,
    commit_id: &str,
    path: &str,
    line: u32,
    body: &str,
) -> Vec<String> {
    vec![
        "api".to_string(),
        format!("repos/:owner/:repo/pulls/{}/comments", pr_number),
        "--method".to_string(),
        "POST".to_string(),
        "-f".to_string(),
        format!("body={}", body),
        "-f".to_string(),
        format!("commit_id={}", commit_id),
        "-f".to_string(),
        format!("path={}", path),
        "-F".to_string(),
        format!("line={}", line),
        "-f".to_string(),
        "side=RIGHT".to_string(),
    ]
}

pub fn add_pr_line_comment(
    repo_path: &Path,
    pr_number: u32,
    commit_id: &str,
    path: &str,
    line: u32,
    body: &str,
    timeout: Duration,
) -> Result<(), String> {
    let mut cmd = tool_command("gh");
    cmd.args(pr_line_comment_args(pr_number, commit_id, path, line, body)).current_dir(repo_path);

    let output = run_gh_with_timeout(cmd, timeout)?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("Failed to add PR comment: {}", err));
    }
    Ok(())
}

pub fn checkout_pr_branch(
    repo_path: &Path,
    pr_number: u32,
    timeout: Duration,
) -> Result<String, String> {
    let mut cmd = tool_command("gh");
    cmd.arg("pr").arg("checkout").arg(pr_number.to_string()).current_dir(repo_path);

    let output = run_gh_with_timeout(cmd, timeout)?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(format!("Failed to checkout PR branch: {}", err));
    }
    Ok(format!("Checked out branch for PR #{}", pr_number))
}

/// Switches to the branch for GitHub issue `issue_number`. In order:
///
/// 1. the branch linked to the issue on GitHub (its Development section, or
///    `gh issue develop`), when there is exactly one;
/// 2. the one local branch, else the one remote-tracking branch, whose name
///    carries the number as a token of its own (see [`branch_names_issue`]);
/// 3. a new `issue-<number>` branch at HEAD.
///
/// Several candidates at any step are reported instead of guessed between.
/// The linked-branch lookup used to ask `gh issue view` for a
/// `developmentBranch` field gh does not have, so it always fell through to a
/// substring match that took issue #2 to `issue-12-login` and #1 to `v1.2`.
pub fn resolve_and_checkout_issue_branch(
    repo_path: &Path,
    issue_number: u32,
    timeout: Duration,
) -> Result<String, String> {
    // No GitHub remote, gh missing or signed out, or a server without linked
    // branches: fall back to matching branch names.
    let linked = linked_issue_branches(repo_path, issue_number, timeout).unwrap_or_default();
    match linked.as_slice() {
        [] => checkout_issue_branch_by_name(repo_path, issue_number),
        [name] => checkout_linked_issue_branch(repo_path, issue_number, name),
        many => Err(format!(
            "Issue #{} has {} linked branches ({}); check one out from the Branches tab",
            issue_number,
            many.len(),
            many.join(", ")
        )),
    }
}

/// Names of the branches linked to the issue on GitHub.
fn linked_issue_branches(
    repo_path: &Path,
    issue_number: u32,
    timeout: Duration,
) -> Result<Vec<String>, String> {
    const QUERY: &str = "query($owner: String!, $repo: String!, $number: Int!) { \
        repository(owner: $owner, name: $repo) { issue(number: $number) { \
        linkedBranches(first: 10) { nodes { ref { name } } } } } }";
    // `-F` fills gh's `{owner}` / `{repo}` placeholders from the current
    // repository and sends `number` as an integer.
    let mut cmd = tool_command("gh");
    cmd.args(["api", "graphql", "-f"])
        .arg(format!("query={}", QUERY))
        .args(["-F", "owner={owner}", "-F", "repo={repo}", "-F"])
        .arg(format!("number={}", issue_number))
        .current_dir(repo_path);
    let output = run_gh_with_timeout(cmd, timeout)?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    parse_linked_branches(&output.stdout)
}

/// Parses the `gh api graphql` response of [`linked_issue_branches`]: the
/// distinct branch names, skipping links whose branch was deleted.
pub(crate) fn parse_linked_branches(json: &[u8]) -> Result<Vec<String>, String> {
    #[derive(serde::Deserialize)]
    struct GhRef {
        name: String,
    }
    #[derive(serde::Deserialize)]
    struct GhLinkedBranch {
        #[serde(rename = "ref")]
        git_ref: Option<GhRef>,
    }
    #[derive(serde::Deserialize)]
    struct GhLinkedBranches {
        nodes: Vec<Option<GhLinkedBranch>>,
    }
    #[derive(serde::Deserialize)]
    struct GhIssue {
        #[serde(rename = "linkedBranches")]
        linked_branches: GhLinkedBranches,
    }
    #[derive(serde::Deserialize)]
    struct GhRepository {
        issue: Option<GhIssue>,
    }
    #[derive(serde::Deserialize)]
    struct GhData {
        repository: Option<GhRepository>,
    }
    #[derive(serde::Deserialize)]
    struct GhResponse {
        data: GhData,
    }

    let response: GhResponse = serde_json::from_slice(json)
        .map_err(|e| format!("Failed to parse linked branches: {}", e))?;
    let mut names: Vec<String> = Vec::new();
    let links = response.data.repository.and_then(|r| r.issue).map(|i| i.linked_branches.nodes);
    for name in links.into_iter().flatten().flatten().filter_map(|n| n.git_ref).map(|r| r.name) {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    Ok(names)
}

/// Whether `branch` names issue `issue_number`: the number appears as a token
/// of its own, after the start of the name or one of `-` `_` `/` `#`, and
/// before its end or one of `-` `_` `/`. So `issue-12-login`, `12-fix` and
/// `fix/12` name #12, while `v1.2`, `issue-123` and `issue12` do not.
pub fn branch_names_issue(branch: &str, issue_number: u32) -> bool {
    let number = issue_number.to_string();
    branch.match_indices(&number).any(|(at, _)| {
        let before = branch[..at].chars().next_back();
        let after = branch[at + number.len()..].chars().next();
        before.is_none_or(|c| matches!(c, '-' | '_' | '/' | '#'))
            && after.is_none_or(|c| matches!(c, '-' | '_' | '/'))
    })
}

/// `git checkout <args>`, with git's message as the error.
fn run_checkout(repo_path: &Path, args: &[&str]) -> Result<(), String> {
    let output = git_command()
        .arg("checkout")
        .args(args)
        .current_dir(repo_path)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(())
}

/// Checks out `branch`, linked to the issue on GitHub: the local branch, or a
/// new one tracking the single remote that has it. Only branches that exist are
/// passed to `git checkout`, as `<branch> --`: a bare name that is no branch is
/// read as a path, and would discard the local changes under it.
pub(crate) fn checkout_linked_issue_branch(
    repo_path: &Path,
    issue_number: u32,
    branch: &str,
) -> Result<String, String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    if repo.find_branch(branch, git2::BranchType::Local).is_ok() {
        run_checkout(repo_path, &[branch, "--"])?;
        return Ok(format!("Checked out '{}', linked to issue #{}", branch, issue_number));
    }
    let mut on_remotes = Vec::new();
    if let Ok(remotes) = repo.remotes() {
        for remote in remotes.iter().flatten().flatten() {
            let tracking = format!("{}/{}", remote, branch);
            if repo.find_branch(&tracking, git2::BranchType::Remote).is_ok() {
                on_remotes.push(tracking);
            }
        }
    }
    match on_remotes.as_slice() {
        [tracking] => {
            run_checkout(repo_path, &["--track", tracking])?;
            Ok(format!(
                "Checked out '{}' from '{}', linked to issue #{}",
                branch, tracking, issue_number
            ))
        }
        [] => Err(format!(
            "Issue #{} is linked to branch '{}', which is not in this clone yet; fetch it first",
            issue_number, branch
        )),
        many => Err(format!(
            "Issue #{} is linked to branch '{}', found on several remotes ({}); check one out from the Branches tab",
            issue_number,
            branch,
            many.join(", ")
        )),
    }
}

/// Steps 2 and 3 of [`resolve_and_checkout_issue_branch`]: a branch whose name
/// names the issue, else a new `issue-<number>` branch at HEAD.
pub fn checkout_issue_branch_by_name(
    repo_path: &Path,
    issue_number: u32,
) -> Result<String, String> {
    let repo = Repository::open(repo_path).map_err(|e| e.to_string())?;
    let mut local = Vec::new();
    let mut remote = Vec::new();
    for (branch, kind) in repo.branches(None).map_err(|e| e.to_string())?.flatten() {
        let Ok(Some(name)) = branch.name() else { continue };
        match kind {
            git2::BranchType::Local => {
                if branch_names_issue(name, issue_number) {
                    local.push(name.to_string());
                }
            }
            git2::BranchType::Remote => {
                // Match on the branch part, so the remote's own name cannot.
                let Some((_, short)) = name.split_once('/') else { continue };
                if short != "HEAD" && branch_names_issue(short, issue_number) {
                    remote.push(name.to_string());
                }
            }
        }
    }
    let ambiguous = |kind: &str, names: &[String]| {
        format!(
            "Several {} branches match issue #{} ({}); check one out from the Branches tab",
            kind,
            issue_number,
            names.join(", ")
        )
    };

    match local.as_slice() {
        [name] => {
            run_checkout(repo_path, &[name, "--"])?;
            return Ok(format!("Checked out matched local branch '{}'", name));
        }
        [] => {}
        many => return Err(ambiguous("local", many)),
    }
    match remote.as_slice() {
        [name] => {
            run_checkout(repo_path, &["--track", name])?;
            return Ok(format!("Checked out matched remote branch '{}'", name));
        }
        [] => {}
        many => return Err(ambiguous("remote", many)),
    }

    let new_branch_name = format!("issue-{}", issue_number);
    run_checkout(repo_path, &["-b", &new_branch_name])
        .map_err(|e| format!("Failed to create new branch: {}", e))?;
    Ok(format!("Created and switched to new branch '{}'", new_branch_name))
}

pub fn open_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = detached_command("open").arg(url).status();
    #[cfg(target_os = "windows")]
    let _ = detached_command("cmd").args(["/C", "start", url]).status();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = detached_command("xdg-open").arg(url).status();
}

pub fn get_remote_url(repo_path: &Path, remote_name: &str) -> Option<String> {
    let remotes = load_tab_remotes(repo_path).ok()?;
    remotes.into_iter().find(|r| r.name == remote_name).map(|r| r.url)
}

pub fn get_default_remote_url(repo_path: &Path) -> Option<String> {
    let remotes = load_tab_remotes(repo_path).ok()?;
    remotes.into_iter().next().map(|r| r.url)
}
