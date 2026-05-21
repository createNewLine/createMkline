use std::fs;
use std::os::windows::fs::symlink_file;
use std::path::{Path, PathBuf};
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

/// Create a symlink (junction for dirs, symlink for files) at `link` pointing to `target`.
pub fn create_link(link: &Path, target: &Path) -> Result<(), String> {
    if target.is_dir() {
        junction::create(target, link)
            .map_err(|e| format!("创建目录联接失败 '{}' -> '{}': {}", link.display(), target.display(), e))
    } else {
        symlink_file(target, link)
            .map_err(|e| format!("创建文件符号链接失败 '{}' -> '{}': {}\n提示: 可能需要启用开发者模式或以管理员身份运行", link.display(), target.display(), e))
    }
}

/// Validate backup preconditions. Returns the backup path to use.
pub fn backup_validate(src: &Path) -> Result<PathBuf, String> {
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

    Ok(backup_path)
}

/// Copy source to backup path.
pub fn backup_copy(src: &Path, backup_path: &Path) -> Result<(), String> {
    copy_entry(src, backup_path)
}

/// Execute the confirm operation for all sources.
/// Phase 1: validate all sources (sequential, fast).
/// Phase 2: copy all sources → target (parallel, the heavy part).
/// Phase 3: delete sources + create links (parallel, fast).
/// On any error, all copied destinations are cleaned up.
/// If `overwrite` is true, existing paths in the target are removed before copying.
pub fn execute_confirm(sources: &[String], target: &str, overwrite: bool) -> Result<String, String> {
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
        if !overwrite && dst_path.exists() {
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
                if overwrite && dst.exists() {
                    let _ = remove_entry(dst);
                }
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
    let results: Arc<Mutex<Vec<(bool, PathBuf)>>> = Arc::new(Mutex::new(Vec::new()));
    let errors: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    std::thread::scope(|s| {
        for (src, dst) in &copied {
            let results = Arc::clone(&results);
            let errors = Arc::clone(&errors);
            s.spawn(move || {
                if let Err(e) = trash::delete(src) {
                    errors
                        .lock()
                        .unwrap()
                        .push(format!("删除源到回收站失败 '{}': {}", src.display(), e));
                    results.lock().unwrap().push((false, dst.clone()));
                    return;
                }
                if let Err(e) = create_link(src, dst) {
                    if let Err(e2) = copy_entry(dst, src) {
                        errors.lock().unwrap().push(format!(
                            "创建链接失败且恢复失败 '{}' -> '{}': {} / {}",
                            src.display(),
                            dst.display(),
                            e,
                            e2
                        ));
                    } else {
                        errors.lock().unwrap().push(format!(
                            "创建链接失败 '{}' -> '{}': {} (源已恢复)",
                            src.display(),
                            dst.display(),
                            e
                        ));
                    }
                    results.lock().unwrap().push((false, dst.clone()));
                    return;
                }
                results.lock().unwrap().push((true, dst.clone()));
            });
        }
    });

    let results = Arc::try_unwrap(results).unwrap().into_inner().unwrap();
    let errors = Arc::try_unwrap(errors).unwrap().into_inner().unwrap();
    if !errors.is_empty() {
        for (success, dst) in &results {
            if !success {
                let _ = remove_entry(dst);
            }
        }
        return Err(errors.join("\n"));
    }

    Ok(format!("成功处理 {} 个源", total))
}
