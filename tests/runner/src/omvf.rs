use anyhow::{bail, Result};
use sha2::{Digest, Sha256};
use std::{
    env,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};
use tar::Archive;
use tempfile::TempDir;
use ureq::Agent;
#[cfg(target_os = "linux")]
use {std::fs::Permissions, std::os::unix::fs::PermissionsExt};

/// Name of the ovmf-prebuilt release tag.
const OVMF_PREBUILT_TAG: &str = "edk2-stable202311-r2";

/// SHA-256 hash of the release tarball.
const OVMF_PREBUILT_HASH: &str = "4a7d01b7dc6b0fdbf3a0e17dacd364b772fb5b712aaf64ecf328273584185ca0";

/// Directory into which the prebuilts will be download (relative to the repo root).
const OVMF_PREBUILT_DIR: &str = "target/ovmf";

/// Environment variable for overriding the path of the OVMF code file.
pub const ENV_VAR_OVMF_CODE: &str = "OVMF_CODE";

/// Environment variable for overriding the path of the OVMF vars file.
pub const ENV_VAR_OVMF_VARS: &str = "OVMF_VARS";

/// Environment variable for overriding the path of the OVMF shell file.
pub const ENV_VAR_OVMF_SHELL: &str = "OVMF_SHELL";

#[derive(Clone, Copy, Debug)]
pub enum OvmfFileType {
    Code,
    Vars,
    Shell,
}

impl OvmfFileType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Vars => "vars",
            Self::Shell => "shell",
        }
    }

    fn extension(&self) -> &'static str {
        match self {
            Self::Code | Self::Vars => "fd",
            Self::Shell => "efi",
        }
    }

    /// Get a user-provided path for the given OVMF file type.
    ///
    /// This uses an environment variable. If not present, returns `None`.
    pub fn get_user_provided_path(self) -> Option<PathBuf> {
        let var_name = match self {
            Self::Code => ENV_VAR_OVMF_CODE,
            Self::Vars => ENV_VAR_OVMF_VARS,
            Self::Shell => ENV_VAR_OVMF_SHELL,
        };
        env::var_os(var_name).map(PathBuf::from)
    }
}

pub struct OvmfPaths {
    code: PathBuf,
    vars: PathBuf,
    shell: PathBuf,
    temp_dir: Option<TempDir>,
    temp_vars: Option<PathBuf>,
}

impl OvmfPaths {
    pub fn code(&self) -> &PathBuf {
        &self.code
    }

    pub fn vars(&self) -> &PathBuf {
        self.temp_vars.as_ref().unwrap_or(&self.vars)
    }

    pub fn shell(&self) -> &PathBuf {
        &self.shell
    }

    /// Search for an OVMF file (either code or vars).
    ///
    /// There are multiple locations where a file is searched at in the following
    /// priority:
    /// 1. Command-line arg
    /// 2. Environment variable
    /// 3. Prebuilt file (automatically downloaded)
    pub fn find_ovmf_file(file_type: OvmfFileType) -> Result<PathBuf> {
        if let Some(path) = file_type.get_user_provided_path() {
            // The user provided an exact path to use; verify that it
            // exists.
            if path.exists() {
                Ok(path)
            } else {
                bail!(
                    "ovmf {} file does not exist: {}",
                    file_type.as_str(),
                    path.display()
                );
            }
        } else {
            let prebuilt_dir = update_prebuilt()?;

            Ok(prebuilt_dir.join(format!(
                "x86_64/{}.{}",
                file_type.as_str(),
                file_type.extension()
            )))
        }
    }

    /// Find path to OVMF files by the strategy documented for
    /// [`Self::find_ovmf_file`].
    pub fn find() -> Result<Self> {
        let code = Self::find_ovmf_file(OvmfFileType::Code)?;
        let vars = Self::find_ovmf_file(OvmfFileType::Vars)?;
        let shell = Self::find_ovmf_file(OvmfFileType::Shell)?;

        Ok(Self {
            code,
            vars,
            shell,
            temp_dir: None,
            temp_vars: None,
        })
    }

    /// Creates a copy with a writable, temp copy
    pub fn with_temp_vars(&mut self) -> Result<()> {
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path();

        // Make a copy of the OVMF vars file so that it can be used
        // read+write without modifying the original. Under AArch64, some
        // versions of OVMF won't boot if the vars file isn't writeable.
        let ovmf_vars = temp_path.join("ovmf_vars");
        fs_err::copy(&self.vars, &ovmf_vars)?;
        // Necessary, as for example on NixOS, the files are read-only inside
        // the Nix store.
        #[cfg(target_os = "linux")]
        fs_err::set_permissions(&ovmf_vars, Permissions::from_mode(0o666))?;

        self.temp_vars = Some(ovmf_vars);
        self.temp_dir = Some(temp_dir);
        Ok(())
    }
}

/// Update the local copy of the prebuilt OVMF files. Does nothing if the local
/// copy is already up to date.
fn update_prebuilt() -> Result<PathBuf> {
    let prebuilt_dir = Path::new(OVMF_PREBUILT_DIR);
    let hash_path = prebuilt_dir.join("sha256");

    // Check if the hash file already has the expected hash in it. If so, assume
    // that we've already got the correct prebuilt downloaded and unpacked.
    if let Ok(current_hash) = fs_err::read_to_string(&hash_path) {
        if current_hash == OVMF_PREBUILT_HASH {
            return Ok(prebuilt_dir.to_path_buf());
        }
    }

    let base_url = "https://github.com/rust-osdev/ovmf-prebuilt/releases/download";
    let url = format!(
        "{base_url}/{release}/{release}-bin.tar.xz",
        release = OVMF_PREBUILT_TAG
    );

    let data = download_url(&url)?;

    // Validate the hash.
    let actual_hash = format!("{:x}", Sha256::digest(&data));
    if actual_hash != OVMF_PREBUILT_HASH {
        bail!(
            "file hash {actual_hash} does not match {}",
            OVMF_PREBUILT_HASH
        );
    }

    // Unpack the tarball.
    println!("decompressing tarball");
    let mut decompressed = Vec::new();
    let mut compressed = Cursor::new(data);
    lzma_rs::xz_decompress(&mut compressed, &mut decompressed)?;

    // Clear out the existing prebuilt dir, if present.
    let _ = fs_err::remove_dir_all(prebuilt_dir);

    // Extract the files.
    extract_prebuilt(&decompressed, prebuilt_dir)?;

    // Rename the x64 directory to x86_64, to match `Arch::as_str`.
    fs_err::rename(prebuilt_dir.join("x64"), prebuilt_dir.join("x86_64"))?;

    // Write out the hash file. When we upgrade to a new release of
    // ovmf-prebuilt, the hash will no longer match, triggering a fresh
    // download.
    fs_err::write(&hash_path, actual_hash)?;

    Ok(prebuilt_dir.to_path_buf())
}

/// Download `url` and return the raw data.
fn download_url(url: &str) -> Result<Vec<u8>> {
    let agent: Agent = ureq::AgentBuilder::new()
        .user_agent("uefi-rs-ovmf-downloader")
        .build();

    // Limit the size of the download.
    let max_size_in_bytes = 5 * 1024 * 1024;

    // Download the file.
    println!("downloading {url}");
    let resp = agent.get(url).call()?;
    let mut data = Vec::with_capacity(max_size_in_bytes);
    resp.into_reader()
        .take(max_size_in_bytes.try_into().unwrap())
        .read_to_end(&mut data)?;
    println!("received {} bytes", data.len());

    Ok(data)
}

// Extract the tarball's files into `prebuilt_dir`.
//
// `tarball_data` is raw decompressed tar data.
fn extract_prebuilt(tarball_data: &[u8], prebuilt_dir: &Path) -> Result<()> {
    let cursor = Cursor::new(tarball_data);
    let mut archive = Archive::new(cursor);

    // Extract each file entry.
    for entry in archive.entries()? {
        let mut entry = entry?;

        // Skip directories.
        if entry.size() == 0 {
            continue;
        }

        let path = entry.path()?;
        // Strip the leading directory, which is the release name.
        let path: PathBuf = path.components().skip(1).collect();

        let dir = path.parent().unwrap();
        let dst_dir = prebuilt_dir.join(dir);
        let dst_path = prebuilt_dir.join(path);
        println!("unpacking to {}", dst_path.display());
        fs_err::create_dir_all(dst_dir)?;
        entry.unpack(dst_path)?;
    }

    Ok(())
}
