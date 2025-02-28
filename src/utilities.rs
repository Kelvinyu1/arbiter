use std::fs;
use std::io;
use std::path::Path;

// pub fn get_directory_contents(path: &str) -> Vec<String> {
//     let mut contents = vec![];
//     if let Ok(entries) = fs::read_dir(path) {
//         for entry in entries.flatten() {
//             let entry_name = entry
//                 .file_name()
//                 .into_string()
//                 .unwrap_or_else(|_| "Invalid UTF-8".to_string());
//             contents.push(entry_name);
//         }
//     }
//     contents
// }

pub fn get_directories_only(path: &str, exclude: Option<&str>) -> Vec<String> {
    let mut contents = vec!["..".to_string()]; // Include parent directory option

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_name = entry
                .file_name()
                .into_string()
                .unwrap_or_else(|_| "Invalid UTF-8".to_string());

            if let Some(excluded_name) = exclude {
                if entry_name == excluded_name {
                    continue; // Skip if it's the same directory being moved
                }
            }

            if fs::metadata(format!("{}/{}", path, entry_name))
                .map(|m| m.is_dir())
                .unwrap_or(false)
            {
                contents.push(entry_name);
            }
        }
    }
    contents
}

pub fn move_file_or_directory(src: &str, dest: &str) -> io::Result<()> {
    let src_path = Path::new(src);
    let dest_path = Path::new(dest).join(src_path.file_name().unwrap());

    if src_path.is_dir() {
        fs::create_dir_all(&dest_path)?;
        for entry in fs::read_dir(src_path)? {
            let entry = entry?;
            let entry_src = entry.path();
            let entry_dest = dest_path.join(entry.file_name());
            if entry_src.is_dir() {
                move_file_or_directory(entry_src.to_str().unwrap(), entry_dest.to_str().unwrap())?;
            } else {
                fs::copy(&entry_src, &entry_dest)?;
            }
        }
        fs::remove_dir_all(src_path)?;
    } else {
        fs::copy(src_path, &dest_path)?;
        fs::remove_file(src_path)?;
    }

    Ok(())
}
