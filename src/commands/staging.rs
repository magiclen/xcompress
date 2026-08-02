use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, anyhow};
use tempfile::{Builder, TempDir};

use super::validation::file_name;

pub struct StagedFile {
    _directory:  TempDir,
    staged_path: PathBuf,
    final_path:  PathBuf,
}

impl StagedFile {
    pub fn new(final_path: &Path) -> anyhow::Result<Self> {
        let parent = final_path.parent().ok_or_else(|| anyhow!("{final_path:?} has no parent."))?;
        let directory = Builder::new()
            .prefix(".xcompress-")
            .tempdir_in(parent)
            .with_context(|| anyhow!("{parent:?}"))?;
        let staged_path = directory.path().join(file_name(final_path)?);

        Ok(Self {
            _directory: directory,
            staged_path,
            final_path: final_path.to_owned(),
        })
    }

    #[inline]
    pub fn path(&self) -> &Path {
        self.staged_path.as_path()
    }

    pub fn publish(self) -> anyhow::Result<()> {
        if !self.staged_path.is_file() {
            return Err(anyhow!("The staged output {:?} is not a file.", self.staged_path));
        }

        fs::rename(self.staged_path.as_path(), self.final_path.as_path())
            .with_context(|| anyhow!("Failed to publish {:?}.", self.final_path))
    }

    #[cfg(test)]
    fn directory(&self) -> &Path {
        self._directory.path()
    }
}

pub fn publish_on_success(
    code: Option<i32>,
    staged_file: StagedFile,
) -> anyhow::Result<Option<i32>> {
    if code == Some(0) {
        staged_file.publish()?;
    }

    Ok(code)
}

pub fn publish_optional(
    code: Option<i32>,
    staged_file: Option<StagedFile>,
) -> anyhow::Result<Option<i32>> {
    match staged_file {
        Some(staged_file) => publish_on_success(code, staged_file),
        None => Ok(code),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_replaces_existing_file() {
        let directory = tempfile::tempdir().unwrap();
        let final_path = directory.path().join("archive.tar.gz");
        fs::write(final_path.as_path(), b"old").unwrap();

        let staged_file = StagedFile::new(final_path.as_path()).unwrap();
        fs::write(staged_file.path(), b"new").unwrap();

        assert_eq!(Some(0), publish_on_success(Some(0), staged_file).unwrap());
        assert_eq!(b"new", fs::read(final_path).unwrap().as_slice());
    }

    #[test]
    fn failed_output_is_discarded() {
        let directory = tempfile::tempdir().unwrap();
        let final_path = directory.path().join("archive.xz");
        fs::write(final_path.as_path(), b"old").unwrap();

        let staged_file = StagedFile::new(final_path.as_path()).unwrap();
        let staging_directory = staged_file.directory().to_owned();
        fs::write(staged_file.path(), b"partial").unwrap();

        assert_eq!(Some(3), publish_on_success(Some(3), staged_file).unwrap());
        assert_eq!(b"old", fs::read(final_path).unwrap().as_slice());
        assert!(!staging_directory.exists());
    }
}
