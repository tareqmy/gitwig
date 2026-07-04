#!/usr/bin/env python3
import subprocess
import re
import sys

def run_cmd(args):
    result = subprocess.run(args, capture_output=True, text=True, check=True)
    return result.stdout.strip()

def get_github_base_url():
    try:
        url = run_cmd(["git", "remote", "get-url", "origin"])
        match = re.search(r"(?:git@|https://)(github\.com)[:/]([^/]+)/([^/.]+)(?:\.git)?", url)
        if match:
            domain, user, repo = match.groups()
            if repo.endswith(".git"):
                repo = repo[:-4]
            return f"https://{domain}/{user}/{repo}"
    except Exception:
        pass
    return "https://github.com/tareqmy/gitwig"

def get_tags():
    output = run_cmd([
        "git", "for-each-ref",
        "--sort=creatordate",
        "--format=%(refname:short) %(creatordate:short)",
        "refs/tags"
    ])
    tags = []
    for line in output.split('\n'):
        if not line.strip():
            continue
        parts = line.split()
        if len(parts) >= 2:
            tags.append((parts[0], parts[1]))
        elif len(parts) == 1:
            tags.append((parts[0], ""))
    return tags

def get_commits_between(tag1, tag2):
    if tag1 is None:
        range_str = tag2
    else:
        range_str = f"{tag1}..{tag2}"
    
    output = run_cmd([
        "git", "log",
        f"--pretty=format:%h|%s|%an|%ad",
        "--date=short",
        range_str
    ])
    commits = []
    for line in output.split('\n'):
        if not line.strip():
            continue
        parts = line.split('|', 3)
        if len(parts) == 4:
            commits.append({
                'hash': parts[0],
                'subject': parts[1],
                'author': parts[2],
                'date': parts[3]
            })
    return commits

def parse_commit(subject):
    match = re.match(r"^(\w+)(?:\(([^)]+)\))?\s*:\s*(.*)$", subject)
    if match:
        ctype, scope, desc = match.groups()
        return ctype.lower(), scope, desc
    return None, None, subject

def classify_by_heuristics(subject):
    subj_lower = subject.lower()
    
    # 1. Fixed
    for kw in ["fix", "bug", "correct", "resolve", "prevent", "issue", "crash", "error", "fail", "repair", "revert"]:
        if kw in subj_lower:
            return "Fixed"
            
    # 2. Testing
    for kw in ["test", "coverage", "unittest", "spec"]:
        if kw in subj_lower:
            return "Testing"
            
    # 3. Documentation
    for kw in ["doc", "readme", "instruction", "guide", "manual", "license", "copyright"]:
        if kw in subj_lower:
            return "Documentation"
            
    # 4. Added
    for kw in ["add", "implement", "support", "introduce", "create", "new", "feat"]:
        if kw in subj_lower:
            return "Added"

    # 4b. Removed
    for kw in ["remove", "delete", "uninstall", "eliminate", "discard"]:
        if kw in subj_lower:
            return "Removed"

    # 4c. Deprecated
    for kw in ["deprecate", "obsolete"]:
        if kw in subj_lower:
            return "Deprecated"
            
    # 5. Refactored
    for kw in ["refactor", "cleanup", "clean up", "simplify", "reorganize", "deconstruct", "modularize", "extract"]:
        if kw in subj_lower:
            return "Refactored"
            
    # 6. Performance
    for kw in ["perf", "optimiz", "speed", "fast", "cache"]:
        if kw in subj_lower:
            return "Performance"
            
    # 7. Chore
    for kw in ["chore", "bump", "release", "version", "build", "ci", "github action", "workflow", "cargo publish"]:
        if kw in subj_lower:
            return "Chore"
            
    # 8. Changed
    for kw in ["change", "update", "use", "improve", "polish", "style", "format", "rename", "move", "adjust", "replace", "modify", "draft", "tweak", "align"]:
        if kw in subj_lower:
            return "Changed"
            
    return "Others"

def categorize_commits(commits, github_base_url):
    categories = {
        'Added': [],
        'Fixed': [],
        'Changed': [],
        'Removed': [],
        'Deprecated': [],
        'Performance': [],
        'Security': [],
        'Documentation': [],
        'Refactored': [],
        'Testing': [],
        'Chore': [],
        'Others': []
    }
    
    type_map = {
        'feat': 'Added',
        'fix': 'Fixed',
        'refactor': 'Refactored',
        'docs': 'Documentation',
        'test': 'Testing',
        'perf': 'Performance',
        'style': 'Changed',
        'chore': 'Chore',
        'ci': 'Chore',
        'build': 'Chore',
    }

    for c in commits:
        subj = c['subject']
        if re.match(r"^(release[:\s]|update version|updated version|version\s|bump\s)", subj, re.IGNORECASE):
            continue
        if re.match(r"^v?\d+\.\d+\.\d+$", subj.strip(), re.IGNORECASE):
            continue
            
        ctype, scope, desc = parse_commit(subj)
        
        scope_str = f"**{scope}**: " if scope else ""
        entry = f"{scope_str}{desc} ([{c['hash']}]({github_base_url}/commit/{c['hash']}))"
        
        if ctype:
            category = type_map.get(ctype, 'Others')
        else:
            category = classify_by_heuristics(subj)
            
        categories[category].append(entry)

    return {k: v for k, v in categories.items() if v}

def main():
    github_base_url = get_github_base_url()
    print(f"Using GitHub Base URL: {github_base_url}")
    
    tags = get_tags()
    if not tags:
        print("No tags found.")
        sys.exit(1)
        
    changelog = []
    changelog.append("# Changelog")
    changelog.append("")
    changelog.append("All notable changes to this project will be documented in this file.")
    changelog.append("")
    changelog.append("The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),")
    changelog.append("and this project adheres to Semantic Versioning.")
    changelog.append("")
    

    cat_order = [
        'Added',
        'Fixed',
        'Changed',
        'Removed',
        'Deprecated',
        'Performance',
        'Security',
        'Documentation',
        'Refactored',
        'Testing',
        'Chore',
        'Others'
    ]
    
    # 1. Unreleased section
    latest_tag = tags[-1][0]
    unreleased_commits = get_commits_between(latest_tag, "HEAD")
    if unreleased_commits:
        cats = categorize_commits(unreleased_commits, github_base_url)
        if cats:
            changelog.append("## [Unreleased]")
            for cat in cat_order:
                if cat in cats:
                    changelog.append(f"### {cat}")
                    for entry in cats[cat]:
                        changelog.append(f"- {entry}")
                    changelog.append("")
            
    # 2. Iterate tags in reverse order
    for i in range(len(tags) - 1, -1, -1):
        tag_name, tag_date = tags[i]
        prev_tag = tags[i - 1][0] if i > 0 else None
        
        commits = get_commits_between(prev_tag, tag_name)
        date_str = f" - {tag_date}" if tag_date else ""
        
        cats = categorize_commits(commits, github_base_url)
        if cats:
            changelog.append(f"## [{tag_name}]{date_str}")
            for cat in cat_order:
                if cat in cats:
                    changelog.append(f"### {cat}")
                    for entry in cats[cat]:
                        changelog.append(f"- {entry}")
                    changelog.append("")
        else:
            changelog.append(f"## [{tag_name}]{date_str}")
            changelog.append("No changes recorded.")
            changelog.append("")
            
    with open("CHANGELOG.md", "w") as f:
        f.write("\n".join(changelog))
    print("CHANGELOG.md generated successfully.")

if __name__ == "__main__":
    main()
