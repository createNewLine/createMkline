use std::sync::OnceLock;

pub(crate) const FONT_SIZE: u16 = 18;

#[derive(Debug)]
pub(crate) struct MkLineExe {
    pub sources: Vec<String>,
    pub target: String,
    pub status: Status,
    pub status_message: String,
    pub icons: OnceLock<crate::svg::Icons>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Idle,
    Running,
    Success,
    Error,
}

impl Default for MkLineExe {
    fn default() -> Self {
        Self {
            sources: vec![String::new()],
            target: String::new(),
            status: Status::Idle,
            status_message: "就绪".into(),
            icons: OnceLock::new(),
        }
    }
}
