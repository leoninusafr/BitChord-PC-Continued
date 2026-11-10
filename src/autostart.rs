//! Windows Autostart: "Start with Windows" registry integration.
//!
//! Adds or removes `BitChord` from `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

#[cfg(windows)]
pub fn is_enabled() -> bool {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW};
    use windows::core::w;
    let mut buf = [0u16; 512];
    let mut size = (buf.len() * 2) as u32;
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!(r"Software\Microsoft\Windows\CurrentVersion\Run"),
            w!("BitChord"),
            RRF_RT_REG_SZ,
            None,
            Some(buf.as_mut_ptr() as *mut _),
            Some(&mut size),
        )
        .is_ok()
    }
}

#[cfg(windows)]
pub fn set_enabled(enable: bool) -> bool {
    use windows::Win32::System::Registry::{HKEY_CURRENT_USER, REG_SZ, RegDeleteKeyValueW, RegSetKeyValueW};
    use windows::core::w;

    if enable {
        if let Ok(exe) = std::env::current_exe() {
            let cmd = format!("\"{}\"", exe.to_string_lossy());
            let wide: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();
            unsafe {
                RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    w!(r"Software\Microsoft\Windows\CurrentVersion\Run"),
                    w!("BitChord"),
                    REG_SZ.0,
                    Some(wide.as_ptr() as *const _),
                    (wide.len() * 2) as u32,
                )
                .is_ok()
            }
        } else {
            false
        }
    } else {
        unsafe {
            RegDeleteKeyValueW(
                HKEY_CURRENT_USER,
                w!(r"Software\Microsoft\Windows\CurrentVersion\Run"),
                w!("BitChord"),
            )
            .is_ok()
        }
    }
}

#[cfg(not(windows))]
pub fn is_enabled() -> bool {
    false
}

#[cfg(not(windows))]
pub fn set_enabled(_enable: bool) -> bool {
    false
}
