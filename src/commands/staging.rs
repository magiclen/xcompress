use std::{
    ffi::{OsStr, OsString},
    fs, io,
    os::unix::{ffi::OsStrExt, fs::symlink},
    path::{Path, PathBuf},
};

use anyhow::{Context, anyhow};
use tempfile::{Builder, TempDir};

use super::validation::file_name;

pub struct StagedInputs {
    _directory: TempDir,
    path:       PathBuf,
    file_names: Vec<OsString>,
}

impl StagedInputs {
    pub fn new(parent: &Path, input_paths: &[PathBuf]) -> anyhow::Result<Self> {
        let directory = Builder::new()
            .prefix(".xcompress-input-")
            .tempdir_in(parent)
            .with_context(|| anyhow!("{parent:?}"))?;
        let path = directory.path().to_owned();
        let mut file_names = Vec::with_capacity(input_paths.len());

        for input_path in input_paths {
            let file_name = file_name(input_path)?;
            symlink(input_path, path.join(file_name)).with_context(|| anyhow!("{input_path:?}"))?;
            file_names.push(file_name.to_owned());
        }

        Ok(Self {
            _directory: directory,
            path,
            file_names,
        })
    }

    #[inline]
    pub fn path(&self) -> &Path {
        self.path.as_path()
    }

    #[inline]
    pub fn file_names(&self) -> impl Iterator<Item = &OsStr> {
        self.file_names.iter().map(OsString::as_os_str)
    }
}

pub struct StagedFile {
    _directory:  TempDir,
    staged_path: PathBuf,
    final_path:  PathBuf,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum VolumeFormat {
    SevenZip,
    Zip,
    Rar,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
enum VolumeMember {
    Number(usize),
    Base,
}

pub struct StagedVolumeSet {
    directory:   Option<TempDir>,
    staged_path: PathBuf,
    final_path:  PathBuf,
    format:      VolumeFormat,
}

impl StagedVolumeSet {
    pub fn new(final_path: &Path, format: VolumeFormat) -> anyhow::Result<Self> {
        let parent = final_path.parent().ok_or_else(|| anyhow!("{final_path:?} has no parent."))?;
        let directory = Builder::new()
            .prefix(".xcompress-volumes-")
            .tempdir_in(parent)
            .with_context(|| anyhow!("{parent:?}"))?;
        let staged_path = directory.path().join(file_name(final_path)?);

        Ok(Self {
            directory: Some(directory),
            staged_path,
            final_path: final_path.to_owned(),
            format,
        })
    }

    #[inline]
    pub fn path(&self) -> &Path {
        self.staged_path.as_path()
    }

    pub fn auxiliary_path(&self, file_name: &str) -> anyhow::Result<PathBuf> {
        let directory = self.directory.as_ref().unwrap().path().join("auxiliary");
        fs::create_dir(directory.as_path()).with_context(|| anyhow!("{directory:?}"))?;

        Ok(directory.join(file_name))
    }

    pub fn publish(self) -> anyhow::Result<()> {
        let staged_members = self.staged_members()?;
        let final_parent = self.final_path.parent().unwrap();
        let existing_members =
            collect_volume_members(final_parent, file_name(&self.final_path)?, self.format, false)?;
        let backup_path = self.directory.as_ref().unwrap().path().join("backup");
        fs::create_dir(backup_path.as_path()).with_context(|| anyhow!("{backup_path:?}"))?;

        let mut backups = Vec::with_capacity(existing_members.len());

        for (_, final_member) in existing_members {
            let backup_member = backup_path.join(file_name(final_member.as_path())?);

            if let Err(error) = fs::rename(final_member.as_path(), backup_member.as_path()) {
                return self.fail_with_rollback(error, &[], &backups);
            }

            backups.push((backup_member, final_member));
        }

        let mut published = Vec::with_capacity(staged_members.len());

        for (_, staged_member) in staged_members {
            let final_member = final_parent.join(file_name(staged_member.as_path())?);

            if let Err(error) = fs::rename(staged_member.as_path(), final_member.as_path()) {
                return self.fail_with_rollback(error, &published, &backups);
            }

            published.push((final_member, staged_member));
        }

        Ok(())
    }

    fn staged_members(&self) -> anyhow::Result<Vec<(VolumeMember, PathBuf)>> {
        let directory = self.directory.as_ref().unwrap().path();
        let members =
            collect_volume_members(directory, file_name(&self.staged_path)?, self.format, true)?;
        let base_count =
            members.iter().filter(|(member, _)| matches!(member, VolumeMember::Base)).count();
        let numbers = members
            .iter()
            .filter_map(|(member, _)| match member {
                VolumeMember::Number(number) => Some(*number),
                VolumeMember::Base => None,
            })
            .collect::<Vec<_>>();
        let numbers_are_contiguous = numbers.iter().copied().eq(1..=numbers.len());
        let valid = match self.format {
            VolumeFormat::SevenZip => {
                base_count == 0 && !numbers.is_empty() && numbers_are_contiguous
            },
            VolumeFormat::Zip => base_count == 1 && numbers_are_contiguous,
            VolumeFormat::Rar => {
                (base_count == 1 && numbers.is_empty())
                    || (base_count == 0 && !numbers.is_empty() && numbers_are_contiguous)
            },
        };

        if !valid {
            return Err(anyhow!("The staged volume set at {directory:?} is incomplete."));
        }

        Ok(members)
    }

    fn fail_with_rollback(
        mut self,
        error: io::Error,
        published: &[(PathBuf, PathBuf)],
        backups: &[(PathBuf, PathBuf)],
    ) -> anyhow::Result<()> {
        if let Err(rollback_error) = rollback_volume_publication(published, backups) {
            let recovery_path = self.directory.take().unwrap().keep();

            return Err(anyhow!(
                "Failed to publish the volume set: {error}. Rollback also failed: \
                 {rollback_error}. Recovery files are at {recovery_path:?}."
            ));
        }

        Err(error.into())
    }
}

fn collect_volume_members(
    directory: &Path,
    base_name: &OsStr,
    format: VolumeFormat,
    require_regular_file: bool,
) -> anyhow::Result<Vec<(VolumeMember, PathBuf)>> {
    let mut members = Vec::new();

    for entry in fs::read_dir(directory).with_context(|| anyhow!("{directory:?}"))? {
        let entry = entry?;
        let file_type = entry.file_type()?;

        if (require_regular_file && !file_type.is_file()) || file_type.is_dir() {
            continue;
        }

        if let Some(member) = volume_member(entry.file_name().as_os_str(), base_name, format) {
            members.push((member, entry.path()));
        }
    }

    members.sort_by_key(|(member, _)| *member);

    Ok(members)
}

fn volume_member(
    file_name: &OsStr,
    base_name: &OsStr,
    format: VolumeFormat,
) -> Option<VolumeMember> {
    let file_name_bytes = file_name.as_bytes();
    let base_name_bytes = base_name.as_bytes();

    if file_name_bytes == base_name_bytes {
        return Some(VolumeMember::Base);
    }

    let number = match format {
        VolumeFormat::SevenZip => file_name_bytes
            .strip_prefix(base_name_bytes)?
            .strip_prefix(b".")
            .filter(|number| number.len() >= 3),
        VolumeFormat::Zip => {
            let stem = Path::new(base_name).file_stem()?.as_bytes();
            file_name_bytes
                .strip_prefix(stem)?
                .strip_prefix(b".z")
                .filter(|number| number.len() >= 2)
        },
        VolumeFormat::Rar => {
            let stem = Path::new(base_name).file_stem()?.as_bytes();
            strip_suffix_ignore_ascii_case(
                file_name_bytes.strip_prefix(stem)?.strip_prefix(b".part")?,
                b".rar",
            )
        },
    }?;

    parse_number(number).map(VolumeMember::Number)
}

fn strip_suffix_ignore_ascii_case<'a>(value: &'a [u8], suffix: &[u8]) -> Option<&'a [u8]> {
    let split = value.len().checked_sub(suffix.len())?;
    let (value, ending) = value.split_at(split);

    ending.eq_ignore_ascii_case(suffix).then_some(value)
}

fn parse_number(number: &[u8]) -> Option<usize> {
    if number.is_empty() || !number.iter().all(u8::is_ascii_digit) {
        return None;
    }

    number.iter().try_fold(0usize, |value, digit| {
        value.checked_mul(10)?.checked_add(usize::from(digit - b'0'))
    })
}

fn rollback_volume_publication(
    published: &[(PathBuf, PathBuf)],
    backups: &[(PathBuf, PathBuf)],
) -> io::Result<()> {
    let mut first_error = None;

    for (final_member, staged_member) in published.iter().rev() {
        if let Err(error) = fs::rename(final_member, staged_member)
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }

    for (backup_member, final_member) in backups.iter().rev() {
        if let Err(error) = fs::rename(backup_member, final_member)
            && first_error.is_none()
        {
            first_error = Some(error);
        }
    }

    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
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

pub fn publish_volumes_on_success(
    code: Option<i32>,
    staged_volumes: StagedVolumeSet,
) -> anyhow::Result<Option<i32>> {
    if code == Some(0) {
        staged_volumes.publish()?;
    }

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn staged_inputs_use_file_names() {
        let directory = tempfile::tempdir().unwrap();
        let first = directory.path().join("first/file-a");
        let second = directory.path().join("second/file-b");
        fs::create_dir_all(first.parent().unwrap()).unwrap();
        fs::create_dir_all(second.parent().unwrap()).unwrap();
        fs::write(first.as_path(), b"first").unwrap();
        fs::write(second.as_path(), b"second").unwrap();

        let staged_inputs =
            StagedInputs::new(directory.path(), &[first.clone(), second.clone()]).unwrap();

        assert_eq!(
            vec![OsStr::new("file-a"), OsStr::new("file-b")],
            staged_inputs.file_names().collect::<Vec<_>>()
        );
        assert_eq!(first, fs::read_link(staged_inputs.path().join("file-a")).unwrap());
        assert_eq!(second, fs::read_link(staged_inputs.path().join("file-b")).unwrap());
    }

    #[test]
    fn shorter_volume_set_replaces_longer_set() {
        let directory = tempfile::tempdir().unwrap();
        let final_path = directory.path().join("archive.7z");
        fs::write(directory.path().join("archive.7z.001"), b"old-1").unwrap();
        fs::write(directory.path().join("archive.7z.002"), b"old-2").unwrap();
        fs::write(directory.path().join("archive.7z.003"), b"old-3").unwrap();
        fs::write(directory.path().join("archive.7z.note"), b"unrelated").unwrap();

        let staged_volumes =
            StagedVolumeSet::new(final_path.as_path(), VolumeFormat::SevenZip).unwrap();
        fs::write(format!("{}.001", staged_volumes.path().display()), b"new-1").unwrap();
        fs::write(format!("{}.002", staged_volumes.path().display()), b"new-2").unwrap();
        staged_volumes.publish().unwrap();

        assert_eq!(b"new-1", fs::read(directory.path().join("archive.7z.001")).unwrap().as_slice());
        assert_eq!(b"new-2", fs::read(directory.path().join("archive.7z.002")).unwrap().as_slice());
        assert!(!directory.path().join("archive.7z.003").exists());
        assert_eq!(
            b"unrelated",
            fs::read(directory.path().join("archive.7z.note")).unwrap().as_slice()
        );
    }

    #[test]
    fn failed_volume_publication_restores_old_files() {
        let directory = tempfile::tempdir().unwrap();
        let final_path = directory.path().join("archive.7z");
        fs::write(directory.path().join("archive.7z.001"), b"old-1").unwrap();
        fs::create_dir(directory.path().join("archive.7z.002")).unwrap();

        let staged_volumes =
            StagedVolumeSet::new(final_path.as_path(), VolumeFormat::SevenZip).unwrap();
        fs::write(format!("{}.001", staged_volumes.path().display()), b"new-1").unwrap();
        fs::write(format!("{}.002", staged_volumes.path().display()), b"new-2").unwrap();

        assert!(staged_volumes.publish().is_err());
        assert_eq!(b"old-1", fs::read(directory.path().join("archive.7z.001")).unwrap().as_slice());
        assert!(directory.path().join("archive.7z.002").is_dir());
    }

    #[test]
    fn volume_names_are_exact() {
        assert_eq!(
            Some(VolumeMember::Number(1)),
            volume_member(
                OsStr::new("archive.7z.001"),
                OsStr::new("archive.7z"),
                VolumeFormat::SevenZip
            )
        );
        assert_eq!(
            Some(VolumeMember::Number(2)),
            volume_member(OsStr::new("archive.z02"), OsStr::new("archive.zip"), VolumeFormat::Zip)
        );
        assert_eq!(
            Some(VolumeMember::Number(3)),
            volume_member(
                OsStr::new("archive.part3.rar"),
                OsStr::new("archive.rar"),
                VolumeFormat::Rar
            )
        );
        assert_eq!(
            Some(VolumeMember::Number(4)),
            volume_member(
                OsStr::new("archive.part4.RAR"),
                OsStr::new("archive.RAR"),
                VolumeFormat::Rar
            )
        );
        assert_eq!(
            None,
            volume_member(
                OsStr::new("archive.7z.note"),
                OsStr::new("archive.7z"),
                VolumeFormat::SevenZip
            )
        );
    }

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
