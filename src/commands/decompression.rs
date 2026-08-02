use std::{
    ffi::{OsStr, OsString},
    fs,
    fs::File,
    io,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, anyhow};
use execute::{Execute, command_args};
use path_absolutize::{Absolutize, CWD};

use super::{
    cpu_count, execute_pipeline, read_password,
    staging::{StagedFile, publish_on_success},
    validation::decompressed_file_path,
};
use crate::{
    archive_format::ArchiveFormat,
    cli::{CLIArgs, CLICommands, ExecutablePaths},
};

struct Decompressor<'a> {
    executable_paths: &'a ExecutablePaths,
    threads:          &'a str,
    cpus:             usize,
}

impl Decompressor<'_> {
    fn command(&self, format: ArchiveFormat, input: &OsStr) -> Command {
        match format {
            ArchiveFormat::Z
            | ArchiveFormat::Gzip
            | ArchiveFormat::TarZ
            | ArchiveFormat::TarGzip => self.gzip(input),
            ArchiveFormat::Bzip2 | ArchiveFormat::TarBzip2 => self.bzip2(input),
            ArchiveFormat::Lz | ArchiveFormat::TarLz => self.lzip(input),
            ArchiveFormat::Xz | ArchiveFormat::TarXz => self.xz(input),
            ArchiveFormat::Lzma | ArchiveFormat::TarLzma => self.lzma(input),
            ArchiveFormat::Zstd | ArchiveFormat::TarZstd => self.zstd(input),
            _ => unreachable!(),
        }
    }

    fn gzip(&self, input: &OsStr) -> Command {
        if self.cpus > 1
            && command_args!(&self.executable_paths.pigz_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
        {
            return command_args!(
                &self.executable_paths.pigz_path,
                "-d",
                "-c",
                "-p",
                self.threads,
                input
            );
        }

        if command_args!(&self.executable_paths.gunzip_path, "-V")
            .execute_check_exit_status_code(0)
            .is_ok()
        {
            return command_args!(&self.executable_paths.gunzip_path, "-c", input);
        }

        command_args!(&self.executable_paths.gzip_path, "-d", "-c", input)
    }

    fn bzip2(&self, input: &OsStr) -> Command {
        if self.cpus > 1 {
            if command_args!(&self.executable_paths.lbzip2_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
            {
                return command_args!(
                    &self.executable_paths.lbzip2_path,
                    "-d",
                    "-c",
                    "-n",
                    self.threads,
                    input
                );
            }

            if command_args!(&self.executable_paths.pbzip2_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
            {
                return command_args!(
                    &self.executable_paths.pbzip2_path,
                    "-d",
                    "-c",
                    format!("-p{}", self.threads),
                    input
                );
            }
        }

        if command_args!(&self.executable_paths.bunzip2_path, "-V")
            .execute_check_exit_status_code(0)
            .is_ok()
        {
            return command_args!(&self.executable_paths.bunzip2_path, "-c", input);
        }

        command_args!(&self.executable_paths.bzip2_path, "-d", "-c", input)
    }

    fn lzip(&self, input: &OsStr) -> Command {
        if self.cpus > 1
            && command_args!(&self.executable_paths.plzip_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
        {
            return command_args!(
                &self.executable_paths.plzip_path,
                "-d",
                "-c",
                "-n",
                self.threads,
                input
            );
        }

        if command_args!(&self.executable_paths.lunzip_path, "-V")
            .execute_check_exit_status_code(0)
            .is_ok()
        {
            return command_args!(&self.executable_paths.lunzip_path, "-c", input);
        }

        command_args!(&self.executable_paths.lzip_path, "-d", "-c", input)
    }

    fn xz(&self, input: &OsStr) -> Command {
        if command_args!(&self.executable_paths.unxz_path, "-V")
            .execute_check_exit_status_code(0)
            .is_ok()
        {
            return command_args!(
                &self.executable_paths.unxz_path,
                "-d",
                "-c",
                "-T",
                self.threads,
                input
            );
        }

        command_args!(&self.executable_paths.xz_path, "-d", "-c", "-T", self.threads, input)
    }

    fn lzma(&self, input: &OsStr) -> Command {
        if command_args!(&self.executable_paths.unlzma_path, "-V")
            .execute_check_exit_status_code(0)
            .is_ok()
        {
            return command_args!(
                &self.executable_paths.unlzma_path,
                "-d",
                "-c",
                "-T",
                self.threads,
                input
            );
        }

        command_args!(&self.executable_paths.lzma_path, "-d", "-c", "-T", self.threads, input)
    }

    fn zstd(&self, input: &OsStr) -> Command {
        if self.cpus > 1
            && command_args!(&self.executable_paths.pzstd_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
        {
            return command_args!(
                &self.executable_paths.pzstd_path,
                "-d",
                "-c",
                "-p",
                self.threads,
                input
            );
        }

        if command_args!(&self.executable_paths.unzstd_path, "-V")
            .execute_check_exit_status_code(0)
            .is_ok()
        {
            return command_args!(&self.executable_paths.unzstd_path, "-c", input);
        }

        command_args!(&self.executable_paths.zstd_path, "-d", "-c", input)
    }
}

/// Build the 7z command which extracts `input_path` into `output_path`.
///
/// The password is always passed, even when it is empty, so that an encrypted archive fails instead of waiting for a prompt.
fn p7z_extract_command(
    executable_path: &str,
    threads: &str,
    password: &str,
    input_path: &Path,
    output_path: &Path,
) -> Command {
    // the output path is not necessarily valid UTF-8, so it must not go through a formatted string
    let mut output_argument = OsString::from("-o");

    output_argument.push(output_path);

    let mut command =
        command_args!(executable_path, "x", "-aoa", format!("-mmt{threads}"), output_argument);

    command.arg(format!("-p{password}"));

    command.arg(input_path);

    command
}

pub fn handle_decompression(cli_args: CLIArgs) -> anyhow::Result<Option<i32>> {
    let CLICommands::X {
        input_path,
        output_path,
        output,
    } = cli_args.command
    else {
        unreachable!();
    };

    let input_path = match input_path.canonicalize() {
        Ok(input_path) => input_path,
        Err(error) => {
            return Err(error).with_context(|| anyhow!("{:?}", input_path.absolutize().unwrap()));
        },
    };

    let output_path = if let Some(output_path) = output_path.as_deref() {
        output_path
    } else if let Some(output_path) = output.as_deref() {
        output_path
    } else {
        CWD.as_path()
    };

    let cpus = cpu_count(cli_args.single_thread);

    let format = ArchiveFormat::from_file_path(input_path.as_path())?;

    if cli_args.password.is_some() && !format.supports_password() {
        return Err(anyhow!("`password` only supports 7Z, ZIP and RAR."));
    }

    let output_path = match output_path.canonicalize() {
        Ok(output_path) => {
            if !output_path.is_dir() {
                return Err(anyhow!("{output_path:?} is not a directory."));
            }

            output_path
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            fs::create_dir_all(output_path)
                .with_context(|| anyhow!("{:?}", output_path.absolutize().unwrap()))?;

            output_path
                .canonicalize()
                .with_context(|| anyhow!("{:?}", output_path.absolutize().unwrap()))?
        },
        Err(error) => {
            return Err(error).with_context(|| anyhow!("{:?}", output_path.absolutize().unwrap()));
        },
    };

    let threads = cpus.to_string();
    let threads = threads.as_str();
    let decompressor = Decompressor {
        executable_paths: &cli_args.executable_paths,
        threads,
        cpus,
    };

    match format {
        ArchiveFormat::TarZ
        | ArchiveFormat::TarGzip
        | ArchiveFormat::TarBzip2
        | ArchiveFormat::TarLz
        | ArchiveFormat::TarXz
        | ArchiveFormat::TarLzma
        | ArchiveFormat::TarZstd => {
            let mut command1 = decompressor.command(format, input_path.as_os_str());
            let mut command2 = command_args!(
                &cli_args.executable_paths.tar_path,
                "-x",
                "-C",
                output_path.as_path(),
                "-f",
                "-"
            );

            if !cli_args.quiet {
                command2.arg("-v");
            }

            let code = execute_pipeline(&mut [&mut command1, &mut command2], cli_args.quiet)?;

            Ok(code)
        },
        ArchiveFormat::Tar7z => {
            let password = read_password(cli_args.password)?;

            let mut command1 = command_args!(
                &cli_args.executable_paths.p7z_path,
                "x",
                "-so",
                format!("-mmt{threads}")
            );

            command1.arg(format!("-p{password}"));

            command1.arg(input_path);

            let mut command2 = command_args!(
                &cli_args.executable_paths.tar_path,
                "-x",
                "-C",
                output_path.as_path(),
                "-f",
                "-"
            );

            if !cli_args.quiet {
                command2.arg("-v");
            }

            let code = execute_pipeline(&mut [&mut command1, &mut command2], cli_args.quiet)?;

            Ok(code)
        },
        ArchiveFormat::Tar => {
            let mut command = command_args!(
                &cli_args.executable_paths.tar_path,
                "-x",
                "-C",
                output_path.as_path(),
                "-f",
                input_path
            );

            if !cli_args.quiet {
                command.arg("-v");
            }

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)?;

            Ok(code)
        },
        ArchiveFormat::Z
        | ArchiveFormat::Gzip
        | ArchiveFormat::Bzip2
        | ArchiveFormat::Lz
        | ArchiveFormat::Xz
        | ArchiveFormat::Lzma
        | ArchiveFormat::Zstd => {
            let file_path = decompressed_file_path(output_path.as_path(), input_path.as_path())?;

            if file_path.is_dir() {
                return Err(anyhow!("{file_path:?} is a directory."));
            }

            let staged_file = StagedFile::new(file_path.as_path())?;
            let file =
                File::create(staged_file.path()).with_context(|| anyhow!("{file_path:?}"))?;
            let mut command = decompressor.command(format, input_path.as_os_str());

            command.stdout(file);

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)?;

            publish_on_success(code, staged_file)
        },
        ArchiveFormat::P7z => {
            let password = read_password(cli_args.password)?;

            let mut command = p7z_extract_command(
                &cli_args.executable_paths.p7z_path,
                threads,
                password.as_str(),
                input_path.as_path(),
                output_path.as_path(),
            );

            if cli_args.quiet {
                command.stdout(Stdio::null());
            }

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)?;

            Ok(code)
        },
        ArchiveFormat::Zip => {
            let password = read_password(cli_args.password)?;

            // 7z is preferred because `unzip` cannot read a split archive
            if command_args!(&cli_args.executable_paths.p7z_path, "i")
                .execute_check_exit_status_code(0)
                .is_ok()
            {
                let mut command = p7z_extract_command(
                    &cli_args.executable_paths.p7z_path,
                    threads,
                    password.as_str(),
                    input_path.as_path(),
                    output_path.as_path(),
                );

                if cli_args.quiet {
                    command.stdout(Stdio::null());
                }

                let code = execute_pipeline(&mut [&mut command], cli_args.quiet)?;

                return Ok(code);
            }

            let mut command = command_args!(&cli_args.executable_paths.unzip_path);

            command.arg("-P");
            command.arg(password);

            if cli_args.quiet {
                command.arg("-qq");
            }

            command.args(["-O", "UTF-8", "-o"]);
            command.arg(input_path);
            command.arg("-d");
            command.arg(output_path.as_path());

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)?;

            Ok(code)
        },
        ArchiveFormat::Rar => {
            let password = read_password(cli_args.password)?;
            let executable_path = if command_args!(&cli_args.executable_paths.unrar_path, "-?")
                .execute_check_exit_status_code(0)
                .is_ok()
            {
                &cli_args.executable_paths.unrar_path
            } else {
                &cli_args.executable_paths.rar_path
            };

            let mut command = command_args!(executable_path, "x", "-o+");

            command.arg(format!("-mt{threads}"));

            if password.is_empty() {
                command.arg("-p-");
            } else {
                command.arg(format!("-p{password}"));
            }

            if cli_args.quiet {
                command.arg("-idq");
            }

            command.arg(input_path);
            command.arg(output_path.as_path());

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)?;

            Ok(code)
        },
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, os::unix::fs::PermissionsExt, path::PathBuf};

    use clap::Parser;

    use super::*;

    /// Create an executable which always fails, because `false` is not at the same path on every platform.
    fn failing_executable(directory: &Path) -> PathBuf {
        let path = directory.join("always-failing");

        fs::write(path.as_path(), b"#!/bin/sh\nexit 1\n").unwrap();
        fs::set_permissions(path.as_path(), fs::Permissions::from_mode(0o755)).unwrap();

        path
    }

    #[test]
    fn failed_decompression_preserves_existing_output() {
        let directory = tempfile::tempdir().unwrap();
        let input_path = directory.path().join("input.gz");
        let output_path = directory.path().join("output");
        let decompressed_path = output_path.join("input");
        fs::write(input_path.as_path(), b"invalid gzip").unwrap();
        fs::create_dir(output_path.as_path()).unwrap();
        fs::write(decompressed_path.as_path(), b"old").unwrap();

        let args = CLIArgs::try_parse_from([
            OsString::from("xcompress"),
            OsString::from("--single-thread"),
            OsString::from("--gunzip-path"),
            OsString::from("/xcompress/missing-gunzip"),
            OsString::from("--gzip-path"),
            failing_executable(directory.path()).into_os_string(),
            OsString::from("x"),
            input_path.into_os_string(),
            output_path.into_os_string(),
        ])
        .unwrap();

        assert_eq!(Some(1), handle_decompression(args).unwrap());
        assert_eq!(b"old", fs::read(decompressed_path).unwrap().as_slice());
    }
}
