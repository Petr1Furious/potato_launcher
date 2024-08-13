use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::io::AsRawHandle;
#[cfg(target_os = "windows")]
use std::ptr::null_mut;
#[cfg(target_os = "windows")]
use winapi::um::fileapi::GetLongPathNameW;
#[cfg(target_os = "windows")]
use winapi::um::namedpipeapi::SetNamedPipeHandleState;
#[cfg(target_os = "windows")]
use winapi::um::winbase::PIPE_NOWAIT;
#[cfg(target_os = "windows")]
use winapi::um::winnt::HANDLE;
#[cfg(target_os = "windows")]
use winapi::shared::minwindef::DWORD;
#[cfg(target_os = "windows")]
use winapi::shared::winerror::ERROR_SUCCESS;
#[cfg(target_os = "windows")]
use winapi::um::errhandlingapi::GetLastError;
#[cfg(target_os = "windows")]
use winapi::um::handleapi::INVALID_HANDLE_VALUE;
#[cfg(target_os = "windows")]
use winapi::um::winnt::LPCWSTR;

fn chmod_x<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(perms.mode() | 0o111);
    fs::set_permissions(path, perms).unwrap();
}

fn perform_forbidden_nixery() {
    if !cfg!(target_os = "linux") || !Path::new("/etc/NIXOS").exists() {
        return;
    }

    println!("Performing forbidden nixery");
    let pkg_names = [
        "xorg.libX11",
        "xorg.libXext",
        "xorg.libXcursor",
        "xorg.libXrandr",
        "xorg.libXxf86vm",
        "libpulseaudio",
        "libGL",
        "glfw",
        "openal",
    ].join(" ");

    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "nix eval --json nixpkgs#legacyPackages.x86_64-linux --apply \"pkgs: with pkgs; [{}]\" --extra-experimental-features nix-command --extra-experimental-features flakes",
            pkg_names
        ))
        .output()
        .expect("Failed to execute command");

    let pkgs: Vec<String> = serde_json::from_slice(&output.stdout).unwrap();
    let mut ld_library_path: Vec<String> = env::var("LD_LIBRARY_PATH")
        .map(|val| vec![val])
        .unwrap_or_else(|_| vec![]);
    ld_library_path.extend(pkgs.iter().map(|x| format!("{}/lib", x)));
    env::set_var("LD_LIBRARY_PATH", ld_library_path.join(":"));
}

#[cfg(target_os = "windows")]
fn win_pipe_nowait(pipefd: std::os::windows::io::RawHandle) -> Result<(), std::io::Error> {
    use std::os::windows::io::AsRawHandle;
    use winapi::um::fileapi::GetLongPathNameW;
    use winapi::um::namedpipeapi::SetNamedPipeHandleState;
    use winapi::um::winbase::PIPE_NOWAIT;
    use winapi::um::winnt::HANDLE;
    use winapi::shared::minwindef::DWORD;
    use winapi::shared::winerror::ERROR_SUCCESS;
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;
    use winapi::um::winnt::LPCWSTR;

    let h = pipefd;
    let mut mode: DWORD = PIPE_NOWAIT;
    let res = unsafe { SetNamedPipeHandleState(h as HANDLE, &mut mode, null_mut(), null_mut()) };
    if res == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn win_get_long_path_name(path: &str) -> Result<String, std::io::Error> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::fileapi::GetLongPathNameW;
    use winapi::um::handleapi::INVALID_HANDLE_VALUE;
    use winapi::um::winnt::LPCWSTR;

    let mut buf: Vec<u16> = vec![0; 1024];
    let path_wide: Vec<u16> = OsStr::new(path).encode_wide().chain(Some(0)).collect();
    let res = unsafe { GetLongPathNameW(path_wide.as_ptr(), buf.as_mut_ptr(), buf.len() as u32) };
    if res == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(String::from_utf16_lossy(&buf[..res as usize]))
}

// on other systems
#[cfg(not(target_os = "windows"))]
pub fn win_get_long_path_name(path: &str) -> String {
    path.to_string()
}
