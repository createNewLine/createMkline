use std::path::Path;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let src = Path::new("src/svg/工具标识1.ico");
        let dst = Path::new(&std::env::var("OUT_DIR").unwrap()).join("icon.ico");
        std::fs::copy(src, &dst).expect("Failed to copy ICO file to OUT_DIR");

        let mut res = winresource::WindowsResource::new();
        res.set_icon(dst.to_str().expect("OUT_DIR path is not valid UTF-8"));
        res.compile().expect("Failed to embed icon");
    }
}
