use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// 파일 항목 정보
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// 파일명
    pub name: String,
    /// 전체 경로
    pub path: PathBuf,
    /// 디렉토리 여부
    pub is_dir: bool,
    /// 파일 크기 (bytes)
    pub size: u64,
    /// 최종 수정 시각 (Unix timestamp)
    pub modified: u64,
    /// 숨김 파일 여부
    pub is_hidden: bool,
}

impl FileEntry {
    /// 경로에서 FileEntry 생성
    pub fn from_path(path: PathBuf) -> Result<Self> {
        let metadata = path.metadata().context("메타데이터 읽기 실패")?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let is_hidden = name.starts_with('.');
        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Ok(Self {
            name,
            is_dir: metadata.is_dir(),
            size: if metadata.is_file() { metadata.len() } else { 0 },
            modified,
            is_hidden,
            path,
        })
    }
}

/// 디렉토리 내용 목록 반환 (디렉토리 먼저, 이름순 정렬)
pub fn list_dir(dir: &Path) -> Result<Vec<FileEntry>> {
    let mut entries: Vec<FileEntry> = fs::read_dir(dir)
        .with_context(|| format!("디렉토리 읽기 실패: {}", dir.display()))?
        .filter_map(|e| e.ok())
        .filter_map(|e| FileEntry::from_path(e.path()).ok())
        .collect();

    // 디렉토리 우선, 이름 오름차순
    entries.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name))
    });

    Ok(entries)
}

/// 파일 복사 (대상 경로로)
pub fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        copy_dir_recursive(src, dst)
    } else {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)
            .with_context(|| format!("복사 실패: {} → {}", src.display(), dst.display()))?;
        Ok(())
    }
}

/// 디렉토리 재귀 복사
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// 파일/디렉토리 이동 (이름변경 포함)
pub fn move_file(src: &Path, dst: &Path) -> Result<()> {
    // 같은 파일시스템이면 rename, 아니면 복사 후 삭제
    match fs::rename(src, dst) {
        Ok(_) => Ok(()),
        Err(_) => {
            copy_file(src, dst)?;
            delete_file(src)?;
            Ok(())
        }
    }
}

/// 파일/디렉토리 삭제
pub fn delete_file(path: &Path) -> Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
            .with_context(|| format!("디렉토리 삭제 실패: {}", path.display()))?;
    } else {
        fs::remove_file(path)
            .with_context(|| format!("파일 삭제 실패: {}", path.display()))?;
    }
    Ok(())
}

/// 파일/디렉토리 이름 변경
pub fn rename_file(path: &Path, new_name: &str) -> Result<PathBuf> {
    let new_path = path
        .parent()
        .map(|p| p.join(new_name))
        .unwrap_or_else(|| PathBuf::from(new_name));
    fs::rename(path, &new_path)
        .with_context(|| format!("이름 변경 실패: {} → {new_name}", path.display()))?;
    Ok(new_path)
}

/// 새 디렉토리 생성
pub fn create_dir(parent: &Path, name: &str) -> Result<PathBuf> {
    let new_dir = parent.join(name);
    fs::create_dir_all(&new_dir)
        .with_context(|| format!("디렉토리 생성 실패: {}", new_dir.display()))?;
    Ok(new_dir)
}

/// 파일 크기 합계 (디렉토리 재귀)
pub fn dir_size(path: &Path) -> u64 {
    if path.is_file() {
        return path.metadata().map(|m| m.len()).unwrap_or(0);
    }
    fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| dir_size(&e.path()))
                .sum()
        })
        .unwrap_or(0)
}

/// 파일 퍼미션 문자열 (Unix 스타일, Windows에서는 간략화)
pub fn permission_string(path: &Path) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = path.metadata() {
            let mode = meta.permissions().mode();
            let chars = [
                if mode & 0o400 != 0 { 'r' } else { '-' },
                if mode & 0o200 != 0 { 'w' } else { '-' },
                if mode & 0o100 != 0 { 'x' } else { '-' },
                if mode & 0o040 != 0 { 'r' } else { '-' },
                if mode & 0o020 != 0 { 'w' } else { '-' },
                if mode & 0o010 != 0 { 'x' } else { '-' },
                if mode & 0o004 != 0 { 'r' } else { '-' },
                if mode & 0o002 != 0 { 'w' } else { '-' },
                if mode & 0o001 != 0 { 'x' } else { '-' },
            ];
            return chars.iter().collect();
        }
    }
    "rwxr-xr-x".to_string()
}
