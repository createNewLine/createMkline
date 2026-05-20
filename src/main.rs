#![windows_subsystem = "windows"]

mod ops;
mod svg;

use iced::{
    alignment,
    theme,
    widget::{button, column, image, row, scrollable, text, text_input, Space},
    window, Application, Color, Command, Element, Font, Length, Settings, Theme,
};
use std::path::Path;
use std::sync::OnceLock;

fn main() -> iced::Result {
    MkLineExe::run(Settings {
        window: window::Settings {
            size: iced::Size::new(680.0, 500.0),
            min_size: Some(iced::Size::new(520.0, 380.0)),
            exit_on_close_request: true,
            ..Default::default()
        },
        default_font: Font::with_name("SimHei"),
        default_text_size: iced::Pixels(15.0),
        ..Default::default()
    })
}

// ── State ──────────────────────────────────────────────────────────

const FONT_SIZE: u16 = 18;

#[derive(Debug)]
struct MkLineExe {
    sources: Vec<String>,
    target: String,
    status: Status,
    status_message: String,
    icons: OnceLock<svg::Icons>,
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
            icons: OnceLock::new(),
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
    SourceDirsPicked(usize, Vec<String>),

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
                let filled = self
                    .sources
                    .iter()
                    .filter(|s| !s.trim().is_empty())
                    .count();
                Command::perform(
                    async move {
                        let paths: Vec<String> = rfd::AsyncFileDialog::new()
                            .set_title("选择源目录（可多选）")
                            .pick_folders()
                            .await
                            .map(|handles| {
                                handles
                                    .into_iter()
                                    .map(|h| h.path().display().to_string())
                                    .collect()
                            })
                            .unwrap_or_default();

                        let max_take = 10usize.saturating_sub(filled);
                        if paths.len() > max_take {
                            tokio::task::spawn_blocking(move || {
                                rfd::MessageDialog::new()
                                    .set_title("提示")
                                    .set_description("最多选择十个源目录/源文件")
                                    .show();
                            })
                            .await
                            .ok();
                            paths.into_iter().take(max_take).collect()
                        } else {
                            paths
                        }
                    },
                    move |paths| Message::SourceDirsPicked(i, paths),
                )
            }
            Message::SourceDirsPicked(i, paths) => {
                if paths.is_empty() || i >= self.sources.len() {
                    return Command::none();
                }

                let mut iter = paths.into_iter();
                self.sources[i] = iter.next().unwrap();

                let filled = self.sources.iter().filter(|s| !s.trim().is_empty()).count();
                let mut slots_left = 10usize.saturating_sub(filled);

                // Existing empty rows (skip i, already set)
                for idx in 0..self.sources.len() {
                    if slots_left == 0 {
                        break;
                    }
                    if !self.sources[idx].trim().is_empty() {
                        continue;
                    }
                    if let Some(p) = iter.next() {
                        self.sources[idx] = p;
                        slots_left -= 1;
                    } else {
                        break;
                    }
                }

                // Add new rows for overflow
                while slots_left > 0 {
                    if let Some(p) = iter.next() {
                        self.sources.push(p);
                        slots_left -= 1;
                    } else {
                        break;
                    }
                }
                Command::none()
            }

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
                for src in &mut self.sources {
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
        let icons = self.icons.get_or_init(svg::Icons::load).clone();

        // ── Title ─────────────────────────────────────────────────
        let accent = Color::from_rgb(0.25, 0.60, 0.95);
        let title = row![
            text("目录软链接迁移工具")
                .size(22)
                .style(theme::Text::Color(Color::from_rgb(0.90, 0.90, 0.95))),
            Space::with_width(Length::Fill),
            text(format!("v{}", env!("CARGO_PKG_VERSION")))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb(0.35, 0.35, 0.40))),
        ];

        // ── Source section ────────────────────────────────────────
        let source_label = text("源目录 / 源文件")
            .size(FONT_SIZE)
            .style(theme::Text::Color(Color::from_rgb(0.50, 0.55, 0.65)));

        let source_rows: Vec<Element<Message>> = self
            .sources
            .iter()
            .enumerate()
            .map(|(i, src)| {
                source_row(
                    i,
                    src,
                    self.sources.len(),
                    running,
                    icons.clone(),
                )
            })
            .collect();

        let can_add = !running && self.sources.len() < 10;
        let add_btn = button(
            text("+ 添加源")
                .horizontal_alignment(alignment::Horizontal::Center)
                .size(FONT_SIZE),
        )
        .on_press_maybe(if can_add {
            Some(Message::AddSource)
        } else {
            None
        })
        .style(theme::Button::Text)
        .padding([2, 12]);

        let source_area = scrollable(
            column![
                Space::with_height(2),
                column(source_rows).spacing(3),
                Space::with_height(4),
                add_btn,
            ],
        )
        .height(Length::Fill);

        // ── Target section ────────────────────────────────────────
        let target_label = text("目标目录")
            .size(FONT_SIZE)
            .style(theme::Text::Color(Color::from_rgb(0.50, 0.55, 0.65)));
        let target_row = target_input(&self.target, running, icons.clone());

        // ── Status ────────────────────────────────────────────────
        let (status_color, status_icon) = match self.status {
            Status::Idle => (Color::from_rgb(0.45, 0.45, 0.50), "●"),
            Status::Running => (accent, "◉"),
            Status::Success => (Color::from_rgb(0.25, 0.80, 0.45), "●"),
            Status::Error => (Color::from_rgb(0.95, 0.30, 0.30), "●"),
        };
        let status_text = text(format!("{}  {}", status_icon, self.status_message))
            .style(theme::Text::Color(status_color))
            .size(FONT_SIZE);

        // ── Separator ─────────────────────────────────────────────
        let separator = Space::with_height(1);

        // ── Bottom buttons ────────────────────────────────────────
        let can_confirm = !running
            && !self.target.trim().is_empty()
            && self.sources.iter().any(|s| !s.trim().is_empty());

        let confirm_btn = button(
            text("确 定")
                .horizontal_alignment(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .on_press_maybe(if can_confirm {
            Some(Message::Confirm)
        } else {
            None
        })
        .style(theme::Button::Primary)
        .width(100);

        let cancel_btn = button(
            text("取 消")
                .horizontal_alignment(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .on_press(Message::Cancel)
        .style(theme::Button::Secondary)
        .width(80);

        let backup_btn = button(
            text("备 份")
                .horizontal_alignment(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .on_press_maybe(if running {
            None
        } else {
            Some(Message::BackupAll)
        })
        .width(70);

        let clear_btn = button(
            text("清 空")
                .horizontal_alignment(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
        .on_press_maybe(if running {
            None
        } else {
            Some(Message::ClearAll)
        })
        .style(theme::Button::Secondary)
        .width(70);

        let bottom_row = row![
            Space::with_width(Length::Fill),
            confirm_btn,
            Space::with_width(6),
            cancel_btn,
            Space::with_width(6),
            backup_btn,
            Space::with_width(6),
            clear_btn,
            Space::with_width(Length::Fill),
        ];

        // ── Main layout ───────────────────────────────────────────
        column![
            title,
            Space::with_height(14),
            source_label,
            Space::with_height(4),
            source_area,
            Space::with_height(12),
            target_label,
            Space::with_height(4),
            target_row,
            Space::with_height(10),
            separator,
            Space::with_height(8),
            status_text,
            Space::with_height(8),
            bottom_row,
        ]
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

// ── View helpers ───────────────────────────────────────────────────

fn icon_btn<'a>(
    handle: image::Handle,
    on_press: Option<Message>,
    width:Option<f32> ,
    height: Option<f32>
) -> Element<'a, Message> {
    let default_size: f32 = 20.0;
    let width = width.unwrap_or(default_size);
    let height = height.unwrap_or(default_size);
    button(
        image::Image::new(handle)
            .width(Length::Fixed(width))
            .height(Length::Fixed(height)),
    )
    .on_press_maybe(on_press)
    .style(theme::Button::Text)
    .padding(5)
    .into()
}

fn source_row(
    i: usize,
    value: &str,
    total: usize,
    disabled: bool,
    icons: svg::Icons,
) -> Element<'static, Message> {
    let input = text_input("输入或选择源目录...", value)
        .on_input(move |v| Message::SourcePath(i, v))
        .padding(6)
        .size(FONT_SIZE)
        .width(Length::Fill);

    let browse_btn = icon_btn(
        icons.folder.clone(),
        if disabled {
            None
        } else {
            Some(Message::BrowseSourceDir(i))
        },
        Some(30.0f32),Some(30.0f32)
    );

    let del_btn = icon_btn(
        icons.delete.clone(),
        if disabled || total <= 1 {
            None
        } else {
            Some(Message::RemoveSource(i))
        },
        Some(30.0f32),Some(30.0f32)
    );

    row![input, browse_btn, del_btn]
        .spacing(3)
        .align_items(iced::Alignment::Center)
        .into()
}

fn target_input(
    value: &str,
    disabled: bool,
    icons: svg::Icons,
) -> Element<'static, Message> {
    let input = text_input("输入或选择目标目录...", value)
        .on_input(Message::TargetPath)
        .padding(6)
        .size(FONT_SIZE)
        .width(Length::Fill);

    let browse_btn = icon_btn(
        icons.folder.clone(),
        if disabled {
            None
        } else {
            Some(Message::BrowseTargetDir)
        },
        Some(30.0f32),Some(30.0f32)
    );

    row![input, browse_btn]
        .spacing(3)
        .align_items(iced::Alignment::Center)
        .into()
}
