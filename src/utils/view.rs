use iced::{
    alignment,
    theme,
    widget::{button, column, image, row, scrollable, text, text_input, Space},
    Color, Element, Length,
};
use crate::svg;
use super::message::Message;
use super::state::{FONT_SIZE, MkLineExe, Status};

pub(crate) fn build_view(app: &MkLineExe) -> Element<'_, Message> {
    let running = app.status == Status::Running;
    let icons = app.icons.get_or_init(svg::Icons::load).clone();

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

    let source_rows: Vec<Element<Message>> = app
        .sources
        .iter()
        .enumerate()
        .map(|(i, src)| {
            source_row(
                i,
                src,
                app.sources.len(),
                running,
                icons.clone(),
            )
        })
        .collect();

    let filled = app.sources.iter().filter(|s| !s.trim().is_empty()).count();
    let can_add = !running && app.sources.len() < 10;
    let add_btn = button(
        text(format!("+ 添加源({}/10)", filled))
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
    let target_row = target_input(&app.target, running, icons.clone());

    // ── Status ────────────────────────────────────────────────
    let (status_color, status_icon) = match app.status {
        Status::Idle => (Color::from_rgb(0.45, 0.45, 0.50), "●"),
        Status::Running => (accent, "◉"),
        Status::Success => (Color::from_rgb(0.25, 0.80, 0.45), "●"),
        Status::Error => (Color::from_rgb(0.95, 0.30, 0.30), "●"),
    };
    let status_text = text(format!("{}  {}", status_icon, app.status_message))
        .style(theme::Text::Color(status_color))
        .size(FONT_SIZE);

    // ── Separator ─────────────────────────────────────────────
    let separator = Space::with_height(1);

    // ── Bottom buttons ────────────────────────────────────────
    let can_confirm = !running
        && !app.target.trim().is_empty()
        && app.sources.iter().any(|s| !s.trim().is_empty());

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

// ── View helpers ───────────────────────────────────────────────────

fn icon_btn<'a>(
    handle: image::Handle,
    on_press: Option<Message>,
    width: Option<f32>,
    height: Option<f32>,
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
        Some(30.0f32),
        Some(30.0f32),
    );

    let del_btn = icon_btn(
        icons.delete.clone(),
        if disabled || total <= 1 {
            None
        } else {
            Some(Message::RemoveSource(i))
        },
        Some(30.0f32),
        Some(30.0f32),
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
        Some(30.0f32),
        Some(30.0f32),
    );

    row![input, browse_btn]
        .spacing(3)
        .align_items(iced::Alignment::Center)
        .into()
}
