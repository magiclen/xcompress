//! # XCompress
//! XCompress is a free file archiver utility on Linux, providing multi-format archiving to and extracting from ZIP, Z, GZIP, BZIP2, LZ, XZ, LZMA, 7ZIP, TAR, RAR ans ZSTD.

use std::path::Path;

#[derive(Debug)]
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
    pub fn get_archive_format_from_file_path<P: AsRef<Path>>(
        file_path: P,
    ) -> Result<ArchiveFormat, &'static str> {
        let file_path = file_path.as_ref();

        if let Some(file_name) = file_path.file_name() {
            if let Some(file_name) = file_name.to_str() {
                let file_name = file_name.to_ascii_lowercase();

                if file_name.ends_with("tar.z") {
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
        }

        Err("Unknown archive format.")
    }
}
