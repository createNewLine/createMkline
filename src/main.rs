#![windows_subsystem = "windows"]

mod ops;

use iced::{
    alignment,
    theme,
    widget::{button, column, row, scrollable, text, text_input, Space},
    window, Application, Command, Element, Length, Settings, Theme, Color, Font,
};
use std::borrow::Cow;
use std::path::Path;

fn main() -> iced::Result {
    // Load SimHei (黑体) from system — no embedded font, saves ~9MB binary size
    let font_data: Cow<'static, [u8]> =
        std::fs::read(r"C:\Windows\Fonts\simhei.ttf")
            .map(Cow::Owned)
            .unwrap_or_default();

    MkLineExe::run(Settings {
        window: window::Settings {
            size: iced::Size::new(680.0, 500.0),
            min_size: Some(iced::Size::new(520.0, 380.0)),
            exit_on_close_request: true,
            ..Default::default()
        },
        fonts: if font_data.is_empty() {
            vec![]
        } else {
            vec![font_data]
        },
        default_font: Font::with_name("SimHei"),
        default_text_size: iced::Pixels(16.0),
        ..Default::default()
    })
}

// ── State ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct MkLineExe {
    sources: Vec<String>,
    target: String,
    status: Status,
    status_message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
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
        }
    }
}

// ── Message ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Message {
    // Source rows
    AddSource,
    RemoveSource(usize),
    SourcePath(usize, String),
    BrowseSourceDir(usize),
    SourceDirPicked(usize, Option<String>),

    // Target
    TargetPath(String),
    BrowseTargetDir,
    TargetDirPicked(Option<String>),

    // Buttons
    Confirm,
    Cancel,
    BackupAll,
    ClearAll,

    // Async outcomes
    ConfirmResult(Result<String, String>),
    BackupResult(Result<String, String>),
}

// ── Application impl ───────────────────────────────────────────────

impl Application for MkLineExe {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (Self::default(), Command::none())
    }

    fn title(&self) -> String {
        "目录软链接迁移工具".into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn update(&mut self, msg: Message) -> Command<Message> {
        match msg {
            // ── Source rows ───────────────────────────────────────
            Message::AddSource => {
                if self.sources.len() < 10 {
                    self.sources.push(String::new());
                }
                Command::none()
            }
            Message::RemoveSource(i) => {
                if self.sources.len() > 1 && i < self.sources.len() {
                    self.sources.remove(i);
                }
                Command::none()
            }
            Message::SourcePath(i, v) => {
                if self.status == Status::Running {
                    return Command::none();
                }
                if i < self.sources.len() {
                    self.sources[i] = v;
                }
                Command::none()
            }
            Message::BrowseSourceDir(i) => {
                Command::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .set_title("选择源目录")
                            .pick_folder()
                            .await
                            .map(|h| h.path().display().to_string())
                    },
                    move |r| Message::SourceDirPicked(i, r),
                )
            }
            Message::SourceDirPicked(i, Some(path)) => {
                if i < self.sources.len() {
                    self.sources[i] = path;
                }
                Command::none()
            }
            Message::SourceDirPicked(_, None) => Command::none(),

            // ── Target ────────────────────────────────────────────
            Message::TargetPath(v) => {
                if self.status == Status::Running {
                    return Command::none();
                }
                self.target = v;
                Command::none()
            }
            Message::BrowseTargetDir => {
                Command::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("选择目标目录")
                            .pick_folder()
                            .await
                            .map(|h| h.path().display().to_string())
                    },
                    Message::TargetDirPicked,
                )
            }
            Message::TargetDirPicked(Some(path)) => {
                self.target = path;
                Command::none()
            }
            Message::TargetDirPicked(None) => Command::none(),

            // ── Cancel ────────────────────────────────────────────
            Message::Cancel => {
                std::process::exit(0);
            }

            // ── Confirm ───────────────────────────────────────────
            Message::Confirm => {
                if self.status == Status::Running {
                    return Command::none();
                }
                let sources: Vec<String> = self
                    .sources
                    .iter()
                    .filter(|s| !s.trim().is_empty())
                    .cloned()
                    .collect();
                let target = self.target.trim().to_string();

                if sources.is_empty() || target.is_empty() {
                    self.status = Status::Error;
                    self.status_message = "请填写源目录和目标目录".into();
                    return Command::none();
                }

                self.status = Status::Running;
                self.status_message = "正在处理...".into();

                Command::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            ops::execute_confirm(&sources, &target)
                        })
                        .await
                        .unwrap_or_else(|e| Err(format!("线程错误: {}", e)))
                    },
                    Message::ConfirmResult,
                )
            }

            Message::ConfirmResult(Ok(msg)) => {
                self.status = Status::Success;
                self.status_message = msg;
                Command::none()
            }
            Message::ConfirmResult(Err(e)) => {
                self.status = Status::Error;
                self.status_message = e;
                Command::none()
            }

            // ── Backup ────────────────────────────────────────────
            Message::BackupAll => {
                if self.status == Status::Running {
                    return Command::none();
                }
                let sources: Vec<String> = self
                    .sources
                    .iter()
                    .filter(|s| !s.trim().is_empty())
                    .cloned()
                    .collect();

                if sources.is_empty() {
                    self.status = Status::Error;
                    self.status_message = "没有可备份的源目录".into();
                    return Command::none();
                }

                self.status = Status::Running;
                self.status_message = "正在备份...".into();

                Command::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let results: Vec<String> = sources
                                .iter()
                                .map(|src| match ops::backup_one(Path::new(src)) {
                                    Ok(path) => format!("✓ {}", path),
                                    Err(e) => format!("✗ {}", e),
                                })
                                .collect();
                            if results.iter().all(|r| r.starts_with('✓')) {
                                Ok(results.join("\n"))
                            } else {
                                Err(results.join("\n"))
                            }
                        })
                        .await
                        .unwrap_or_else(|e| Err(format!("线程错误: {}", e)))
                    },
                    Message::BackupResult,
                )
            }

            Message::BackupResult(Ok(msg)) => {
                self.status = Status::Success;
                self.status_message = msg;
                Command::none()
            }
            Message::BackupResult(Err(e)) => {
                self.status = Status::Error;
                self.status_message = e;
                Command::none()
            }

            // ── Clear ─────────────────────────────────────────────
            Message::ClearAll => {
                if self.status == Status::Running {
                    return Command::none();
                }
                for src in &mut self.sources{
                    src.clear();
                }
                self.target.clear();
                self.status = Status::Idle;
                self.status_message = "就绪".into();
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let running = self.status == Status::Running;

        // ── Title ─────────────────────────────────────────────────
        let title = text("目录软链接迁移工具").size(22);

        // ── Source section ────────────────────────────────────────
        let source_header = text("源目录 / 源文件:").size(15);

        let source_rows: Vec<Element<Message>> = self
            .sources
            .iter()
            .enumerate()
            .map(|(i, src)| source_row(i, src, self.sources.len(), running))
            .collect();

        let can_add = !running && self.sources.len() < 10;
        let add_btn = button(
            text("+ 添加源目录")
                .horizontal_alignment(alignment::Horizontal::Center)
                .width(140),
        )
        .on_press_maybe(if can_add { Some(Message::AddSource) } else { None })
        .style(theme::Button::Text)
        .width(140);

        // Fill remaining height, scroll when overflow
        let source_area = scrollable(
            column![
                Space::with_height(2),
                column(source_rows).spacing(4),
                Space::with_height(4),
                add_btn,
            ]
        )
        .height(Length::Fill);

        // ── Target section ────────────────────────────────────────
        let target_header = text("目标目录:").size(15);
        let target_row = target_input(&self.target, running);

        // ── Status ────────────────────────────────────────────────
        let (status_color, status_icon) = match self.status {
            Status::Idle => (Color::from_rgb(0.55, 0.55, 0.55), "●"),
            Status::Running => (Color::from_rgb(0.35, 0.65, 1.0), "◉"),
            Status::Success => (Color::from_rgb(0.25, 0.80, 0.45), "●"),
            Status::Error => (Color::from_rgb(0.95, 0.30, 0.30), "●"),
        };
        let status_text = text(format!("{}  {}", status_icon, self.status_message))
            .style(theme::Text::Color(status_color))
            .size(14);

        // ── Bottom buttons ────────────────────────────────────────
        let can_confirm = !running
            && !self.target.trim().is_empty()
            && self.sources.iter().any(|s| !s.trim().is_empty());

        let confirm_btn = button(
            text("确 定")
                .horizontal_alignment(alignment::Horizontal::Center)
                .width(120),
        )
        .on_press_maybe(if can_confirm { Some(Message::Confirm) } else { None })
        .style(theme::Button::Primary)
        .width(120);

        let cancel_btn = button(
            text("取 消")
                .horizontal_alignment(alignment::Horizontal::Center)
                .width(120),
        )
        .on_press(Message::Cancel)
        .style(theme::Button::Secondary)
        .width(120);

        let backup_btn = button(
            text("备 份")
                .horizontal_alignment(alignment::Horizontal::Center)
                .width(80),
        )
        .on_press_maybe(if running { None } else { Some(Message::BackupAll) })
        .width(80);

        let clear_btn = button(
            text("清 空")
                .horizontal_alignment(alignment::Horizontal::Center)
                .width(80),
        )
        .on_press_maybe(if running { None } else { Some(Message::ClearAll) })
        .style(theme::Button::Secondary)
        .width(80);

        let bottom_row = row![
            Space::with_width(Length::Fill),
            confirm_btn,
            Space::with_width(8),
            cancel_btn,
            Space::with_width(8),
            backup_btn,
            Space::with_width(8),
            clear_btn,
            Space::with_width(Length::Fill),
        ];

        // ── Main layout ───────────────────────────────────────────
        column![
            title,
            Space::with_height(12),
            source_header,
            Space::with_height(6),
            source_area,
            Space::with_height(12),
            target_header,
            Space::with_height(6),
            target_row,
            Space::with_height(8),
            status_text,
            Space::with_height(12),
            bottom_row,
        ]
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

// ── View helpers ───────────────────────────────────────────────────

fn source_row(i: usize, value: &str, total: usize, disabled: bool) -> Element<'_, Message> {
    let input = text_input("输入或选择源目录...", value)
        .on_input(move |v| Message::SourcePath(i, v))
        .padding(6)
        .width(Length::Fill);

    let browse_btn = button(
        text("浏览")
            .horizontal_alignment(alignment::Horizontal::Center)
            .width(40),
    )
    .on_press_maybe(if disabled {
        None
    } else {
        Some(Message::BrowseSourceDir(i))
    })
    .style(theme::Button::Text)
    .width(40);

    let del_btn = button(
        text("删除")
            .horizontal_alignment(alignment::Horizontal::Center)
            .width(40),
    )
    .on_press_maybe(if disabled || total <= 1 {
        None
    } else {
        Some(Message::RemoveSource(i))
    })
    .style(theme::Button::Text)
    .width(40);

    row![input, browse_btn, del_btn]
        .spacing(4)
        .align_items(iced::Alignment::Center)
        .into()
}

fn target_input(value: &str, disabled: bool) -> Element<'_, Message> {
    let input = text_input("输入或选择目标目录...", value)
        .on_input(Message::TargetPath)
        .padding(6)
        .width(Length::Fill);

    let browse_btn = button(
        text("浏览")
            .horizontal_alignment(alignment::Horizontal::Center)
            .width(40),
    )
    .on_press_maybe(if disabled {
        None
    } else {
        Some(Message::BrowseTargetDir)
    })
    .style(theme::Button::Text)
    .width(40);

    row![input, browse_btn]
        .spacing(4)
        .align_items(iced::Alignment::Center)
        .into()
}
