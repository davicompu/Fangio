//! Small platform differences between macOS and Windows (and Linux), kept in
//! one place: opening files with their default app, revealing them in the file
//! manager, display paths, the user's name, and running helper programs
//! without a console window.

use std::path::{Path, PathBuf};

/// Windows `canonicalize` returns verbatim paths (`\\?\C:\...`) that some
/// programs refuse to open. Turn `\\?\C:\x` into `C:\x` and `\\?\UNC\srv\share`
/// into `\\srv\share`; leave every other path alone.
pub fn simplify_verbatim(p: &str) -> String {
    if let Some(rest) = p.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = p.strip_prefix(r"\\?\") {
        // Only drive paths like C:\ are safe to shorten.
        if rest.len() >= 2 && rest.as_bytes()[1] == b':' {
            rest.to_string()
        } else {
            p.to_string()
        }
    } else {
        p.to_string()
    }
}

/// `canonicalize` that returns a path other programs can open on Windows too.
pub fn canonicalize(p: &Path) -> std::io::Result<PathBuf> {
    let c = std::fs::canonicalize(p)?;
    Ok(PathBuf::from(simplify_verbatim(&c.to_string_lossy())))
}

/// Open a file or folder with the system's default application.
pub fn open_path(p: &Path) -> Result<(), String> {
    tauri_plugin_opener::open_path(p, None::<&str>).map_err(|e| format!("Failed to open: {e}"))
}

/// Show a file selected in Finder / File Explorer.
pub fn reveal_in_file_manager(p: &Path) -> Result<(), String> {
    tauri_plugin_opener::reveal_item_in_dir(p).map_err(|e| format!("Failed to reveal: {e}"))
}

/// The signed-in user's name: `USER` on macOS/Linux, `USERNAME` on Windows.
pub fn user_name() -> Option<String> {
    std::env::var("USER")
        .ok()
        .or_else(|| std::env::var("USERNAME").ok())
        .filter(|s| !s.trim().is_empty())
}

/// A `Command` for a helper program that never flashes a console window on
/// Windows (no effect elsewhere).
pub fn quiet_command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    #[allow(unused_mut)]
    let mut cmd = std::process::Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::simplify_verbatim;

    #[test]
    fn verbatim_paths_are_simplified() {
        assert_eq!(simplify_verbatim(r"\\?\C:\Users\Ana\Review\pdfs\a.pdf"), r"C:\Users\Ana\Review\pdfs\a.pdf");
        assert_eq!(simplify_verbatim(r"\\?\UNC\server\share\x.pdf"), r"\\server\share\x.pdf");
        assert_eq!(simplify_verbatim(r"\\?\Volume{abc}\x"), r"\\?\Volume{abc}\x", "non-drive verbatim kept");
        assert_eq!(simplify_verbatim("/Users/ana/a.pdf"), "/Users/ana/a.pdf");
    }
}
