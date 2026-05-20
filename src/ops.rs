use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

/// Recursively copy a file or directory.
pub fn copy_entry(src: &Path, dst: &Path) -> Result<(), String> {
    if src.is_file() {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建父目录失败 '{}': {}", parent.display(), e))?;
        }
        fs::copy(src, dst)
            .map_err(|e| format!("复制文件失败 '{}' -> '{}': {}", src.display(), dst.display(), e))?;
    } else if src.is_dir() {
        fs::create_dir_all(dst)
            .map_err(|e| format!("创建目标目录失败 '{}': {}", dst.display(), e))?;
        let entries = fs::read_dir(src)
            .map_err(|e| format!("读取源目录失败 '{}': {}", src.display(), e))?;
        for entry in entries {
            let entry =
                entry.map_err(|e| format!("读取目录项失败: {}", e))?;
            let src_child = entry.path();
            let dst_child = dst.join(entry.file_name());
            copy_entry(&src_child, &dst_child)?;
        }
    }
    Ok(())
}

/// Recursively remove a file or directory.
pub fn remove_entry(path: &Path) -> Result<(), String> {
    if path.is_dir() {
        fs::remove_dir_all(path)
            .map_err(|e| format!("删除目录失败 '{}': {}", path.display(), e))?;
    } else {
        fs::remove_file(path)
            .map_err(|e| format!("删除文件失败 '{}': {}", path.display(), e))?;
    }
    Ok(())
}

/// Create a directory junction on Windows (mklink /J — no admin required).
fn create_junction(link: &Path, target: &Path) -> Result<(), String> {
    let output = Command::new("cmd")
        .args([
            "/c",
            "mklink",
            "/J",
            &link.to_string_lossy(),
            &target.to_string_lossy(),
        ])
        .output()
        .map_err(|e| format!("执行 mklink 命令失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("创建目录联接失败: {}", stderr));
    }
    Ok(())
}

/// Create a file symbolic link on Windows (mklink — may require admin / developer mode).
fn create_file_symlink(link: &Path, target: &Path) -> Result<(), String> {
    let output = Command::new("cmd")
        .args([
            "/c",
            "mklink",
            &link.to_string_lossy(),
            &target.to_string_lossy(),
        ])
        .output()
        .map_err(|e| format!("执行 mklink 命令失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "创建文件符号链接失败: {}\n提示: 可能需要启用开发者模式或以管理员身份运行",
            stderr
        ));
    }
    Ok(())
}

/// Create a symlink (junction for dirs, symlink for files) at `link` pointing to `target`.
pub fn create_link(link: &Path, target: &Path) -> Result<(), String> {
    // Determine if the target is a directory by checking the actual filesystem.
    if target.is_dir() {
        create_junction(link, target)
    } else {
        create_file_symlink(link, target)
    }
}

/// Backup a single source: copy to `<parent>/<name>(1)`.
pub fn backup_one(src: &Path) -> Result<String, String> {
    let parent = src
        .parent()
        .ok_or_else(|| format!("无法获取父目录: '{}'", src.display()))?;
    let name = src
        .file_name()
        .ok_or_else(|| format!("无法获取文件名: '{}'", src.display()))?
        .to_string_lossy();
    let backup_name = format!("{}(1)", name);
    let backup_path = parent.join(&backup_name);

    if backup_path.exists() {
        return Err(format!("备份路径已存在: '{}'", backup_path.display()));
    }

    copy_entry(src, &backup_path)?;
    Ok(backup_path.display().to_string())
}

/// Execute the confirm operation for all sources.
/// Phase 1: validate all sources (sequential, fast).
/// Phase 2: copy all sources → target (parallel, the heavy part).
/// Phase 3: delete sources + create links (parallel, fast).
/// On any error, all copied destinations are cleaned up.
pub fn execute_confirm(sources: &[String], target: &str) -> Result<String, String> {
    let target_dir = Path::new(target);
    let total = sources.len();

    // ── Phase 1: Validate all sources ──────────────────────────────
    let mut plans: Vec<(PathBuf, PathBuf)> = Vec::with_capacity(total);
    for (i, src_str) in sources.iter().enumerate() {
        let src_path = Path::new(src_str);
        if !src_path.exists() {
            return Err(format!(
                "[{}/{}] 源路径不存在: '{}'",
                i + 1,
                total,
                src_path.display()
            ));
        }
        let name = src_path.file_name().ok_or_else(|| {
            format!(
                "[{}/{}] 无效的源路径: '{}'",
                i + 1,
                total,
                src_path.display()
            )
        })?;
        let dst_path = target_dir.join(name);
        if dst_path.exists() {
            return Err(format!(
                "[{}/{}] 目标路径已存在: '{}'",
                i + 1,
                total,
                dst_path.display()
            ));
        }
        if plans.iter().any(|(_, d)| d == &dst_path) {
            return Err(format!(
                "[{}/{}] 目标路径冲突: '{}'",
                i + 1,
                total,
                dst_path.display()
            ));
        }
        plans.push((src_path.to_path_buf(), dst_path));
    }

    // ── Phase 2: Copy all in parallel ──────────────────────────────
    let copied: Arc<Mutex<Vec<(PathBuf, PathBuf)>>> = Arc::new(Mutex::new(Vec::new()));
    let copy_errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    std::thread::scope(|s| {
        for (idx, (src, dst)) in plans.iter().enumerate() {
            let copied = Arc::clone(&copied);
            let copy_errors = Arc::clone(&copy_errors);
            s.spawn(move || {
                match copy_entry(src, dst) {
                    Ok(()) => {
                        copied.lock().unwrap().push((src.clone(), dst.clone()));
                    }
                    Err(e) => {
                        let _ = remove_entry(dst);
                        copy_errors
                            .lock()
                            .unwrap()
                            .push(format!("[{}/{}] {}", idx + 1, total, e));
                    }
                }
            });
        }
    });

    let copy_errors = Arc::try_unwrap(copy_errors).unwrap().into_inner().unwrap();
    if !copy_errors.is_empty() {
        let copied = Arc::try_unwrap(copied).unwrap().into_inner().unwrap();
        for (_, dst) in &copied {
            let _ = remove_entry(dst);
        }
        return Err(copy_errors.join("\n"));
    }

    let copied = Arc::try_unwrap(copied).unwrap().into_inner().unwrap();

    // ── Phase 3: Delete sources + create links in parallel ────────
    let link_errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    std::thread::scope(|s| {
        for (src, dst) in &copied {
            let link_errors = Arc::clone(&link_errors);
            s.spawn(move || {
                if let Err(e) = remove_entry(src) {
                    link_errors
                        .lock()
                        .unwrap()
                        .push(format!("删除源失败 '{}': {}", src.display(), e));
                    return;
                }
                if let Err(e) = create_link(src, dst) {
                    link_errors
                        .lock()
                        .unwrap()
                        .push(format!(
                            "创建链接失败 '{}' -> '{}': {}",
                            src.display(),
                            dst.display(),
                            e
                        ));
                }
            });
        }
    });

    let link_errors = Arc::try_unwrap(link_errors).unwrap().into_inner().unwrap();
    if !link_errors.is_empty() {
        for (_, dst) in &copied {
            let _ = remove_entry(dst);
        }
        return Err(link_errors.join("\n"));
    }

    Ok(format!("成功处理 {} 个源", total))
}
