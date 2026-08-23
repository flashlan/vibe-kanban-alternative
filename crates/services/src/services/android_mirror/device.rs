//! `adb` resolution and device discovery/selection for the mirror feature.
//! Deliberately minimal compared to `android-dev-server.sh`'s Tailscale/
//! Termux/mDNS reconnect dance — v1 assumes a device is already reachable
//! via plain `adb` (USB, or already `adb connect`-ed) by the time someone
//! opens the mirror panel.

use std::path::PathBuf;

use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdbDevice {
    pub serial: String,
    pub state: String,
}

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("adb not found (checked PATH and common SDK install locations)")]
    AdbNotFound,
    #[error("failed to run adb: {0}")]
    Io(#[from] std::io::Error),
    #[error("`adb devices` failed: {0}")]
    AdbCommandFailed(String),
    #[error("no device is connected via adb")]
    NoDevice,
    #[error("multiple devices connected via adb ({0:?}) — pin one for this workspace")]
    MultipleDevices(Vec<String>),
    #[error("device '{0}' is not connected via adb")]
    DeviceNotFound(String),
    #[error("'{0}' doesn't look like an Android package name")]
    InvalidPackageName(String),
}

/// Resolve the `adb` binary: PATH first (covers Homebrew), then the
/// Android Studio SDK's default install location — same fallback
/// `android-dev-server.sh`'s `get_adb()` uses, minus the Intel-Homebrew path
/// (covered by `resolve_executable_path`'s own PATH refresh).
pub async fn resolve_adb() -> Result<PathBuf, DeviceError> {
    if let Some(path) = utils::shell::resolve_executable_path("adb").await {
        return Ok(path);
    }
    if let Some(home) = dirs::home_dir() {
        let sdk_adb = home.join("Library/Android/sdk/platform-tools/adb");
        if sdk_adb.is_file() {
            return Ok(sdk_adb);
        }
    }
    Err(DeviceError::AdbNotFound)
}

/// Parse `adb devices -l` output (header line + one line per device: serial,
/// whitespace, state, then optional key:value fields we ignore).
pub fn parse_adb_devices_output(raw: &str) -> Vec<AdbDevice> {
    raw.lines()
        .skip(1) // "List of devices attached"
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let serial = parts.next()?;
            let state = parts.next()?;
            if serial.is_empty() || state.is_empty() {
                return None;
            }
            Some(AdbDevice {
                serial: serial.to_string(),
                state: state.to_string(),
            })
        })
        .collect()
}

pub async fn list_devices(adb_path: &std::path::Path) -> Result<Vec<AdbDevice>, DeviceError> {
    let output = Command::new(adb_path)
        .arg("devices")
        .arg("-l")
        .output()
        .await?;
    if !output.status.success() {
        return Err(DeviceError::AdbCommandFailed(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    Ok(parse_adb_devices_output(&raw)
        .into_iter()
        .filter(|d| d.state == "device")
        .collect())
}

/// `Some(serial)` → require that exact device to be present. `None` →
/// require exactly one device (auto-select), erroring otherwise rather than
/// guessing which one the caller meant.
pub fn select_device<'a>(
    devices: &'a [AdbDevice],
    serial: Option<&str>,
) -> Result<&'a AdbDevice, DeviceError> {
    match serial {
        Some(serial) => devices
            .iter()
            .find(|d| d.serial == serial)
            .ok_or_else(|| DeviceError::DeviceNotFound(serial.to_string())),
        None => match devices {
            [] => Err(DeviceError::NoDevice),
            [only] => Ok(only),
            many => Err(DeviceError::MultipleDevices(
                many.iter().map(|d| d.serial.clone()).collect(),
            )),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_no_devices() {
        let raw = "List of devices attached\n\n";
        assert_eq!(parse_adb_devices_output(raw), vec![]);
    }

    #[test]
    fn parses_one_device() {
        let raw = "List of devices attached\nRQCX303C1EL           device usb:1-1 product:foo model:Pixel_7 device:panther transport_id:1\n";
        let devices = parse_adb_devices_output(raw);
        assert_eq!(
            devices,
            vec![AdbDevice {
                serial: "RQCX303C1EL".to_string(),
                state: "device".to_string(),
            }]
        );
    }

    #[test]
    fn parses_multiple_devices_and_filters_by_state_elsewhere() {
        let raw = "List of devices attached\n\
100.114.33.93:41287    device product:foo\n\
emulator-5554          offline\n";
        let devices = parse_adb_devices_output(raw);
        assert_eq!(
            devices,
            vec![
                AdbDevice {
                    serial: "100.114.33.93:41287".to_string(),
                    state: "device".to_string(),
                },
                AdbDevice {
                    serial: "emulator-5554".to_string(),
                    state: "offline".to_string(),
                },
            ]
        );
    }

    fn dev(serial: &str) -> AdbDevice {
        AdbDevice {
            serial: serial.to_string(),
            state: "device".to_string(),
        }
    }

    #[test]
    fn select_auto_with_zero_devices_errors() {
        assert!(matches!(
            select_device(&[], None),
            Err(DeviceError::NoDevice)
        ));
    }

    #[test]
    fn select_auto_with_one_device_succeeds() {
        let devices = vec![dev("abc")];
        assert_eq!(select_device(&devices, None).unwrap().serial, "abc");
    }

    #[test]
    fn select_auto_with_multiple_devices_errors() {
        let devices = vec![dev("abc"), dev("def")];
        assert!(matches!(
            select_device(&devices, None),
            Err(DeviceError::MultipleDevices(_))
        ));
    }

    #[test]
    fn select_pinned_serial_found() {
        let devices = vec![dev("abc"), dev("def")];
        assert_eq!(select_device(&devices, Some("def")).unwrap().serial, "def");
    }

    #[test]
    fn select_pinned_serial_missing_errors() {
        let devices = vec![dev("abc")];
        assert!(matches!(
            select_device(&devices, Some("zzz")),
            Err(DeviceError::DeviceNotFound(_))
        ));
    }
}
