//! Descriptor-relative, read-only primitives for inspecting a Codex npm tree.

use std::ffi::{CString, OsStr, OsString};
use std::fmt;
use std::fs::{File, Metadata, OpenOptions};
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

use super::codex_command_identity::{
    evidence_identity_digest, metadata_identity, metadata_identity_from_stat,
    metadata_is_executable, MetadataIdentity,
};
use super::codex_npm_fs_stable::CodexNpmStableRegularFile;

const READ_BUFFER_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodexNpmFsError {
    RootNotAbsolute,
    InvalidComponent,
    RootOpenFailed,
    RootMetadataFailed,
    DirectoryOpenFailed,
    DirectoryMetadataFailed,
    OwnershipRejected,
    PermissionsRejected,
    CrossDeviceRejected,
    RevalidationFailed,
    LinkReadFailed,
    LinkTargetTooLong,
    FileOpenFailed,
    FileMetadataFailed,
    FileNotRegular,
    FileTooLarge,
    FileReadFailed,
    FileChanged,
    CapacityUnavailable,
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct CodexNpmRegularFile {
    pub bytes: Vec<u8>,
    pub content_digest: [u8; 32],
    pub identity: MetadataIdentity,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexNpmSymlink {
    pub target: OsString,
    pub identity: MetadataIdentity,
}

impl fmt::Debug for CodexNpmRegularFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CodexNpmRegularFile")
            .field("byte_count", &self.bytes.len())
            .field("content_digest", &self.content_digest)
            .field("identity", &self.identity)
            .field("executable", &self.executable)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CodexNpmRegularFileHash {
    pub digest: [u8; 32],
    pub identity: MetadataIdentity,
    pub executable: bool,
}

pub(super) struct CodexNpmDirectory {
    root_path: PathBuf,
    components: Vec<OsString>,
    identities: Vec<MetadataIdentity>,
    directory: File,
    expected_user_id: u32,
    expected_device: u64,
}

impl CodexNpmDirectory {
    pub(super) fn open(root: &Path, components: &[&OsStr]) -> Result<Self, CodexNpmFsError> {
        if !root.is_absolute() {
            return Err(CodexNpmFsError::RootNotAbsolute);
        }
        let validated = components
            .iter()
            .map(|component| validated_component(component).map(|_| OsString::from(*component)))
            .collect::<Result<Vec<_>, _>>()?;

        let expected_user_id = unsafe { libc::getuid() };
        let mut directory = open_root(root)?;
        let root_metadata = directory
            .metadata()
            .map_err(|_| CodexNpmFsError::RootMetadataFailed)?;
        let expected_device = root_metadata.dev();
        require_safe_metadata(&root_metadata, expected_user_id, expected_device)?;
        let mut identities = vec![metadata_identity(&root_metadata)];
        for component in &validated {
            directory = open_directory_at(directory.as_raw_fd(), component)?;
            identities.push(directory_identity_with_policy(
                &directory,
                CodexNpmFsError::DirectoryMetadataFailed,
                expected_user_id,
                expected_device,
            )?);
        }

        Ok(Self {
            root_path: root.to_path_buf(),
            components: validated,
            identities,
            directory,
            expected_user_id,
            expected_device,
        })
    }

    pub(super) fn identity_digest(&self, role: &str) -> String {
        let identities = self.identities.iter().collect::<Vec<_>>();
        let mut values = Vec::with_capacity(self.components.len() + 1);
        values.push(role.as_bytes());
        values.extend(self.components.iter().map(|value| value.as_bytes()));
        evidence_identity_digest(
            b"ai-switchboard-codex-npm-directory-chain-v1\0",
            &identities,
            &values,
        )
    }

    pub(super) fn revalidate(&self) -> Result<(), CodexNpmFsError> {
        self.rewalk().map(drop)
    }

    pub(super) fn read_link(
        &self,
        leaf: &OsStr,
        max_bytes: usize,
    ) -> Result<CodexNpmSymlink, CodexNpmFsError> {
        let leaf = validated_component(leaf)?;
        self.revalidate()?;
        let identity = symlink_identity_at(self.directory.as_raw_fd(), &leaf)?;
        let target = read_link_at(self.directory.as_raw_fd(), &leaf, max_bytes)?;
        if symlink_identity_at(self.directory.as_raw_fd(), &leaf)? != identity {
            return Err(CodexNpmFsError::RevalidationFailed);
        }
        let revalidated = self.rewalk()?;
        if symlink_identity_at(revalidated.as_raw_fd(), &leaf)
            .map_err(|_| CodexNpmFsError::RevalidationFailed)?
            != identity
        {
            return Err(CodexNpmFsError::RevalidationFailed);
        }
        let final_target = read_link_at(revalidated.as_raw_fd(), &leaf, max_bytes)
            .map_err(|_| CodexNpmFsError::RevalidationFailed)?;
        if target != final_target {
            return Err(CodexNpmFsError::RevalidationFailed);
        }
        Ok(CodexNpmSymlink { target, identity })
    }

    pub(super) fn read_regular_file(
        &self,
        leaf: &OsStr,
        max_bytes: u64,
    ) -> Result<CodexNpmRegularFile, CodexNpmFsError> {
        let leaf = validated_component(leaf)?;
        self.revalidate()?;
        let mut file = open_regular_candidate_at(self.directory.as_raw_fd(), &leaf)?;
        let before = self.regular_file_metadata(&file, max_bytes)?;
        let identity = metadata_identity(&before);
        let executable = metadata_is_executable(&before);
        let mut bytes = Vec::new();
        let mut hasher = Sha256::new();
        let mut total = 0u64;
        let mut buffer = [0u8; READ_BUFFER_BYTES];
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|_| CodexNpmFsError::FileReadFailed)?;
            if read == 0 {
                break;
            }
            total = total
                .checked_add(read as u64)
                .filter(|total| *total <= max_bytes)
                .ok_or(CodexNpmFsError::FileTooLarge)?;
            bytes
                .try_reserve(read)
                .map_err(|_| CodexNpmFsError::CapacityUnavailable)?;
            bytes.extend_from_slice(&buffer[..read]);
            hasher.update(&buffer[..read]);
        }
        require_unchanged_metadata(&file, &identity)?;
        self.revalidate_regular_leaf(&leaf, &identity, max_bytes)?;
        Ok(CodexNpmRegularFile {
            bytes,
            content_digest: hasher.finalize().into(),
            identity,
            executable,
        })
    }

    pub(super) fn hash_regular_file(
        &self,
        leaf: &OsStr,
        max_bytes: u64,
    ) -> Result<CodexNpmRegularFileHash, CodexNpmFsError> {
        self.open_stable_regular_file(leaf, max_bytes)?
            .hash_and_revalidate()
    }

    pub(super) fn open_stable_regular_file(
        &self,
        leaf: &OsStr,
        max_bytes: u64,
    ) -> Result<CodexNpmStableRegularFile<'_>, CodexNpmFsError> {
        let leaf = validated_component(leaf)?;
        self.revalidate()?;
        let file = open_regular_candidate_at(self.directory.as_raw_fd(), &leaf)?;
        let before = self.regular_file_metadata(&file, max_bytes)?;
        let identity = metadata_identity(&before);
        let executable = metadata_is_executable(&before);
        Ok(CodexNpmStableRegularFile::new(
            self,
            leaf,
            file,
            identity,
            executable,
            before.len(),
            max_bytes,
        ))
    }

    pub(super) fn revalidate_regular_file(
        &self,
        leaf: &OsStr,
        expected: &MetadataIdentity,
        max_bytes: u64,
    ) -> Result<(), CodexNpmFsError> {
        let leaf = validated_component(leaf)?;
        self.revalidate_regular_leaf(&leaf, expected, max_bytes)
    }

    fn rewalk(&self) -> Result<File, CodexNpmFsError> {
        let mut directory =
            open_root(&self.root_path).map_err(|_| CodexNpmFsError::RevalidationFailed)?;
        if directory_identity_with_policy(
            &directory,
            CodexNpmFsError::RevalidationFailed,
            self.expected_user_id,
            self.expected_device,
        )? != self.identities[0]
        {
            return Err(CodexNpmFsError::RevalidationFailed);
        }
        for (index, component) in self.components.iter().enumerate() {
            directory = open_directory_at(directory.as_raw_fd(), component)
                .map_err(|_| CodexNpmFsError::RevalidationFailed)?;
            if directory_identity_with_policy(
                &directory,
                CodexNpmFsError::RevalidationFailed,
                self.expected_user_id,
                self.expected_device,
            )? != self.identities[index + 1]
            {
                return Err(CodexNpmFsError::RevalidationFailed);
            }
        }
        Ok(directory)
    }

    pub(super) fn revalidate_regular_leaf(
        &self,
        leaf: &CString,
        expected: &MetadataIdentity,
        max_bytes: u64,
    ) -> Result<(), CodexNpmFsError> {
        let directory = self.rewalk()?;
        let file = open_regular_candidate_at(directory.as_raw_fd(), leaf)
            .map_err(|_| CodexNpmFsError::RevalidationFailed)?;
        let metadata = self
            .regular_file_metadata(&file, max_bytes)
            .map_err(|_| CodexNpmFsError::RevalidationFailed)?;
        if metadata_identity(&metadata) != *expected {
            return Err(CodexNpmFsError::FileChanged);
        }
        Ok(())
    }

    fn regular_file_metadata(
        &self,
        file: &File,
        max_bytes: u64,
    ) -> Result<Metadata, CodexNpmFsError> {
        let metadata = file
            .metadata()
            .map_err(|_| CodexNpmFsError::FileMetadataFailed)?;
        if !metadata.is_file() {
            return Err(CodexNpmFsError::FileNotRegular);
        }
        require_safe_metadata(&metadata, self.expected_user_id, self.expected_device)?;
        if metadata.len() > max_bytes {
            return Err(CodexNpmFsError::FileTooLarge);
        }
        Ok(metadata)
    }
}

fn validated_component(component: &OsStr) -> Result<CString, CodexNpmFsError> {
    if component.is_empty() {
        return Err(CodexNpmFsError::InvalidComponent);
    }
    let mut parts = Path::new(component).components();
    match (parts.next(), parts.next()) {
        (Some(Component::Normal(normal)), None) if normal == component => {}
        _ => return Err(CodexNpmFsError::InvalidComponent),
    }
    CString::new(component.as_bytes()).map_err(|_| CodexNpmFsError::InvalidComponent)
}

fn open_root(root: &Path) -> Result<File, CodexNpmFsError> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(root)
        .map_err(|_| CodexNpmFsError::RootOpenFailed)
}

fn open_directory_at(parent: RawFd, component: &OsStr) -> Result<File, CodexNpmFsError> {
    let component = validated_component(component)?;
    let descriptor = unsafe {
        libc::openat(
            parent,
            component.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(CodexNpmFsError::DirectoryOpenFailed);
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

fn open_regular_candidate_at(parent: RawFd, leaf: &CString) -> Result<File, CodexNpmFsError> {
    let descriptor = unsafe {
        libc::openat(
            parent,
            leaf.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(CodexNpmFsError::FileOpenFailed);
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

fn directory_identity_with_policy(
    directory: &File,
    error: CodexNpmFsError,
    expected_user_id: u32,
    expected_device: u64,
) -> Result<MetadataIdentity, CodexNpmFsError> {
    let metadata = directory.metadata().map_err(|_| error)?;
    if !metadata.is_dir() {
        return Err(error);
    }
    require_safe_metadata(&metadata, expected_user_id, expected_device)?;
    Ok(metadata_identity(&metadata))
}

fn require_safe_metadata(
    metadata: &Metadata,
    expected_user_id: u32,
    expected_device: u64,
) -> Result<(), CodexNpmFsError> {
    if metadata.uid() != expected_user_id && metadata.uid() != 0 {
        return Err(CodexNpmFsError::OwnershipRejected);
    }
    if metadata.mode() & 0o6022 != 0 {
        return Err(CodexNpmFsError::PermissionsRejected);
    }
    if metadata.dev() != expected_device {
        return Err(CodexNpmFsError::CrossDeviceRejected);
    }
    Ok(())
}

fn require_unchanged_metadata(
    file: &File,
    expected: &MetadataIdentity,
) -> Result<(), CodexNpmFsError> {
    let after = file
        .metadata()
        .map_err(|_| CodexNpmFsError::FileMetadataFailed)?;
    if metadata_identity(&after) != *expected {
        return Err(CodexNpmFsError::FileChanged);
    }
    Ok(())
}

fn read_link_at(
    parent: RawFd,
    leaf: &CString,
    max_bytes: usize,
) -> Result<OsString, CodexNpmFsError> {
    let capacity = max_bytes
        .checked_add(1)
        .ok_or(CodexNpmFsError::CapacityUnavailable)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| CodexNpmFsError::CapacityUnavailable)?;
    bytes.resize(capacity, 0);
    let read = unsafe {
        libc::readlinkat(
            parent,
            leaf.as_ptr(),
            bytes.as_mut_ptr().cast(),
            bytes.len(),
        )
    };
    if read < 0 {
        return Err(CodexNpmFsError::LinkReadFailed);
    }
    let read = read as usize;
    if read > max_bytes {
        return Err(CodexNpmFsError::LinkTargetTooLong);
    }
    bytes.truncate(read);
    Ok(OsString::from_vec(bytes))
}

fn symlink_identity_at(parent: RawFd, leaf: &CString) -> Result<MetadataIdentity, CodexNpmFsError> {
    let mut value = unsafe { std::mem::zeroed::<libc::stat>() };
    let status =
        unsafe { libc::fstatat(parent, leaf.as_ptr(), &mut value, libc::AT_SYMLINK_NOFOLLOW) };
    if status != 0 || value.st_mode & libc::S_IFMT != libc::S_IFLNK {
        return Err(CodexNpmFsError::LinkReadFailed);
    }
    Ok(metadata_identity_from_stat(&value))
}
