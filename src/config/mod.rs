use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 외부 프로그램 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenerConfig {
    /// 메뉴에 표시되는 이름
    pub name: String,
    /// 실행 명령어
    pub command: String,
    /// 파일 경로 앞에 붙는 추가 인수
    #[serde(default)]
    pub args: Vec<String>,
    /// true: TUI를 일시 해제하고 종료 대기 (vim 등 터미널 프로그램)
    #[serde(default)]
    pub terminal: bool,
}

fn default_openers() -> Vec<OpenerConfig> {
    vec![OpenerConfig {
        name: "VS Code".to_string(),
        command: "code".to_string(),
        args: vec![],
        terminal: false,
    }]
}

/// VEX 전체 설정 구조체
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 기본 설정
    pub general: GeneralConfig,
    /// UI 설정
    pub ui: UiConfig,
    /// 키맵 설정
    pub keymap: KeymapConfig,
    /// 미리보기 설정
    pub preview: PreviewConfig,
    /// 즐겨찾기 목록 (영속 저장)
    #[serde(default)]
    pub bookmarks: Vec<PathBuf>,
    /// 외부 프로그램 목록
    #[serde(default = "default_openers")]
    pub openers: Vec<OpenerConfig>,
}

/// 일반 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// 시작 디렉토리 (기본: 홈 디렉토리)
    pub start_dir: Option<PathBuf>,
    /// 숨김 파일 표시 여부
    pub show_hidden: bool,
    /// 파일 정렬 방식
    pub sort_by: SortBy,
    /// 정렬 방향
    pub sort_descending: bool,
    /// 첫 실행 온보딩 완료 여부
    pub onboarding_done: bool,
}

/// 파일 정렬 기준
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SortBy {
    Name,
    Size,
    Modified,
    Extension,
}

/// UI 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// 테마 이름
    pub theme: String,
    /// 즐겨찾기 패널 표시 여부
    pub show_bookmarks_panel: bool,
    /// 미리보기 패널 표시 여부
    pub show_preview_panel: bool,
    /// 힌트 바 표시 여부
    pub show_hint_bar: bool,
    /// 아이콘 표시 여부 (Nerd Font 필요)
    pub show_icons: bool,
}

/// 키맵 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeymapConfig {
    /// vim 스타일 hjkl 활성화
    pub vim_keys: bool,
}

/// 미리보기 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviewConfig {
    /// 마크다운 렌더링 기본 모드 (true=렌더링, false=raw)
    pub markdown_render: bool,
    /// 이미지 렌더링 프로토콜 우선순위
    pub image_protocol: ImageProtocol,
    /// 코드 하이라이팅 테마
    pub syntax_theme: String,
    /// 미리보기 최대 파일 크기 (bytes)
    pub max_file_size: u64,
    /// 자동 줄바꿈 여부
    #[serde(default)]
    pub wrap: bool,
}

/// 이미지 렌더링 프로토콜
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImageProtocol {
    Auto,
    Kitty,
    ITerm2,
    Sixel,
    Braille,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                start_dir: dirs::home_dir(),
                show_hidden: false,
                sort_by: SortBy::Name,
                sort_descending: false,
                onboarding_done: false,
            },
            ui: UiConfig {
                theme: "default".to_string(),
                show_bookmarks_panel: true,
                show_preview_panel: true,
                show_hint_bar: true,
                show_icons: true,
            },
            keymap: KeymapConfig {
                vim_keys: true,
            },
            preview: PreviewConfig {
                markdown_render: true,
                image_protocol: ImageProtocol::Auto,
                syntax_theme: "Solarized (dark)".to_string(),
                max_file_size: 10 * 1024 * 1024, // 10MB
                wrap: false,
            },
            bookmarks: Vec::new(),
            openers: default_openers(),
        }
    }
}

impl Config {
    /// 설정 파일에서 로드 (없으면 기본값 사용)
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();
        if let Some(path) = &config_path {
            if path.exists() {
                let content = std::fs::read_to_string(path)?;
                match toml::from_str(&content) {
                    Ok(config) => return Ok(config),
                    Err(e) => {
                        log::warn!("설정 파일 파싱 실패, 기본값 사용: {e}");
                    }
                }
            }
        }
        Ok(Self::default())
    }

    /// 설정 파일로 저장
    pub fn save(&self) -> Result<()> {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let content = toml::to_string_pretty(self)?;
            std::fs::write(path, content)?;
        }
        Ok(())
    }

    /// 설정 파일 경로
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("vex").join("config.toml"))
    }
}
