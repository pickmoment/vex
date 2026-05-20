use crate::app::GitSection;

pub enum ConfirmKind {
    DeleteBranchSoft(String),
    DeleteBranchForce(String),
    CheckoutFile(String),
    ForcePush(String),
}

pub enum AsyncKind {
    Push { force: bool, branch: String },
    Pull,
    Fetch,
}

pub struct GitState {
    pub status: Option<crate::git::GitStatus>,
    pub section: GitSection,
    pub staged_idx: usize,
    pub unstaged_idx: usize,
    pub diff: Vec<String>,
    pub diff_scroll: u16,
    pub is_committing: bool,
    pub commit_input: String,
    pub show_log: bool,
    pub log: Vec<String>,
    pub log_focused: bool,
    pub log_idx: usize,
    pub log_file_focused: bool,
    pub commit_files: Vec<(char, String)>,
    pub commit_file_idx: usize,
    pub commit_show: Vec<String>,
    pub commit_show_scroll: u16,
    pub diff_h_scroll: u16,
    pub diff_wrap: bool,
    pub diff_fullscreen: bool,
    pub diff_panel_height: u16,

    // 브랜치 패널
    pub branches: Vec<crate::git::BranchInfo>,
    pub branch_panel_open: bool,
    pub branch_idx: usize,

    // 브랜치 생성 입력 모달
    pub branch_input_active: bool,
    pub branch_input: String,

    // 위험 명령 확인 모달
    pub confirm: Option<ConfirmKind>,

    // 비동기 명령 진행
    pub async_kind: Option<AsyncKind>,
    pub async_child: Option<std::process::Child>,
    pub async_started_at: Option<std::time::Instant>,
    pub spinner_tick: u8,
}

impl GitState {
    pub fn new(root: &std::path::Path) -> Self {
        Self {
            status: crate::git::get_status(root),
            section: GitSection::Unstaged,
            staged_idx: 0,
            unstaged_idx: 0,
            diff: Vec::new(),
            diff_scroll: 0,
            is_committing: false,
            commit_input: String::new(),
            show_log: false,
            log: Vec::new(),
            log_focused: false,
            log_idx: 0,
            log_file_focused: false,
            commit_files: Vec::new(),
            commit_file_idx: 0,
            commit_show: Vec::new(),
            commit_show_scroll: 0,
            diff_h_scroll: 0,
            diff_wrap: false,
            diff_fullscreen: false,
            diff_panel_height: 0,
            branches: Vec::new(),
            branch_panel_open: false,
            branch_idx: 0,
            branch_input_active: false,
            branch_input: String::new(),
            confirm: None,
            async_kind: None,
            async_child: None,
            async_started_at: None,
            spinner_tick: 0,
        }
    }

    pub fn refresh(&mut self, root: &std::path::Path) {
        self.status = crate::git::get_status(root);
        if let Some(ref status) = self.status {
            if self.staged_idx >= status.staged.len().max(1) {
                self.staged_idx = status.staged.len().saturating_sub(1);
            }
            if self.unstaged_idx >= status.unstaged.len().max(1) {
                self.unstaged_idx = status.unstaged.len().saturating_sub(1);
            }
        }
    }

    pub fn load_diff(&mut self) {
        self.diff_scroll = 0;
        self.diff_h_scroll = 0;
        if let Some(ref status) = self.status {
            let (file, is_staged) = match self.section {
                GitSection::Staged => (status.staged.get(self.staged_idx), true),
                GitSection::Unstaged => (status.unstaged.get(self.unstaged_idx), false),
            };
            if let Some(f) = file {
                let root = status.root.clone();
                let path = f.path.clone();
                self.diff = crate::git::get_diff(&root, &path, is_staged);
            }
        }
    }

    pub fn load_commit_show(&mut self) {
        if let Some(ref status) = self.status {
            if let Some(entry) = self.log.get(self.log_idx) {
                let hash = entry.split_whitespace().next().unwrap_or("").to_string();
                if !hash.is_empty() {
                    let root = status.root.clone();
                    self.commit_files = crate::git::get_commit_files(&root, &hash);
                    self.commit_file_idx = 0;
                    self.commit_show.clear();
                    self.commit_show_scroll = 0;
                }
            }
        }
    }

    pub fn load_commit_file_diff(&mut self) {
        if let Some(ref status) = self.status {
            if let Some(log_entry) = self.log.get(self.log_idx) {
                let hash = log_entry.split_whitespace().next().unwrap_or("").to_string();
                if let Some((_, path)) = self.commit_files.get(self.commit_file_idx) {
                    let root = status.root.clone();
                    let path = path.clone();
                    self.commit_show = crate::git::get_commit_file_diff(&root, &hash, &path);
                    self.commit_show_scroll = 0;
                    self.diff_h_scroll = 0;
                }
            }
        }
    }
}
