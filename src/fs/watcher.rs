use anyhow::Result;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// 파일시스템 변경 감시자
pub struct FsWatcher {
    /// notify 감시자 인스턴스 (drop시 감시 중단)
    _watcher: RecommendedWatcher,
    /// 이벤트 수신 채널
    pub receiver: Receiver<notify::Result<Event>>,
}

impl FsWatcher {
    /// 새 감시자 생성 및 경로 감시 시작
    pub fn new(path: &Path) -> Result<Self> {
        let (tx, rx) = channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                if tx.send(res).is_err() {
                    // 수신자가 닫힌 경우 조용히 무시
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(500)),
        )?;

        watcher.watch(path, RecursiveMode::NonRecursive)?;

        Ok(Self {
            _watcher: watcher,
            receiver: rx,
        })
    }

    /// 감시 경로 추가
    pub fn watch(&mut self, path: &Path) -> Result<()> {
        self._watcher.watch(path, RecursiveMode::NonRecursive)?;
        Ok(())
    }

    /// 감시 경로 제거
    pub fn unwatch(&mut self, path: &Path) -> Result<()> {
        self._watcher.unwatch(path)?;
        Ok(())
    }

    /// 보류 중인 이벤트 모두 수집 (논블로킹)
    pub fn poll_events(&self) -> Vec<Event> {
        let mut events = Vec::new();
        while let Ok(Ok(event)) = self.receiver.try_recv() {
            events.push(event);
        }
        events
    }

    /// 이벤트 종류 분류
    pub fn classify_event(event: &Event) -> FsEventKind {
        use notify::EventKind;
        match &event.kind {
            EventKind::Create(_) => FsEventKind::Created,
            EventKind::Modify(_) => FsEventKind::Modified,
            EventKind::Remove(_) => FsEventKind::Removed,
            EventKind::Access(_) => FsEventKind::Accessed,
            _ => FsEventKind::Other,
        }
    }
}

/// 파일시스템 이벤트 종류
#[derive(Debug, Clone, PartialEq)]
pub enum FsEventKind {
    Created,
    Modified,
    Removed,
    Accessed,
    Other,
}
