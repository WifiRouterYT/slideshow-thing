use std::path::{Path, PathBuf};
use std::fs;
use walkdir::WalkDir;

// Downscaling: 18MP camera photos decode to ~70-100MB bitmaps in WebKit,
// which is more than a 1GB Raspberry Pi can handle. Cap images at screen
// resolution and cache the results on disk so they're only built once.
const MAX_DIM: u32 = 1920;
const THUMB_DIR: &str = "/var/cache/slideshow-thumbs";

fn folder_has_no_images(folder_path: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let has_images = fs::read_dir(folder_path)?
        .filter_map(Result::ok)
        .any(|entry| {
            entry.path().extension()
                .and_then(|ext| Some(ext.to_string_lossy().to_lowercase()))
                .map_or(false, |ext| {
                    ext == "jpg" || ext == "jpeg" || ext == "png"
                })
        });
    
    Ok(!has_images)
}

fn find_image_files_recursive(folder_path: &str) -> Vec<String> {
    WalkDir::new(folder_path)
        .into_iter()
        .filter_map(Result::ok)
        .map(|entry| entry.path().to_path_buf())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|ext| Some(ext.to_string_lossy().to_lowercase()))
                .map_or(false, |ext| {
                    ext == "jpg" || ext == "jpeg" || ext == "png"
                })
        })
        .map(|path| path.to_string_lossy().to_string())
        .collect()
}

#[tauri::command]
fn check_dev() -> bool {
    return tauri::is_dev();
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn validate_path() -> String {
    // expects images in /mnt/slideshow-images
    if !Path::new("/mnt/slideshow-images").exists() {
        return "fail::not_setup".to_string();
    }

    if Path::new("/mnt/slideshow-images/.nodevice").exists() {
        return "fail::no_device".to_string();
    }

    if !Path::new("/mnt/slideshow-images/img").exists() {
        return "fail::invalid_usb".to_string();
    }

    match folder_has_no_images("/mnt/slideshow-images/img") {
        Ok(has_no_images) => {
            if has_no_images {
                return "fail::no_img".to_string();
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            return "fail::read_fail".to_string();
        }
    }

    return "OK".to_string();
}

#[tauri::command]
fn get_images() -> Vec<String> {
    let files = find_image_files_recursive("/mnt/slideshow-images/img");
    return files;
}

// Cache key: path + size + mtime, so edited/replaced photos are rebuilt.
fn thumb_path_for(path: &str) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    if let Ok(meta) = fs::metadata(path) {
        meta.len().hash(&mut hasher);
        if let Ok(mtime) = meta.modified() {
            if let Ok(d) = mtime.duration_since(std::time::UNIX_EPOCH) {
                d.as_secs().hash(&mut hasher);
            }
        }
    }
    Path::new(THUMB_DIR).join(format!("{:016x}.jpg", hasher.finish()))
}

// Returns the best file to show for a photo: the original when it already
// fits the screen, otherwise a JPEG scaled to at most MAX_DIM pixels wide/tall.
fn prepare_display(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let img = image::open(path)?;
    let (w, h) = (img.width(), img.height());

    if w <= MAX_DIM && h <= MAX_DIM {
        return Ok(path.to_string());
    }

    let tp = thumb_path_for(path);
    if tp.metadata().map_or(false, |m| m.len() > 0) {
        return Ok(tp.to_string_lossy().into_owned());
    }

    let scale = MAX_DIM as f64 / w.max(h) as f64;
    let nw = (w as f64 * scale).round().max(1.0) as u32;
    let nh = (h as f64 * scale).round().max(1.0) as u32;
    let resized = img
        .resize(nw, nh, image::imageops::FilterType::Triangle)
        .to_rgb8();

    fs::create_dir_all(THUMB_DIR)?;
    let tmp = tp.with_extension("jpg.tmp");
    resized.save_with_format(&tmp, image::ImageFormat::Jpeg)?;
    fs::rename(&tmp, &tp)?;
    Ok(tp.to_string_lossy().into_owned())
}

#[tauri::command]
fn get_display_path(path: String) -> String {
    match prepare_display(&path) {
        Ok(display) => display,
        Err(e) => {
            // Never break the slideshow over a bad image or unwritable cache
            // (e.g. dev machine): fall back to showing the original.
            eprintln!("display prep failed for {path}: {e}");
            path
        }
    }
}

// Builds all thumbnails in the background at startup so the slideshow never
// blocks on a cold cache.
#[tauri::command]
fn start_thumb_pregen() {
    std::thread::spawn(|| {
        let _ = fs::create_dir_all(THUMB_DIR);
        for path in find_image_files_recursive("/mnt/slideshow-images/img") {
            let tp = thumb_path_for(&path);
            if tp.metadata().map_or(true, |m| m.len() == 0) {
                if let Err(e) = prepare_display(&path) {
                    eprintln!("pregen failed for {path}: {e}");
                }
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![validate_path, get_images, check_dev, get_display_path, start_thumb_pregen])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
