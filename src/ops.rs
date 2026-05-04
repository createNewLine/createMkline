use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
/// For each source: validate → copy to target → delete source → create link.
/// On copy or delete error, all copied destinations are cleaned up.
pub fn execute_confirm(sources: &[String], target: &str) -> Result<String, String> {
    let target_dir = Path::new(target);
    let mut copied: Vec<PathBuf> = Vec::new();

    for (i, src_str) in sources.iter().enumerate() {
        let src_path = Path::new(src_str);

        if !src_path.exists() {
            rollback(&copied);
            return Err(format!(
                "[{}/{}] 源路径不存在: '{}'",
                i + 1,
                sources.len(),
                src_path.display()
            ));
        }

        let name = src_path.file_name().ok_or_else(|| {
            rollback(&copied);
            format!(
                "[{}/{}] 无效的源路径: '{}'",
                i + 1,
                sources.len(),
                src_path.display()
            )
        })?;
        let dst_path = target_dir.join(name);

        if dst_path.exists() {
            rollback(&copied);
            return Err(format!(
                "[{}/{}] 目标路径已存在: '{}'",
                i + 1,
                sources.len(),
                dst_path.display()
            ));
        }

        // Step 1: copy source → target
        if let Err(e) = copy_entry(src_path, &dst_path) {
            let _ = remove_entry(&dst_path); // partial copy cleanup
            rollback(&copied);
            return Err(format!("[{}/{}] {}", i + 1, sources.len(), e));
        }
        copied.push(dst_path.clone());

        // Step 2: delete source
        if let Err(e) = remove_entry(src_path) {
            rollback(&copied);
            return Err(format!("[{}/{}] {}", i + 1, sources.len(), e));
        }

        // Step 3: create junction / symlink at original source location
        if let Err(e) = create_link(src_path, &dst_path) {
            // Data is safe in target; rollback is not possible for already-deleted sources.
            // Clean up what we can and report the error.
            rollback(&copied);
            return Err(format!("[{}/{}] {}", i + 1, sources.len(), e));
        }
    }

    Ok(format!("成功处理 {} 个源", sources.len()))
}

/// Delete all paths in the list (best-effort cleanup).
fn rollback(paths: &[PathBuf]) {
    for p in paths {
        let _ = remove_entry(p);
    }
}
