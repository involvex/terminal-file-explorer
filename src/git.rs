use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GitStatus {
    pub branch: String,
    pub files: Vec<GitFileStatus>,
}

#[derive(Debug, Clone)]
pub struct GitFileStatus {
    pub path: String,
    pub status: FileGitStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileGitStatus {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Ignored,
    StagedModified,
    StagedDeleted,
    StagedNew,
}

impl GitStatus {
    pub fn get_for_path(path: &Path) -> Option<Self> {
        let repo_root = find_repo_root(path)?;

        let branch = get_current_branch(&repo_root)?;
        let files = get_status_files(&repo_root);

        Some(GitStatus { branch, files })
    }

    pub fn get_file_status(&self, filename: &str) -> FileGitStatus {
        for file in &self.files {
            let file_name = file.path.split('/').last().unwrap_or(&file.path);
            if file_name == filename {
                return file.status.clone();
            }
        }
        FileGitStatus::Unmodified
    }

    pub fn get_file_diff(path: &Path, filename: &str) -> Option<String> {
        let repo_root = find_repo_root(path)?;
        let output = Command::new("git")
            .args(["diff", filename])
            .current_dir(&repo_root)
            .output()
            .ok()?;

        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            None
        }
    }
}

fn find_repo_root(path: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(path)
        .output()
        .ok()?;

    if output.status.success() {
        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some(root)
    } else {
        None
    }
}

fn get_current_branch(repo_root: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

fn get_status_files(repo_root: &str) -> Vec<GitFileStatus> {
    let output = Command::new("git")
        .args(["status", "--porcelain=v1", "-uall"])
        .current_dir(repo_root)
        .output()
        .ok();

    let output = match output {
        Some(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();

    for line in output_str.lines() {
        if line.len() < 3 {
            continue;
        }

        let index_status = line.chars().next().unwrap_or(' ');
        let worktree_status = line.chars().nth(1).unwrap_or(' ');
        let path = line[3..].trim().to_string();

        let status = classify_status(index_status, worktree_status);

        files.push(GitFileStatus { path, status });
    }

    files
}

fn classify_status(index: char, worktree: char) -> FileGitStatus {
    match (index, worktree) {
        (' ', ' ') => FileGitStatus::Unmodified,
        (' ', 'M') => FileGitStatus::Modified,
        (' ', 'D') => FileGitStatus::Deleted,
        (' ', 'A') => FileGitStatus::Added,
        (' ', 'R') => FileGitStatus::Renamed,
        (' ', 'C') => FileGitStatus::Copied,
        ('?', '?') => FileGitStatus::Untracked,
        ('!', '!') => FileGitStatus::Ignored,
        ('M', ' ') => FileGitStatus::StagedModified,
        ('M', 'M') => FileGitStatus::StagedModified,
        ('A', ' ') => FileGitStatus::StagedNew,
        ('A', 'A') => FileGitStatus::StagedNew,
        ('D', ' ') => FileGitStatus::StagedDeleted,
        ('R', ' ') => FileGitStatus::Renamed,
        ('C', ' ') => FileGitStatus::Copied,
        _ => FileGitStatus::Unmodified,
    }
}
