use std::{fs, fs::File, io, process::Stdio};

use anyhow::{Context, anyhow};
use execute::{Execute, command_args};
use path_absolutize::{Absolutize, CWD};

use super::{cleanup_output, cpu_count, execute_pipeline, read_password, try_delete_file};
use crate::{
    archive_format::ArchiveFormat,
    cli::{CLIArgs, CLICommands},
};

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

    match format {
        ArchiveFormat::TarZ | ArchiveFormat::TarGzip => {
            if cpus > 1
                && command_args!(&cli_args.executable_paths.pigz_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
            {
                let mut command1 = command_args!(
                    &cli_args.executable_paths.pigz_path,
                    "-d",
                    "-c",
                    "-p",
                    threads,
                    input_path
                );
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

                return Ok(code);
            }

            let mut command = command_args!(
                &cli_args.executable_paths.tar_path,
                "-z",
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

            return Ok(code);
        },
        ArchiveFormat::TarBzip2 => {
            if cpus > 1 {
                if command_args!(&cli_args.executable_paths.lbzip2_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
                {
                    let mut command1 = command_args!(
                        &cli_args.executable_paths.lbzip2_path,
                        "-d",
                        "-c",
                        "-n",
                        threads,
                        input_path
                    );
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

                    let code =
                        execute_pipeline(&mut [&mut command1, &mut command2], cli_args.quiet)?;

                    return Ok(code);
                }

                if command_args!(&cli_args.executable_paths.pbzip2_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
                {
                    let mut command1 = command_args!(
                        &cli_args.executable_paths.pbzip2_path,
                        "-d",
                        "-c",
                        format!("-p{threads}"),
                        input_path
                    );
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

                    let code =
                        execute_pipeline(&mut [&mut command1, &mut command2], cli_args.quiet)?;

                    return Ok(code);
                }
            }

            let mut command = command_args!(
                &cli_args.executable_paths.tar_path,
                "-j",
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

            return Ok(code);
        },
        ArchiveFormat::TarLz => {
            if cpus > 1
                && command_args!(&cli_args.executable_paths.plzip_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
            {
                let mut command1 = command_args!(
                    &cli_args.executable_paths.plzip_path,
                    "-d",
                    "-c",
                    "-n",
                    threads,
                    input_path
                );
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

                return Ok(code);
            }

            if command_args!(&cli_args.executable_paths.lunzip_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
            {
                let mut command = command_args!(
                    &cli_args.executable_paths.tar_path,
                    "-I",
                    &cli_args.executable_paths.lunzip_path,
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

                return Ok(code);
            }

            let mut command = command_args!(
                &cli_args.executable_paths.tar_path,
                "-I",
                &cli_args.executable_paths.lzip_path,
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

            return Ok(code);
        },
        ArchiveFormat::TarXz => {
            if cpus > 1
                && command_args!(&cli_args.executable_paths.pxz_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
            {
                let mut command1 = command_args!(
                    &cli_args.executable_paths.pxz_path,
                    "-d",
                    "-c",
                    "-T",
                    threads,
                    input_path
                );
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

                return Ok(code);
            }

            let mut command = command_args!(
                &cli_args.executable_paths.tar_path,
                "-J",
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

            return Ok(code);
        },
        ArchiveFormat::TarLzma => {
            if cpus > 1
                && command_args!(&cli_args.executable_paths.pxz_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
            {
                let mut command1 = command_args!(
                    &cli_args.executable_paths.pxz_path,
                    "-d",
                    "-c",
                    "-T",
                    threads,
                    "-F",
                    "lzma",
                    input_path
                );
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

                return Ok(code);
            }

            if command_args!(&cli_args.executable_paths.unlzma_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
            {
                let mut command = command_args!(
                    &cli_args.executable_paths.tar_path,
                    "-I",
                    &cli_args.executable_paths.unlzma_path,
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

                return Ok(code);
            }

            let mut command = command_args!(
                &cli_args.executable_paths.tar_path,
                "-I",
                &cli_args.executable_paths.lzma_path,
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

            return Ok(code);
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

            return Ok(code);
        },
        ArchiveFormat::TarZstd => {
            if cpus > 1
                && command_args!(&cli_args.executable_paths.pzstd_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
            {
                let mut command1 = command_args!(
                    &cli_args.executable_paths.pzstd_path,
                    "-d",
                    "-c",
                    "-p",
                    threads,
                    input_path
                );
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

                return Ok(code);
            }

            if command_args!(&cli_args.executable_paths.unzstd_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
            {
                let mut command = command_args!(
                    &cli_args.executable_paths.tar_path,
                    "-I",
                    &cli_args.executable_paths.unzstd_path,
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

                return Ok(code);
            }

            let mut command = command_args!(
                &cli_args.executable_paths.tar_path,
                "-I",
                &cli_args.executable_paths.zstd_path,
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

            return Ok(code);
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

            return Ok(code);
        },
        ArchiveFormat::Z | ArchiveFormat::Gzip => {
            let file_path = output_path.join(input_path.file_stem().unwrap());

            if file_path.is_dir() {
                return Err(anyhow!("{file_path:?} is a directory."));
            }

            let file =
                File::create(file_path.as_path()).with_context(|| anyhow!("{file_path:?}"))?;

            if cpus > 1
                && command_args!(&cli_args.executable_paths.pigz_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
            {
                let mut command = command_args!(
                    &cli_args.executable_paths.pigz_path,
                    "-d",
                    "-c",
                    "-p",
                    threads,
                    input_path
                );

                command.stdout(file);

                let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                    .inspect_err(|_| try_delete_file(file_path.as_path()))?;

                return Ok(cleanup_output(code, file_path.as_path()));
            }

            if command_args!(&cli_args.executable_paths.gunzip_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
            {
                let mut command =
                    command_args!(&cli_args.executable_paths.gunzip_path, "-c", input_path);

                command.stdout(file);

                let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                    .inspect_err(|_| try_delete_file(file_path.as_path()))?;

                return Ok(cleanup_output(code, file_path.as_path()));
            }

            let mut command =
                command_args!(&cli_args.executable_paths.gzip_path, "-d", "-c", input_path);

            command.stdout(file);

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                .inspect_err(|_| try_delete_file(file_path.as_path()))?;

            return Ok(cleanup_output(code, file_path.as_path()));
        },
        ArchiveFormat::Bzip2 => {
            let file_path = output_path.join(input_path.file_stem().unwrap());

            if file_path.is_dir() {
                return Err(anyhow!("{file_path:?} is a directory."));
            }

            let file =
                File::create(file_path.as_path()).with_context(|| anyhow!("{file_path:?}"))?;

            if cpus > 1 {
                if command_args!(&cli_args.executable_paths.lbzip2_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
                {
                    let mut command = command_args!(
                        &cli_args.executable_paths.lbzip2_path,
                        "-d",
                        "-c",
                        "-n",
                        threads,
                        input_path
                    );

                    command.stdout(file);

                    let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                        .inspect_err(|_| try_delete_file(file_path.as_path()))?;

                    return Ok(cleanup_output(code, file_path.as_path()));
                }

                if command_args!(&cli_args.executable_paths.pbzip2_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
                {
                    let mut command = command_args!(
                        &cli_args.executable_paths.pbzip2_path,
                        "-d",
                        "-c",
                        format!("-p{threads}"),
                        input_path
                    );

                    command.stdout(file);

                    let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                        .inspect_err(|_| try_delete_file(file_path.as_path()))?;

                    return Ok(cleanup_output(code, file_path.as_path()));
                }
            }

            if command_args!(&cli_args.executable_paths.bunzip2_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
            {
                let mut command =
                    command_args!(&cli_args.executable_paths.bunzip2_path, "-c", input_path);

                command.stdout(file);

                let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                    .inspect_err(|_| try_delete_file(file_path.as_path()))?;

                return Ok(cleanup_output(code, file_path.as_path()));
            }

            let mut command =
                command_args!(&cli_args.executable_paths.bzip2_path, "-d", "-c", input_path);

            command.stdout(file);

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                .inspect_err(|_| try_delete_file(file_path.as_path()))?;

            return Ok(cleanup_output(code, file_path.as_path()));
        },
        ArchiveFormat::Lz => {
            let file_path = output_path.join(input_path.file_stem().unwrap());

            if file_path.is_dir() {
                return Err(anyhow!("{file_path:?} is a directory."));
            }

            let file =
                File::create(file_path.as_path()).with_context(|| anyhow!("{file_path:?}"))?;

            if cpus > 1
                && command_args!(&cli_args.executable_paths.plzip_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
            {
                let mut command = command_args!(
                    &cli_args.executable_paths.plzip_path,
                    "-d",
                    "-c",
                    "-n",
                    threads,
                    input_path
                );

                command.stdout(file);

                let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                    .inspect_err(|_| try_delete_file(file_path.as_path()))?;

                return Ok(cleanup_output(code, file_path.as_path()));
            }

            if command_args!(&cli_args.executable_paths.lunzip_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
            {
                let mut command =
                    command_args!(&cli_args.executable_paths.lunzip_path, "-c", input_path);

                command.stdout(file);

                let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                    .inspect_err(|_| try_delete_file(file_path.as_path()))?;

                return Ok(cleanup_output(code, file_path.as_path()));
            }

            let mut command =
                command_args!(&cli_args.executable_paths.lzip_path, "-d", "-c", input_path);

            command.stdout(file);

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                .inspect_err(|_| try_delete_file(file_path.as_path()))?;

            return Ok(cleanup_output(code, file_path.as_path()));
        },
        ArchiveFormat::Xz => {
            let file_path = output_path.join(input_path.file_stem().unwrap());

            if file_path.is_dir() {
                return Err(anyhow!("{file_path:?} is a directory."));
            }

            let file =
                File::create(file_path.as_path()).with_context(|| anyhow!("{file_path:?}"))?;

            if cpus > 1
                && command_args!(&cli_args.executable_paths.pxz_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
            {
                let mut command = command_args!(
                    &cli_args.executable_paths.pxz_path,
                    "-d",
                    "-c",
                    "-T",
                    threads,
                    input_path
                );

                command.stdout(file);

                let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                    .inspect_err(|_| try_delete_file(file_path.as_path()))?;

                return Ok(cleanup_output(code, file_path.as_path()));
            }

            if command_args!(&cli_args.executable_paths.unxz_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
            {
                let mut command =
                    command_args!(&cli_args.executable_paths.unxz_path, "-c", input_path);

                command.stdout(file);

                let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                    .inspect_err(|_| try_delete_file(file_path.as_path()))?;

                return Ok(cleanup_output(code, file_path.as_path()));
            }

            let mut command =
                command_args!(&cli_args.executable_paths.xz_path, "-d", "-c", input_path);

            command.stdout(file);

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                .inspect_err(|_| try_delete_file(file_path.as_path()))?;

            return Ok(cleanup_output(code, file_path.as_path()));
        },
        ArchiveFormat::Lzma => {
            let file_path = output_path.join(input_path.file_stem().unwrap());

            if file_path.is_dir() {
                return Err(anyhow!("{file_path:?} is a directory."));
            }

            let file =
                File::create(file_path.as_path()).with_context(|| anyhow!("{file_path:?}"))?;

            if cpus > 1
                && command_args!(&cli_args.executable_paths.pxz_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
            {
                let mut command = command_args!(
                    &cli_args.executable_paths.pxz_path,
                    "-d",
                    "-c",
                    "-T",
                    threads,
                    "-F",
                    "lzma",
                    input_path
                );

                command.stdout(file);

                let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                    .inspect_err(|_| try_delete_file(file_path.as_path()))?;

                return Ok(cleanup_output(code, file_path.as_path()));
            }

            if command_args!(&cli_args.executable_paths.unlzma_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
            {
                let mut command =
                    command_args!(&cli_args.executable_paths.unlzma_path, "-c", input_path);

                command.stdout(file);

                let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                    .inspect_err(|_| try_delete_file(file_path.as_path()))?;

                return Ok(cleanup_output(code, file_path.as_path()));
            }

            let mut command =
                command_args!(&cli_args.executable_paths.lzma_path, "-d", "-c", input_path);

            command.stdout(file);

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                .inspect_err(|_| try_delete_file(file_path.as_path()))?;

            return Ok(cleanup_output(code, file_path.as_path()));
        },
        ArchiveFormat::P7z => {
            let password = read_password(cli_args.password)?;

            let mut command = command_args!(
                &cli_args.executable_paths.p7z_path,
                "x",
                "-aoa",
                format!("-mmt{threads}"),
                format!("-o{}", output_path.to_string_lossy())
            );

            command.arg(format!("-p{password}"));

            command.arg(input_path);

            if cli_args.quiet {
                command.stdout(Stdio::null());
            }

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)?;

            return Ok(code);
        },
        ArchiveFormat::Zip => {
            let password = read_password(cli_args.password)?;

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

            return Ok(code);
        },
        ArchiveFormat::Rar => {
            let password = read_password(cli_args.password)?;

            if command_args!(&cli_args.executable_paths.unrar_path, "-?")
                .execute_check_exit_status_code(0)
                .is_ok()
            {
                let mut command = command_args!(&cli_args.executable_paths.unrar_path, "x", "-o+");

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

                return Ok(code);
            }

            let mut command = command_args!(&cli_args.executable_paths.rar_path, "x", "-o+");

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

            return Ok(code);
        },
        ArchiveFormat::Zstd => {
            let file_path = output_path.join(input_path.file_stem().unwrap());

            if file_path.is_dir() {
                return Err(anyhow!("{file_path:?} is a directory."));
            }

            let file =
                File::create(file_path.as_path()).with_context(|| anyhow!("{file_path:?}"))?;

            if cpus > 1
                && command_args!(&cli_args.executable_paths.pzstd_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
            {
                let mut command = command_args!(
                    &cli_args.executable_paths.pzstd_path,
                    "-d",
                    "-c",
                    "-p",
                    threads,
                    input_path
                );

                command.stdout(file);

                let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                    .inspect_err(|_| try_delete_file(file_path.as_path()))?;

                return Ok(cleanup_output(code, file_path.as_path()));
            }

            if command_args!(&cli_args.executable_paths.unzstd_path, "-V")
                .execute_check_exit_status_code(0)
                .is_ok()
            {
                let mut command =
                    command_args!(&cli_args.executable_paths.unzstd_path, "-c", input_path);

                command.stdout(file);

                let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                    .inspect_err(|_| try_delete_file(file_path.as_path()))?;

                return Ok(cleanup_output(code, file_path.as_path()));
            }

            let mut command =
                command_args!(&cli_args.executable_paths.zstd_path, "-d", "-c", input_path);

            command.stdout(file);

            let code = execute_pipeline(&mut [&mut command], cli_args.quiet)
                .inspect_err(|_| try_delete_file(file_path.as_path()))?;

            return Ok(cleanup_output(code, file_path.as_path()));
        },
    }
}
