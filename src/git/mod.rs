use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct GitFile {
    pub path: String,
    pub x: char,
    pub y: char,
}

#[derive(Debug, Clone)]
pub struct GitStatus {
    pub branch: String,
    pub root: PathBuf,
    pub staged: Vec<GitFile>,
    pub unstaged: Vec<GitFile>,
    /// 파일 상대경로 → (staged_status, worktree_status)
    pub file_map: HashMap<String, (char, char)>,
}

pub fn find_git_root(dir: &Path) -> Option<PathBuf> {
    let dir_str = dir.to_str()?;
    let output = Command::new("git")
        .args(["-C", dir_str, "rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if output.status.success() {
        let s = String::from_utf8(output.stdout).ok()?;
        Some(PathBuf::from(s.trim()))
    } else {
        None
    }
}

pub fn get_status(dir: &Path) -> Option<GitStatus> {
    let root = find_git_root(dir)?;
    let root_str = root.to_str()?;

    let branch = Command::new("git")
        .args(["-C", root_str, "branch", "--show-current"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "HEAD".to_string());

    let status_out = Command::new("git")
        .args([
            "-c", "core.quotepath=false",
            "-C", root_str,
            "status", "--porcelain=v1", "-u",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())?;

    let status_str = String::from_utf8(status_out.stdout).ok()?;
    let mut staged = Vec::new();
    let mut unstaged = Vec::new();
    let mut file_map = HashMap::new();

    for line in status_str.lines() {
        if line.len() < 3 {
            continue;
        }
        let bytes = line.as_bytes();
        let x = bytes[0] as char;
        let y = bytes[1] as char;
        let raw_path = &line[3..];

        // rename 형식: "old -> new" → new path 사용
        let path = if raw_path.contains(" -> ") {
            raw_path.splitn(2, " -> ").nth(1).unwrap_or(raw_path).to_string()
        } else {
            raw_path.to_string()
        };

        file_map.insert(path.clone(), (x, y));

        if x != ' ' && x != '?' {
            staged.push(GitFile { path: path.clone(), x, y });
        }
        if y != ' ' || (x == '?' && y == '?') {
            unstaged.push(GitFile { path: path.clone(), x, y });
        }
    }

    Some(GitStatus { branch, root, staged, unstaged, file_map })
}

pub fn get_diff(root: &Path, path: &str, staged: bool) -> Vec<String> {
    let root_str = match root.to_str() {
        Some(s) => s,
        None => return vec!["경로 변환 오류".to_string()],
    };

    let output = if staged {
        Command::new("git")
            .args(["-C", root_str, "diff", "--cached", "--", path])
            .output()
    } else {
        Command::new("git")
            .args(["-C", root_str, "diff", "--", path])
            .output()
    };

    let result = output
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .filter(|s| !s.is_empty())
        .map(|s| s.lines().map(|l| l.to_string()).collect::<Vec<_>>());

    if let Some(lines) = result {
        return lines;
    }

    // untracked 파일이면 파일 내용의 처음 50줄 표시
    let full_path = root.join(path);
    if full_path.is_file() {
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            let mut lines = vec![
                format!("# 새 파일 (untracked): {path}"),
                String::new(),
            ];
            lines.extend(content.lines().take(50).map(|l| format!("+{l}")));
            return lines;
        }
    }

    vec!["diff를 가져올 수 없습니다.".to_string()]
}

pub fn stage_file(root: &Path, path: &str) -> Result<(), String> {
    let out = Command::new("git")
        .args(["-C", root.to_str().unwrap_or(""), "add", "--", path])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn unstage_file(root: &Path, path: &str) -> Result<(), String> {
    let out = Command::new("git")
        .args([
            "-C", root.to_str().unwrap_or(""),
            "restore", "--staged", "--", path,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

pub fn commit_changes(root: &Path, message: &str) -> Result<(), String> {
    let out = Command::new("git")
        .args(["-C", root.to_str().unwrap_or(""), "commit", "-m", message])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// 커밋에 포함된 파일 목록: (status_char, path)
pub fn get_commit_files(root: &Path, hash: &str) -> Vec<(char, String)> {
    let root_str = root.to_str().unwrap_or("");
    Command::new("git")
        .args([
            "-C", root_str,
            "diff-tree", "--root", "--no-commit-id", "-r", "--name-status", hash,
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| {
            s.lines()
                .filter_map(|l| {
                    let bytes = l.as_bytes();
                    if bytes.is_empty() {
                        return None;
                    }
                    let status = bytes[0] as char;
                    let path = l[1..].trim().to_string();
                    if path.is_empty() {
                        return None;
                    }
                    Some((status, path))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// 커밋 내 특정 파일의 diff
pub fn get_commit_file_diff(root: &Path, hash: &str, path: &str) -> Vec<String> {
    Command::new("git")
        .args(["-C", root.to_str().unwrap_or(""), "show", hash, "--", path])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.lines().map(|l| l.to_string()).collect())
        .unwrap_or_else(|| vec!["diff를 가져올 수 없습니다.".to_string()])
}

pub fn get_log(root: &Path) -> Vec<String> {
    Command::new("git")
        .args([
            "-C", root.to_str().unwrap_or(""),
            "log", "--oneline", "--decorate", "-20",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.lines().map(|l| l.to_string()).collect())
        .unwrap_or_default()
}
