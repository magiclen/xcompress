use std::{os::unix::ffi::OsStrExt, path::Path};

use anyhow::anyhow;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ArchiveFormat {
    Z,
    Zip,
    Gzip,
    Bzip2,
    Lz,
    Xz,
    Lzma,
    P7z,
    Tar,
    TarZ,
    TarGzip,
    TarBzip2,
    TarLz,
    TarXz,
    TarLzma,
    Tar7z,
    TarZstd,
    Rar,
    Zstd,
}

impl ArchiveFormat {
    pub fn from_file_path<P: AsRef<Path>>(file_path: P) -> anyhow::Result<Self> {
        let file_path = file_path.as_ref();

        if let Some(file_name) = file_path.file_name() {
            let file_name = file_name.as_bytes();
            let ends_with = |suffix: &[u8]| {
                file_name
                    .get(file_name.len().saturating_sub(suffix.len())..)
                    .is_some_and(|ending| ending.eq_ignore_ascii_case(suffix))
            };

            if ends_with(b".tar.z") || ends_with(b".taz") || ends_with(b".tz") {
                return Ok(ArchiveFormat::TarZ);
            } else if ends_with(b".tar.gz") || ends_with(b".tgz") {
                return Ok(ArchiveFormat::TarGzip);
            } else if ends_with(b".tar.bz2")
                || ends_with(b".tbz2")
                || ends_with(b".tbz")
                || ends_with(b".tb2")
            {
                return Ok(ArchiveFormat::TarBzip2);
            } else if ends_with(b".tar.lz") {
                return Ok(ArchiveFormat::TarLz);
            } else if ends_with(b".tar.xz") || ends_with(b".txz") {
                return Ok(ArchiveFormat::TarXz);
            } else if ends_with(b".tar.lzma") || ends_with(b".tlz") {
                return Ok(ArchiveFormat::TarLzma);
            } else if ends_with(b".tar.7z") || ends_with(b".tar.7z.001") || ends_with(b".t7z") {
                return Ok(ArchiveFormat::Tar7z);
            } else if ends_with(b".tar.zst") || ends_with(b".tzst") {
                return Ok(ArchiveFormat::TarZstd);
            } else if ends_with(b".tar") {
                return Ok(ArchiveFormat::Tar);
            } else if ends_with(b".z") {
                return Ok(ArchiveFormat::Z);
            } else if ends_with(b".zip") {
                return Ok(ArchiveFormat::Zip);
            } else if ends_with(b".gz") {
                return Ok(ArchiveFormat::Gzip);
            } else if ends_with(b".bz2") {
                return Ok(ArchiveFormat::Bzip2);
            } else if ends_with(b".lz") {
                return Ok(ArchiveFormat::Lz);
            } else if ends_with(b".xz") {
                return Ok(ArchiveFormat::Xz);
            } else if ends_with(b".lzma") {
                return Ok(ArchiveFormat::Lzma);
            } else if ends_with(b".7z") || ends_with(b".7z.001") {
                return Ok(ArchiveFormat::P7z);
            } else if ends_with(b".rar") {
                return Ok(ArchiveFormat::Rar);
            } else if ends_with(b".zst") {
                return Ok(ArchiveFormat::Zstd);
            }
        }

        Err(anyhow!("Unknown archive format."))
    }

    /// Whether the `password` option can be used with this format.
    #[inline]
    pub const fn supports_password(self) -> bool {
        matches!(self, Self::Tar7z | Self::P7z | Self::Zip | Self::Rar)
    }

    /// Whether the `split` option can be used with this format.
    #[inline]
    pub const fn supports_split(self) -> bool {
        matches!(self, Self::Tar7z | Self::P7z | Self::Zip | Self::Rar)
    }

    /// The extension of the tar-based format to suggest, if this format can only store a single file.
    #[inline]
    pub const fn tar_alternative_extension(self) -> Option<&'static str> {
        match self {
            Self::Z => Some(".tar.Z"),
            Self::Gzip => Some(".tar.gz"),
            Self::Bzip2 => Some(".tar.bz2"),
            Self::Lz => Some(".tar.lz"),
            Self::Xz => Some(".tar.xz"),
            Self::Lzma => Some(".tar.lzma"),
            Self::Zstd => Some(".tar.zst"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    use super::*;

    #[test]
    fn from_file_path() {
        assert_eq!(ArchiveFormat::TarZ, ArchiveFormat::from_file_path("foo.tar.Z").unwrap());
        assert_eq!(ArchiveFormat::TarZ, ArchiveFormat::from_file_path("foo.taz").unwrap());
        assert_eq!(ArchiveFormat::TarZ, ArchiveFormat::from_file_path("foo.tz").unwrap());
        assert_eq!(ArchiveFormat::TarGzip, ArchiveFormat::from_file_path("foo.tar.gz").unwrap());
        assert_eq!(ArchiveFormat::TarGzip, ArchiveFormat::from_file_path("foo.tgz").unwrap());
        assert_eq!(ArchiveFormat::TarBzip2, ArchiveFormat::from_file_path("foo.tar.bz2").unwrap());
        assert_eq!(ArchiveFormat::TarBzip2, ArchiveFormat::from_file_path("foo.tbz2").unwrap());
        assert_eq!(ArchiveFormat::TarBzip2, ArchiveFormat::from_file_path("foo.tbz").unwrap());
        assert_eq!(ArchiveFormat::TarBzip2, ArchiveFormat::from_file_path("foo.tb2").unwrap());
        assert_eq!(ArchiveFormat::TarLz, ArchiveFormat::from_file_path("foo.tar.lz").unwrap());
        assert_eq!(ArchiveFormat::TarXz, ArchiveFormat::from_file_path("foo.tar.xz").unwrap());
        assert_eq!(ArchiveFormat::TarLzma, ArchiveFormat::from_file_path("foo.tar.lzma").unwrap());
        assert_eq!(ArchiveFormat::Tar7z, ArchiveFormat::from_file_path("foo.tar.7z.001").unwrap());
        assert_eq!(ArchiveFormat::TarZstd, ArchiveFormat::from_file_path("foo.tar.zst").unwrap());
        assert_eq!(ArchiveFormat::TarZstd, ArchiveFormat::from_file_path("foo.tzst").unwrap());
        assert_eq!(ArchiveFormat::Tar, ArchiveFormat::from_file_path("foo.tar").unwrap());

        assert_eq!(ArchiveFormat::Z, ArchiveFormat::from_file_path("foo.Z").unwrap());
        assert_eq!(ArchiveFormat::Zip, ArchiveFormat::from_file_path("foo.zip").unwrap());
        assert_eq!(ArchiveFormat::Gzip, ArchiveFormat::from_file_path("foo.gz").unwrap());
        assert_eq!(ArchiveFormat::Bzip2, ArchiveFormat::from_file_path("foo.bz2").unwrap());
        assert_eq!(ArchiveFormat::Lz, ArchiveFormat::from_file_path("foo.lz").unwrap());
        assert_eq!(ArchiveFormat::Xz, ArchiveFormat::from_file_path("foo.xz").unwrap());
        assert_eq!(ArchiveFormat::Lzma, ArchiveFormat::from_file_path("foo.lzma").unwrap());
        assert_eq!(ArchiveFormat::P7z, ArchiveFormat::from_file_path("foo.7z").unwrap());
        assert_eq!(ArchiveFormat::P7z, ArchiveFormat::from_file_path("foo.7z.001").unwrap());
        assert_eq!(ArchiveFormat::Rar, ArchiveFormat::from_file_path("foo.rar").unwrap());
        assert_eq!(ArchiveFormat::Zstd, ArchiveFormat::from_file_path("foo.zst").unwrap());

        // the `tar` here is a part of the file name instead of an extension
        assert_eq!(ArchiveFormat::Z, ArchiveFormat::from_file_path("footar.z").unwrap());

        assert_eq!(ArchiveFormat::Rar, ArchiveFormat::from_file_path("/path/to/foo.rar").unwrap());

        assert!(ArchiveFormat::from_file_path("foo.unknown").is_err());
        assert!(ArchiveFormat::from_file_path("foo").is_err());
    }

    #[test]
    fn non_utf8_file_name() {
        let file_name = OsString::from_vec(b"archive-\xFF.tar.GZ".to_vec());

        assert_eq!(ArchiveFormat::TarGzip, ArchiveFormat::from_file_path(file_name).unwrap());
    }
}
