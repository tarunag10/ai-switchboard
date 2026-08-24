//! Stable regular-file handle shared by bounded npm payload inspectors.

use std::ffi::CString;
use std::fs::File;
use std::io::{ErrorKind, Read};

use sha2::{Digest, Sha256};

use super::codex_command_identity::{metadata_identity, MetadataIdentity};
use super::codex_npm_fs::{CodexNpmDirectory, CodexNpmFsError, CodexNpmRegularFileHash};

pub(super) struct CodexNpmStableRegularFile<'a> {
    directory: &'a CodexNpmDirectory,
    leaf: CString,
    file: File,
    identity: MetadataIdentity,
    executable: bool,
    byte_count: u64,
    max_bytes: u64,
}

pub(super) struct CodexNpmStableHashReader<'a> {
    stable: CodexNpmStableRegularFile<'a>,
    next_offset: u64,
    hasher: Sha256,
}

impl<'a> CodexNpmStableRegularFile<'a> {
    pub(super) fn new(
        directory: &'a CodexNpmDirectory,
        leaf: CString,
        file: File,
        identity: MetadataIdentity,
        executable: bool,
        byte_count: u64,
        max_bytes: u64,
    ) -> Self {
        Self {
            directory,
            leaf,
            file,
            identity,
            executable,
            byte_count,
            max_bytes,
        }
    }

    pub(super) fn byte_count(&self) -> u64 {
        self.byte_count
    }

    pub(super) fn hash_and_revalidate(self) -> Result<CodexNpmRegularFileHash, CodexNpmFsError> {
        self.into_hash_reader().finish()
    }

    pub(super) fn into_hash_reader(self) -> CodexNpmStableHashReader<'a> {
        CodexNpmStableHashReader {
            stable: self,
            next_offset: 0,
            hasher: Sha256::new(),
        }
    }
}

impl CodexNpmStableHashReader<'_> {
    pub(super) fn byte_count(&self) -> u64 {
        self.stable.byte_count()
    }

    pub(super) fn read_exact(&mut self, mut destination: &mut [u8]) -> Result<(), CodexNpmFsError> {
        while !destination.is_empty() {
            let read = match self.stable.file.read(destination) {
                Ok(0) => return Err(CodexNpmFsError::FileChanged),
                Ok(read) => read,
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(_) => return Err(CodexNpmFsError::FileReadFailed),
            };
            self.hasher.update(&destination[..read]);
            self.next_offset = self
                .next_offset
                .checked_add(read as u64)
                .filter(|offset| *offset <= self.stable.byte_count)
                .ok_or(CodexNpmFsError::FileChanged)?;
            destination = &mut destination[read..];
        }
        Ok(())
    }

    pub(super) fn hash_until(&mut self, end_offset: u64) -> Result<(), CodexNpmFsError> {
        if end_offset < self.next_offset || end_offset > self.stable.byte_count {
            return Err(CodexNpmFsError::FileChanged);
        }
        let mut buffer = [0u8; 64 * 1024];
        while self.next_offset < end_offset {
            let remaining = (end_offset - self.next_offset) as usize;
            let read = remaining.min(buffer.len());
            self.read_exact(&mut buffer[..read])?;
        }
        Ok(())
    }

    pub(super) fn finish(mut self) -> Result<CodexNpmRegularFileHash, CodexNpmFsError> {
        self.hash_until(self.stable.byte_count)?;
        let after = self
            .stable
            .file
            .metadata()
            .map_err(|_| CodexNpmFsError::FileMetadataFailed)?;
        if metadata_identity(&after) != self.stable.identity {
            return Err(CodexNpmFsError::FileChanged);
        }
        self.stable.directory.revalidate_regular_leaf(
            &self.stable.leaf,
            &self.stable.identity,
            self.stable.max_bytes,
        )?;
        Ok(CodexNpmRegularFileHash {
            digest: self.hasher.finalize().into(),
            identity: self.stable.identity,
            executable: self.stable.executable,
        })
    }
}
