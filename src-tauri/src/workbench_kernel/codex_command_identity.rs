//! Bounded identity primitives for the native fixed-location Codex collector.

use std::fs::{File, Metadata, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::ffi::CStr;

use sha2::{Digest, Sha256};

const HASH_BUFFER_BYTES: usize = 64 * 1024;
const INITIAL_ACCOUNT_BUFFER_BYTES: usize = 16 * 1024;
const MAX_ACCOUNT_BUFFER_BYTES: usize = 1024 * 1024;

pub(super) enum HashError {
    GrewPastLimit,
    ReadFailed,
}

pub(super) fn hash_bounded_file(
    mut file: File,
    max_bytes: u64,
) -> Result<([u8; 32], Metadata), HashError> {
    let mut hasher = Sha256::new();
    let mut total = 0u64;
    let mut buffer = [0u8; HASH_BUFFER_BYTES];
    loop {
        let read = file.read(&mut buffer).map_err(|_| HashError::ReadFailed)?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .ok_or(HashError::GrewPastLimit)?;
        if total > max_bytes {
            return Err(HashError::GrewPastLimit);
        }
        hasher.update(&buffer[..read]);
    }
    let metadata = file.metadata().map_err(|_| HashError::ReadFailed)?;
    Ok((hasher.finalize().into(), metadata))
}

pub(super) fn identity_digest(
    candidate_id: &str,
    leaf: &MetadataIdentity,
    target: &MetadataIdentity,
    content_digest: [u8; 32],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"ai-switchboard-codex-binary-identity-v1\0");
    hasher.update((candidate_id.len() as u64).to_be_bytes());
    hasher.update(candidate_id.as_bytes());
    update_identity_digest(&mut hasher, leaf);
    update_identity_digest(&mut hasher, target);
    hasher.update(content_digest);
    format!("sha256:{:x}", hasher.finalize())
}

fn update_identity_digest(hasher: &mut Sha256, identity: &MetadataIdentity) {
    for value in [
        identity.device,
        identity.inode,
        u64::from(identity.mode),
        u64::from(identity.user_id),
        u64::from(identity.group_id),
        identity.size,
        identity.modified_seconds as u64,
        identity.modified_nanoseconds as u64,
        identity.changed_seconds as u64,
        identity.changed_nanoseconds as u64,
    ] {
        hasher.update(value.to_be_bytes());
    }
}

#[cfg(unix)]
pub(super) fn open_without_following(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)
}

#[cfg(not(unix))]
pub(super) fn open_without_following(path: &Path) -> std::io::Result<File> {
    OpenOptions::new().read(true).open(path)
}

#[cfg(unix)]
pub(super) fn metadata_is_executable(metadata: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    metadata.mode() & 0o111 != 0
}

#[cfg(not(unix))]
pub(super) fn metadata_is_executable(_metadata: &Metadata) -> bool {
    false
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MetadataIdentity {
    device: u64,
    inode: u64,
    mode: u32,
    user_id: u32,
    group_id: u32,
    size: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(unix)]
pub(super) fn metadata_identity(metadata: &Metadata) -> MetadataIdentity {
    use std::os::unix::fs::MetadataExt;

    MetadataIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        mode: metadata.mode(),
        user_id: metadata.uid(),
        group_id: metadata.gid(),
        size: metadata.size(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    }
}

#[cfg(not(unix))]
pub(super) fn metadata_identity(metadata: &Metadata) -> MetadataIdentity {
    use std::time::UNIX_EPOCH;

    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok());
    MetadataIdentity {
        device: 0,
        inode: 0,
        mode: 0,
        user_id: 0,
        group_id: 0,
        size: metadata.len(),
        modified_seconds: modified.map(|value| value.as_secs() as i64).unwrap_or(-1),
        modified_nanoseconds: modified
            .map(|value| value.subsec_nanos() as i64)
            .unwrap_or(-1),
        changed_seconds: -1,
        changed_nanoseconds: -1,
    }
}

#[cfg(unix)]
pub(super) fn account_home_directory() -> Result<PathBuf, ()> {
    use std::os::unix::ffi::OsStringExt;

    let mut capacity = INITIAL_ACCOUNT_BUFFER_BYTES;
    loop {
        let mut record = unsafe { std::mem::zeroed::<libc::passwd>() };
        let mut buffer = vec![0u8; capacity];
        let mut result = std::ptr::null_mut();
        let status = unsafe {
            libc::getpwuid_r(
                libc::getuid(),
                &mut record,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                &mut result,
            )
        };
        if status == libc::ERANGE && capacity < MAX_ACCOUNT_BUFFER_BYTES {
            capacity = (capacity * 2).min(MAX_ACCOUNT_BUFFER_BYTES);
            continue;
        }
        if status != 0 || result.is_null() || record.pw_dir.is_null() {
            return Err(());
        }
        let bytes = unsafe { CStr::from_ptr(record.pw_dir) }.to_bytes();
        if bytes.is_empty() {
            return Err(());
        }
        let home = PathBuf::from(std::ffi::OsString::from_vec(bytes.to_vec()));
        return home.is_absolute().then_some(home).ok_or(());
    }
}

#[cfg(not(unix))]
pub(super) fn account_home_directory() -> Result<PathBuf, ()> {
    Err(())
}
