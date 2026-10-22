use crate::app_state::AppDataDir;
use crate::error::{AppError, AppResult};
use futures::StreamExt;
use image::GenericImageView;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::State;
use uuid::Uuid;

const MAX_ARCHIVE_SIZE: u64 = 5 * 1024 * 1024;
const MAX_UNCOMPRESSED_SIZE: u64 = 2 * 1024 * 1024;
const MAX_ENTRY_COUNT: usize = 16;
const MAX_METADATA_SIZE: usize = 64 * 1024;
const MAX_CSS_SIZE: usize = 100 * 1024;
const MAX_PREVIEW_SIZE: usize = 500 * 1024;
const MAX_PREVIEW_WIDTH: u32 = 800;
const MAX_PREVIEW_HEIGHT: u32 = 600;

const REQUIRED_FILES: &[&str] = &[
    "metadata.json",
    "theme.css",
    "preview-light.png",
    "preview-dark.png",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeLocalizedText {
    pub zh: Option<String>,
    pub en: Option<String>,
    pub tw: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemePackageMetadata {
    #[serde(default = "default_api_version")]
    pub api_version: u32,
    pub id: String,
    pub name: ThemeLocalizedText,
    pub description: Option<ThemeLocalizedText>,
    pub category: String,
    #[serde(default = "default_theme_version")]
    pub version: String,
    #[serde(default = "default_min_app_version")]
    pub min_app_version: String,
    #[serde(default)]
    pub supports_custom_background: bool,
    #[serde(default)]
    pub supports_surface_opacity: bool,
    #[serde(default = "default_window_material")]
    pub window_material: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledTheme {
    pub id: String,
    pub package_id: String,
    pub name: ThemeLocalizedText,
    pub description: Option<ThemeLocalizedText>,
    pub category: String,
    pub version: String,
    pub min_app_version: String,
    pub supports_custom_background: bool,
    pub supports_surface_opacity: bool,
    pub window_material: String,
}

fn default_api_version() -> u32 {
    1
}

fn default_theme_version() -> String {
    "1.0.0".to_string()
}

fn default_min_app_version() -> String {
    "1.0.0".to_string()
}

fn default_window_material() -> String {
    "none".to_string()
}

fn themes_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("themes")
}

fn installed_id(package_id: &str) -> String {
    format!("store-{package_id}")
}

fn validate_theme_id(id: &str) -> AppResult<()> {
    let id_regex =
        Regex::new(r"^[a-z0-9-]{3,50}$").map_err(|error| AppError::Internal(error.to_string()))?;
    if !id_regex.is_match(id) || id.starts_with("store-") {
        return Err(AppError::Validation(
            "主题标识需为 3–50 个字符。\n只用小写字母、数字或连字符。\n不能以 store- 开头。".to_string(),
        ));
    }
    Ok(())
}

fn validate_localized_text(
    value: &ThemeLocalizedText,
    field: &str,
    required: bool,
) -> AppResult<()> {
    let values = [
        value.zh.as_deref(),
        value.en.as_deref(),
        value.tw.as_deref(),
    ];
    if required
        && [value.zh.as_deref(), value.en.as_deref()]
            .iter()
            .all(|item| item.unwrap_or("").trim().is_empty())
    {
        return Err(AppError::Validation(format!(
            "{field} 至少需要中文或英文内容"
        )));
    }
    if values
        .into_iter()
        .flatten()
        .any(|item| item.trim().len() > 160)
    {
        return Err(AppError::Validation(format!("{field} 内容过长")));
    }
    Ok(())
}

fn validate_metadata(metadata: &ThemePackageMetadata) -> AppResult<()> {
    if metadata.api_version != 1 {
        return Err(AppError::Validation(format!(
            "不支持主题包协议版本 {}",
            metadata.api_version
        )));
    }
    validate_theme_id(&metadata.id)?;
    validate_localized_text(&metadata.name, "主题名称", true)?;
    if let Some(description) = &metadata.description {
        validate_localized_text(description, "主题描述", false)?;
    }
    if !matches!(
        metadata.category.as_str(),
        "nature" | "minimal" | "retro" | "dark" | "cute" | "other"
    ) {
        return Err(AppError::Validation("主题分类无效".to_string()));
    }
    if metadata.version.trim().is_empty() || metadata.version.len() > 32 {
        return Err(AppError::Validation("主题版本无效".to_string()));
    }
    if metadata.min_app_version.trim().is_empty() || metadata.min_app_version.len() > 32 {
        return Err(AppError::Validation("最低应用版本无效".to_string()));
    }
    if !matches!(
        metadata.window_material.as_str(),
        "none" | "mica" | "acrylic"
    ) {
        return Err(AppError::Validation("窗口材质声明无效".to_string()));
    }
    Ok(())
}

fn validate_theme_css(css: &str, runtime_id: &str) -> AppResult<()> {
    if css.len() > MAX_CSS_SIZE {
        return Err(AppError::Validation("theme.css 超过 100 KiB".to_string()));
    }

    let lower = css.to_ascii_lowercase();
    for forbidden in [
        "@import",
        "javascript:",
        "http:",
        "https:",
        "file:",
        "asset:",
        "//asset.localhost",
    ] {
        if lower.contains(forbidden) {
            return Err(AppError::Validation(format!(
                "theme.css 包含不允许的内容：{forbidden}"
            )));
        }
    }

    let required_selector = format!(".theme-{runtime_id}");
    if !css.contains(&required_selector) {
        return Err(AppError::Validation(format!(
            "theme.css 必须包含选择器 {required_selector}"
        )));
    }

    Ok(())
}

fn validate_preview(bytes: &[u8], filename: &str) -> AppResult<()> {
    if bytes.len() > MAX_PREVIEW_SIZE {
        return Err(AppError::Validation(format!("{filename} 超过 500 KiB")));
    }
    let format = image::guess_format(bytes)
        .map_err(|_| AppError::Validation(format!("{filename} 不是有效图片")))?;
    if !matches!(
        format,
        image::ImageFormat::Png | image::ImageFormat::Jpeg | image::ImageFormat::WebP
    ) {
        return Err(AppError::Validation(format!(
            "{filename} 仅支持 PNG、JPEG 或 WebP"
        )));
    }
    let image = image::load_from_memory_with_format(bytes, format)
        .map_err(|_| AppError::Validation(format!("{filename} 不是有效图片")))?;
    let (width, height) = image.dimensions();
    if width > MAX_PREVIEW_WIDTH || height > MAX_PREVIEW_HEIGHT {
        return Err(AppError::Validation(format!(
            "{filename} 尺寸不能超过 800×600"
        )));
    }
    Ok(())
}

fn read_package(path: &Path) -> AppResult<(ThemePackageMetadata, HashMap<String, Vec<u8>>)> {
    let archive_size = fs::metadata(path).map_err(AppError::from)?.len();
    if archive_size > MAX_ARCHIVE_SIZE {
        return Err(AppError::Validation("主题包超过 5 MiB".to_string()));
    }

    let file = File::open(path).map_err(AppError::from)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| AppError::Validation(format!("主题包不是有效 ZIP：{error}")))?;
    if archive.len() > MAX_ENTRY_COUNT {
        return Err(AppError::Validation("主题包文件数量过多".to_string()));
    }

    let mut total_size = 0u64;
    let mut files = HashMap::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| AppError::Validation(format!("无法读取主题包：{error}")))?;
        if entry.is_dir() {
            continue;
        }
        let enclosed = entry
            .enclosed_name()
            .ok_or_else(|| AppError::Validation("主题包包含不安全路径".to_string()))?;
        if enclosed.components().count() != 1 {
            return Err(AppError::Validation("主题包 v1 不允许嵌套目录".to_string()));
        }
        let filename = enclosed
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| AppError::Validation("主题包文件名无效".to_string()))?
            .to_string();
        if !REQUIRED_FILES.contains(&filename.as_str()) {
            return Err(AppError::Validation(format!(
                "主题包包含未知文件：{filename}"
            )));
        }
        if files.contains_key(&filename) {
            return Err(AppError::Validation(format!(
                "主题包包含重复文件：{filename}"
            )));
        }

        total_size = total_size.saturating_add(entry.size());
        if total_size > MAX_UNCOMPRESSED_SIZE {
            return Err(AppError::Validation("主题包解压后超过 2 MiB".to_string()));
        }

        let per_file_limit = match filename.as_str() {
            "metadata.json" => MAX_METADATA_SIZE,
            "theme.css" => MAX_CSS_SIZE,
            _ => MAX_PREVIEW_SIZE,
        };
        if entry.size() > per_file_limit as u64 {
            return Err(AppError::Validation(format!("{filename} 文件过大")));
        }

        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes).map_err(AppError::from)?;
        files.insert(filename, bytes);
    }

    for required in REQUIRED_FILES {
        if !files.contains_key(*required) {
            return Err(AppError::Validation(format!("主题文件不完整。\n缺少：{required}")));
        }
    }

    let metadata: ThemePackageMetadata = serde_json::from_slice(&files["metadata.json"])
        .map_err(|error| AppError::Validation(format!("无法读取主题信息。\n文件：metadata.json\n详情：{error}")))?;
    validate_metadata(&metadata)?;
    let runtime_id = installed_id(&metadata.id);
    let css = std::str::from_utf8(&files["theme.css"])
        .map_err(|_| AppError::Validation("请用 UTF-8 保存样式文件。\n文件：theme.css".to_string()))?;
    validate_theme_css(css, &runtime_id)?;
    validate_preview(&files["preview-light.png"], "preview-light.png")?;
    validate_preview(&files["preview-dark.png"], "preview-dark.png")?;

    Ok((metadata, files))
}

fn to_installed_theme(metadata: ThemePackageMetadata) -> InstalledTheme {
    InstalledTheme {
        id: installed_id(&metadata.id),
        package_id: metadata.id,
        name: metadata.name,
        description: metadata.description,
        category: metadata.category,
        version: metadata.version,
        min_app_version: metadata.min_app_version,
        supports_custom_background: metadata.supports_custom_background,
        supports_surface_opacity: metadata.supports_surface_opacity,
        window_material: metadata.window_material,
    }
}

fn read_installed_metadata(path: &Path) -> AppResult<ThemePackageMetadata> {
    let raw = fs::read(path.join("metadata.json")).map_err(AppError::from)?;
    let metadata = serde_json::from_slice::<ThemePackageMetadata>(&raw)
        .map_err(|error| AppError::Validation(format!("无法读取已安装主题的信息。\n详情：{error}")))?;
    validate_metadata(&metadata)?;
    Ok(metadata)
}

fn install_validated_theme(
    metadata: ThemePackageMetadata,
    files: HashMap<String, Vec<u8>>,
    data_dir: &Path,
) -> AppResult<InstalledTheme> {
    let runtime_id = installed_id(&metadata.id);
    let root = themes_dir(data_dir);
    fs::create_dir_all(&root).map_err(AppError::from)?;

    let temporary = root.join(format!(".{runtime_id}-{}", Uuid::new_v4()));
    fs::create_dir(&temporary).map_err(AppError::from)?;
    let install_result = (|| -> AppResult<()> {
        let canonical_metadata = serde_json::to_vec_pretty(&metadata)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        fs::write(temporary.join("metadata.json"), canonical_metadata).map_err(AppError::from)?;
        for filename in ["theme.css", "preview-light.png", "preview-dark.png"] {
            fs::write(temporary.join(filename), &files[filename]).map_err(AppError::from)?;
        }
        Ok(())
    })();
    if let Err(error) = install_result {
        let _ = fs::remove_dir_all(&temporary);
        return Err(error);
    }

    let destination = root.join(&runtime_id);
    let backup = root.join(format!(".{runtime_id}-backup-{}", Uuid::new_v4()));
    if destination.exists() {
        fs::rename(&destination, &backup).map_err(AppError::from)?;
    }
    if let Err(error) = fs::rename(&temporary, &destination) {
        if backup.exists() {
            let _ = fs::rename(&backup, &destination);
        }
        let _ = fs::remove_dir_all(&temporary);
        return Err(AppError::from(error));
    }
    if backup.exists() {
        let _ = fs::remove_dir_all(backup);
    }

    Ok(to_installed_theme(metadata))
}

async fn download_limited(url: &str, maximum_size: usize) -> AppResult<Vec<u8>> {
    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|error| AppError::Network(error.to_string()))?
        .error_for_status()
        .map_err(|error| AppError::Network(error.to_string()))?;
    if response.content_length().unwrap_or(0) > maximum_size as u64 {
        return Err(AppError::Validation("主题文件过大，无法下载。".to_string()));
    }

    let mut result = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| AppError::Network(error.to_string()))?;
        if result.len().saturating_add(chunk.len()) > maximum_size {
            return Err(AppError::Validation("主题文件过大，无法下载。".to_string()));
        }
        result.extend_from_slice(&chunk);
    }
    Ok(result)
}

#[tauri::command]
pub fn install_theme_package(
    package_path: String,
    data_dir: State<'_, AppDataDir>,
) -> AppResult<InstalledTheme> {
    let package_path = PathBuf::from(package_path);
    if !package_path.is_file() {
        return Err(AppError::Validation("找不到主题文件，请重新选择。".to_string()));
    }

    let (metadata, files) = read_package(&package_path)?;
    let data_root = {
        let data_dir = data_dir
            .0
            .lock()
            .map_err(|_| AppError::Internal("无法打开数据位置，请检查访问权限。".to_string()))?;
        data_dir.clone()
    };
    install_validated_theme(metadata, files, &data_root)
}

#[tauri::command]
pub async fn install_theme_from_store(
    theme_id: String,
    data_dir: State<'_, AppDataDir>,
) -> AppResult<InstalledTheme> {
    let package_id = theme_id
        .strip_prefix("store-")
        .ok_or_else(|| AppError::Validation("主题标识不正确，请重新选择主题。".to_string()))?;
    validate_theme_id(package_id)?;

    let package_url = format!(
        "{}/api/v1/themes/{}/package",
        crate::build_config::theme_store_base_url(),
        theme_id
    );
    let package = download_limited(&package_url, MAX_ARCHIVE_SIZE as usize).await?;
    let data_root = {
        let data_dir = data_dir
            .0
            .lock()
            .map_err(|_| AppError::Internal("无法打开数据位置，请检查访问权限。".to_string()))?;
        data_dir.clone()
    };
    let download_directory = themes_dir(&data_root);
    fs::create_dir_all(&download_directory).map_err(AppError::from)?;
    let temporary_package = download_directory.join(format!(".download-{}.zip", Uuid::new_v4()));
    fs::write(&temporary_package, package).map_err(AppError::from)?;

    let result = (|| -> AppResult<InstalledTheme> {
        let (metadata, files) = read_package(&temporary_package)?;
        if installed_id(&metadata.id) != theme_id {
            return Err(AppError::Validation(
                "下载的文件与所选主题不同。\n请重新下载。".to_string(),
            ));
        }
        install_validated_theme(metadata, files, &data_root)
    })();
    let _ = fs::remove_file(temporary_package);
    result
}

#[tauri::command]
pub fn list_installed_themes(data_dir: State<'_, AppDataDir>) -> AppResult<Vec<InstalledTheme>> {
    let root = {
        let data_dir = data_dir
            .0
            .lock()
            .map_err(|_| AppError::Internal("无法打开数据位置，请检查访问权限。".to_string()))?;
        themes_dir(&data_dir)
    };
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut themes = Vec::new();
    for entry in fs::read_dir(root).map_err(AppError::from)? {
        let entry = entry.map_err(AppError::from)?;
        if !entry.file_type().map_err(AppError::from)?.is_dir() {
            continue;
        }
        let directory_name = entry.file_name().to_string_lossy().to_string();
        if !directory_name.starts_with("store-") {
            continue;
        }
        match read_installed_metadata(&entry.path()) {
            Ok(metadata) if installed_id(&metadata.id) == directory_name => {
                themes.push(to_installed_theme(metadata));
            }
            Ok(_) => {
                eprintln!("Ignoring installed theme with mismatched directory: {directory_name}")
            }
            Err(error) => eprintln!("Ignoring invalid installed theme {directory_name}: {error}"),
        }
    }
    themes.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(themes)
}

#[tauri::command]
pub fn get_installed_theme_css(
    theme_id: String,
    data_dir: State<'_, AppDataDir>,
) -> AppResult<String> {
    let package_id = theme_id
        .strip_prefix("store-")
        .ok_or_else(|| AppError::Validation("请先安装这个商店主题。".to_string()))?;
    validate_theme_id(package_id)?;
    let theme_directory = {
        let data_dir = data_dir
            .0
            .lock()
            .map_err(|_| AppError::Internal("无法打开数据位置，请检查访问权限。".to_string()))?;
        themes_dir(&data_dir).join(&theme_id)
    };
    let metadata = read_installed_metadata(&theme_directory)?;
    if metadata.id != package_id {
        return Err(AppError::Validation("主题文件位置与主题信息不符。\n请重新导入。".to_string()));
    }
    let css = fs::read_to_string(theme_directory.join("theme.css")).map_err(AppError::from)?;
    validate_theme_css(&css, &theme_id)?;
    Ok(css)
}

#[tauri::command]
pub fn uninstall_theme_package(theme_id: String, data_dir: State<'_, AppDataDir>) -> AppResult<()> {
    let package_id = theme_id
        .strip_prefix("store-")
        .ok_or_else(|| AppError::Validation("这里只能卸载从商店安装的主题。".to_string()))?;
    validate_theme_id(package_id)?;
    let theme_directory = {
        let data_dir = data_dir
            .0
            .lock()
            .map_err(|_| AppError::Internal("无法打开数据位置，请检查访问权限。".to_string()))?;
        themes_dir(&data_dir).join(&theme_id)
    };
    if theme_directory.exists() {
        fs::remove_dir_all(theme_directory).map_err(AppError::from)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::ImageEncoder;
    use std::io::Write;

    fn metadata() -> ThemePackageMetadata {
        ThemePackageMetadata {
            api_version: 1,
            id: "retro".to_string(),
            name: ThemeLocalizedText {
                zh: Some("3D复古".to_string()),
                en: Some("Retro 3D".to_string()),
                tw: None,
            },
            description: None,
            category: "retro".to_string(),
            version: "1.0.0".to_string(),
            min_app_version: "1.0.0".to_string(),
            supports_custom_background: false,
            supports_surface_opacity: false,
            window_material: "none".to_string(),
        }
    }

    #[test]
    fn accepts_valid_metadata_and_scoped_css() {
        let metadata = metadata();
        assert!(validate_metadata(&metadata).is_ok());
        assert!(validate_theme_css(
            ".theme-store-retro { --accent-color: #487bdb; }",
            "store-retro"
        )
        .is_ok());
    }

    #[test]
    fn rejects_reserved_id_and_remote_css() {
        let mut invalid = metadata();
        invalid.id = "store-retro".to_string();
        assert!(validate_metadata(&invalid).is_err());
        assert!(validate_theme_css(
            ".theme-store-retro { background: url(https://example.com/a.png); }",
            "store-retro"
        )
        .is_err());
    }

    #[test]
    fn reads_a_complete_theme_archive() {
        let archive_path =
            std::env::temp_dir().join(format!("tiez-theme-package-test-{}.zip", Uuid::new_v4()));
        let file = File::create(&archive_path).unwrap();
        let mut archive = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        let metadata = serde_json::to_vec(&metadata()).unwrap();
        let css = b".theme-store-retro { --accent-color: #487bdb; }";

        let mut preview = Vec::new();
        image::codecs::png::PngEncoder::new(&mut preview)
            .write_image(&[72, 123, 219, 255], 1, 1, image::ExtendedColorType::Rgba8)
            .unwrap();

        for (name, bytes) in [
            ("metadata.json", metadata.as_slice()),
            ("theme.css", css.as_slice()),
            ("preview-light.png", preview.as_slice()),
            ("preview-dark.png", preview.as_slice()),
        ] {
            archive.start_file(name, options).unwrap();
            archive.write_all(bytes).unwrap();
        }
        archive.finish().unwrap();

        let result = read_package(&archive_path).unwrap();
        assert_eq!(result.0.id, "retro");
        assert_eq!(result.1.len(), 4);
        fs::remove_file(archive_path).unwrap();
    }
}
