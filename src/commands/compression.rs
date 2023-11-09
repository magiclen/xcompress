use std::{borrow::Cow, fs, fs::File, io, process};

use anyhow::{anyhow, Context};
use byte_unit::{Byte, ByteUnit};
use execute::{command_args, Execute};
use path_absolutize::{Absolutize, CWD};

use super::{read_password, try_delete_file};
use crate::{
    archive_format::ArchiveFormat,
    cli::{CLIArgs, CLICommands},
};

#[derive(Debug, Eq, PartialEq)]
enum CompressionLevel {
    Default,
    Best,
    Fast,
}

pub fn handle_compression(cli_args: CLIArgs) -> anyhow::Result<()> {
    debug_assert!(matches!(cli_args.command, CLICommands::A { .. }));

    if let CLICommands::A {
        mut input_paths,
        output_path,
        best_compression,
        fastest_compression,
        split,
        recovery_record,
        ..
    } = cli_args.command
    {
        for input_path in input_paths.iter_mut() {
            match input_path.canonicalize() {
                Ok(path) => {
                    *input_path = path;
                },
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("{:?}", input_path.absolutize().unwrap()));
                },
            }
        }

        let output_path = match output_path {
            Some(output_path) => output_path,
            None => {
                CWD.join(format!("{}.rar", input_paths[0].file_name().unwrap().to_string_lossy()))
            },
        };

        let cpus = if cli_args.single_thread { 1 } else { num_cpus::get() };

        let format = ArchiveFormat::get_archive_format_from_file_path(output_path.as_path())?;

        if cli_args.password.is_some()
            && !matches!(
                format,
                ArchiveFormat::Tar7z | ArchiveFormat::P7z | ArchiveFormat::Zip | ArchiveFormat::Rar
            )
        {
            return Err(anyhow!("`password` only supports 7Z, ZIP and RAR."));
        }

        if split.is_some()
            && !matches!(
                format,
                ArchiveFormat::Tar7z | ArchiveFormat::P7z | ArchiveFormat::Zip | ArchiveFormat::Rar
            )
        {
            return Err(anyhow!("`split` only supports 7Z, ZIP and RAR."));
        }

        if recovery_record.is_some() && !matches!(format, ArchiveFormat::Rar) {
            return Err(anyhow!("`recovery-record` only supports RAR."));
        }

        let output_path = match output_path.canonicalize() {
            Ok(output_path) => {
                if output_path.is_dir() {
                    return Err(anyhow!("{output_path:?} is a directory."));
                }

                fs::remove_file(output_path.as_path())?;

                output_path
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                match output_path.absolutize()? {
                    Cow::Borrowed(_) => output_path,
                    Cow::Owned(path) => path,
                }
            },
            Err(error) => {
                return Err(error.into());
            },
        };

        let compression_level = if best_compression {
            CompressionLevel::Best
        } else if fastest_compression {
            CompressionLevel::Fast
        } else {
            CompressionLevel::Default
        };

        let threads = cpus.to_string();
        let threads = threads.as_str();

        match format {
            ArchiveFormat::TarZ
            | ArchiveFormat::TarGzip
            | ArchiveFormat::TarBzip2
            | ArchiveFormat::TarLz
            | ArchiveFormat::TarXz
            | ArchiveFormat::TarLzma
            | ArchiveFormat::Tar7z
            | ArchiveFormat::TarZstd => {
                let mut command1 =
                    command_args!(&cli_args.executable_paths.tar_path, "-c", "-f", "-");

                for input_path in input_paths {
                    let input_path = input_path.absolutize()?;
                    let input_path_parent = input_path.parent().unwrap();
                    let file_name = input_path.file_name().unwrap();

                    command1.arg("-C");
                    command1.arg(input_path_parent);
                    command1.arg(file_name);
                }

                match format {
                    ArchiveFormat::TarZ => {
                        let mut command2 =
                            command_args!(&cli_args.executable_paths.compress_path, "-c", "-");

                        command2.stdout(File::create(output_path.as_path())?);

                        let output = command1
                            .execute_multiple_output(&mut [&mut command2])
                            .map_err(|err| {
                                try_delete_file(output_path.as_path());
                                err
                            })?;

                        match output.status.code() {
                            Some(code) => {
                                if code != 0 && code != 2 {
                                    try_delete_file(output_path.as_path());
                                }

                                process::exit(code);
                            },
                            None => {
                                try_delete_file(output_path.as_path());
                                process::exit(1);
                            },
                        }
                    },
                    ArchiveFormat::TarGzip => {
                        if (cpus > 1 || compression_level == CompressionLevel::Best)
                            && command_args!(&cli_args.executable_paths.pigz_path, "-V")
                                .execute_check_exit_status_code(0)
                                .is_ok()
                        {
                            let mut command2 = command_args!(
                                &cli_args.executable_paths.pigz_path,
                                "-c",
                                "-p",
                                threads,
                                "-"
                            );

                            if cli_args.quiet {
                                command2.arg("-q");
                            }

                            match compression_level {
                                CompressionLevel::Best => {
                                    command2.arg("-11");
                                },
                                CompressionLevel::Fast => {
                                    command2.arg("-1");
                                },
                                CompressionLevel::Default => (),
                            }

                            command2.stdout(File::create(output_path.as_path())?);

                            let output = command1
                                .execute_multiple_output(&mut [&mut command2])
                                .map_err(|err| {
                                    try_delete_file(output_path.as_path());
                                    err
                                })?;

                            match output.status.code() {
                                Some(code) => {
                                    if code != 0 {
                                        try_delete_file(output_path.as_path());
                                    }

                                    process::exit(code);
                                },
                                None => {
                                    try_delete_file(output_path.as_path());
                                    process::exit(1);
                                },
                            }
                        }

                        let mut command2 =
                            command_args!(&cli_args.executable_paths.gzip_path, "-c", "-");

                        if cli_args.quiet {
                            command2.arg("-q");
                        }

                        match compression_level {
                            CompressionLevel::Best => {
                                command2.arg("-9");
                            },
                            CompressionLevel::Fast => {
                                command2.arg("-1");
                            },
                            CompressionLevel::Default => (),
                        }

                        command2.stdout(File::create(output_path.as_path())?);

                        let output = command1
                            .execute_multiple_output(&mut [&mut command2])
                            .map_err(|err| {
                                try_delete_file(output_path.as_path());
                                err
                            })?;

                        match output.status.code() {
                            Some(code) => {
                                if code != 0 {
                                    try_delete_file(output_path.as_path());
                                }

                                process::exit(code);
                            },
                            None => {
                                try_delete_file(output_path.as_path());
                                process::exit(1);
                            },
                        }
                    },
                    ArchiveFormat::TarBzip2 => {
                        if cpus > 1 {
                            if command_args!(&cli_args.executable_paths.lbzip2_path, "-V")
                                .execute_check_exit_status_code(0)
                                .is_ok()
                            {
                                let mut command2 = command_args!(
                                    &cli_args.executable_paths.lbzip2_path,
                                    "-z",
                                    "-c",
                                    "-n",
                                    threads,
                                    "-"
                                );

                                if cli_args.quiet {
                                    command2.arg("-q");
                                }

                                match compression_level {
                                    CompressionLevel::Best => {
                                        command2.arg("-9");
                                    },
                                    CompressionLevel::Fast => {
                                        command2.arg("-1");
                                    },
                                    CompressionLevel::Default => (),
                                }

                                command2.stdout(File::create(output_path.as_path())?);

                                let output = command1
                                    .execute_multiple_output(&mut [&mut command2])
                                    .map_err(|err| {
                                        try_delete_file(output_path.as_path());
                                        err
                                    })?;

                                match output.status.code() {
                                    Some(code) => {
                                        if code != 0 {
                                            try_delete_file(output_path.as_path());
                                        }

                                        process::exit(code);
                                    },
                                    None => {
                                        try_delete_file(output_path.as_path());
                                        process::exit(1);
                                    },
                                }
                            }

                            if command_args!(&cli_args.executable_paths.pbzip2_path, "-V")
                                .execute_check_exit_status_code(0)
                                .is_ok()
                            {
                                let mut command2 = command_args!(
                                    &cli_args.executable_paths.pbzip2_path,
                                    "-z",
                                    "-c",
                                    format!("-p{threads}"),
                                    "-"
                                );

                                if cli_args.quiet {
                                    command2.arg("-q");
                                }

                                match compression_level {
                                    CompressionLevel::Best => {
                                        command2.arg("-9");
                                    },
                                    CompressionLevel::Fast => {
                                        command2.arg("-1");
                                    },
                                    CompressionLevel::Default => (),
                                }

                                command2.stdout(File::create(output_path.as_path())?);

                                let output = command1
                                    .execute_multiple_output(&mut [&mut command2])
                                    .map_err(|err| {
                                        try_delete_file(output_path.as_path());
                                        err
                                    })?;

                                match output.status.code() {
                                    Some(code) => {
                                        if code != 0 {
                                            try_delete_file(output_path.as_path());
                                        }

                                        process::exit(code);
                                    },
                                    None => {
                                        try_delete_file(output_path.as_path());
                                        process::exit(1);
                                    },
                                }
                            }
                        }

                        let mut command2 =
                            command_args!(&cli_args.executable_paths.bzip2_path, "-z", "-c", "-");

                        if cli_args.quiet {
                            command2.arg("-q");
                        }

                        match compression_level {
                            CompressionLevel::Best => {
                                command2.arg("-9");
                            },
                            CompressionLevel::Fast => {
                                command2.arg("-1");
                            },
                            CompressionLevel::Default => (),
                        }

                        command2.stdout(File::create(output_path.as_path())?);

                        let output = command1
                            .execute_multiple_output(&mut [&mut command2])
                            .map_err(|err| {
                                try_delete_file(output_path.as_path());
                                err
                            })?;

                        match output.status.code() {
                            Some(code) => {
                                if code != 0 {
                                    try_delete_file(output_path.as_path());
                                }

                                process::exit(code);
                            },
                            None => {
                                try_delete_file(output_path.as_path());
                                process::exit(1);
                            },
                        }
                    },
                    ArchiveFormat::TarLz => {
                        if cpus > 1
                            && command_args!(&cli_args.executable_paths.plzip_path, "-V")
                                .execute_check_exit_status_code(0)
                                .is_ok()
                        {
                            let mut command2 = command_args!(
                                &cli_args.executable_paths.plzip_path,
                                "-F",
                                "-c",
                                "-n",
                                threads,
                                "-"
                            );

                            if cli_args.quiet {
                                command2.arg("-q");
                            }

                            match compression_level {
                                CompressionLevel::Best => {
                                    command2.arg("-9");
                                },
                                CompressionLevel::Fast => {
                                    command2.arg("-1");
                                },
                                CompressionLevel::Default => (),
                            }

                            command2.stdout(File::create(output_path.as_path())?);

                            let output = command1
                                .execute_multiple_output(&mut [&mut command2])
                                .map_err(|err| {
                                    try_delete_file(output_path.as_path());
                                    err
                                })?;

                            match output.status.code() {
                                Some(code) => {
                                    if code != 0 {
                                        try_delete_file(output_path.as_path());
                                    }

                                    process::exit(code);
                                },
                                None => {
                                    try_delete_file(output_path.as_path());
                                    process::exit(1);
                                },
                            }
                        }

                        let mut command2 =
                            command_args!(&cli_args.executable_paths.lzip_path, "-F", "-c", "-");

                        if cli_args.quiet {
                            command2.arg("-q");
                        }

                        match compression_level {
                            CompressionLevel::Best => {
                                command2.arg("-9");
                            },
                            CompressionLevel::Fast => {
                                command2.arg("-1");
                            },
                            CompressionLevel::Default => (),
                        }

                        command2.stdout(File::create(output_path.as_path())?);

                        let output = command1
                            .execute_multiple_output(&mut [&mut command2])
                            .map_err(|err| {
                                try_delete_file(output_path.as_path());
                                err
                            })?;

                        match output.status.code() {
                            Some(code) => {
                                if code != 0 {
                                    try_delete_file(output_path.as_path());
                                }

                                process::exit(code);
                            },
                            None => {
                                try_delete_file(output_path.as_path());
                                process::exit(1);
                            },
                        }
                    },
                    ArchiveFormat::TarXz => {
                        if cpus > 1
                            && command_args!(&cli_args.executable_paths.pxz_path, "-V")
                                .execute_check_exit_status_code(0)
                                .is_ok()
                        {
                            let mut command2 = command_args!(
                                &cli_args.executable_paths.pxz_path,
                                "-z",
                                "-c",
                                "-T",
                                threads,
                                "-"
                            );

                            if cli_args.quiet {
                                command2.arg("-q");
                            }

                            match compression_level {
                                CompressionLevel::Best => {
                                    command2.args(["-9", "-e"]);
                                },
                                CompressionLevel::Fast => {
                                    command2.arg("-0");
                                },
                                CompressionLevel::Default => (),
                            }

                            command2.stdout(File::create(output_path.as_path())?);

                            let output = command1
                                .execute_multiple_output(&mut [&mut command2])
                                .map_err(|err| {
                                    try_delete_file(output_path.as_path());
                                    err
                                })?;

                            match output.status.code() {
                                Some(code) => {
                                    if code != 0 {
                                        try_delete_file(output_path.as_path());
                                    }

                                    process::exit(code);
                                },
                                None => {
                                    try_delete_file(output_path.as_path());
                                    process::exit(1);
                                },
                            }
                        }

                        let mut command2 =
                            command_args!(&cli_args.executable_paths.xz_path, "-z", "-c", "-");

                        if cli_args.quiet {
                            command2.arg("-q");
                        }

                        match compression_level {
                            CompressionLevel::Best => {
                                command2.args(["-9", "-e"]);
                            },
                            CompressionLevel::Fast => {
                                command2.arg("-0");
                            },
                            CompressionLevel::Default => (),
                        }

                        command2.stdout(File::create(output_path.as_path())?);

                        let output = command1
                            .execute_multiple_output(&mut [&mut command2])
                            .map_err(|err| {
                                try_delete_file(output_path.as_path());
                                err
                            })?;

                        match output.status.code() {
                            Some(code) => {
                                if code != 0 {
                                    try_delete_file(output_path.as_path());
                                }

                                process::exit(code);
                            },
                            None => {
                                try_delete_file(output_path.as_path());
                                process::exit(1);
                            },
                        }
                    },
                    ArchiveFormat::TarLzma => {
                        if cpus > 1
                            && command_args!(&cli_args.executable_paths.pxz_path, "-V")
                                .execute_check_exit_status_code(0)
                                .is_ok()
                        {
                            let mut command2 = command_args!(
                                &cli_args.executable_paths.pxz_path,
                                "-z",
                                "-c",
                                "-T",
                                threads,
                                "-F",
                                "lzma",
                                "-"
                            );

                            if cli_args.quiet {
                                command2.arg("-q");
                            }

                            match compression_level {
                                CompressionLevel::Best => {
                                    command2.args(["-9", "-e"]);
                                },
                                CompressionLevel::Fast => {
                                    command2.arg("-0");
                                },
                                CompressionLevel::Default => (),
                            }

                            command2.stdout(File::create(output_path.as_path())?);

                            let output = command1
                                .execute_multiple_output(&mut [&mut command2])
                                .map_err(|err| {
                                    try_delete_file(output_path.as_path());
                                    err
                                })?;

                            match output.status.code() {
                                Some(code) => {
                                    if code != 0 {
                                        try_delete_file(output_path.as_path());
                                    }

                                    process::exit(code);
                                },
                                None => {
                                    try_delete_file(output_path.as_path());
                                    process::exit(1);
                                },
                            }
                        }

                        let mut command2 =
                            command_args!(&cli_args.executable_paths.lzma_path, "-z", "-c", "-");

                        if cli_args.quiet {
                            command2.arg("-q");
                        }

                        match compression_level {
                            CompressionLevel::Best => {
                                command2.args(["-9", "-e"]);
                            },
                            CompressionLevel::Fast => {
                                command2.arg("-0");
                            },
                            CompressionLevel::Default => (),
                        }

                        command2.stdout(File::create(output_path.as_path())?);

                        let output = command1
                            .execute_multiple_output(&mut [&mut command2])
                            .map_err(|err| {
                                try_delete_file(output_path.as_path());
                                err
                            })?;

                        match output.status.code() {
                            Some(code) => {
                                if code != 0 {
                                    try_delete_file(output_path.as_path());
                                }

                                process::exit(code);
                            },
                            None => {
                                try_delete_file(output_path.as_path());
                                process::exit(1);
                            },
                        }
                    },
                    ArchiveFormat::Tar7z => {
                        let password = read_password(cli_args.password)?;

                        let mut command2 = command_args!(
                            &cli_args.executable_paths.p7z_path,
                            "a",
                            "-t7z",
                            "-aoa",
                            format!("-mmt{threads}"),
                            "-si",
                        );

                        match compression_level {
                            CompressionLevel::Best => {
                                command2.args(["-m0=lzma2", "-mx", "-ms=on"]);
                            },
                            CompressionLevel::Fast => {
                                command2.arg("-m0=copy");
                            },
                            CompressionLevel::Default => (),
                        }

                        if !password.is_empty() {
                            command2.arg("-mhe=on");
                            command2.arg(format!("-p{password}"));
                        }

                        if let Some(d) = split {
                            let byte = Byte::from_str(d)?;

                            if byte.get_bytes() < 65536 {
                                return Err(anyhow!("The split size is too small."));
                            } else {
                                command2.arg(format!(
                                    "-v{}k",
                                    byte.get_adjusted_unit(ByteUnit::KiB).get_value().round()
                                        as u32
                                ));
                            }
                        }

                        command2.arg(output_path.as_path());

                        if cli_args.quiet {
                            process::exit(
                                command1.execute_multiple(&mut [&mut command2])?.unwrap_or(1),
                            );
                        } else {
                            let output = command1.execute_multiple_output(&mut [&mut command2])?;

                            process::exit(output.status.code().unwrap_or(1));
                        }
                    },
                    ArchiveFormat::TarZstd => {
                        if cpus > 1
                            && command_args!(&cli_args.executable_paths.pzstd_path, "-V")
                                .execute_check_exit_status_code(0)
                                .is_ok()
                        {
                            let mut command2 = command_args!(
                                &cli_args.executable_paths.pzstd_path,
                                "-p",
                                threads,
                                "-",
                                "-o",
                                output_path.as_path()
                            );

                            if cli_args.quiet {
                                command2.arg("-q");
                            }

                            match compression_level {
                                CompressionLevel::Best => {
                                    command2.args(["--ultra", "-22"]);
                                },
                                CompressionLevel::Fast => {
                                    command2.arg("-1");
                                },
                                CompressionLevel::Default => (),
                            }

                            let output = command1.execute_multiple_output(&mut [&mut command2])?;

                            process::exit(output.status.code().unwrap_or(1));
                        }

                        let mut command2 = command_args!(
                            &cli_args.executable_paths.zstd_path,
                            "-",
                            "-o",
                            output_path.as_path()
                        );

                        if cli_args.quiet {
                            command2.arg("-q");
                        }

                        match compression_level {
                            CompressionLevel::Best => {
                                command2.args(["--ultra", "-22"]);
                            },
                            CompressionLevel::Fast => {
                                command2.arg("-1");
                            },
                            CompressionLevel::Default => (),
                        }

                        let output = command1.execute_multiple_output(&mut [&mut command2])?;

                        process::exit(output.status.code().unwrap_or(1));
                    },
                    _ => unreachable!(),
                }
            },
            ArchiveFormat::Tar => {
                let mut command = command_args!(&cli_args.executable_paths.tar_path, "-c");

                if !cli_args.quiet {
                    command.arg("-v");
                }

                command.arg("-f");

                command.arg(output_path.as_path());

                for input_path in input_paths {
                    let input_path = input_path.absolutize()?;
                    let input_path_parent = input_path.parent().unwrap();
                    let file_name = input_path.file_name().unwrap();

                    command.arg("-C");
                    command.arg(input_path_parent);
                    command.arg(file_name);
                }

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
            },
            ArchiveFormat::Z => {
                if input_paths.len() > 1 || input_paths[0].is_dir() {
                    return Err(anyhow!(
                        "Obviously, you should use .tar.Z for filename extension to support \
                         multiple files."
                    ));
                }

                let input_path = &input_paths[0];

                let mut command =
                    command_args!(&cli_args.executable_paths.compress_path, "-c", input_path);

                command.stdout(File::create(output_path.as_path())?);

                let output = command.execute_output().map_err(|err| {
                    try_delete_file(output_path.as_path());
                    err
                })?;

                match output.status.code() {
                    Some(code) => {
                        if code != 0 {
                            try_delete_file(output_path.as_path());
                        }

                        process::exit(code);
                    },
                    None => {
                        try_delete_file(output_path.as_path());
                        process::exit(1);
                    },
                }
            },
            ArchiveFormat::Gzip => {
                if input_paths.len() > 1 || input_paths[0].is_dir() {
                    return Err(anyhow!(
                        "Obviously, you should use .tar.gz for filename extension to support \
                         multiple files."
                    ));
                }

                let input_path = &input_paths[0];

                if (cpus > 1 || compression_level == CompressionLevel::Best)
                    && command_args!(&cli_args.executable_paths.pigz_path, "-V")
                        .execute_check_exit_status_code(0)
                        .is_ok()
                {
                    let mut command = command_args!(
                        &cli_args.executable_paths.pigz_path,
                        "-c",
                        "-p",
                        threads,
                        input_path
                    );

                    if cli_args.quiet {
                        command.arg("-q");
                    }

                    match compression_level {
                        CompressionLevel::Best => {
                            command.arg("-9");
                        },
                        CompressionLevel::Fast => {
                            command.arg("-1");
                        },
                        CompressionLevel::Default => (),
                    }

                    command.stdout(File::create(output_path.as_path())?);

                    let output = command.execute_output().map_err(|err| {
                        try_delete_file(output_path.as_path());
                        err
                    })?;

                    match output.status.code() {
                        Some(code) => {
                            if code != 0 {
                                try_delete_file(output_path.as_path());
                            }

                            process::exit(code);
                        },
                        None => {
                            try_delete_file(output_path.as_path());
                            process::exit(1);
                        },
                    }
                }

                let mut command =
                    command_args!(&cli_args.executable_paths.gzip_path, "-c", input_path);

                if cli_args.quiet {
                    command.arg("-q");
                }

                match compression_level {
                    CompressionLevel::Best => {
                        command.arg("-9");
                    },
                    CompressionLevel::Fast => {
                        command.arg("-1");
                    },
                    CompressionLevel::Default => (),
                }

                command.stdout(File::create(output_path.as_path())?);

                let output = command.execute_output().map_err(|err| {
                    try_delete_file(output_path.as_path());
                    err
                })?;

                match output.status.code() {
                    Some(code) => {
                        if code != 0 {
                            try_delete_file(output_path.as_path());
                        }

                        process::exit(code);
                    },
                    None => {
                        try_delete_file(output_path.as_path());
                        process::exit(1);
                    },
                }
            },
            ArchiveFormat::Bzip2 => {
                if input_paths.len() > 1 || input_paths[0].is_dir() {
                    return Err(anyhow!(
                        "Obviously, you should use .tar.bz2 for filename extension to support \
                         multiple files."
                    ));
                }

                let input_path = &input_paths[0];

                if cpus > 1 {
                    if command_args!(&cli_args.executable_paths.lbzip2_path, "-V")
                        .execute_check_exit_status_code(0)
                        .is_ok()
                    {
                        let mut command = command_args!(
                            &cli_args.executable_paths.lbzip2_path,
                            "-z",
                            "-c",
                            "-n",
                            threads,
                            input_path
                        );

                        if cli_args.quiet {
                            command.arg("-q");
                        }

                        match compression_level {
                            CompressionLevel::Best => {
                                command.arg("-9");
                            },
                            CompressionLevel::Fast => {
                                command.arg("-1");
                            },
                            CompressionLevel::Default => (),
                        }

                        command.stdout(File::create(output_path.as_path())?);

                        let output = command.execute_output().map_err(|err| {
                            try_delete_file(output_path.as_path());
                            err
                        })?;

                        match output.status.code() {
                            Some(code) => {
                                if code != 0 {
                                    try_delete_file(output_path.as_path());
                                }

                                process::exit(code);
                            },
                            None => {
                                try_delete_file(output_path.as_path());
                                process::exit(1);
                            },
                        }
                    }

                    if command_args!(&cli_args.executable_paths.pbzip2_path, "-V")
                        .execute_check_exit_status_code(0)
                        .is_ok()
                    {
                        let mut command = command_args!(
                            &cli_args.executable_paths.pbzip2_path,
                            "-z",
                            "-c",
                            format!("-p{threads}"),
                            input_path
                        );

                        if cli_args.quiet {
                            command.arg("-q");
                        }

                        match compression_level {
                            CompressionLevel::Best => {
                                command.arg("-9");
                            },
                            CompressionLevel::Fast => {
                                command.arg("-1");
                            },
                            CompressionLevel::Default => (),
                        }

                        command.stdout(File::create(output_path.as_path())?);

                        let output = command.execute_output().map_err(|err| {
                            try_delete_file(output_path.as_path());
                            err
                        })?;

                        match output.status.code() {
                            Some(code) => {
                                if code != 0 {
                                    try_delete_file(output_path.as_path());
                                }

                                process::exit(code);
                            },
                            None => {
                                try_delete_file(output_path.as_path());
                                process::exit(1);
                            },
                        }
                    }
                }

                let mut command =
                    command_args!(&cli_args.executable_paths.bzip2_path, "-z", "-c", input_path);

                if cli_args.quiet {
                    command.arg("-q");
                }

                match compression_level {
                    CompressionLevel::Best => {
                        command.arg("-9");
                    },
                    CompressionLevel::Fast => {
                        command.arg("-1");
                    },
                    CompressionLevel::Default => (),
                }

                command.stdout(File::create(output_path.as_path())?);

                let output = command.execute_output().map_err(|err| {
                    try_delete_file(output_path.as_path());
                    err
                })?;

                match output.status.code() {
                    Some(code) => {
                        if code != 0 {
                            try_delete_file(output_path.as_path());
                        }

                        process::exit(code);
                    },
                    None => {
                        try_delete_file(output_path.as_path());
                        process::exit(1);
                    },
                }
            },
            ArchiveFormat::Lz => {
                if input_paths.len() > 1 || input_paths[0].is_dir() {
                    return Err(anyhow!(
                        "Obviously, you should use .tar.lz for filename extension to support \
                         multiple files."
                    ));
                }

                let input_path = &input_paths[0];

                if cpus > 1
                    && command_args!(&cli_args.executable_paths.plzip_path, "-V")
                        .execute_check_exit_status_code(0)
                        .is_ok()
                {
                    let mut command = command_args!(
                        &cli_args.executable_paths.plzip_path,
                        "-F",
                        "-c",
                        "-n",
                        threads,
                        input_path
                    );

                    if cli_args.quiet {
                        command.arg("-q");
                    }

                    match compression_level {
                        CompressionLevel::Best => {
                            command.arg("-9");
                        },
                        CompressionLevel::Fast => {
                            command.arg("-1");
                        },
                        CompressionLevel::Default => (),
                    }

                    command.stdout(File::create(output_path.as_path())?);

                    let output = command.execute_output().map_err(|err| {
                        try_delete_file(output_path.as_path());
                        err
                    })?;

                    match output.status.code() {
                        Some(code) => {
                            if code != 0 {
                                try_delete_file(output_path.as_path());
                            }

                            process::exit(code);
                        },
                        None => {
                            try_delete_file(output_path.as_path());
                            process::exit(1);
                        },
                    }
                }

                let mut command =
                    command_args!(&cli_args.executable_paths.lzip_path, "-F", "-c", input_path);

                if cli_args.quiet {
                    command.arg("-q");
                }

                match compression_level {
                    CompressionLevel::Best => {
                        command.arg("-9");
                    },
                    CompressionLevel::Fast => {
                        command.arg("-1");
                    },
                    CompressionLevel::Default => (),
                }

                command.stdout(File::create(output_path.as_path())?);

                let output = command.execute_output().map_err(|err| {
                    try_delete_file(output_path.as_path());
                    err
                })?;

                match output.status.code() {
                    Some(code) => {
                        if code != 0 {
                            try_delete_file(output_path.as_path());
                        }

                        process::exit(code);
                    },
                    None => {
                        try_delete_file(output_path.as_path());
                        process::exit(1);
                    },
                }
            },
            ArchiveFormat::Xz => {
                if input_paths.len() > 1 || input_paths[0].is_dir() {
                    return Err(anyhow!(
                        "Obviously, you should use .tar.xz for filename extension to support \
                         multiple files."
                    ));
                }

                let input_path = &input_paths[0];

                if cpus > 1
                    && command_args!(&cli_args.executable_paths.pxz_path, "-V")
                        .execute_check_exit_status_code(0)
                        .is_ok()
                {
                    let mut command = command_args!(
                        &cli_args.executable_paths.pxz_path,
                        "-z",
                        "-c",
                        "-T",
                        threads,
                        input_path
                    );

                    if cli_args.quiet {
                        command.arg("-q");
                    }

                    match compression_level {
                        CompressionLevel::Best => {
                            command.args(["-9", "-e"]);
                        },
                        CompressionLevel::Fast => {
                            command.arg("-0");
                        },
                        CompressionLevel::Default => (),
                    }

                    command.stdout(File::create(output_path.as_path())?);

                    let output = command.execute_output().map_err(|err| {
                        try_delete_file(output_path.as_path());
                        err
                    })?;

                    match output.status.code() {
                        Some(code) => {
                            if code != 0 {
                                try_delete_file(output_path.as_path());
                            }

                            process::exit(code);
                        },
                        None => {
                            try_delete_file(output_path.as_path());
                            process::exit(1);
                        },
                    }
                }

                let mut command =
                    command_args!(&cli_args.executable_paths.xz_path, "-z", "-c", input_path);

                if cli_args.quiet {
                    command.arg("-q");
                }

                match compression_level {
                    CompressionLevel::Best => {
                        command.args(["-9", "-e"]);
                    },
                    CompressionLevel::Fast => {
                        command.arg("-0");
                    },
                    CompressionLevel::Default => (),
                }

                command.stdout(File::create(output_path.as_path())?);

                let output = command.execute_output().map_err(|err| {
                    try_delete_file(output_path.as_path());
                    err
                })?;

                match output.status.code() {
                    Some(code) => {
                        if code != 0 {
                            try_delete_file(output_path.as_path());
                        }

                        process::exit(code);
                    },
                    None => {
                        try_delete_file(output_path.as_path());
                        process::exit(1);
                    },
                }
            },
            ArchiveFormat::Lzma => {
                if input_paths.len() > 1 || input_paths[0].is_dir() {
                    return Err(anyhow!(
                        "Obviously, you should use .tar.lzma for filename extension to support \
                         multiple files."
                    ));
                }

                let input_path = &input_paths[0];

                if cpus > 1
                    && command_args!(&cli_args.executable_paths.pxz_path, "-V")
                        .execute_check_exit_status_code(0)
                        .is_ok()
                {
                    let mut command = command_args!(
                        &cli_args.executable_paths.pxz_path,
                        "-z",
                        "-c",
                        "-T",
                        threads,
                        "-F",
                        "lzma",
                        input_path
                    );

                    if cli_args.quiet {
                        command.arg("-q");
                    }

                    match compression_level {
                        CompressionLevel::Best => {
                            command.args(["-9", "-e"]);
                        },
                        CompressionLevel::Fast => {
                            command.arg("-0");
                        },
                        CompressionLevel::Default => (),
                    }

                    command.stdout(File::create(output_path.as_path())?);

                    let output = command.execute_output().map_err(|err| {
                        try_delete_file(output_path.as_path());
                        err
                    })?;

                    match output.status.code() {
                        Some(code) => {
                            if code != 0 {
                                try_delete_file(output_path.as_path());
                            }

                            process::exit(code);
                        },
                        None => {
                            try_delete_file(output_path.as_path());
                            process::exit(1);
                        },
                    }
                }

                let mut command =
                    command_args!(&cli_args.executable_paths.lzma_path, "-z", "-c", input_path);

                if cli_args.quiet {
                    command.arg("-q");
                }

                match compression_level {
                    CompressionLevel::Best => {
                        command.args(["-9", "-e"]);
                    },
                    CompressionLevel::Fast => {
                        command.arg("-0");
                    },
                    CompressionLevel::Default => (),
                }

                command.stdout(File::create(output_path.as_path())?);

                let output = command.execute_output().map_err(|err| {
                    try_delete_file(output_path.as_path());
                    err
                })?;

                match output.status.code() {
                    Some(code) => {
                        if code != 0 {
                            try_delete_file(output_path.as_path());
                        }

                        process::exit(code);
                    },
                    None => {
                        try_delete_file(output_path.as_path());
                        process::exit(1);
                    },
                }
            },
            ArchiveFormat::P7z => {
                let password = read_password(cli_args.password)?;

                let mut command = command_args!(
                    &cli_args.executable_paths.p7z_path,
                    "a",
                    "-t7z",
                    "-aoa",
                    format!("-mmt{threads}")
                );

                match compression_level {
                    CompressionLevel::Best => {
                        command.args(["-m0=lzma2", "-mx", "-ms=on"]);
                    },
                    CompressionLevel::Fast => {
                        command.arg("-m0=copy");
                    },
                    CompressionLevel::Default => (),
                }

                if !password.is_empty() {
                    command.arg("-mhe=on");
                    command.arg(format!("-p{password}"));
                }

                if let Some(d) = split {
                    let mut volume = String::from("-v");

                    let byte = Byte::from_str(d)?;

                    if byte.get_bytes() < 65536 {
                        return Err(anyhow!("The split size is too small."));
                    } else {
                        volume.push_str(
                            format!(
                                "{}k",
                                byte.get_adjusted_unit(ByteUnit::KiB).get_value().round() as u32
                            )
                            .as_str(),
                        );
                    }

                    command.arg(volume.as_str());
                }

                command.arg(output_path.as_path());

                command.args(input_paths);

                if cli_args.quiet {
                    process::exit(command.execute()?.unwrap_or(1));
                } else {
                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }
            },
            ArchiveFormat::Zip => {
                let password = read_password(cli_args.password)?;

                let split = if let Some(d) = split {
                    let byte = Byte::from_str(d)?;

                    if byte.get_bytes() < 65536 {
                        return Err(anyhow!("The split size is too small."));
                    }

                    Some(byte)
                } else {
                    None
                };

                let output_tmp_path = if split.is_some() {
                    let new_filename =
                        format!("{}.tmp.zip", output_path.file_stem().unwrap().to_string_lossy());

                    let output_tmp_path = output_path.parent().unwrap().join(new_filename);

                    if let Ok(metadata) = output_tmp_path.metadata() {
                        if metadata.is_dir() {
                            return Err(anyhow!("{output_path:?} is a directory."));
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

                match compression_level {
                    CompressionLevel::Best => {
                        command.arg("-mx");
                    },
                    CompressionLevel::Fast => {
                        command.arg("-mx=0");
                    },
                    CompressionLevel::Default => (),
                }

                if !password.is_empty() {
                    command.arg(format!("-p{password}"));
                }

                command.arg(output_tmp_path.as_ref());

                command.args(input_paths);

                let exit_code = if cli_args.quiet {
                    command.execute()?
                } else {
                    let output = command.execute_output()?;

                    output.status.code()
                };

                if let Some(byte) = split {
                    match exit_code {
                        Some(code) => {
                            if code != 0 {
                                try_delete_file(output_path.as_path());
                                process::exit(code);
                            }
                        },
                        None => {
                            try_delete_file(output_path.as_path());
                            process::exit(1);
                        },
                    }

                    let mut command = command_args!(
                        &cli_args.executable_paths.zip_path,
                        "-s",
                        format!(
                            "{}k",
                            byte.get_adjusted_unit(ByteUnit::KiB).get_value().round() as u32
                        )
                    );

                    match compression_level {
                        CompressionLevel::Best => {
                            command.arg("-9");
                        },
                        CompressionLevel::Fast => {
                            command.arg("-0");
                        },
                        CompressionLevel::Default => (),
                    }

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

                    let output = command.execute_output().map_err(|err| {
                        try_delete_file(output_tmp_path.as_ref());
                        err
                    })?;

                    match output.status.code() {
                        Some(code) => {
                            try_delete_file(output_tmp_path.as_ref());
                            process::exit(code);
                        },
                        None => {
                            try_delete_file(output_tmp_path.as_ref());
                            try_delete_file(output_path.as_path());
                            process::exit(1);
                        },
                    }
                }

                process::exit(exit_code.unwrap_or(1));
            },
            ArchiveFormat::Rar => {
                let password = read_password(cli_args.password)?;

                let mut command = command_args!(
                    &cli_args.executable_paths.rar_path,
                    "a",
                    "-ep1",
                    format!("-mt{threads}")
                );

                match compression_level {
                    CompressionLevel::Best => {
                        command.args(["-ma5", "-m5", "-s"]);
                    },
                    CompressionLevel::Fast => {
                        command.arg("-m0");
                    },
                    CompressionLevel::Default => (),
                }

                if !password.is_empty() {
                    command.arg(format!("-hp{password}"));
                }

                if cli_args.quiet {
                    command.arg("-idq");
                }

                if let Some(d) = split {
                    let byte = Byte::from_str(d)?;

                    if byte.get_bytes() < 65536 {
                        return Err(anyhow!("The split size is too small."));
                    } else {
                        command.arg(format!(
                            "-v{}k",
                            byte.get_adjusted_unit(ByteUnit::KiB).get_value().round() as u32
                        ));
                    }
                }

                if let Some(rr) = recovery_record {
                    command.arg(format!("-rr{rr}",));
                }

                command.arg(output_path.as_path());

                command.args(input_paths);

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
            },
            ArchiveFormat::Zstd => {
                if input_paths.len() > 1 || input_paths[0].is_dir() {
                    return Err(anyhow!(
                        "Obviously, you should use .tar.zst for filename extension to support \
                         multiple files."
                    ));
                }

                let input_path = &input_paths[0];

                if cpus > 1
                    && command_args!(&cli_args.executable_paths.pzstd_path, "-V")
                        .execute_check_exit_status_code(0)
                        .is_ok()
                {
                    let mut command = command_args!(
                        &cli_args.executable_paths.pzstd_path,
                        "-p",
                        threads,
                        input_path,
                        "-o",
                        output_path.as_path()
                    );

                    if cli_args.quiet {
                        command.arg("-q");
                    }

                    match compression_level {
                        CompressionLevel::Best => {
                            command.args(["--ultra", "-22"]);
                        },
                        CompressionLevel::Fast => {
                            command.arg("-1");
                        },
                        CompressionLevel::Default => (),
                    }

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }

                let mut command = command_args!(
                    &cli_args.executable_paths.zstd_path,
                    input_path,
                    "-o",
                    output_path.as_path()
                );

                if cli_args.quiet {
                    command.arg("-q");
                }

                match compression_level {
                    CompressionLevel::Best => {
                        command.args(["--ultra", "-22"]);
                    },
                    CompressionLevel::Fast => {
                        command.arg("-1");
                    },
                    CompressionLevel::Default => (),
                }

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
            },
        }
    }

    Ok(())
}
