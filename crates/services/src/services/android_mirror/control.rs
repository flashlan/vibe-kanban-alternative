//! Simple device actions that don't need scrcpy's control socket at all —
//! navigation keys and force-stopping an app are plain `adb shell` commands.
//! Real touch/tap forwarding (scrcpy's control channel) is still out of
//! scope for v1 (see the project plan); these are unrelated to that and
//! reuse only `device::resolve_adb`/`select_device`.

use tokio::process::Command;

use super::device::{self, DeviceError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavAction {
    Home,
    Back,
    Recents,
}

impl NavAction {
    /// Android `KEYCODE_*` values (`android.view.KeyEvent`).
    fn keycode(self) -> &'static str {
        match self {
            NavAction::Home => "3",      // KEYCODE_HOME
            NavAction::Back => "4",      // KEYCODE_BACK
            NavAction::Recents => "187", // KEYCODE_APP_SWITCH
        }
    }
}

async fn resolved_serial(
    device_serial: Option<&str>,
) -> Result<(std::path::PathBuf, String), DeviceError> {
    let adb_path = device::resolve_adb().await?;
    let devices = device::list_devices(&adb_path).await?;
    let device = device::select_device(&devices, device_serial)?;
    Ok((adb_path, device.serial.clone()))
}

pub async fn send_nav_action(
    device_serial: Option<&str>,
    action: NavAction,
) -> Result<(), DeviceError> {
    let (adb_path, serial) = resolved_serial(device_serial).await?;
    let output = Command::new(&adb_path)
        .args([
            "-s",
            &serial,
            "shell",
            "input",
            "keyevent",
            action.keycode(),
        ])
        .output()
        .await?;
    if !output.status.success() {
        return Err(DeviceError::AdbCommandFailed(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    Ok(())
}

/// Force-stop an app by package name (`adb shell am force-stop <package>`).
/// `package` is passed as a single argv element (no shell involved), so
/// there's no injection concern from arbitrary characters — but a name that
/// isn't a plausible Android package id is rejected up front so a typo
/// doesn't silently no-op against `am`.
pub async fn force_stop(device_serial: Option<&str>, package: &str) -> Result<(), DeviceError> {
    if package.is_empty()
        || !package
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
    {
        return Err(DeviceError::InvalidPackageName(package.to_string()));
    }
    let (adb_path, serial) = resolved_serial(device_serial).await?;
    let output = Command::new(&adb_path)
        .args(["-s", &serial, "shell", "am", "force-stop", package])
        .output()
        .await?;
    if !output.status.success() {
        return Err(DeviceError::AdbCommandFailed(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    Ok(())
}
