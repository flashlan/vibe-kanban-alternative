//! Listing and launching Android emulators (AVDs) via the SDK's `emulator`
//! binary — separate from `device.rs`'s `adb`-based device discovery, since
//! `adb` only sees an emulator *after* it's already booted; this is the
//! "there's nothing to mirror yet, start one" step before that.

use std::{path::PathBuf, process::Stdio};

use tokio::process::Command;

#[derive(Debug, thiserror::Error)]
pub enum EmulatorError {
    #[error(
        "emulator binary not found (checked PATH and the Android SDK's default install location)"
    )]
    EmulatorNotFound,
    #[error("failed to run emulator: {0}")]
    Io(#[from] std::io::Error),
    #[error("`emulator -list-avds` failed: {0}")]
    ListAvdsFailed(String),
    #[error("no AVD named '{0}' — check `emulator -list-avds`")]
    AvdNotFound(String),
}

/// Resolve the `emulator` binary: PATH first, then the Android Studio SDK's
/// default install location — same fallback shape as `device::resolve_adb`.
pub async fn resolve_emulator() -> Result<PathBuf, EmulatorError> {
    if let Some(path) = utils::shell::resolve_executable_path("emulator").await {
        return Ok(path);
    }
    if let Some(home) = dirs::home_dir() {
        let sdk_emulator = home.join("Library/Android/sdk/emulator/emulator");
        if sdk_emulator.is_file() {
            return Ok(sdk_emulator);
        }
    }
    Err(EmulatorError::EmulatorNotFound)
}

/// `emulator -list-avds` output is just one AVD name per line.
pub fn parse_avd_names(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

pub async fn list_avds(emulator_path: &std::path::Path) -> Result<Vec<String>, EmulatorError> {
    let output = Command::new(emulator_path)
        .arg("-list-avds")
        .output()
        .await?;
    if !output.status.success() {
        return Err(EmulatorError::ListAvdsFailed(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    Ok(parse_avd_names(&String::from_utf8_lossy(&output.stdout)))
}

/// Launches an AVD and returns immediately — the emulator process runs for
/// the lifetime of that virtual device (until the user closes its window),
/// so this deliberately doesn't wait on it or tie its lifetime to this
/// request the way `client::connect`'s scrcpy server child is tied to the WS
/// session. It'll show up in `adb devices` (and this app's device picker,
/// which already polls that) once boot finishes, same as any device plugged
/// in externally.
pub async fn launch_avd(
    emulator_path: &std::path::Path,
    avd_name: &str,
) -> Result<(), EmulatorError> {
    let avd_names = list_avds(emulator_path).await?;
    if !avd_names.iter().any(|n| n == avd_name) {
        return Err(EmulatorError::AvdNotFound(avd_name.to_string()));
    }
    // `-no-window`: the mirror panel already shows this device's screen
    // in-app, so the emulator's own native OS window would just be a
    // redundant, confusing second copy of the same display floating
    // outside the app. `-no-audio` matches (nothing plays through a
    // window that no longer exists).
    Command::new(emulator_path)
        .args(["-avd", avd_name, "-no-window", "-no-audio"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_no_avds() {
        assert_eq!(parse_avd_names(""), Vec::<String>::new());
    }

    #[test]
    fn parses_multiple_avds() {
        let raw = "Pixel_7_API_34\nPixel_Tablet_API_33\n";
        assert_eq!(
            parse_avd_names(raw),
            vec![
                "Pixel_7_API_34".to_string(),
                "Pixel_Tablet_API_33".to_string()
            ]
        );
    }

    #[test]
    fn trims_whitespace_and_skips_blank_lines() {
        let raw = "  Pixel_7_API_34  \n\n\nPixel_Tablet_API_33\n";
        assert_eq!(
            parse_avd_names(raw),
            vec![
                "Pixel_7_API_34".to_string(),
                "Pixel_Tablet_API_33".to_string()
            ]
        );
    }
}
