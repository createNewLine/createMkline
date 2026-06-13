use iced::widget::image;
use resvg::usvg::TreeParsing;

const FOLDER_SVG: &str = include_str!("../svg/文件夹.svg");
const DELETE_SVG: &str = include_str!("../svg/删除.svg");

const ICON_SIZE: u32 = 36;

#[derive(Debug, Clone)]
pub struct Icons {
    pub folder: image::Handle,
    pub delete: image::Handle,
}

impl Icons {
    pub fn load() -> Self {
        Self {
            folder: render(FOLDER_SVG),
            delete: render(DELETE_SVG),
        }
    }
}

fn render(svg_text: &str) -> image::Handle {
    let tree = resvg::usvg::Tree::from_str(svg_text, &resvg::usvg::Options::default())
        .expect("Failed to parse SVG icon");
    let rtree = resvg::Tree::from_usvg(&tree);
    let mut pixmap =
        tiny_skia::Pixmap::new(ICON_SIZE, ICON_SIZE).expect("Failed to create pixmap");
    let scale = ICON_SIZE as f32 / tree.size.width();
    let mut pm = pixmap.as_mut();
    rtree.render(
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pm,
    );
    drop(pm);
    image::Handle::from_pixels(pixmap.width(), pixmap.height(), pixmap.take())
}
