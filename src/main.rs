use std::{
    borrow::Cow,
    error::Error,
    fs::{self, File},
    io::{self, Write},
    path::Path,
    process,
};

use byte_unit::{Byte, ByteUnit};
use clap::{Arg, ArgMatches, Command};
use concat_with::concat_line;
use execute::{command_args, Execute};
use path_absolutize::{Absolutize, CWD};
use scanner_rust::{generic_array::typenum::U32, Scanner};
use terminal_size::terminal_size;
use xcompress::*;

const APP_NAME: &str = "XCompress";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

const DEFAULT_COMPRESS_PATH: &str = "compress";
const DEFAULT_ZIP_PATH: &str = "zip";
const DEFAULT_UNZIP_PATH: &str = "unzip";
const DEFAULT_GZIP_PATH: &str = "gzip";
const DEFAULT_GUNZIP_PATH: &str = "gunzip";
const DEFAULT_PIGZ_PATH: &str = "pigz";
const DEFAULT_BZIP2_PATH: &str = "bzip2";
const DEFAULT_BUNZIP2_PATH: &str = "bunzip2";
const DEFAULT_LBZIP2_PATH: &str = "lbzip2";
const DEFAULT_PBZIP2_PATH: &str = "pbzip2";
const DEFAULT_LZIP_PATH: &str = "lzip";
const DEFAULT_LUNZIP_PATH: &str = "lunzip";
const DEFAULT_PLZIP_PATH: &str = "plzip";
const DEFAULT_XZ_PATH: &str = "xz";
const DEFAULT_UNXZ_PATH: &str = "unxz";
const DEFAULT_PXZ_PATH: &str = "pxz";
const DEFAULT_LZMA_PATH: &str = "lzma";
const DEFAULT_UNLZMA_PATH: &str = "unlzma";
const DEFAULT_7Z_PATH: &str = "7z";
const DEFAULT_TAR_PATH: &str = "tar";
const DEFAULT_RAR_PATH: &str = "rar";
const DEFAULT_UNRAR_PATH: &str = "unrar";
const DEFAULT_ZSTD_PATH: &str = "zstd";
const DEFAULT_UNZSTD_PATH: &str = "unzstd";
const DEFAULT_PZSTD_PATH: &str = "pzstd";

fn main() -> Result<(), Box<dyn Error>> {
    let matches = get_matches();

    if let Some(sub_matches) = matches.subcommand_matches("x") {
        let input_path = sub_matches.value_of("INPUT_PATH").unwrap();

        let input_path = Path::new(input_path);

        if !input_path.exists() {
            return Err(
                format!("{} does not exist.", input_path.absolutize()?.to_string_lossy()).into()
            );
        }

        let mut output_path = sub_matches.value_of("OUTPUT_PATH");
        let output_path2 = sub_matches.value_of("OUTPUT_PATH2");

        if let Some(a) = output_path.as_ref() {
            if let Some(b) = output_path2.as_ref() {
                if a != b {
                    return Err("You input different output paths.".into());
                }
            }
        } else {
            output_path = output_path2;
        }

        let output_path = match output_path {
            Some(output_path) => Path::new(output_path),
            None => CWD.as_path(),
        };

        handle_extract(&matches, input_path, output_path)
    } else if let Some(sub_matches) = matches.subcommand_matches("a") {
        let input_paths_iter = sub_matches.values_of("INPUT_PATH").unwrap();

        let mut input_paths = Vec::with_capacity(input_paths_iter.len());

        for input_path in input_paths_iter {
            let input_path = Path::new(input_path);

            if !input_path.exists() {
                return Err(format!(
                    "{} does not exist.",
                    input_path.absolutize()?.to_string_lossy()
                )
                .into());
            }

            input_paths.push(input_path);
        }

        let output_path = sub_matches.value_of("OUTPUT_PATH");

        let output_path = match output_path {
            Some(output_path) => Cow::from(Path::new(output_path)),
            None => Cow::from(CWD.join(format!(
                "{}.rar",
                input_paths[0].absolutize()?.file_name().unwrap().to_string_lossy()
            ))),
        };

        let best_compression = sub_matches.is_present("BEST_COMPRESSION");

        let split = sub_matches.value_of("SPLIT");

        handle_archive(&matches, &input_paths, output_path, best_compression, split)
    } else {
        Err("Please input a subcommand. Use `help` to see how to use this program.".into())
    }
}

fn handle_archive(
    matches: &ArgMatches,
    input_paths: &[&Path],
    output_path: Cow<Path>,
    best_compression: bool,
    split: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let single_thread = matches.is_present("SINGLE_THREAD");
    let quiet = matches.is_present("QUIET");

    let cpus = if single_thread { 1 } else { num_cpus::get() };

    let format = ArchiveFormat::get_archive_format_from_file_path(output_path.as_ref())?;

    let output_path = output_path.absolutize()?;

    match output_path.metadata() {
        Ok(metadata) => {
            if metadata.is_dir() {
                return Err(format!("{} is a directory.", output_path.to_string_lossy()).into());
            }

            fs::remove_file(output_path.as_ref())?;
        },
        Err(_) => {
            let output_path_parent = output_path.parent().unwrap();

            fs::create_dir_all(output_path_parent)?;
        },
    }

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
            let tar_path = matches.value_of("TAR_PATH").unwrap();

            let mut command1 = command_args!(tar_path, "-c", "-f", "-");

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
                    let compress_path = matches.value_of("COMPRESS_PATH").unwrap();

                    let mut command2 = command_args!(compress_path, "-c", "-");

                    command2.stdout(File::create(output_path.as_ref())?);

                    let output =
                        command1.execute_multiple_output(&mut [&mut command2]).map_err(|err| {
                            try_delete_file(output_path.as_ref());
                            err
                        })?;

                    match output.status.code() {
                        Some(code) => {
                            if code != 0 && code != 2 {
                                try_delete_file(output_path.as_ref());
                            }

                            process::exit(code);
                        },
                        None => {
                            try_delete_file(output_path.as_ref());
                            process::exit(1);
                        },
                    }
                },
                ArchiveFormat::TarGzip => {
                    if cpus > 1 || best_compression {
                        let pigz_path = matches.value_of("PIGZ_PATH").unwrap();

                        if command_args!(pigz_path, "-V").execute_check_exit_status_code(0).is_ok()
                        {
                            let mut command2 = command_args!(pigz_path, "-c", "-p", threads, "-");

                            if quiet {
                                command2.arg("-q");
                            }

                            if best_compression {
                                command2.arg("-11");
                            }

                            command2.stdout(File::create(output_path.as_ref())?);

                            let output = command1
                                .execute_multiple_output(&mut [&mut command2])
                                .map_err(|err| {
                                    try_delete_file(output_path.as_ref());
                                    err
                                })?;

                            match output.status.code() {
                                Some(code) => {
                                    if code != 0 {
                                        try_delete_file(output_path.as_ref());
                                    }

                                    process::exit(code);
                                },
                                None => {
                                    try_delete_file(output_path.as_ref());
                                    process::exit(1);
                                },
                            }
                        }
                    }

                    let gzip_path = matches.value_of("GZIP_PATH").unwrap();

                    let mut command2 = command_args!(gzip_path, "-c", "-");

                    if quiet {
                        command2.arg("-q");
                    }

                    if best_compression {
                        command2.arg("-9");
                    }

                    command2.stdout(File::create(output_path.as_ref())?);

                    let output =
                        command1.execute_multiple_output(&mut [&mut command2]).map_err(|err| {
                            try_delete_file(output_path.as_ref());
                            err
                        })?;

                    match output.status.code() {
                        Some(code) => {
                            if code != 0 {
                                try_delete_file(output_path.as_ref());
                            }

                            process::exit(code);
                        },
                        None => {
                            try_delete_file(output_path.as_ref());
                            process::exit(1);
                        },
                    }
                },
                ArchiveFormat::TarBzip2 => {
                    if cpus > 1 {
                        let lbzip2_path = matches.value_of("LBZIP2_PATH").unwrap();

                        if command_args!(lbzip2_path, "-V")
                            .execute_check_exit_status_code(0)
                            .is_ok()
                        {
                            let mut command2 =
                                command_args!(lbzip2_path, "-z", "-c", "-n", threads, "-");

                            if quiet {
                                command2.arg("-q");
                            }

                            if best_compression {
                                command2.arg("-9");
                            }

                            command2.stdout(File::create(output_path.as_ref())?);

                            let output = command1
                                .execute_multiple_output(&mut [&mut command2])
                                .map_err(|err| {
                                    try_delete_file(output_path.as_ref());
                                    err
                                })?;

                            match output.status.code() {
                                Some(code) => {
                                    if code != 0 {
                                        try_delete_file(output_path.as_ref());
                                    }

                                    process::exit(code);
                                },
                                None => {
                                    try_delete_file(output_path.as_ref());
                                    process::exit(1);
                                },
                            }
                        }

                        let pbzip2_path = matches.value_of("PBZIP2_PATH").unwrap();

                        if command_args!(pbzip2_path, "-V")
                            .execute_check_exit_status_code(0)
                            .is_ok()
                        {
                            let mut command2 = command_args!(
                                pbzip2_path,
                                "-z",
                                "-c",
                                format!("-p{}", threads),
                                "-"
                            );

                            if quiet {
                                command2.arg("-q");
                            }

                            if best_compression {
                                command2.arg("-9");
                            }

                            command2.stdout(File::create(output_path.as_ref())?);

                            let output = command1
                                .execute_multiple_output(&mut [&mut command2])
                                .map_err(|err| {
                                    try_delete_file(output_path.as_ref());
                                    err
                                })?;

                            match output.status.code() {
                                Some(code) => {
                                    if code != 0 {
                                        try_delete_file(output_path.as_ref());
                                    }

                                    process::exit(code);
                                },
                                None => {
                                    try_delete_file(output_path.as_ref());
                                    process::exit(1);
                                },
                            }
                        }
                    }

                    let bzip2_path = matches.value_of("BZIP2_PATH").unwrap();

                    let mut command2 = command_args!(bzip2_path, "-z", "-c", "-");

                    if quiet {
                        command2.arg("-q");
                    }

                    if best_compression {
                        command2.arg("-9");
                    }

                    command2.stdout(File::create(output_path.as_ref())?);

                    let output =
                        command1.execute_multiple_output(&mut [&mut command2]).map_err(|err| {
                            try_delete_file(output_path.as_ref());
                            err
                        })?;

                    match output.status.code() {
                        Some(code) => {
                            if code != 0 {
                                try_delete_file(output_path.as_ref());
                            }

                            process::exit(code);
                        },
                        None => {
                            try_delete_file(output_path.as_ref());
                            process::exit(1);
                        },
                    }
                },
                ArchiveFormat::TarLz => {
                    if cpus > 1 {
                        let plzip_path = matches.value_of("PLZIP_PATH").unwrap();

                        if command_args!(plzip_path, "-V").execute_check_exit_status_code(0).is_ok()
                        {
                            let mut command2 =
                                command_args!(plzip_path, "-F", "-c", "-n", threads, "-");

                            if quiet {
                                command2.arg("-q");
                            }

                            if best_compression {
                                command2.arg("-9");
                            }

                            command2.stdout(File::create(output_path.as_ref())?);

                            let output = command1
                                .execute_multiple_output(&mut [&mut command2])
                                .map_err(|err| {
                                    try_delete_file(output_path.as_ref());
                                    err
                                })?;

                            match output.status.code() {
                                Some(code) => {
                                    if code != 0 {
                                        try_delete_file(output_path.as_ref());
                                    }

                                    process::exit(code);
                                },
                                None => {
                                    try_delete_file(output_path.as_ref());
                                    process::exit(1);
                                },
                            }
                        }
                    }

                    let lzip_path = matches.value_of("LZIP_PATH").unwrap();

                    let mut command2 = command_args!(lzip_path, "-F", "-c", "-");

                    if quiet {
                        command2.arg("-q");
                    }

                    if best_compression {
                        command2.arg("-9");
                    }

                    command2.stdout(File::create(output_path.as_ref())?);

                    let output =
                        command1.execute_multiple_output(&mut [&mut command2]).map_err(|err| {
                            try_delete_file(output_path.as_ref());
                            err
                        })?;

                    match output.status.code() {
                        Some(code) => {
                            if code != 0 {
                                try_delete_file(output_path.as_ref());
                            }

                            process::exit(code);
                        },
                        None => {
                            try_delete_file(output_path.as_ref());
                            process::exit(1);
                        },
                    }
                },
                ArchiveFormat::TarXz => {
                    if cpus > 1 {
                        let pxz_path = matches.value_of("PXZ_PATH").unwrap();

                        if command_args!(pxz_path, "-V").execute_check_exit_status_code(0).is_ok() {
                            let mut command2 =
                                command_args!(pxz_path, "-z", "-c", "-T", threads, "-");

                            if quiet {
                                command2.arg("-q");
                            }

                            if best_compression {
                                command2.args(["-9", "-e"]);
                            }

                            command2.stdout(File::create(output_path.as_ref())?);

                            let output = command1
                                .execute_multiple_output(&mut [&mut command2])
                                .map_err(|err| {
                                    try_delete_file(output_path.as_ref());
                                    err
                                })?;

                            match output.status.code() {
                                Some(code) => {
                                    if code != 0 {
                                        try_delete_file(output_path.as_ref());
                                    }

                                    process::exit(code);
                                },
                                None => {
                                    try_delete_file(output_path.as_ref());
                                    process::exit(1);
                                },
                            }
                        }
                    }

                    let xz_path = matches.value_of("XZ_PATH").unwrap();

                    let mut command2 = command_args!(xz_path, "-z", "-c", "-");

                    if quiet {
                        command2.arg("-q");
                    }

                    if best_compression {
                        command2.args(["-9", "-e"]);
                    }

                    command2.stdout(File::create(output_path.as_ref())?);

                    let output =
                        command1.execute_multiple_output(&mut [&mut command2]).map_err(|err| {
                            try_delete_file(output_path.as_ref());
                            err
                        })?;

                    match output.status.code() {
                        Some(code) => {
                            if code != 0 {
                                try_delete_file(output_path.as_ref());
                            }

                            process::exit(code);
                        },
                        None => {
                            try_delete_file(output_path.as_ref());
                            process::exit(1);
                        },
                    }
                },
                ArchiveFormat::TarLzma => {
                    if cpus > 1 {
                        let pxz_path = matches.value_of("PXZ_PATH").unwrap();

                        if command_args!(pxz_path, "-V").execute_check_exit_status_code(0).is_ok() {
                            let mut command2 = command_args!(
                                pxz_path, "-z", "-c", "-T", threads, "-F", "lzma", "-"
                            );

                            if quiet {
                                command2.arg("-q");
                            }

                            if best_compression {
                                command2.args(["-9", "-e"]);
                            }

                            command2.stdout(File::create(output_path.as_ref())?);

                            let output = command1
                                .execute_multiple_output(&mut [&mut command2])
                                .map_err(|err| {
                                    try_delete_file(output_path.as_ref());
                                    err
                                })?;

                            match output.status.code() {
                                Some(code) => {
                                    if code != 0 {
                                        try_delete_file(output_path.as_ref());
                                    }

                                    process::exit(code);
                                },
                                None => {
                                    try_delete_file(output_path.as_ref());
                                    process::exit(1);
                                },
                            }
                        }
                    }

                    let lzma_path = matches.value_of("LZMA_PATH").unwrap();

                    let mut command2 = command_args!(lzma_path, "-z", "-c", "-");

                    if quiet {
                        command2.arg("-q");
                    }

                    if best_compression {
                        command2.args(["-9", "-e"]);
                    }

                    command2.stdout(File::create(output_path.as_ref())?);

                    let output =
                        command1.execute_multiple_output(&mut [&mut command2]).map_err(|err| {
                            try_delete_file(output_path.as_ref());
                            err
                        })?;

                    match output.status.code() {
                        Some(code) => {
                            if code != 0 {
                                try_delete_file(output_path.as_ref());
                            }

                            process::exit(code);
                        },
                        None => {
                            try_delete_file(output_path.as_ref());
                            process::exit(1);
                        },
                    }
                },
                ArchiveFormat::Tar7z => {
                    let p7z_path = matches.value_of("7Z_PATH").unwrap();

                    let password = matches.value_of("PASSWORD");
                    let password = read_password(password)?;

                    let mut command2 = command_args!(
                        p7z_path,
                        "a",
                        "-t7z",
                        "-aoa",
                        format!("-mmt{}", threads),
                        "-si",
                    );

                    if best_compression {
                        command2.args(["-m0=lzma2", "-mx", "-ms=on"]);
                    }

                    if !password.is_empty() {
                        command2.arg("-mhe=on");
                        command2.arg(format!("-p{}", password));
                    }

                    if let Some(d) = split {
                        let byte = Byte::from_str(d)?;

                        if byte.get_bytes() < 65536 {
                            return Err("The split size is too small.".into());
                        } else {
                            command2.arg(format!(
                                "-v{}k",
                                byte.get_adjusted_unit(ByteUnit::KiB).get_value().round() as u32
                            ));
                        }
                    }

                    command2.arg(output_path.as_ref());

                    if quiet {
                        process::exit(
                            command1.execute_multiple(&mut [&mut command2])?.unwrap_or(1),
                        );
                    } else {
                        let output = command1.execute_multiple_output(&mut [&mut command2])?;

                        process::exit(output.status.code().unwrap_or(1));
                    }
                },
                ArchiveFormat::TarZstd => {
                    if cpus > 1 {
                        let pzstd_path = matches.value_of("PZSTD_PATH").unwrap();

                        if command_args!(pzstd_path, "-V").execute_check_exit_status_code(0).is_ok()
                        {
                            let mut command2 = command_args!(
                                pzstd_path,
                                "-p",
                                threads,
                                "-",
                                "-o",
                                output_path.as_ref()
                            );

                            if quiet {
                                command2.arg("-q");
                            }

                            if best_compression {
                                command2.args(["--ultra", "-22"]);
                            }

                            let output = command1.execute_multiple_output(&mut [&mut command2])?;

                            process::exit(output.status.code().unwrap_or(1));
                        }
                    }

                    let zstd_path = matches.value_of("ZSTD_PATH").unwrap();

                    let mut command2 = command_args!(zstd_path, "-", "-o", output_path.as_ref());

                    if quiet {
                        command2.arg("-q");
                    }

                    if best_compression {
                        command2.args(["--ultra", "-22"]);
                    }

                    let output = command1.execute_multiple_output(&mut [&mut command2])?;

                    process::exit(output.status.code().unwrap_or(1));
                },
                _ => unreachable!(),
            }
        },
        ArchiveFormat::Tar => {
            let tar_path = matches.value_of("TAR_PATH").unwrap();

            let mut command = command_args!(tar_path, "-c");

            if !quiet {
                command.arg("-v");
            }

            command.arg("-f");

            command.arg(output_path.as_ref());

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
                return Err("Obviously, you should use .tar.Z for filename extension to support \
                            multiple files."
                    .into());
            }

            let input_path = &input_paths[0];

            let compress_path = matches.value_of("COMPRESS_PATH").unwrap();

            let mut command = command_args!(compress_path, "-c", input_path);

            command.stdout(File::create(output_path.as_ref())?);

            let output = command.execute_output().map_err(|err| {
                try_delete_file(output_path.as_ref());
                err
            })?;

            match output.status.code() {
                Some(code) => {
                    if code != 0 {
                        try_delete_file(output_path.as_ref());
                    }

                    process::exit(code);
                },
                None => {
                    try_delete_file(output_path.as_ref());
                    process::exit(1);
                },
            }
        },
        ArchiveFormat::Gzip => {
            if input_paths.len() > 1 || input_paths[0].is_dir() {
                return Err("Obviously, you should use .tar.gz for filename extension to support \
                            multiple files."
                    .into());
            }

            let input_path = &input_paths[0];

            if cpus > 1 || best_compression {
                let pigz_path = matches.value_of("PIGZ_PATH").unwrap();

                if command_args!(pigz_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command = command_args!(pigz_path, "-c", "-p", threads, input_path);

                    if quiet {
                        command.arg("-q");
                    }

                    if best_compression {
                        command.arg("-11");
                    }

                    command.stdout(File::create(output_path.as_ref())?);

                    let output = command.execute_output().map_err(|err| {
                        try_delete_file(output_path.as_ref());
                        err
                    })?;

                    match output.status.code() {
                        Some(code) => {
                            if code != 0 {
                                try_delete_file(output_path.as_ref());
                            }

                            process::exit(code);
                        },
                        None => {
                            try_delete_file(output_path.as_ref());
                            process::exit(1);
                        },
                    }
                }
            }

            let gzip_path = matches.value_of("GZIP_PATH").unwrap();

            let mut command = command_args!(gzip_path, "-c", input_path);

            if quiet {
                command.arg("-q");
            }

            if best_compression {
                command.arg("-9");
            }

            command.stdout(File::create(output_path.as_ref())?);

            let output = command.execute_output().map_err(|err| {
                try_delete_file(output_path.as_ref());
                err
            })?;

            match output.status.code() {
                Some(code) => {
                    if code != 0 {
                        try_delete_file(output_path.as_ref());
                    }

                    process::exit(code);
                },
                None => {
                    try_delete_file(output_path.as_ref());
                    process::exit(1);
                },
            }
        },
        ArchiveFormat::Bzip2 => {
            if input_paths.len() > 1 || input_paths[0].is_dir() {
                return Err("Obviously, you should use .tar.bz2 for filename extension to \
                            support multiple files."
                    .into());
            }

            let input_path = &input_paths[0];

            if cpus > 1 {
                let lbzip2_path = matches.value_of("LBZIP2_PATH").unwrap();

                if command_args!(lbzip2_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command =
                        command_args!(lbzip2_path, "-z", "-c", "-n", threads, input_path);

                    if quiet {
                        command.arg("-q");
                    }

                    if best_compression {
                        command.arg("-9");
                    }

                    command.stdout(File::create(output_path.as_ref())?);

                    let output = command.execute_output().map_err(|err| {
                        try_delete_file(output_path.as_ref());
                        err
                    })?;

                    match output.status.code() {
                        Some(code) => {
                            if code != 0 {
                                try_delete_file(output_path.as_ref());
                            }

                            process::exit(code);
                        },
                        None => {
                            try_delete_file(output_path.as_ref());
                            process::exit(1);
                        },
                    }
                }

                let pbzip2_path = matches.value_of("PBZIP2_PATH").unwrap();

                if command_args!(pbzip2_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command = command_args!(
                        pbzip2_path,
                        "-z",
                        "-c",
                        format!("-p{}", threads),
                        input_path
                    );

                    if quiet {
                        command.arg("-q");
                    }

                    if best_compression {
                        command.arg("-9");
                    }

                    command.stdout(File::create(output_path.as_ref())?);

                    let output = command.execute_output().map_err(|err| {
                        try_delete_file(output_path.as_ref());
                        err
                    })?;

                    match output.status.code() {
                        Some(code) => {
                            if code != 0 {
                                try_delete_file(output_path.as_ref());
                            }

                            process::exit(code);
                        },
                        None => {
                            try_delete_file(output_path.as_ref());
                            process::exit(1);
                        },
                    }
                }
            }

            let bzip2_path = matches.value_of("BZIP2_PATH").unwrap();

            let mut command = command_args!(bzip2_path, "-z", "-c", input_path);

            if quiet {
                command.arg("-q");
            }

            if best_compression {
                command.arg("-9");
            }

            command.stdout(File::create(output_path.as_ref())?);

            let output = command.execute_output().map_err(|err| {
                try_delete_file(output_path.as_ref());
                err
            })?;

            match output.status.code() {
                Some(code) => {
                    if code != 0 {
                        try_delete_file(output_path.as_ref());
                    }

                    process::exit(code);
                },
                None => {
                    try_delete_file(output_path.as_ref());
                    process::exit(1);
                },
            }
        },
        ArchiveFormat::Lz => {
            if input_paths.len() > 1 || input_paths[0].is_dir() {
                return Err("Obviously, you should use .tar.lz for filename extension to support \
                            multiple files."
                    .into());
            }

            let input_path = &input_paths[0];

            if cpus > 1 {
                let plzip_path = matches.value_of("PLZIP_PATH").unwrap();

                if command_args!(plzip_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command =
                        command_args!(plzip_path, "-F", "-c", "-n", threads, input_path);

                    if quiet {
                        command.arg("-q");
                    }

                    if best_compression {
                        command.arg("-9");
                    }

                    command.stdout(File::create(output_path.as_ref())?);

                    let output = command.execute_output().map_err(|err| {
                        try_delete_file(output_path.as_ref());
                        err
                    })?;

                    match output.status.code() {
                        Some(code) => {
                            if code != 0 {
                                try_delete_file(output_path.as_ref());
                            }

                            process::exit(code);
                        },
                        None => {
                            try_delete_file(output_path.as_ref());
                            process::exit(1);
                        },
                    }
                }
            }

            let lzip_path = matches.value_of("LZIP_PATH").unwrap();

            let mut command = command_args!(lzip_path, "-F", "-c", input_path);

            if quiet {
                command.arg("-q");
            }

            if best_compression {
                command.arg("-9");
            }

            command.stdout(File::create(output_path.as_ref())?);

            let output = command.execute_output().map_err(|err| {
                try_delete_file(output_path.as_ref());
                err
            })?;

            match output.status.code() {
                Some(code) => {
                    if code != 0 {
                        try_delete_file(output_path.as_ref());
                    }

                    process::exit(code);
                },
                None => {
                    try_delete_file(output_path.as_ref());
                    process::exit(1);
                },
            }
        },
        ArchiveFormat::Xz => {
            if input_paths.len() > 1 || input_paths[0].is_dir() {
                return Err("Obviously, you should use .tar.xz for filename extension to support \
                            multiple files."
                    .into());
            }

            let input_path = &input_paths[0];

            if cpus > 1 {
                let pxz_path = matches.value_of("PXZ_PATH").unwrap();

                if command_args!(pxz_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command =
                        command_args!(pxz_path, "-z", "-c", "-T", threads, input_path);

                    if quiet {
                        command.arg("-q");
                    }

                    if best_compression {
                        command.args(["-9", "-e"]);
                    }

                    command.stdout(File::create(output_path.as_ref())?);

                    let output = command.execute_output().map_err(|err| {
                        try_delete_file(output_path.as_ref());
                        err
                    })?;

                    match output.status.code() {
                        Some(code) => {
                            if code != 0 {
                                try_delete_file(output_path.as_ref());
                            }

                            process::exit(code);
                        },
                        None => {
                            try_delete_file(output_path.as_ref());
                            process::exit(1);
                        },
                    }
                }
            }

            let xz_path = matches.value_of("XZ_PATH").unwrap();

            let mut command = command_args!(xz_path, "-z", "-c", input_path);

            if quiet {
                command.arg("-q");
            }

            if best_compression {
                command.args(["-9", "-e"]);
            }

            command.stdout(File::create(output_path.as_ref())?);

            let output = command.execute_output().map_err(|err| {
                try_delete_file(output_path.as_ref());
                err
            })?;

            match output.status.code() {
                Some(code) => {
                    if code != 0 {
                        try_delete_file(output_path.as_ref());
                    }

                    process::exit(code);
                },
                None => {
                    try_delete_file(output_path.as_ref());
                    process::exit(1);
                },
            }
        },
        ArchiveFormat::Lzma => {
            if input_paths.len() > 1 || input_paths[0].is_dir() {
                return Err("Obviously, you should use .tar.lzma for filename extension to \
                            support multiple files."
                    .into());
            }

            let input_path = &input_paths[0];

            if cpus > 1 {
                let pxz_path = matches.value_of("PXZ_PATH").unwrap();

                if command_args!(pxz_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command = command_args!(
                        pxz_path, "-z", "-c", "-T", threads, "-F", "lzma", input_path
                    );

                    if quiet {
                        command.arg("-q");
                    }

                    if best_compression {
                        command.args(["-9", "-e"]);
                    }

                    command.stdout(File::create(output_path.as_ref())?);

                    let output = command.execute_output().map_err(|err| {
                        try_delete_file(output_path.as_ref());
                        err
                    })?;

                    match output.status.code() {
                        Some(code) => {
                            if code != 0 {
                                try_delete_file(output_path.as_ref());
                            }

                            process::exit(code);
                        },
                        None => {
                            try_delete_file(output_path.as_ref());
                            process::exit(1);
                        },
                    }
                }
            }

            let lzma_path = matches.value_of("LZMA_PATH").unwrap();

            let mut command = command_args!(lzma_path, "-z", "-c", input_path);

            if quiet {
                command.arg("-q");
            }

            if best_compression {
                command.args(["-9", "-e"]);
            }

            command.stdout(File::create(output_path.as_ref())?);

            let output = command.execute_output().map_err(|err| {
                try_delete_file(output_path.as_ref());
                err
            })?;

            match output.status.code() {
                Some(code) => {
                    if code != 0 {
                        try_delete_file(output_path.as_ref());
                    }

                    process::exit(code);
                },
                None => {
                    try_delete_file(output_path.as_ref());
                    process::exit(1);
                },
            }
        },
        ArchiveFormat::P7z => {
            let p7z_path = matches.value_of("7Z_PATH").unwrap();

            let password = matches.value_of("PASSWORD");
            let password = read_password(password)?;

            let mut command =
                command_args!(p7z_path, "a", "-t7z", "-aoa", format!("-mmt{}", threads));

            if best_compression {
                command.args(["-m0=lzma2", "-mx", "-ms=on"]);
            }

            if !password.is_empty() {
                command.arg("-mhe=on");
                command.arg(format!("-p{}", password));
            }

            if let Some(d) = split {
                let mut volume = String::from("-v");

                let byte = Byte::from_str(d)?;

                if byte.get_bytes() < 65536 {
                    return Err("The split size is too small.".into());
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

            command.arg(output_path.as_ref());

            command.args(input_paths);

            if quiet {
                process::exit(command.execute()?.unwrap_or(1));
            } else {
                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
            }
        },
        ArchiveFormat::Zip => {
            let p7z_path = matches.value_of("7Z_PATH").unwrap();

            let password = matches.value_of("PASSWORD");
            let password = read_password(password)?;

            let split = if let Some(d) = split {
                let byte = Byte::from_str(d)?;

                if byte.get_bytes() < 65536 {
                    return Err("The split size is too small.".into());
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
                        return Err(format!(
                            "{} is a directory.",
                            output_tmp_path.to_string_lossy()
                        )
                        .into());
                    } else {
                        fs::remove_file(output_tmp_path.as_path())?;
                    }
                }

                Cow::from(output_tmp_path)
            } else {
                Cow::from(output_path.as_ref())
            };

            let mut command =
                command_args!(p7z_path, "a", "-tzip", "-aoa", format!("-mmt{}", threads));

            if best_compression {
                command.arg("-mx");
            }

            if !password.is_empty() {
                command.arg(format!("-p{}", create_cli_string(&password)));
            }

            command.arg(output_tmp_path.as_ref());

            command.args(input_paths);

            let exit_code = if quiet {
                command.execute()?
            } else {
                let output = command.execute_output()?;

                output.status.code()
            };

            if let Some(byte) = split {
                match exit_code {
                    Some(code) => {
                        if code != 0 {
                            try_delete_file(output_path.as_ref());
                            process::exit(code);
                        }
                    },
                    None => {
                        try_delete_file(output_path.as_ref());
                        process::exit(1);
                    },
                }

                let zip_path = matches.value_of("ZIP_PATH").unwrap();

                let mut command = command_args!(
                    zip_path,
                    "-s",
                    format!(
                        "{}k",
                        byte.get_adjusted_unit(ByteUnit::KiB).get_value().round() as u32
                    )
                );

                if best_compression {
                    command.arg("-9");
                }

                if !password.is_empty() {
                    command.arg("--password");
                    command.arg(password.as_ref());
                }

                if quiet {
                    command.arg("-q");
                }

                command.arg(output_tmp_path.as_ref());

                command.arg("--out");

                command.arg(output_path.as_ref());

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
                        try_delete_file(output_path.as_ref());
                        process::exit(1);
                    },
                }
            }

            process::exit(exit_code.unwrap_or(1));
        },
        ArchiveFormat::Rar => {
            let rar_path = matches.value_of("RAR_PATH").unwrap();

            let password = matches.value_of("PASSWORD");
            let password = read_password(password)?;

            let mut command = command_args!(rar_path, "a", "-ep1", format!("-mt{}", threads));

            if best_compression {
                command.args(["-ma5", "-m5", "-s"]);
            }

            if !password.is_empty() {
                command.arg(format!("-hp{}", create_cli_string(&password)));
            }

            if quiet {
                command.arg("-idq");
            }

            if let Some(d) = split {
                let byte = Byte::from_str(d)?;

                if byte.get_bytes() < 65536 {
                    return Err("The split size is too small.".into());
                } else {
                    command.arg(format!(
                        "-v{}k",
                        byte.get_adjusted_unit(ByteUnit::KiB).get_value().round() as u32
                    ));
                }
            }

            command.arg(output_path.as_ref());

            command.args(input_paths);

            let output = command.execute_output()?;

            process::exit(output.status.code().unwrap_or(1));
        },
        ArchiveFormat::Zstd => {
            if input_paths.len() > 1 || input_paths[0].is_dir() {
                return Err("Obviously, you should use .tar.zst for filename extension to \
                            support multiple files."
                    .into());
            }

            let input_path = &input_paths[0];

            if cpus > 1 {
                let pzstd_path = matches.value_of("PZSTD_PATH").unwrap();

                if command_args!(pzstd_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command = command_args!(
                        pzstd_path,
                        "-p",
                        threads,
                        input_path,
                        "-o",
                        output_path.as_ref()
                    );

                    if quiet {
                        command.arg("-q");
                    }

                    if best_compression {
                        command.args(["--ultra", "-22"]);
                    }

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }
            }

            let zstd_path = matches.value_of("ZSTD_PATH").unwrap();

            let mut command = command_args!(zstd_path, input_path, "-o", output_path.as_ref());

            if quiet {
                command.arg("-q");
            }

            if best_compression {
                command.args(["--ultra", "-22"]);
            }

            let output = command.execute_output()?;

            process::exit(output.status.code().unwrap_or(1));
        },
    }
}

fn handle_extract(
    matches: &ArgMatches,
    input_path: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn Error>> {
    let single_thread = matches.is_present("SINGLE_THREAD");
    let quiet = matches.is_present("QUIET");

    let cpus = if single_thread { 1 } else { num_cpus::get() };

    let format = ArchiveFormat::get_archive_format_from_file_path(input_path)?;

    let output_path = output_path.absolutize()?;

    match output_path.metadata() {
        Ok(metadata) => {
            if !metadata.is_dir() {
                return Err(format!("{} is not a directory.", output_path.to_string_lossy()).into());
            }
        },
        Err(_) => {
            fs::create_dir_all(output_path.as_ref())?;
        },
    }

    let threads = cpus.to_string();
    let threads = threads.as_str();

    match format {
        ArchiveFormat::TarZ | ArchiveFormat::TarGzip => {
            let tar_path = matches.value_of("TAR_PATH").unwrap();

            if cpus > 1 {
                let pigz_path = matches.value_of("PIGZ_PATH").unwrap();

                if command_args!(pigz_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command1 =
                        command_args!(pigz_path, "-d", "-c", "-p", threads, input_path);
                    let mut command2 =
                        command_args!(tar_path, "-x", "-C", output_path.as_ref(), "-f", "-");

                    if !quiet {
                        command2.arg("-v");
                    }

                    let output = command1.execute_multiple_output(&mut [&mut command2])?;

                    process::exit(output.status.code().unwrap_or(1));
                }
            }

            let mut command =
                command_args!(tar_path, "-z", "-x", "-C", output_path.as_ref(), "-f", input_path);

            if !quiet {
                command.arg("-v");
            }

            let output = command.execute_output()?;

            process::exit(output.status.code().unwrap_or(1));
        },
        ArchiveFormat::TarBzip2 => {
            let tar_path = matches.value_of("TAR_PATH").unwrap();

            if cpus > 1 {
                let lbzip2_path = matches.value_of("LBZIP2_PATH").unwrap();

                if command_args!(lbzip2_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command1 =
                        command_args!(lbzip2_path, "-d", "-c", "-n", threads, input_path);
                    let mut command2 =
                        command_args!(tar_path, "-x", "-C", output_path.as_ref(), "-f", "-");

                    if !quiet {
                        command2.arg("-v");
                    }

                    let output = command1.execute_multiple_output(&mut [&mut command2])?;

                    process::exit(output.status.code().unwrap_or(1));
                }

                let pbzip2_path = matches.value_of("PBZIP2_PATH").unwrap();

                if command_args!(pbzip2_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command1 = command_args!(
                        pbzip2_path,
                        "-d",
                        "-c",
                        format!("-p{}", threads),
                        input_path
                    );
                    let mut command2 =
                        command_args!(tar_path, "-x", "-C", output_path.as_ref(), "-f", "-");

                    if !quiet {
                        command2.arg("-v");
                    }

                    let output = command1.execute_multiple_output(&mut [&mut command2])?;

                    process::exit(output.status.code().unwrap_or(1));
                }
            }

            let mut command =
                command_args!(tar_path, "-j", "-x", "-C", output_path.as_ref(), "-f", input_path);

            if !quiet {
                command.arg("-v");
            }

            let output = command.execute_output()?;

            process::exit(output.status.code().unwrap_or(1));
        },
        ArchiveFormat::TarLz => {
            let tar_path = matches.value_of("TAR_PATH").unwrap();

            if cpus > 1 {
                let plzip_path = matches.value_of("PLZIP_PATH").unwrap();

                if command_args!(plzip_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command1 =
                        command_args!(plzip_path, "-d", "-c", "-n", threads, input_path);
                    let mut command2 =
                        command_args!(tar_path, "-x", "-C", output_path.as_ref(), "-f", "-");

                    if !quiet {
                        command2.arg("-v");
                    }

                    let output = command1.execute_multiple_output(&mut [&mut command2])?;

                    process::exit(output.status.code().unwrap_or(1));
                }
            }

            let lunzip_path = matches.value_of("LUNZIP_PATH").unwrap();

            if command_args!(lunzip_path, "-V").execute_check_exit_status_code(0).is_ok() {
                let mut command = command_args!(
                    tar_path,
                    "-I",
                    lunzip_path,
                    "-x",
                    "-C",
                    output_path.as_ref(),
                    "-f",
                    input_path
                );

                if !quiet {
                    command.arg("-v");
                }

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
            }

            let lzip_path = matches.value_of("LZIP_PATH").unwrap();

            let mut command = command_args!(
                tar_path,
                "-I",
                lzip_path,
                "-x",
                "-C",
                output_path.as_ref(),
                "-f",
                input_path
            );

            if !quiet {
                command.arg("-v");
            }

            let output = command.execute_output()?;

            process::exit(output.status.code().unwrap_or(1));
        },
        ArchiveFormat::TarXz => {
            let tar_path = matches.value_of("TAR_PATH").unwrap();

            if cpus > 1 {
                let pxz_path = matches.value_of("PXZ_PATH").unwrap();

                if command_args!(pxz_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command1 =
                        command_args!(pxz_path, "-d", "-c", "-T", threads, input_path);
                    let mut command2 =
                        command_args!(tar_path, "-x", "-C", output_path.as_ref(), "-f", "-");

                    if !quiet {
                        command2.arg("-v");
                    }

                    let output = command1.execute_multiple_output(&mut [&mut command2])?;

                    process::exit(output.status.code().unwrap_or(1));
                }
            }

            let mut command =
                command_args!(tar_path, "-J", "-x", "-C", output_path.as_ref(), "-f", input_path);

            if !quiet {
                command.arg("-v");
            }

            let output = command.execute_output()?;

            process::exit(output.status.code().unwrap_or(1));
        },
        ArchiveFormat::TarLzma => {
            let tar_path = matches.value_of("TAR_PATH").unwrap();

            if cpus > 1 {
                let pxz_path = matches.value_of("PXZ_PATH").unwrap();

                if command_args!(pxz_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command1 = command_args!(
                        pxz_path, "-d", "-c", "-T", threads, "-F", "lzma", input_path
                    );
                    let mut command2 =
                        command_args!(tar_path, "-x", "-C", output_path.as_ref(), "-f", "-");

                    if !quiet {
                        command2.arg("-v");
                    }

                    let output = command1.execute_multiple_output(&mut [&mut command2])?;

                    process::exit(output.status.code().unwrap_or(1));
                }
            }

            let unlzma_path = matches.value_of("UNLZMA_PATH").unwrap();

            if command_args!(unlzma_path, "-V").execute_check_exit_status_code(0).is_ok() {
                let mut command = command_args!(
                    tar_path,
                    "-I",
                    unlzma_path,
                    "-x",
                    "-C",
                    output_path.as_ref(),
                    "-f",
                    input_path
                );

                if !quiet {
                    command.arg("-v");
                }

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
            }

            let lzma_path = matches.value_of("LZMA_PATH").unwrap();

            let mut command = command_args!(
                tar_path,
                "-I",
                lzma_path,
                "-x",
                "-C",
                output_path.as_ref(),
                "-f",
                input_path
            );

            if !quiet {
                command.arg("-v");
            }

            let output = command.execute_output()?;

            process::exit(output.status.code().unwrap_or(1));
        },
        ArchiveFormat::Tar7z => {
            let p7z_path = matches.value_of("7Z_PATH").unwrap();
            let tar_path = matches.value_of("TAR_PATH").unwrap();

            let password = matches.value_of("PASSWORD");
            let password = read_password(password)?;

            let mut command1 = command_args!(p7z_path, "x", "-so", format!("-mmt{}", threads));

            command1.arg(format!("-p{}", create_cli_string(&password)));

            command1.arg(input_path);

            let mut command2 = command_args!(tar_path, "-x", "-C", output_path.as_ref(), "-f", "-");

            if !quiet {
                command2.arg("-v");
            }

            let output = command1.execute_multiple_output(&mut [&mut command2])?;

            process::exit(output.status.code().unwrap_or(1));
        },
        ArchiveFormat::TarZstd => {
            let tar_path = matches.value_of("TAR_PATH").unwrap();

            if cpus > 1 {
                let pzstd_path = matches.value_of("PZSTD_PATH").unwrap();

                if command_args!(pzstd_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command1 =
                        command_args!(pzstd_path, "-d", "-c", "-p", threads, input_path);
                    let mut command2 =
                        command_args!(tar_path, "-x", "-C", output_path.as_ref(), "-f", "-");

                    if !quiet {
                        command2.arg("-v");
                    }

                    let output = command1.execute_multiple_output(&mut [&mut command2])?;

                    process::exit(output.status.code().unwrap_or(1));
                }
            }

            let unzstd_path = matches.value_of("UNZSTD_PATH").unwrap();

            if command_args!(unzstd_path, "-V").execute_check_exit_status_code(0).is_ok() {
                let mut command = command_args!(
                    tar_path,
                    "-I",
                    unzstd_path,
                    "-x",
                    "-C",
                    output_path.as_ref(),
                    "-f",
                    input_path
                );

                if !quiet {
                    command.arg("-v");
                }

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
            }

            let zstd_path = matches.value_of("ZSTD_PATH").unwrap();

            let mut command = command_args!(
                tar_path,
                "-I",
                zstd_path,
                "-x",
                "-C",
                output_path.as_ref(),
                "-f",
                input_path
            );

            if !quiet {
                command.arg("-v");
            }

            let output = command.execute_output()?;

            process::exit(output.status.code().unwrap_or(1));
        },
        ArchiveFormat::Tar => {
            let tar_path = matches.value_of("TAR_PATH").unwrap();

            let mut command =
                command_args!(tar_path, "-x", "-C", output_path.as_ref(), "-f", input_path);

            if !quiet {
                command.arg("-v");
            }

            let output = command.execute_output()?;

            process::exit(output.status.code().unwrap_or(1));
        },
        ArchiveFormat::Z | ArchiveFormat::Gzip => {
            let file_path = output_path.join(Path::new(input_path).file_stem().unwrap());

            if file_path.is_dir() {
                return Err(format!("`{}` it is a directory.", file_path.to_string_lossy()).into());
            }

            let file = File::create(file_path)?;

            if cpus > 1 {
                let pigz_path = matches.value_of("PIGZ_PATH").unwrap();

                if command_args!(pigz_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command =
                        command_args!(pigz_path, "-d", "-c", "-p", threads, input_path);

                    command.stdout(file);

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }
            }

            let gunzip_path = matches.value_of("GUNZIP_PATH").unwrap();

            if command_args!(gunzip_path, "-V").execute_check_exit_status_code(0).is_ok() {
                let mut command = command_args!(gunzip_path, "-c", input_path);

                command.stdout(file);

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
            }

            let gzip_path = matches.value_of("GZIP_PATH").unwrap();

            let mut command = command_args!(gzip_path, "-d", "-c", input_path);

            command.stdout(file);

            let output = command.execute_output()?;

            process::exit(output.status.code().unwrap_or(1));
        },
        ArchiveFormat::Bzip2 => {
            let file_path = output_path.join(Path::new(input_path).file_stem().unwrap());

            if file_path.is_dir() {
                return Err(format!("`{}` it is a directory.", file_path.to_string_lossy()).into());
            }

            let file = File::create(file_path)?;

            if cpus > 1 {
                let lbzip2_path = matches.value_of("LBZIP2_PATH").unwrap();

                if command_args!(lbzip2_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command =
                        command_args!(lbzip2_path, "-d", "-c", "-n", threads, input_path);

                    command.stdout(file);

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }

                let pbzip2_path = matches.value_of("PBZIP2_PATH").unwrap();

                if command_args!(pbzip2_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command = command_args!(
                        pbzip2_path,
                        "-d",
                        "-c",
                        format!("-p{}", threads),
                        input_path
                    );

                    command.stdout(file);

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }
            }

            let bunzip2_path = matches.value_of("BUNZIP2_PATH").unwrap();

            if command_args!(bunzip2_path, "-V").execute_check_exit_status_code(0).is_ok() {
                let mut command = command_args!(bunzip2_path, "-c", input_path);

                command.stdout(file);

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
            }

            let bzip2_path = matches.value_of("BZIP2_PATH").unwrap();

            let mut command = command_args!(bzip2_path, "-d", "-c", input_path);

            command.stdout(file);

            let output = command.execute_output()?;

            process::exit(output.status.code().unwrap_or(1));
        },
        ArchiveFormat::Lz => {
            let file_path = output_path.join(Path::new(input_path).file_stem().unwrap());

            if file_path.is_dir() {
                return Err(format!("`{}` it is a directory.", file_path.to_string_lossy()).into());
            }

            let file = File::create(file_path)?;

            if cpus > 1 {
                let plzip_path = matches.value_of("PLZIP_PATH").unwrap();

                if command_args!(plzip_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command =
                        command_args!(plzip_path, "-d", "-c", "-n", threads, input_path);

                    command.stdout(file);

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }
            }

            let lunzip_path = matches.value_of("LUNZIP_PATH").unwrap();

            if command_args!(lunzip_path, "-V").execute_check_exit_status_code(0).is_ok() {
                let mut command = command_args!(lunzip_path, "-c", input_path);

                command.stdout(file);

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
            }

            let lzip_path = matches.value_of("LZIP_PATH").unwrap();

            let mut command = command_args!(lzip_path, "-d", "-c", input_path);

            command.stdout(file);

            let output = command.execute_output()?;

            process::exit(output.status.code().unwrap_or(1));
        },
        ArchiveFormat::Xz => {
            let file_path = output_path.join(Path::new(input_path).file_stem().unwrap());

            if file_path.is_dir() {
                return Err(format!("`{}` it is a directory.", file_path.to_string_lossy()).into());
            }

            let file = File::create(file_path)?;

            if cpus > 1 {
                let pxz_path = matches.value_of("PXZ_PATH").unwrap();

                if command_args!(pxz_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command =
                        command_args!(pxz_path, "-d", "-c", "-T", threads, input_path);

                    command.stdout(file);

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }
            }

            let unxz_path = matches.value_of("UNXZ_PATH").unwrap();

            if command_args!(unxz_path, "-V").execute_check_exit_status_code(0).is_ok() {
                let mut command = command_args!(unxz_path, "-c", input_path);

                command.stdout(file);

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
            }

            let xz_path = matches.value_of("XZ_PATH").unwrap();

            let mut command = command_args!(xz_path, "-d", "-c", input_path);

            command.stdout(file);

            let output = command.execute_output()?;

            process::exit(output.status.code().unwrap_or(1));
        },
        ArchiveFormat::Lzma => {
            let file_path = output_path.join(Path::new(input_path).file_stem().unwrap());

            if file_path.is_dir() {
                return Err(format!("`{}` it is a directory.", file_path.to_string_lossy()).into());
            }

            let file = File::create(file_path)?;

            if cpus > 1 {
                let pxz_path = matches.value_of("PXZ_PATH").unwrap();

                if command_args!(pxz_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command = command_args!(
                        pxz_path, "-d", "-c", "-T", threads, "-F", "lzma", input_path
                    );

                    command.stdout(file);

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }
            }

            let unlzma_path = matches.value_of("UNLZMA_PATH").unwrap();

            if command_args!(unlzma_path, "-V").execute_check_exit_status_code(0).is_ok() {
                let mut command = command_args!(unlzma_path, "-c", input_path);

                command.stdout(file);

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
            }

            let lzma_path = matches.value_of("LZMA_PATH").unwrap();

            let mut command = command_args!(lzma_path, "-d", "-c", input_path);

            command.stdout(file);

            let output = command.execute_output()?;

            process::exit(output.status.code().unwrap_or(1));
        },
        ArchiveFormat::P7z => {
            let p7z_path = matches.value_of("7Z_PATH").unwrap();

            let password = matches.value_of("PASSWORD");
            let password = read_password(password)?;

            let mut command = command_args!(
                p7z_path,
                "x",
                "-aoa",
                format!("-mmt{}", threads),
                format!("-o{}", output_path.to_string_lossy())
            );

            command.arg(format!("-p{}", password));

            command.arg(input_path);

            println!("{:?}", command);

            if quiet {
                process::exit(command.execute()?.unwrap_or(1));
            } else {
                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
            }
        },
        ArchiveFormat::Zip => {
            let unzip_path = matches.value_of("UNZIP_PATH").unwrap();

            let password = matches.value_of("PASSWORD");
            let password = read_password(password)?;

            let mut command = command_args!(unzip_path);

            command.arg("-P");
            command.arg(password.as_ref());

            if quiet {
                command.arg("-qq");
            }

            command.args(["-O", "UTF-8", "-o"]);
            command.arg(input_path);
            command.arg("-d");
            command.arg(output_path.as_ref());

            let output = command.execute_output()?;

            process::exit(output.status.code().unwrap_or(1));
        },
        ArchiveFormat::Rar => {
            let unrar_path = matches.value_of("UNRAR_PATH").unwrap();

            let password = matches.value_of("PASSWORD");
            let password = read_password(password)?;

            if command_args!(unrar_path, "-?").execute_check_exit_status_code(0).is_ok() {
                let mut command = command_args!(unrar_path, "x", "-o+");

                command.arg(format!("-mt{}", threads));

                if password.is_empty() {
                    command.arg("-p-");
                } else {
                    command.arg(format!("-p{}", create_cli_string(&password)));
                }

                if quiet {
                    command.arg("-idq");
                }

                command.arg(input_path);
                command.arg(output_path.as_ref());

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
            }

            let rar_path = matches.value_of("RAR_PATH").unwrap();

            let mut command = command_args!(rar_path, "x", "-o+");

            command.arg(format!("-mt{}", threads));

            if password.is_empty() {
                command.arg("-p-");
            } else {
                command.arg(format!("-p{}", create_cli_string(&password)));
            }

            if quiet {
                command.arg("-idq");
            }

            command.arg(input_path);
            command.arg(output_path.as_ref());

            let output = command.execute_output()?;

            process::exit(output.status.code().unwrap_or(1));
        },
        ArchiveFormat::Zstd => {
            let file_path = output_path.join(Path::new(input_path).file_stem().unwrap());

            if file_path.is_dir() {
                return Err(format!("`{}` it is a directory.", file_path.to_string_lossy()).into());
            }

            let file = File::create(file_path)?;

            if cpus > 1 {
                let pzstd_path = matches.value_of("PZSTD_PATH").unwrap();

                if command_args!(pzstd_path, "-V").execute_check_exit_status_code(0).is_ok() {
                    let mut command =
                        command_args!(pzstd_path, "-d", "-c", "-p", threads, input_path);

                    command.stdout(file);

                    let output = command.execute_output()?;

                    process::exit(output.status.code().unwrap_or(1));
                }
            }

            let unzstd_path = matches.value_of("UNZSTD_PATH").unwrap();

            if command_args!(unzstd_path, "-V").execute_check_exit_status_code(0).is_ok() {
                let mut command = command_args!(unzstd_path, "-c", input_path);

                command.stdout(file);

                let output = command.execute_output()?;

                process::exit(output.status.code().unwrap_or(1));
            }

            let zstd_path = matches.value_of("ZSTD_PATH").unwrap();

            let mut command = command_args!(zstd_path, "-d", "-c", input_path);

            command.stdout(file);

            let output = command.execute_output()?;

            process::exit(output.status.code().unwrap_or(1));
        },
    }
}

#[inline]
fn try_delete_file<P: AsRef<Path>>(file_path: P) {
    if fs::remove_file(file_path).is_err() {}
}

#[inline]
fn create_cli_string(string: &str) -> String {
    string.replace(' ', "\\ ")
}

fn read_password(password: Option<&str>) -> Result<Cow<str>, Box<dyn Error>> {
    match password {
        Some(password) => {
            if password.is_empty() {
                print!("Password (visible): ");
                io::stdout().flush()?;

                let mut sc: Scanner<_, U32> = Scanner::new2(io::stdin());

                sc.next_line()?.map(Cow::from).ok_or_else(|| "Stdin is closed.".into())
            } else {
                Ok(Cow::from(password))
            }
        },
        None => Ok(Cow::from("")),
    }
}

fn get_matches() -> ArgMatches {
    Command::new(APP_NAME)
        .term_width(terminal_size().map(|(width, _)| width.0 as usize).unwrap_or(0))
        .version(CARGO_PKG_VERSION)
        .author(CARGO_PKG_AUTHORS)
        .about(concat!("XCompress is a free file archiver utility on Linux, providing multi-format archiving to and extracting from ZIP, Z, GZIP, BZIP2, LZ, XZ, LZMA, 7ZIP, TAR and RAR.\n\nEXAMPLES:\n", concat_line!(prefix "xcompress ",
                "a foo.wav                      # Archive foo.wav to foo.rar",
                "a foo.wav /root/bar.txt        # Archive foo.wav and /root/bar.txt to foo.rar",
                "a -o /tmp/out.7z foo.wav       # Archive foo.wav to /tmp/out.7z",
                "a -b foo/bar                   # Archive foo/bar folder to bar.rar as small as possible",
                "a -p password foo.wav          # Archive foo.wav to foo.rar with a password",
                "x foo.rar                      # Extract foo.rar into current working directory",
                "x foo.tar.gz /tmp/out_folder   # Extract foo.tar.gz into /tmp/out_folder",
                "x -p password foo.rar          # Extract foo.rar with a password into current working directory"
            )))
        .arg(Arg::new("QUIET")
            .global(true)
            .long("quiet")
            .short('q')
            .help("Make programs not print anything on the screen.")
        )
        .arg(Arg::new("SINGLE_THREAD")
            .global(true)
            .long("single-thread")
            .short('s')
            .help("Use only one thread.")
        )
        .arg(Arg::new("PASSWORD")
            .global(true)
            .long("password")
            .short('p')
            .help("Set password for your archive file. (Only supports 7Z, ZIP and RAR.) Set an empty string to read a password from stdin.")
            .takes_value(true)
            .forbid_empty_values(false)
            .display_order(0)
        )
        .arg(Arg::new("COMPRESS_PATH")
            .global(true)
            .long("compress-path")
            .help("Specify the path of your compress executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_COMPRESS_PATH)
        )
        .arg(Arg::new("ZIP_PATH")
            .global(true)
            .long("zip-path")
            .help("Specify the path of your zip executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_ZIP_PATH)
        )
        .arg(Arg::new("UNZIP_PATH")
            .global(true)
            .long("unzip-path")
            .help("Specify the path of your unzip executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_UNZIP_PATH)
        )
        .arg(Arg::new("GZIP_PATH")
            .global(true)
            .long("gzip-path")
            .help("Specify the path of your gzip executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_GZIP_PATH)
        )
        .arg(Arg::new("GUNZIP_PATH")
            .global(true)
            .long("gunzip-path")
            .help("Specify the path of your gunzip executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_GUNZIP_PATH)
        )
        .arg(Arg::new("PIGZ_PATH")
            .global(true)
            .long("pigz-path")
            .help("Specify the path of your pigz executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_PIGZ_PATH)
        )
        .arg(Arg::new("BZIP2_PATH")
            .global(true)
            .long("bzip2-path")
            .help("Specify the path of your bzip2 executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_BZIP2_PATH)
        )
        .arg(Arg::new("BUNZIP2_PATH")
            .global(true)
            .long("bunzip2-path")
            .help("Specify the path of your bunzip2 executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_BUNZIP2_PATH)
        )
        .arg(Arg::new("LBZIP2_PATH")
            .global(true)
            .long("lbzip2-path")
            .help("Specify the path of your lbzip2 executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_LBZIP2_PATH)
        )
        .arg(Arg::new("PBZIP2_PATH")
            .global(true)
            .long("pbzip2-path")
            .help("Specify the path of your pbzip2 executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_PBZIP2_PATH)
        )
        .arg(Arg::new("LZIP_PATH")
            .global(true)
            .long("lzip-path")
            .help("Specify the path of your lzip executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_LZIP_PATH)
        )
        .arg(Arg::new("LUNZIP_PATH")
            .global(true)
            .long("lunzip-path")
            .help("Specify the path of your lunzip executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_LUNZIP_PATH)
        )
        .arg(Arg::new("PLZIP_PATH")
            .global(true)
            .long("plzip-path")
            .help("Specify the path of your plzip executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_PLZIP_PATH)
        )
        .arg(Arg::new("XZ_PATH")
            .global(true)
            .long("xz-path")
            .help("Specify the path of your xz executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_XZ_PATH)
        )
        .arg(Arg::new("UNXZ_PATH")
            .global(true)
            .long("unxz-path")
            .help("Specify the path of your unxz executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_UNXZ_PATH)
        )
        .arg(Arg::new("PXZ_PATH")
            .global(true)
            .long("pxz-path")
            .help("Specify the path of your pxz executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_PXZ_PATH)
        )
        .arg(Arg::new("LZMA_PATH")
            .global(true)
            .long("lzma-path")
            .help("Specify the path of your lzma executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_LZMA_PATH)
        )
        .arg(Arg::new("UNLZMA_PATH")
            .global(true)
            .long("unlzma-path")
            .help("Specify the path of your unlzma executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_UNLZMA_PATH)
        )
        .arg(Arg::new("7Z_PATH")
            .global(true)
            .long("7z-path")
            .help("Specify the path of your 7z executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_7Z_PATH)
        )
        .arg(Arg::new("TAR_PATH")
            .global(true)
            .long("tar-path")
            .help("Specify the path of your tar executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_TAR_PATH)
        )
        .arg(Arg::new("RAR_PATH")
            .global(true)
            .long("rar-path")
            .help("Specify the path of your rar executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_RAR_PATH)
        )
        .arg(Arg::new("UNRAR_PATH")
            .global(true)
            .long("unrar-path")
            .help("Specify the path of your unrar executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_UNRAR_PATH)
        )
        .arg(Arg::new("ZSTD_PATH")
            .global(true)
            .long("zstd-path")
            .help("Specify the path of your zstd executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_ZSTD_PATH)
        )
        .arg(Arg::new("UNZSTD_PATH")
            .global(true)
            .long("unzstd-path")
            .help("Specify the path of your unzstd executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_UNZSTD_PATH)
        )
        .arg(Arg::new("PZSTD_PATH")
            .global(true)
            .long("pzstd-path")
            .help("Specify the path of your pzstd executable binary file.")
            .takes_value(true)
            .default_value(DEFAULT_PZSTD_PATH)
        )
        .subcommand(Command::new("x")
            .about("Extract files with full path.")
            .arg(Arg::new("INPUT_PATH")
                .required(true)
                .help("Assign the source of your archived file. It should be a file path.")
            )
            .arg(Arg::new("OUTPUT_PATH")
                .required(false)
                .help("Assign a destination of your extracted files. It should be a directory path.")
            )
            .arg(Arg::new("OUTPUT_PATH2")
                .long("output")
                .short('o')
                .help("Assign a destination of your extracted files. It should be a directory path.")
                .takes_value(true)
                .value_name("OUTPUT_PATH")
                .display_order(1)
            )
            .after_help("Enjoy it! https://magiclen.org")
        )
        .subcommand(Command::new("a")
            .about("Add files to archive. Excludes base directory from names. (e.g. add /path/to/folder, you can always get the \"folder\" in the root of the archive file, instead of /path/to/folder.)")
            .arg(Arg::new("INPUT_PATH")
                .required(true)
                .help("Assign the source of your original files. It should be at least one file path.")
                .multiple_values(true)
            )
            .arg(Arg::new("OUTPUT_PATH")
                .long("output")
                .short('o')
                .help("Assign a destination of your extracted files. It should be a file path. Specify the file extension name in order to determine which archive format you want to use. [default archive format: RAR]")
                .takes_value(true)
                .display_order(1)
            )
            .arg(Arg::new("BEST_COMPRESSION")
                .long("best-compression")
                .short('b')
                .help("If you are OK about the compression and depression time and want to save more disk space and network traffic, it will make the archive file as small as possible.")
                .display_order(1)
            )
            .arg(Arg::new("SPLIT")
                .long("split")
                .short('d')
                .help("Split the archive file into volumes with a specified size. The unit of value is byte. You can also use KB, MB, KiB, MiB, etc, as a suffix. The minimum volume is 64 KiB. (Only supports 7Z, ZIP and RAR.)")
                .takes_value(true)
                .value_name("SIZE_OF_EACH_VOLUME")
                .display_order(1)
            )
            .after_help("Enjoy it! https://magiclen.org")
        )
        .after_help("Enjoy it! https://magiclen.org")
        .get_matches()
}
