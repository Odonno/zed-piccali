//! Piccali — Gherkin language support for Zed.
//!
//! Launches the piccali-cli language server (`piccali-cli lsp`, stdio).
//! Binary acquisition order:
//!   1. user override via `"lsp": { "piccali": { "binary": { "path": ... } } }`
//!   2. an existing install on PATH (`piccali-cli` or `piccali`; mise shims resolve through the worktree's shell environment)
//!   3. latest GitHub release, downloaded into a per-version cache dir

use zed_extension_api::{self as zed, LanguageServerId, Result, settings::LspSettings};

const BINARY_NAME: &str = "piccali-cli";
const GITHUB_REPO: &str = "Odonno/piccali-cli";
const LSP_NAME: &str = "piccali";

struct PiccaliExtension {
    /// Path to a downloaded binary, cached so repeat server starts skip the network.
    cached_binary_path: Option<String>,
}

impl PiccaliExtension {
    fn binary_path(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        // (1) User override.
        if let Ok(settings) = LspSettings::for_worktree(LSP_NAME, worktree)
            && let Some(binary) = settings.binary
            && let Some(path) = binary.path
        {
            return Ok(path);
        }

        // (2) Existing install on PATH (also covers mise shims).
        for name in [BINARY_NAME, "piccali"] {
            if let Some(path) = worktree.which(name) {
                return Ok(path);
            }
        }

        // (3) Reuse a previous download while it is still on disk.
        if let Some(path) = &self.cached_binary_path
            && std::fs::metadata(path).is_ok_and(|stat| stat.is_file())
        {
            return Ok(path.clone());
        }

        // (4) Download the latest GitHub release for this platform.
        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );

        let release = zed::latest_github_release(
            GITHUB_REPO,
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (os, arch) = zed::current_platform();
        let arch_str = match arch {
            zed::Architecture::Aarch64 => "aarch64",
            zed::Architecture::X8664 => "x86_64",
            zed::Architecture::X86 => {
                return Err("piccali-cli does not publish x86 release assets".into());
            }
        };
        let (os_str, file_type, archive_ext) = match os {
            zed::Os::Mac => ("apple-darwin", zed::DownloadedFileType::GzipTar, "tar.gz"),
            zed::Os::Linux => (
                "unknown-linux-musl",
                zed::DownloadedFileType::GzipTar,
                "tar.gz",
            ),
            zed::Os::Windows => ("pc-windows-msvc", zed::DownloadedFileType::Zip, "zip"),
        };

        // Assets are named like `piccali-cli-v0.11.0-x86_64-unknown-linux-musl.tar.gz`.
        let version = release.version.trim_start_matches('v');
        let asset_name = format!("{BINARY_NAME}-v{version}-{arch_str}-{os_str}.{archive_ext}");
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no release asset matching `{asset_name}`"))?;

        let version_dir = format!("{BINARY_NAME}-v{version}");
        let executable = match os {
            zed::Os::Windows => format!("{version_dir}/{BINARY_NAME}.exe"),
            _ => format!("{version_dir}/{BINARY_NAME}"),
        };

        if !std::fs::metadata(&executable).is_ok_and(|stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(&asset.download_url, &version_dir, file_type)
                .map_err(|err| format!("failed to download {asset_name}: {err}"))?;

            if !matches!(os, zed::Os::Windows) {
                zed::make_file_executable(&executable)?;
            }

            // Prune older version directories.
            if let Ok(entries) = std::fs::read_dir(".") {
                for entry in entries.flatten() {
                    let name = entry.file_name().into_string().unwrap_or_default();
                    if name.starts_with(&format!("{BINARY_NAME}-v")) && name != version_dir {
                        let _ = std::fs::remove_dir_all(entry.path());
                    }
                }
            }
        }

        self.cached_binary_path = Some(executable.clone());
        Ok(executable)
    }
}

impl zed::Extension for PiccaliExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        let path = self.binary_path(language_server_id, worktree)?;

        // User-provided arguments replace the default `lsp` subcommand.
        let args = LspSettings::for_worktree(LSP_NAME, worktree)
            .ok()
            .and_then(|settings| settings.binary)
            .and_then(|binary| binary.arguments)
            .unwrap_or_else(|| vec!["lsp".to_string()]);

        Ok(zed::Command {
            command: path,
            args,
            env: worktree.shell_env(),
        })
    }
}

zed::register_extension!(PiccaliExtension);
