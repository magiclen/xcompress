use std::{fs, fs::File, io, process};

use anyhow::{anyhow, Context};
use execute::{command_args, Execute};
use path_absolutize::{Absolutize, CWD};

use super::read_password;
use crate::{
    archive_format::ArchiveFormat,
    cli::{CLIArgs, CLICommands},
};

pub fn handle_decompression(cli_args: CLIArgs) -> anyhow::Result<()> {
    debug_assert!(matches!(cli_args.command, CLICommands::X { .. }));

    if let CLICommands::X {
        input_path,
        output_path,
        output,
    } = cli_args.command
    {
        let input_path = match input_path.canonicalize() {
            Ok(input_path) => input_path,
            Err(error) => {
                return Err(error)
                    .with_context(|| anyhow!("{:?}", input_path.absolutize().unwrap()));
            },
        };

        let output_path = if let Some(output_path) = output_path.as_deref() {
            output_path
        } else if let Some(output_path) = output.as_deref() {
            output_path
        } else {
            CWD.as_path()
        };

        let cpus = if cli_args.single_thread { 1 } else { num_cpus::get() };

        let format = ArchiveFormat::get_archive_format_from_file_path(input_path.as_path())?;

        if cli_args.password.is_some()
            && !matches!(
                format,
                ArchiveFormat::Tar7z | ArchiveFormat::P7z | ArchiveFormat::Zip | ArchiveFormat::Rar
            )
        {
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
                return Err(error)
                    .with_context(|| anyhow!("{:?}", output_path.absolutize().unwrap()));
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

                    let output = command1.execute_multiple_output(&mut [&mut command2])?;

                    process::exit(output.status.code().unwrap_or(1));
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

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
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

                        let output = command1.execute_multiple_output(&mut [&mut command2])?;

                        process::exit(output.status.code().unwrap_or(1));
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

                        let output = command1.execute_multiple_output(&mut [&mut command2])?;

                        process::exit(output.status.code().unwrap_or(1));
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

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
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

                    let output = command1.execute_multiple_output(&mut [&mut command2])?;

                    process::exit(output.status.code().unwrap_or(1));
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

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
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

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
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

                    let output = command1.execute_multiple_output(&mut [&mut command2])?;

                    process::exit(output.status.code().unwrap_or(1));
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

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
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

                    let output = command1.execute_multiple_output(&mut [&mut command2])?;

                    process::exit(output.status.code().unwrap_or(1));
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

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
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

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
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

                let output = command1.execute_multiple_output(&mut [&mut command2])?;

                process::exit(output.status.code().unwrap_or(1));
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

                    let output = command1.execute_multiple_output(&mut [&mut command2])?;

                    process::exit(output.status.code().unwrap_or(1));
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

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
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

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
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

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
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

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }

                if command_args!(&cli_args.executable_paths.gnuzip_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
                {
                    let mut command =
                        command_args!(&cli_args.executable_paths.gnuzip_path, "-c", input_path);

                    command.stdout(file);

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }

                let mut command =
                    command_args!(&cli_args.executable_paths.gzip_path, "-d", "-c", input_path);

                command.stdout(file);

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
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

                        let output = command.execute_output()?;

                        process::exit(output.status.code().unwrap_or(1));
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

                        let output = command.execute_output()?;

                        process::exit(output.status.code().unwrap_or(1));
                    }
                }

                if command_args!(&cli_args.executable_paths.bunzip2_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
                {
                    let mut command =
                        command_args!(&cli_args.executable_paths.bunzip2_path, "-c", input_path);

                    command.stdout(file);

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }

                let mut command =
                    command_args!(&cli_args.executable_paths.bzip2_path, "-d", "-c", input_path);

                command.stdout(file);

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
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

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }

                if command_args!(&cli_args.executable_paths.lunzip_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
                {
                    let mut command =
                        command_args!(&cli_args.executable_paths.lunzip_path, "-c", input_path);

                    command.stdout(file);

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }

                let mut command =
                    command_args!(&cli_args.executable_paths.lzip_path, "-d", "-c", input_path);

                command.stdout(file);

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
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

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }

                if command_args!(&cli_args.executable_paths.unxz_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
                {
                    let mut command =
                        command_args!(&cli_args.executable_paths.unxz_path, "-c", input_path);

                    command.stdout(file);

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }

                let mut command =
                    command_args!(&cli_args.executable_paths.xz_path, "-d", "-c", input_path);

                command.stdout(file);

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
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

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }

                if command_args!(&cli_args.executable_paths.unlzma_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
                {
                    let mut command =
                        command_args!(&cli_args.executable_paths.unlzma_path, "-c", input_path);

                    command.stdout(file);

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }

                let mut command =
                    command_args!(&cli_args.executable_paths.lzma_path, "-d", "-c", input_path);

                command.stdout(file);

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
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
                    process::exit(command.execute()?.unwrap_or(1));
                } else {
                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }
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

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
            },
            ArchiveFormat::Rar => {
                let password = read_password(cli_args.password)?;

                if command_args!(&cli_args.executable_paths.unrar_path, "-?")
                    .execute_check_exit_status_code(0)
                    .is_ok()
                {
                    let mut command =
                        command_args!(&cli_args.executable_paths.unrar_path, "x", "-o+");

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

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
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

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
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

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }

                if command_args!(&cli_args.executable_paths.unzstd_path, "-V")
                    .execute_check_exit_status_code(0)
                    .is_ok()
                {
                    let mut command =
                        command_args!(&cli_args.executable_paths.unzstd_path, "-c", input_path);

                    command.stdout(file);

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }

                let mut command =
                    command_args!(&cli_args.executable_paths.zstd_path, "-d", "-c", input_path);

                command.stdout(file);

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
            },
        }
    }

    Ok(())
}
