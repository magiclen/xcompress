use std::{
    borrow::Cow,
    ffi::OsStr,
    fs,
    fs::File,
    io,
    path::{Path, PathBuf},
    process,
    process::{Command, Stdio},
};

use anyhow::{Context, anyhow};
use byte_unit::{Byte, Unit};
use execute::{Execute, command_args};
use path_absolutize::{Absolutize, CWD};

use super::{cpu_count, execute_pipeline, exit_with_cleanup, read_password, try_delete_file};
use crate::{
    archive_format::ArchiveFormat,
    cli::{CLIArgs, CLICommands, ExecutablePaths},
};

/// The minimum volume size supported by the archivers.
const MIN_SPLIT_SIZE: u64 = 65536;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum CompressionLevel {
    Default,
    Best,
    Fast,
}

impl CompressionLevel {
    /// Append the arguments which are specific to this level.
    fn apply(self, command: &mut Command, best: &[&str], fast: &[&str]) {
        match self {
            Self::Best => {
                command.args(best);
            },
            Self::Fast => {
                command.args(fast);
            },
            Self::Default => (),
        }
    }
}

/// Builds the command of a compressor, preferring its parallel implementation when that helps.
///
/// Every compressor reads from `input`, which is either a file path or `-` for stdin, and writes to stdout, so the same command works both on its own and as a part of a tar pipeline.
struct Compressor<'a> {
    executable_paths: &'a ExecutablePaths,
    level:            CompressionLevel,
    threads:          &'a str,
    cpus:             usize,
    quiet:            bool,
}

impl Compressor<'_> {
    fn quiet_arg(&self, command: &mut Command) {
        if self.quiet {
            command.arg("-q");
        }
    }

    fn compress(&self, input: &OsStr) -> Command {
        command_args!(&self.executable_paths.compress_path, "-c", input)
    }

    fn gzip(&self, input: &OsStr) -> Command {
        // zopfli, which is only available in pigz, is worth a try even with a single thread
        if (self.cpus > 1 || self.level == CompressionLevel::Best)
            && command_args!(&self.executable_paths.pigz_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
        {
            let mut command =
                command_args!(&self.executable_paths.pigz_path, "-c", "-p", self.threads, input);

            self.quiet_arg(&mut command);
            self.level.apply(&mut command, &["-11"], &["-1"]);

            return command;
        }

        let mut command = command_args!(&self.executable_paths.gzip_path, "-c", input);

        self.quiet_arg(&mut command);
        self.level.apply(&mut command, &["-9"], &["-1"]);

        command
    }

    fn bzip2(&self, input: &OsStr) -> Command {
        if self.cpus > 1 {
            if command_args!(&self.executable_paths.lbzip2_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
            {
                let mut command = command_args!(
                    &self.executable_paths.lbzip2_path,
                    "-z",
                    "-c",
                    "-n",
                    self.threads,
                    input
                );

                self.quiet_arg(&mut command);
                self.level.apply(&mut command, &["-9"], &["-1"]);

                return command;
            }

            if command_args!(&self.executable_paths.pbzip2_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
            {
                let mut command = command_args!(
                    &self.executable_paths.pbzip2_path,
                    "-z",
                    "-c",
                    format!("-p{}", self.threads),
                    input
                );

                self.quiet_arg(&mut command);
                self.level.apply(&mut command, &["-9"], &["-1"]);

                return command;
            }
        }

        let mut command = command_args!(&self.executable_paths.bzip2_path, "-z", "-c", input);

        self.quiet_arg(&mut command);
        self.level.apply(&mut command, &["-9"], &["-1"]);

        command
    }

    fn lzip(&self, input: &OsStr) -> Command {
        if self.cpus > 1
            && command_args!(&self.executable_paths.plzip_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
        {
            let mut command = command_args!(
                &self.executable_paths.plzip_path,
                "-F",
                "-c",
                "-n",
                self.threads,
                input
            );

            self.quiet_arg(&mut command);
            self.level.apply(&mut command, &["-9"], &["-1"]);

            return command;
        }

        let mut command = command_args!(&self.executable_paths.lzip_path, "-F", "-c", input);

        self.quiet_arg(&mut command);
        self.level.apply(&mut command, &["-9"], &["-1"]);

        command
    }

    fn xz(&self, input: &OsStr) -> Command {
        if self.cpus > 1
            && command_args!(&self.executable_paths.pxz_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
        {
            let mut command = command_args!(
                &self.executable_paths.pxz_path,
                "-z",
                "-c",
                "-T",
                self.threads,
                input
            );

            self.quiet_arg(&mut command);
            self.level.apply(&mut command, &["-9", "-e"], &["-0"]);

            return command;
        }

        let mut command = command_args!(&self.executable_paths.xz_path, "-z", "-c", input);

        self.quiet_arg(&mut command);
        self.level.apply(&mut command, &["-9", "-e"], &["-0"]);

        command
    }

    fn lzma(&self, input: &OsStr) -> Command {
        if self.cpus > 1
            && command_args!(&self.executable_paths.pxz_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
        {
            let mut command = command_args!(
                &self.executable_paths.pxz_path,
                "-z",
                "-c",
                "-T",
                self.threads,
                "-F",
                "lzma",
                input
            );

            self.quiet_arg(&mut command);
            self.level.apply(&mut command, &["-9", "-e"], &["-0"]);

            return command;
        }

        let mut command = command_args!(&self.executable_paths.lzma_path, "-z", "-c", input);

        self.quiet_arg(&mut command);
        self.level.apply(&mut command, &["-9", "-e"], &["-0"]);

        command
    }

    /// Unlike the other compressors, zstd writes to `output_path` instead of stdout.
    fn zstd(&self, input: &OsStr, output_path: &Path) -> Command {
        if self.cpus > 1
            && command_args!(&self.executable_paths.pzstd_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
        {
            let mut command = command_args!(
                &self.executable_paths.pzstd_path,
                "-p",
                self.threads,
                input,
                "-o",
                output_path
            );

            self.quiet_arg(&mut command);
            self.level.apply(&mut command, &["--ultra", "-22"], &["-1"]);

            return command;
        }

        let mut command = command_args!(&self.executable_paths.zstd_path, input, "-o", output_path);

        self.quiet_arg(&mut command);
        self.level.apply(&mut command, &["--ultra", "-22"], &["-1"]);

        command
    }
}

/// Parse the split size and return it in KiB, which is the unit understood by the archivers.
fn parse_split_size(split: &str) -> anyhow::Result<u32> {
    let byte = Byte::parse_str(split, true)?;

    if byte.as_u64() < MIN_SPLIT_SIZE {
        return Err(anyhow!("The split size is too small."));
    }

    let kib = byte.get_adjusted_unit(Unit::KiB).get_value().round();

    if kib > f64::from(u32::MAX) {
        return Err(anyhow!("The split size is too large."));
    }

    Ok(kib as u32)
}

/// Add each input path as a `-C <parent> <name>` pair, so the base directory is excluded from the archive.
fn add_input_paths(command: &mut Command, input_paths: &[PathBuf]) -> anyhow::Result<()> {
    for input_path in input_paths {
        let input_path = input_path.absolutize()?;

        command.arg("-C");
        command.arg(input_path.parent().unwrap());
        command.arg(input_path.file_name().unwrap());
    }

    Ok(())
}

pub fn handle_compression(cli_args: CLIArgs) -> anyhow::Result<()> {
    let CLICommands::A {
        mut input_paths,
        output_path,
        best_compression,
        fastest_compression,
        split,
        recovery_record,
        ..
    } = cli_args.command
    else {
        unreachable!();
    };

    for input_path in input_paths.iter_mut() {
        match input_path.canonicalize() {
            Ok(path) => {
                *input_path = path;
            },
            Err(error) => {
                return Err(error)
                    .with_context(|| anyhow!("{:?}", input_path.absolutize().unwrap()));
            },
        }
    }

    let output_path = match output_path {
        Some(output_path) => output_path,
        None => CWD.join(format!("{}.rar", input_paths[0].file_name().unwrap().to_string_lossy())),
    };

    let cpus = cpu_count(cli_args.single_thread);

    let format = ArchiveFormat::from_file_path(output_path.as_path())?;

    if cli_args.password.is_some() && !format.supports_password() {
        return Err(anyhow!("`password` only supports 7Z, ZIP and RAR."));
    }

    if split.is_some() && !format.supports_split() {
        return Err(anyhow!("`split` only supports 7Z, ZIP and RAR."));
    }

    if recovery_record.is_some() && !matches!(format, ArchiveFormat::Rar) {
        return Err(anyhow!("`recovery-record` only supports RAR."));
    }

    // everything which may fail has to be done before the existing output file is removed
    let split = match split {
        Some(split) => Some(parse_split_size(split.as_str())?),
        None => None,
    };

    if let Some(extension) = format.tar_alternative_extension()
        && (input_paths.len() > 1 || input_paths[0].is_dir())
    {
        return Err(anyhow!(
            "Obviously, you should use {extension} for filename extension to support multiple \
             files."
        ));
    }

    let password = read_password(cli_args.password)?;

    let output_path = match output_path.canonicalize() {
        Ok(output_path) => {
            if output_path.is_dir() {
                return Err(anyhow!("{output_path:?} is a directory."));
            }

            fs::remove_file(output_path.as_path())?;

            output_path
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => match output_path.absolutize()? {
            Cow::Borrowed(_) => output_path,
            Cow::Owned(path) => path,
        },
        Err(error) => {
            return Err(error.into());
        },
    };

    let level = if best_compression {
        CompressionLevel::Best
    } else if fastest_compression {
        CompressionLevel::Fast
    } else {
        CompressionLevel::Default
    };

    let threads = cpus.to_string();
    let threads = threads.as_str();

    let compressor = Compressor {
        executable_paths: &cli_args.executable_paths,
        level,
        threads,
        cpus,
        quiet: cli_args.quiet,
    };

    let stdin = OsStr::new("-");

    match format {
        ArchiveFormat::TarZ
        | ArchiveFormat::TarGzip
        | ArchiveFormat::TarBzip2
        | ArchiveFormat::TarLz
        | ArchiveFormat::TarXz
        | ArchiveFormat::TarLzma => {
            let mut command1 = command_args!(&cli_args.executable_paths.tar_path, "-c", "-f", "-");

            add_input_paths(&mut command1, &input_paths)?;

            let mut command2 = match format {
                ArchiveFormat::TarZ => compressor.compress(stdin),
                ArchiveFormat::TarGzip => compressor.gzip(stdin),
                ArchiveFormat::TarBzip2 => compressor.bzip2(stdin),
                ArchiveFormat::TarLz => compressor.lzip(stdin),
                ArchiveFormat::TarXz => compressor.xz(stdin),
                ArchiveFormat::TarLzma => compressor.lzma(stdin),
                _ => unreachable!(),
            };

            command2.stdout(File::create(output_path.as_path())?);

            let code = execute_pipeline(&mut [&mut command1, &mut command2], cli_args.quiet)
                .inspect_err(|_| try_delete_file(output_path.as_path()))?;

            // `compress` exits with 2 when the input cannot be made smaller, but its output is still complete
            if format == ArchiveFormat::TarZ && code == Some(2) {
                process::exit(2);
            }

            exit_with_cleanup(code, output_path.as_path());
        },
        ArchiveFormat::Z
        | ArchiveFormat::Gzip
        | ArchiveFormat::Bzip2
        | ArchiveFormat::Lz
        | ArchiveFormat::Xz
        | ArchiveFormat::Lzma => {
            let input_path = input_paths[0].as_os_str();

            let mut command = match format {
                ArchiveFormat::Z => compressor.compress(input_path),
                ArchiveFormat::Gzip => compressor.gzip(input_path),
                ArchiveFormat::Bzip2 => compressor.bzip2(input_path),
                ArchiveFormat::Lz => compressor.lzip(input_path),
                ArchiveFormat::Xz => compressor.xz(input_path),
                ArchiveFormat::Lzma => compressor.lzma(input_path),
                _ => unreachable!(),
            };

            command.stdout(File::create(output_path.as_path())?);

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                .inspect_err(|_| try_delete_file(output_path.as_path()))?;

            exit_with_cleanup(code, output_path.as_path());
        },
        ArchiveFormat::TarZstd => {
            let mut command1 = command_args!(&cli_args.executable_paths.tar_path, "-c", "-f", "-");

            add_input_paths(&mut command1, &input_paths)?;

            let mut command2 = compressor.zstd(stdin, output_path.as_path());

            let code = execute_pipeline(&mut [&mut command1, &mut command2], cli_args.quiet)?;

            process::exit(code.unwrap_or(1));
        },
        ArchiveFormat::Zstd => {
            let mut command = compressor.zstd(input_paths[0].as_os_str(), output_path.as_path());

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)?;

            process::exit(code.unwrap_or(1));
        },
        ArchiveFormat::Tar7z => {
            let mut command1 = command_args!(&cli_args.executable_paths.tar_path, "-c", "-f", "-");

            add_input_paths(&mut command1, &input_paths)?;

            let mut command2 = command_args!(
                &cli_args.executable_paths.p7z_path,
                "a",
                "-t7z",
                "-aoa",
                format!("-mmt{threads}"),
                "-si",
            );

            level.apply(&mut command2, &["-m0=lzma2", "-mx", "-ms=on"], &["-m0=copy"]);

            if !password.is_empty() {
                command2.arg("-mhe=on");
                command2.arg(format!("-p{password}"));
            }

            if let Some(kib) = split {
                command2.arg(format!("-v{kib}k"));
            }

            command2.arg(output_path.as_path());

            if cli_args.quiet {
                command2.stdout(Stdio::null());
            }

            let code = execute_pipeline(&mut [&mut command1, &mut command2], cli_args.quiet)?;

            process::exit(code.unwrap_or(1));
        },
        ArchiveFormat::Tar => {
            let mut command = command_args!(&cli_args.executable_paths.tar_path, "-c");

            if !cli_args.quiet {
                command.arg("-v");
            }

            command.arg("-f");
            command.arg(output_path.as_path());

            add_input_paths(&mut command, &input_paths)?;

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)?;

            process::exit(code.unwrap_or(1));
        },
        ArchiveFormat::P7z => {
            let mut command = command_args!(
                &cli_args.executable_paths.p7z_path,
                "a",
                "-t7z",
                "-aoa",
                format!("-mmt{threads}")
            );

            level.apply(&mut command, &["-m0=lzma2", "-mx", "-ms=on"], &["-m0=copy"]);

            if !password.is_empty() {
                command.arg("-mhe=on");
                command.arg(format!("-p{password}"));
            }

            if let Some(kib) = split {
                command.arg(format!("-v{kib}k"));
            }

            command.arg(output_path.as_path());
            command.args(input_paths);

            if cli_args.quiet {
                command.stdout(Stdio::null());
            }

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)?;

            process::exit(code.unwrap_or(1));
        },
        ArchiveFormat::Zip => {
            // 7z cannot split a zip archive, so a whole one is created first and then split by zip
            let output_tmp_path = if split.is_some() {
                let new_filename =
                    format!("{}.tmp.zip", output_path.file_stem().unwrap().to_string_lossy());

                let output_tmp_path = output_path.parent().unwrap().join(new_filename);

                if let Ok(metadata) = output_tmp_path.metadata() {
                    if metadata.is_dir() {
                        return Err(anyhow!("{output_tmp_path:?} is a directory."));
                    } else {
                        fs::remove_file(output_tmp_path.as_path())?;
                    }
                }

                Cow::from(output_tmp_path)
            } else {
                Cow::from(output_path.as_path())
            };

            let mut command = command_args!(
                &cli_args.executable_paths.p7z_path,
                "a",
                "-tzip",
                "-aoa",
                format!("-mmt{threads}")
            );

            level.apply(&mut command, &["-mx"], &["-mx=0"]);

            if !password.is_empty() {
                command.arg(format!("-p{password}"));
            }

            command.arg(output_tmp_path.as_ref());
            command.args(input_paths);

            if cli_args.quiet {
                command.stdout(Stdio::null());
            }

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)?;

            let Some(kib) = split else {
                process::exit(code.unwrap_or(1));
            };

            if code != Some(0) {
                exit_with_cleanup(code, output_tmp_path.as_ref());
            }

            let mut command =
                command_args!(&cli_args.executable_paths.zip_path, "-s", format!("{kib}k"));

            level.apply(&mut command, &["-9"], &["-0"]);

            if !password.is_empty() {
                command.arg("--password");
                command.arg(password);
            }

            if cli_args.quiet {
                command.arg("-q");
            }

            command.arg(output_tmp_path.as_ref());
            command.arg("--out");
            command.arg(output_path.as_path());

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                .inspect_err(|_| try_delete_file(output_tmp_path.as_ref()))?;

            try_delete_file(output_tmp_path.as_ref());

            exit_with_cleanup(code, output_path.as_path());
        },
        ArchiveFormat::Rar => {
            let mut command = command_args!(
                &cli_args.executable_paths.rar_path,
                "a",
                "-ep1",
                format!("-mt{threads}")
            );

            level.apply(&mut command, &["-ma5", "-m5", "-s"], &["-m0"]);

            if !password.is_empty() {
                command.arg(format!("-hp{password}"));
            }

            if cli_args.quiet {
                command.arg("-idq");
            }

            if let Some(kib) = split {
                command.arg(format!("-v{kib}k"));
            }

            if let Some(rr) = recovery_record {
                command.arg(format!("-rr{rr}"));
            }

            command.arg(output_path.as_path());
            command.args(input_paths);

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)?;

            process::exit(code.unwrap_or(1));
        },
    }
}
