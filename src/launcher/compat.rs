#[cfg(target_os = "linux")]
use std::path::Path;

#[cfg(target_os = "linux")]
pub fn chmod_x<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let path = path.as_ref();
    let mut perms = fs::metadata(path)?.permissions();
    perms.set_mode(perms.mode() | 0o111);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn win_get_long_path_name(path: &str) -> Result<String, std::io::Error> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::fileapi::GetLongPathNameW;

    let mut buf: Vec<u16> = vec![0; 1024];
    let path_wide: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();
    let res = unsafe { GetLongPathNameW(path_wide.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) };
    if res == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(String::from_utf16_lossy(&buf[..res as usize]))
}

#[cfg(not(target_os = "windows"))]
pub fn win_get_long_path_name(_path: &str) -> Result<String, std::io::Error> {
    unimplemented!();
}
