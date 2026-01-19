use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

// Включаем встроенный бинарник (генерируется build.rs)
include!(concat!(env!("OUT_DIR"), "/embedded_slurp.rs"));

/// Получает путь к исполняемому файлу slurp
/// Приоритет: системный slurp > встроенный slurp
pub fn get_slurp_path() -> Result<PathBuf> {
    // 1. Проверяем системный slurp
    if Command::new("slurp").arg("--version").output().is_ok() {
        return Ok(PathBuf::from("slurp"));
    }
    
    // 2. Используем встроенный slurp
    if EMBEDDED_SLURP.is_empty() {
        anyhow::bail!(
            "Slurp not found in system PATH and embedded slurp is not available.\n\
             Please install slurp: pacman -S slurp (Arch) or equivalent"
        );
    }
    
    let cache_dir = dirs::cache_dir()
        .context("Failed to get cache directory")?
        .join("hyprshot-rs");
    
    std::fs::create_dir_all(&cache_dir)
        .context("Failed to create cache directory")?;
    
    let slurp_path = cache_dir.join("slurp");
    
    // Извлекаем бинарник, если его нет или версия устарела
    if !slurp_path.exists() || needs_update(&slurp_path)? {
        extract_slurp(&slurp_path)?;
    }
    
    Ok(slurp_path)
}

/// Извлекает встроенный slurp в файловую систему
fn extract_slurp(target_path: &PathBuf) -> Result<()> {
    std::fs::write(target_path, EMBEDDED_SLURP)
        .context("Failed to write embedded slurp binary")?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            target_path,
            std::fs::Permissions::from_mode(0o755)
        ).context("Failed to set executable permissions")?;
    }
    
    Ok(())
}

/// Проверяет, нужно ли обновить встроенный slurp
fn needs_update(slurp_path: &PathBuf) -> Result<bool> {
    // Сравниваем размер файла (простая проверка)
    let metadata = std::fs::metadata(slurp_path)?;
    Ok(metadata.len() != EMBEDDED_SLURP.len() as u64)
}
