use std::path::Path;

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

        if let Some(file_name) = file_path.file_name()
            && let Some(file_name) = file_name.to_str()
        {
            let file_name = file_name.to_ascii_lowercase();

            if file_name.ends_with(".tar.z") {
                return Ok(ArchiveFormat::TarZ);
            } else if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
                return Ok(ArchiveFormat::TarGzip);
            } else if file_name.ends_with(".tar.bz2") || file_name.ends_with(".tbz2") {
                return Ok(ArchiveFormat::TarBzip2);
            } else if file_name.ends_with(".tar.lz") {
                return Ok(ArchiveFormat::TarLz);
            } else if file_name.ends_with(".tar.xz") || file_name.ends_with(".txz") {
                return Ok(ArchiveFormat::TarXz);
            } else if file_name.ends_with(".tar.lzma") || file_name.ends_with(".tlz") {
                return Ok(ArchiveFormat::TarLzma);
            } else if file_name.ends_with(".tar.7z")
                || file_name.ends_with(".tar.7z.001")
                || file_name.ends_with(".t7z")
            {
                return Ok(ArchiveFormat::Tar7z);
            } else if file_name.ends_with(".tar.zst") {
                return Ok(ArchiveFormat::TarZstd);
            } else if file_name.ends_with(".tar") {
                return Ok(ArchiveFormat::Tar);
            } else if file_name.ends_with(".z") {
                return Ok(ArchiveFormat::Z);
            } else if file_name.ends_with(".zip") {
                return Ok(ArchiveFormat::Zip);
            } else if file_name.ends_with(".gz") {
                return Ok(ArchiveFormat::Gzip);
            } else if file_name.ends_with(".bz2") {
                return Ok(ArchiveFormat::Bzip2);
            } else if file_name.ends_with(".lz") {
                return Ok(ArchiveFormat::Lz);
            } else if file_name.ends_with(".xz") {
                return Ok(ArchiveFormat::Xz);
            } else if file_name.ends_with(".lzma") {
                return Ok(ArchiveFormat::Lzma);
            } else if file_name.ends_with(".7z") || file_name.ends_with(".7z.001") {
                return Ok(ArchiveFormat::P7z);
            } else if file_name.ends_with(".rar") {
                return Ok(ArchiveFormat::Rar);
            } else if file_name.ends_with(".zst") {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_file_path() {
        assert_eq!(ArchiveFormat::TarZ, ArchiveFormat::from_file_path("foo.tar.Z").unwrap());
        assert_eq!(ArchiveFormat::TarGzip, ArchiveFormat::from_file_path("foo.tar.gz").unwrap());
        assert_eq!(ArchiveFormat::TarGzip, ArchiveFormat::from_file_path("foo.tgz").unwrap());
        assert_eq!(ArchiveFormat::TarBzip2, ArchiveFormat::from_file_path("foo.tar.bz2").unwrap());
        assert_eq!(ArchiveFormat::TarLz, ArchiveFormat::from_file_path("foo.tar.lz").unwrap());
        assert_eq!(ArchiveFormat::TarXz, ArchiveFormat::from_file_path("foo.tar.xz").unwrap());
        assert_eq!(ArchiveFormat::TarLzma, ArchiveFormat::from_file_path("foo.tar.lzma").unwrap());
        assert_eq!(ArchiveFormat::Tar7z, ArchiveFormat::from_file_path("foo.tar.7z.001").unwrap());
        assert_eq!(ArchiveFormat::TarZstd, ArchiveFormat::from_file_path("foo.tar.zst").unwrap());
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
}
