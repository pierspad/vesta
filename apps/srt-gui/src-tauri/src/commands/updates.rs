use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{io::Write, path::Path, process::Command, time::Duration};
use tauri::{Emitter, Manager};

const RELEASE_API: &str = "https://api.github.com/repos/pierspad/vesta/releases/latest";
static INSTALLING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
struct InstallGuard;
impl Drop for InstallGuard {
    fn drop(&mut self) {
        INSTALLING.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}
const MAX_INSTALLER_SIZE: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInstallation {
    pub channel: String,
    pub os: String,
    pub arch: String,
}

fn owned_by(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .is_ok_and(|o| o.status.success())
}

fn installation() -> UpdateInstallation {
    let executable = std::env::current_exe().unwrap_or_default();
    let path = executable.to_string_lossy();
    let channel = if cfg!(debug_assertions) {
        "development"
    } else if std::env::var_os("FLATPAK_ID").is_some() || Path::new("/.flatpak-info").exists() {
        "flatpak"
    } else if std::env::var_os("SNAP").is_some() {
        "snap"
    } else if std::env::var_os("APPIMAGE").is_some() {
        "appimage"
    } else if cfg!(target_os = "linux") && owned_by("pacman", &["-Qo", &path]) {
        "aur"
    } else if cfg!(target_os = "linux") && owned_by("dpkg-query", &["-S", &path]) {
        "deb"
    } else if cfg!(target_os = "linux") && owned_by("rpm", &["-qf", &path]) {
        "rpm"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    };
    UpdateInstallation {
        channel: channel.into(),
        os: std::env::consts::OS.into(),
        arch: std::env::consts::ARCH.into(),
    }
}

#[tauri::command]
pub async fn get_update_installation() -> Result<UpdateInstallation, String> {
    tokio::task::spawn_blocking(installation)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
    digest: Option<String>,
    size: u64,
}
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    prerelease: bool,
    draft: bool,
    assets: Vec<ReleaseAsset>,
}

fn matching_asset(asset: &ReleaseAsset, install: &UpdateInstallation) -> bool {
    let name = asset.name.to_lowercase();
    let arch_matches = match install.arch.as_str() {
        "x86_64" => name.contains("x64") || name.contains("x86_64") || name.contains("amd64"),
        "aarch64" => name.contains("arm64") || name.contains("aarch64"),
        _ => false,
    };
    arch_matches
        && match install.channel.as_str() {
            "windows" => name.ends_with(".exe"),
            "deb" => name.ends_with(".deb"),
            "rpm" => name.ends_with(".rpm"),
            _ => false,
        }
}

fn expected_digest(asset: &ReleaseAsset) -> Result<String, String> {
    let digest = asset.digest.as_deref().and_then(|d| d.strip_prefix("sha256:"))
        .filter(|d| d.len() == 64 && d.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("La release non fornisce un checksum SHA-256 verificabile. Scarica il pacchetto dalla pagina della release.")?;
    Ok(digest.to_lowercase())
}

#[tauri::command]
pub async fn install_release_update(app: tauri::AppHandle, version: String) -> Result<(), String> {
    INSTALLING
        .compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        )
        .map_err(|_| "Download aggiornamento già in corso")?;
    let _guard = InstallGuard;
    // Re-resolve the official release here rather than accepting a URL/path from the webview.
    let install = get_update_installation().await?;
    if !matches!(install.channel.as_str(), "windows" | "deb" | "rpm") {
        return Err("Questa installazione va aggiornata tramite il suo gestore o dalla pagina della release.".into());
    }
    let client = reqwest::Client::builder()
        .user_agent("Vesta-update-check")
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;
    let release: Release = client
        .get(RELEASE_API)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    if release.tag_name != version || release.prerelease || release.draft {
        return Err("La release è cambiata. Controlla nuovamente gli aggiornamenti.".into());
    }
    let asset = release
        .assets
        .iter()
        .find(|a| matching_asset(a, &install))
        .ok_or("Nessun installer compatibile disponibile per questa piattaforma e architettura.")?;
    let digest = expected_digest(asset)?;
    if asset.size == 0 || asset.size > MAX_INSTALLER_SIZE {
        return Err("Dimensione installer non valida".into());
    }
    let prefix = format!(
        "https://github.com/pierspad/vesta/releases/download/{}/",
        release.tag_name
    );
    if !asset.browser_download_url.starts_with(&prefix) {
        return Err("URL installer non valido".into());
    }
    let directory = app
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("updates");
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let extension = match install.channel.as_str() {
        "windows" => "exe",
        "deb" => "deb",
        _ => "rpm",
    };
    // A private unique directory keeps the verified installer separate from older downloads.
    let stage = tempfile::Builder::new()
        .prefix("release-")
        .tempdir_in(&directory)
        .map_err(|e| e.to_string())?;
    let destination = stage.path().join(format!("vesta-update.{extension}"));
    let mut file = std::fs::File::create(&destination).map_err(|e| e.to_string())?;
    let mut response = client
        .get(&asset.browser_download_url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0u64;
    let mut last_percent = 0;
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        downloaded += chunk.len() as u64;
        if downloaded > asset.size || downloaded > MAX_INSTALLER_SIZE {
            return Err("Dimensione download inattesa".into());
        }
        hasher.update(&chunk);
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        let percent = (downloaded * 100 / asset.size) as u32;
        if percent != last_percent {
            last_percent = percent;
            let _ = app.emit("update-download-progress", percent);
        }
    }
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    if downloaded != asset.size
        || hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
            != digest
    {
        return Err("Verifica SHA-256 del download fallita. Installer non aperto.".into());
    }
    #[cfg(target_os = "windows")]
    let launched = Command::new(&destination).spawn();
    #[cfg(not(target_os = "windows"))]
    let launched = Command::new("xdg-open").arg(&destination).spawn();
    launched.map_err(|e| format!("Impossibile aprire l'installer: {e}"))?;
    let _ = stage.keep();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn asset(name: &str) -> ReleaseAsset {
        ReleaseAsset {
            name: name.into(),
            browser_download_url: String::new(),
            digest: None,
            size: 1,
        }
    }
    #[test]
    fn selects_platform_arch_and_never_managed_installations() {
        let mut install = UpdateInstallation {
            channel: "windows".into(),
            os: "windows".into(),
            arch: "x86_64".into(),
        };
        assert!(matching_asset(
            &asset("vesta_1.2.3_x64-setup.exe"),
            &install
        ));
        assert!(!matching_asset(
            &asset("vesta_1.2.3_arm64-setup.exe"),
            &install
        ));
        install.channel = "aur".into();
        assert!(!matching_asset(&asset("vesta_1.2.3_amd64.deb"), &install));
        install.channel = "deb".into();
        assert!(matching_asset(&asset("vesta_1.2.3_amd64.deb"), &install));
    }
    #[test]
    fn requires_a_well_formed_sha256() {
        let mut candidate = asset("installer.exe");
        assert!(expected_digest(&candidate).is_err());
        candidate.digest = Some(format!("sha256:{}", "A".repeat(64)));
        assert_eq!(expected_digest(&candidate).unwrap(), "a".repeat(64));
        candidate.digest = Some("sha256:invalid".into());
        assert!(expected_digest(&candidate).is_err());
    }
}
