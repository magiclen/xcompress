//! # XCompress
//! XCompress is a free file archiver utility on Linux, providing multi-format archiving to and extracting from ZIP, Z, GZIP, BZIP2, LZ, XZ, LZMA, 7ZIP, TAR and RAR.

extern crate clap;
extern crate num_cpus;
extern crate subprocess;

use std::io::{ErrorKind, Read, BufReader, BufWriter, Write};
use std::path::Path;
use std::env;
use std::fs;

use subprocess::{Exec, ExitStatus, PopenError, Pipeline, NullFile};

use clap::{App, Arg, SubCommand};

// TODO -----Config START-----

const APP_NAME: &str = "XCompress";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const BUFFER_SIZE: usize = 4096 * 4;
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

#[derive(Debug)]
pub enum Mode {
    Archive(bool, Vec<String>, Option<String>),
    Extract(String, Option<String>),
}

#[derive(Debug)]
pub struct ExePaths {
    pub compress_path: String,
    pub zip_path: String,
    pub unzip_path: String,
    pub gzip_path: String,
    pub gunzip_path: String,
    pub pigz_path: String,
    pub bzip2_path: String,
    pub bunzip2_path: String,
    pub lbzip2_path: String,
    pub pbzip2_path: String,
    pub lzip_path: String,
    pub lunzip_path: String,
    pub plzip_path: String,
    pub xz_path: String,
    pub unxz_path: String,
    pub pxz_path: String,
    pub lzma_path: String,
    pub unlzma_path: String,
    pub p7z_path: String,
    pub tar_path: String,
    pub rar_path: String,
    pub unrar_path: String,
}

impl ExePaths {
    pub fn new_default() -> ExePaths {
        ExePaths {
            compress_path: String::from(DEFAULT_COMPRESS_PATH),
            zip_path: String::from(DEFAULT_ZIP_PATH),
            unzip_path: String::from(DEFAULT_UNZIP_PATH),
            gzip_path: String::from(DEFAULT_GZIP_PATH),
            gunzip_path: String::from(DEFAULT_GUNZIP_PATH),
            pigz_path: String::from(DEFAULT_PIGZ_PATH),
            bzip2_path: String::from(DEFAULT_BZIP2_PATH),
            bunzip2_path: String::from(DEFAULT_BUNZIP2_PATH),
            lbzip2_path: String::from(DEFAULT_LBZIP2_PATH),
            pbzip2_path: String::from(DEFAULT_PBZIP2_PATH),
            lzip_path: String::from(DEFAULT_LZIP_PATH),
            lunzip_path: String::from(DEFAULT_LUNZIP_PATH),
            plzip_path: String::from(DEFAULT_PLZIP_PATH),
            xz_path: String::from(DEFAULT_XZ_PATH),
            unxz_path: String::from(DEFAULT_UNXZ_PATH),
            pxz_path: String::from(DEFAULT_PXZ_PATH),
            lzma_path: String::from(DEFAULT_LZMA_PATH),
            unlzma_path: String::from(DEFAULT_UNLZMA_PATH),
            p7z_path: String::from(DEFAULT_7Z_PATH),
            tar_path: String::from(DEFAULT_TAR_PATH),
            rar_path: String::from(DEFAULT_RAR_PATH),
            unrar_path: String::from(DEFAULT_UNRAR_PATH),
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub paths: ExePaths,
    pub quiet: bool,
    pub single_thread: bool,
    pub password: String,
    pub mode: Mode,
}

impl Config {
    pub fn from_cli() -> Result<Config, String> {
        let arg0 = env::args().next().unwrap();

        let examples = vec![
            "a foo.wav                      # Archives foo.wav to foo.rar",
            "a foo.wav /root/bar.txt        # Archives foo.wav and /root/bar.txt to foo.rar",
            "a -o /tmp/out.rar foo.wav      # Archives foo.wav to /tmp/out.rar",
            "a -b foo/bar                   # Archives foo/bar folder to bar.rar as small as possible",
            "a -p password foo.wav          # Archives foo.wav to foo.rar with a password",
            "x foo.rar                      # Extracts foo.rar into current working directory",
            "x foo.tar.gz /tmp/out_folder    # Extracts foo.tar.gz into /tmp/out_folder",
            "x -p password foo.rar          # Extracts foo.rar with a password into current working directory"
        ];

        let matches = App::new(APP_NAME)
            .version(CARGO_PKG_VERSION)
            .author(CARGO_PKG_AUTHORS)
            .about(format!("XCompress is a free file archiver utility on Linux, providing multi-format archiving to and extracting from ZIP, Z, GZIP, BZIP2, LZ, XZ, LZMA, 7ZIP, TAR and RAR.\n\nEXAMPLES:\n{}", examples.iter()
                .map(|e| format!("  {} {}\n", arg0, e))
                .collect::<Vec<String>>()
                .concat()
            ).as_str()
            )
            .arg(Arg::with_name("QUIET")
                .global(true)
                .long("quiet")
                .short("q")
                .help("Makes programs not print anything on the screen.")
            )
            .arg(Arg::with_name("SINGLE_THREAD")
                .global(true)
                .long("single-thread")
                .short("s")
                .help("Uses only one thread.")
            )
            .arg(Arg::with_name("PASSWORD")
                .global(true)
                .long("password")
                .short("p")
                .help("Sets password for your archive file. (Only supports 7Z, ZIP and RAR)")
                .takes_value(true)
            )
            .arg(Arg::with_name("COMPRESS_PATH")
                .global(true)
                .long("compress-path")
                .help("Specifies the path of your compress executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_COMPRESS_PATH)
            )
            .arg(Arg::with_name("ZIP_PATH")
                .global(true)
                .long("zip-path")
                .help("Specifies the path of your zip executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_ZIP_PATH)
            )
            .arg(Arg::with_name("UNZIP_PATH")
                .global(true)
                .long("unzip-path")
                .help("Specifies the path of your unzip executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_UNZIP_PATH)
            )
            .arg(Arg::with_name("GZIP_PATH")
                .global(true)
                .long("gzip-path")
                .help("Specifies the path of your gzip executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_GZIP_PATH)
            )
            .arg(Arg::with_name("GUNZIP_PATH")
                .global(true)
                .long("gunzip-path")
                .help("Specifies the path of your gunzip executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_GUNZIP_PATH)
            )
            .arg(Arg::with_name("PIGZ_PATH")
                .global(true)
                .long("pigz-path")
                .help("Specifies the path of your pigz executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_PIGZ_PATH)
            )
            .arg(Arg::with_name("BZIP2_PATH")
                .global(true)
                .long("bzip2-path")
                .help("Specifies the path of your bzip2 executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_BZIP2_PATH)
            )
            .arg(Arg::with_name("BUNZIP2_PATH")
                .global(true)
                .long("bunzip2-path")
                .help("Specifies the path of your bunzip2 executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_BUNZIP2_PATH)
            )
            .arg(Arg::with_name("LBZIP2_PATH")
                .global(true)
                .long("lbzip2-path")
                .help("Specifies the path of your lbzip2 executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_LBZIP2_PATH)
            )
            .arg(Arg::with_name("PBZIP2_PATH")
                .global(true)
                .long("pbzip2-path")
                .help("Specifies the path of your pbzip2 executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_PBZIP2_PATH)
            )
            .arg(Arg::with_name("LZIP_PATH")
                .global(true)
                .long("lzip-path")
                .help("Specifies the path of your lzip executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_LZIP_PATH)
            )
            .arg(Arg::with_name("LUNZIP_PATH")
                .global(true)
                .long("lunzip-path")
                .help("Specifies the path of your lunzip executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_LUNZIP_PATH)
            )
            .arg(Arg::with_name("PLZIP_PATH")
                .global(true)
                .long("plzip-path")
                .help("Specifies the path of your plzip executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_PLZIP_PATH)
            )
            .arg(Arg::with_name("XZ_PATH")
                .global(true)
                .long("xz-path")
                .help("Specifies the path of your xz executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_XZ_PATH)
            )
            .arg(Arg::with_name("UNXZ_PATH")
                .global(true)
                .long("unxz-path")
                .help("Specifies the path of your unxz executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_UNXZ_PATH)
            )
            .arg(Arg::with_name("PXZ_PATH")
                .global(true)
                .long("pxz-path")
                .help("Specifies the path of your pxz executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_PXZ_PATH)
            )
            .arg(Arg::with_name("LZMA_PATH")
                .global(true)
                .long("lzma-path")
                .help("Specifies the path of your lzma executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_LZMA_PATH)
            )
            .arg(Arg::with_name("UNLZMA_PATH")
                .global(true)
                .long("unlzma-path")
                .help("Specifies the path of your unlzma executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_UNLZMA_PATH)
            )
            .arg(Arg::with_name("7Z_PATH")
                .global(true)
                .long("7z-path")
                .help("Specifies the path of your 7z executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_7Z_PATH)
            )
            .arg(Arg::with_name("TAR_PATH")
                .global(true)
                .long("tar-path")
                .help("Specifies the path of your tar executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_TAR_PATH)
            )
            .arg(Arg::with_name("RAR_PATH")
                .global(true)
                .long("rar-path")
                .help("Specifies the path of your rar executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_RAR_PATH)
            )
            .arg(Arg::with_name("UNRAR_PATH")
                .global(true)
                .long("unrar-path")
                .help("Specifies the path of your unrar executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_UNRAR_PATH)
            )
            .subcommand(SubCommand::with_name("x")
                .about("Extracts files with full path.")
                .arg(Arg::with_name("INPUT_PATH")
                    .required(true)
                    .help("Assigns the source of your archived file. It should be a file path.")
                )
                .arg(Arg::with_name("OUTPUT_PATH")
                    .required(false)
                    .help("Assigns a destination of your extracted files. It should be a file path.")
                )
                .arg(Arg::with_name("OUTPUT_PATH2")
                    .long("output")
                    .short("o")
                    .help("Assigns a destination of your extracted files. It should be a file path.")
                    .takes_value(true)
                    .value_name("OUTPUT_PATH")
                )
                .after_help("Enjoy it! https://magiclen.org")
            )
            .subcommand(SubCommand::with_name("a")
                .about("Adds files to archive.")
                .arg(Arg::with_name("OUTPUT_PATH")
                    .long("output")
                    .short("o")
                    .help("Assigns a destination of your extracted files. It should be a file path. Specifies the file extension name in order to determine which archive format you want to use. [default archive format: RAR]")
                    .takes_value(true)
                )
                .arg(Arg::with_name("INPUT_PATH")
                    .required(true)
                    .help("Assigns the source of your original files. It should be at least one file path.")
                    .multiple(true)
                )
                .arg(Arg::with_name("BEST_COMPRESSION")
                    .long("best-compression")
                    .short("b")
                    .help("If you are OK about the compression and depression time and want to save more disk space and network traffic, it will make the archive file as small as possible.")
                )
                .after_help("Enjoy it! https://magiclen.org")
            )
            .after_help("Enjoy it! https://magiclen.org")
            .get_matches();

        let compress_path;
        let zip_path;
        let unzip_path;
        let gzip_path;
        let gunzip_path;
        let pigz_path;
        let bzip2_path;
        let bunzip2_path;
        let lbzip2_path;
        let pbzip2_path;
        let lzip_path;
        let lunzip_path;
        let plzip_path;
        let xz_path;
        let unxz_path;
        let pxz_path;
        let lzma_path;
        let unlzma_path;
        let p7z_path;
        let tar_path;
        let rar_path;
        let unrar_path;
        let password;
        let single_thread = matches.is_present("SINGLE_THREAD");
        let quiet = matches.is_present("QUIET");

        {
            let get_executable_path = |name, default_path| {
                let path = matches.value_of(name).unwrap();

                if path.ne(default_path) {
                    let path = Path::new(path);

                    let path = match path.canonicalize() {
                        Ok(path) => {
                            path
                        }
                        Err(_) => {
                            return Err(String::from("FFMPEG_PATH is incorrect."));
                        }
                    };

                    let path = path.to_str().unwrap();

                    Ok(String::from(path))
                } else {
                    Ok(String::from(path))
                }
            };

            compress_path = get_executable_path("COMPRESS_PATH", DEFAULT_COMPRESS_PATH)?;
            zip_path = get_executable_path("ZIP_PATH", DEFAULT_ZIP_PATH)?;
            unzip_path = get_executable_path("UNZIP_PATH", DEFAULT_UNZIP_PATH)?;
            gzip_path = get_executable_path("GZIP_PATH", DEFAULT_GZIP_PATH)?;
            gunzip_path = get_executable_path("GUNZIP_PATH", DEFAULT_GUNZIP_PATH)?;
            pigz_path = get_executable_path("PIGZ_PATH", DEFAULT_PIGZ_PATH)?;
            bzip2_path = get_executable_path("BZIP2_PATH", DEFAULT_BZIP2_PATH)?;
            bunzip2_path = get_executable_path("BUNZIP2_PATH", DEFAULT_BUNZIP2_PATH)?;
            lbzip2_path = get_executable_path("LBZIP2_PATH", DEFAULT_LBZIP2_PATH)?;
            pbzip2_path = get_executable_path("PBZIP2_PATH", DEFAULT_PBZIP2_PATH)?;
            lzip_path = get_executable_path("LZIP_PATH", DEFAULT_LZIP_PATH)?;
            lunzip_path = get_executable_path("LUNZIP_PATH", DEFAULT_LUNZIP_PATH)?;
            plzip_path = get_executable_path("PLZIP_PATH", DEFAULT_PLZIP_PATH)?;
            xz_path = get_executable_path("XZ_PATH", DEFAULT_XZ_PATH)?;
            unxz_path = get_executable_path("UNXZ_PATH", DEFAULT_UNXZ_PATH)?;
            pxz_path = get_executable_path("PXZ_PATH", DEFAULT_PXZ_PATH)?;
            lzma_path = get_executable_path("LZMA_PATH", DEFAULT_LZMA_PATH)?;
            unlzma_path = get_executable_path("UNLZMA_PATH", DEFAULT_UNLZMA_PATH)?;
            p7z_path = get_executable_path("7Z_PATH", DEFAULT_7Z_PATH)?;
            tar_path = get_executable_path("TAR_PATH", DEFAULT_TAR_PATH)?;
            rar_path = get_executable_path("RAR_PATH", DEFAULT_RAR_PATH)?;
            unrar_path = get_executable_path("UNRAR_PATH", DEFAULT_UNRAR_PATH)?;
        }

        password = match matches.value_of("PASSWORD") {
            Some(p) => String::from(p),
            None => String::from("")
        };

        let mode = if matches.is_present("x") {
            let sub_matches = matches.subcommand_matches("x").unwrap();

            let input_path = sub_matches.value_of("INPUT_PATH").unwrap();

            let mut path = Path::new(input_path);

            match path.canonicalize() {
                Ok(path) => {
                    let path = path.to_str().unwrap();

                    let input_path = String::from(path);

                    let output_path = sub_matches.value_of("OUTPUT_PATH");
                    let mut output_path = match output_path {
                        Some(p) => {
                            Some(String::from(p))
                        }
                        None => None
                    };

                    let output_path2 = sub_matches.value_of("OUTPUT_PATH2");
                    let output_path2 = match output_path2 {
                        Some(p) => {
                            Some(String::from(p))
                        }
                        None => None
                    };

                    if output_path2 != None {
                        if output_path != None {
                            if let Some(ref a) = output_path {
                                if let Some(ref b) = output_path2 {
                                    if a.ne(b) {
                                        return Err(String::from("You input different output paths."));
                                    }
                                }
                            }
                        } else {
                            output_path = output_path2;
                        }
                    }

                    Ok(Mode::Extract(input_path, output_path))
                }
                Err(ref error) if error.kind() == ErrorKind::NotFound => {
                    Err(format!("{} does not exist.", input_path))
                }
                Err(_) => {
                    Err(format!("{} is incorrect.", input_path))
                }
            }
        } else if matches.is_present("a") {
            let sub_matches = matches.subcommand_matches("a").unwrap();

            let input_path = sub_matches.values_of("INPUT_PATH").unwrap();

            let output_path = sub_matches.value_of("OUTPUT_PATH");
            let mut output_path = match output_path {
                Some(p) => {
                    Some(String::from(p))
                }
                None => None
            };

            let best_compression = sub_matches.is_present("BEST_COMPRESSION");

            let mut input_paths = Vec::new();

            for input_path in input_path {
                let mut path = Path::new(input_path);

                match path.canonicalize() {
                    Ok(path) => {
                        let path = path.to_str().unwrap();

                        input_paths.push(String::from(path));
                    }
                    Err(ref error) if error.kind() == ErrorKind::NotFound => {
                        return Err(format!("{} does not exist.", input_path));
                    }
                    Err(_) => {
                        return Err(format!("{} is incorrect.", input_path));
                    }
                }
            }

            Ok(Mode::Archive(best_compression, input_paths, output_path))
        } else {
            Err(String::from("Please input a subcommand. Use `help` to see how to use this program."))
        }?;

        let paths = ExePaths {
            compress_path,
            zip_path,
            unzip_path,
            gzip_path,
            gunzip_path,
            pigz_path,
            bzip2_path,
            bunzip2_path,
            lbzip2_path,
            pbzip2_path,
            lzip_path,
            lunzip_path,
            plzip_path,
            xz_path,
            unxz_path,
            pxz_path,
            lzma_path,
            unlzma_path,
            p7z_path,
            tar_path,
            rar_path,
            unrar_path,
        };

        Ok(Config {
            paths,
            single_thread,
            quiet,
            password,
            mode,
        })
    }
}

// TODO -----Config END-----

// TODO -----ArchiveFormat START-----

#[derive(Debug)]
enum ArchiveFormat {
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
    Rar,
}

impl ArchiveFormat {
    fn get_archive_format_from_file_path(file_path: &str) -> Result<ArchiveFormat, &'static str> {
        let file_path = file_path.to_lowercase();

        if file_path.ends_with(".tar.z") {
            Ok(ArchiveFormat::TarZ)
        } else if file_path.ends_with(".tar.gz") || file_path.ends_with(".tgz") {
            Ok(ArchiveFormat::TarGzip)
        } else if file_path.ends_with(".tar.bz2") || file_path.ends_with(".tbz2") {
            Ok(ArchiveFormat::TarBzip2)
        } else if file_path.ends_with(".tar.lz") {
            Ok(ArchiveFormat::TarLz)
        } else if file_path.ends_with(".tar.xz") || file_path.ends_with(".txz") {
            Ok(ArchiveFormat::TarXz)
        } else if file_path.ends_with(".tar.lzma") || file_path.ends_with(".tlz") {
            Ok(ArchiveFormat::TarLzma)
        } else if file_path.ends_with(".tar.7z") || file_path.ends_with(".t7z") {
            Ok(ArchiveFormat::Tar7z)
        } else if file_path.ends_with(".tar") {
            Ok(ArchiveFormat::Tar)
        } else if file_path.ends_with(".z") {
            Ok(ArchiveFormat::Z)
        } else if file_path.ends_with(".zip") {
            Ok(ArchiveFormat::Zip)
        } else if file_path.ends_with(".gz") {
            Ok(ArchiveFormat::Gzip)
        } else if file_path.ends_with(".bz2") {
            Ok(ArchiveFormat::Bzip2)
        } else if file_path.ends_with(".lz") {
            Ok(ArchiveFormat::Lz)
        } else if file_path.ends_with(".xz") {
            Ok(ArchiveFormat::Xz)
        } else if file_path.ends_with(".lzma") {
            Ok(ArchiveFormat::Lzma)
        } else if file_path.ends_with(".7z") {
            Ok(ArchiveFormat::P7z)
        } else if file_path.ends_with(".rar") {
            Ok(ArchiveFormat::Rar)
        } else {
            Err("Unknown archive format.")
        }
    }
}

// TODO -----ArchiveFormat END-----

// TODO -----Process START-----

fn check_executable(cmd: &[&str]) -> Result<(), ()> {
    let process = Exec::cmd(cmd[0]).args(&cmd[1..]).stdout(NullFile {}).stderr(NullFile {});

    match execute_join(process) {
        Ok(es) => {
            if es == 0 {
                Ok(())
            } else {
                Err(())
            }
        }
        Err(_) => Err(())
    }
}

fn execute_one_stream_to_file(cmd: &[&str], cwd: &str, file_name: &str) -> Result<(), String> {
    match execute_one_stream(&cmd, cwd) {
        Ok(read) => {
            let mut reader = BufReader::new(read);

            let new_file_path = Path::join(Path::new(cwd), Path::new(file_name));

            let output_file = match fs::File::create(&new_file_path) {
                Ok(file) => file,
                Err(error) => {
                    return Err(error.to_string());
                }
            };

            let mut writer = BufWriter::new(output_file);

            let mut buffer = [0u8; BUFFER_SIZE];

            loop {
                match reader.read(&mut buffer) {
                    Ok(c) => {
                        if c == 0 {
                            break;
                        }
                        if let Err(error) = writer.write(&buffer[0..c]) {
                            try_delete_file(new_file_path.to_str().unwrap());
                            return Err(error.to_string());
                        }
                        if let Err(error) = writer.flush() {
                            try_delete_file(new_file_path.to_str().unwrap());
                            return Err(error.to_string());
                        }
                    }
                    Err(error) => {
                        try_delete_file(new_file_path.to_str().unwrap());
                        return Err(error.to_string());
                    }
                }
            }

            return Ok(());
        }
        Err(error) => {
            return Err(error);
        }
    }
}

fn execute_one_stream(cmd: &[&str], cwd: &str) -> Result<Box<Read>, String> {
    if let Err(error) = fs::create_dir_all(cwd) {
        return Err(error.to_string());
    }

    let process = Exec::cmd(cmd[0]).cwd(cwd).args(&cmd[1..]);

    match process.stream_stdout() {
        Ok(read) => Ok(read),
        Err(error) => Err(error.to_string())
    }
}

fn execute_one_quiet(cmd: &[&str], cwd: &str) -> Result<i32, String> {
    if let Err(error) = fs::create_dir_all(cwd) {
        return Err(error.to_string());
    }

    let process = Exec::cmd(cmd[0]).cwd(cwd).args(&cmd[1..]).stdout(NullFile {});

    match execute_join(process) {
        Ok(es) => Ok(es),
        Err(error) => Err(error.to_string())
    }
}

fn execute_one(cmd: &[&str], cwd: &str) -> Result<i32, String> {
    if let Err(error) = fs::create_dir_all(cwd) {
        return Err(error.to_string());
    }

    let process = Exec::cmd(cmd[0]).cwd(cwd).args(&cmd[1..]);

    match execute_join(process) {
        Ok(es) => Ok(es),
        Err(error) => Err(error.to_string())
    }
}

fn execute_two(cmd1: &[&str], cmd2: &[&str], cwd: &str) -> Result<i32, String> {
    if let Err(error) = fs::create_dir_all(cwd) {
        return Err(error.to_string());
    }

    let process = { Exec::cmd(cmd1[0]).cwd(cwd).args(&cmd1[1..]) | Exec::cmd(cmd2[0]).cwd(cwd).args(&cmd2[1..]) };

    match execute_join_pipeline(process) {
        Ok(es) => Ok(es),
        Err(error) => Err(error.to_string())
    }
}

fn execute_join_pipeline(process: Pipeline) -> Result<i32, PopenError> {
    match process.join() {
        Ok(es) => {
            match es {
                ExitStatus::Exited(c) => Ok(c as i32),
                ExitStatus::Signaled(c) => Ok(c as i32),
                ExitStatus::Other(c) => Ok(c),
                _ => Ok(-1),
            }
        }
        Err(error) => {
            Err(error)
        }
    }
}


fn execute_join(process: Exec) -> Result<i32, PopenError> {
    match process.join() {
        Ok(es) => {
            match es {
                ExitStatus::Exited(c) => Ok(c as i32),
                ExitStatus::Signaled(c) => Ok(c as i32),
                ExitStatus::Other(c) => Ok(c),
                _ => Ok(-1),
            }
        }
        Err(error) => {
            Err(error)
        }
    }
}

// TODO -----Process END-----

pub fn run(config: Config) -> Result<i32, String> {
    let password = config.password;
    let paths = config.paths;
    let cpus = if config.single_thread {
        1
    } else {
        num_cpus::get()
    };
    let quiet = config.quiet;

    match config.mode {
        Mode::Archive(best_compression, input_paths, output_path) => {
            match output_path {
                Some(p) => {
                    archive(paths, quiet, cpus, &password, best_compression, &input_paths, &p)?;
                }
                None => {
                    let current_dir = env::current_dir().unwrap();

                    let input_path = Path::new(&input_paths[0]);

                    let output_path = Path::join(&current_dir, Path::new(&format!("{}.rar", input_path.file_stem().unwrap().to_str().unwrap())));

                    archive(paths, quiet, cpus, &password, best_compression, &input_paths, output_path.to_str().unwrap())?;
                }
            }
        }
        Mode::Extract(input_path, output_path) => {
            match output_path {
                Some(p) => {
                    extract(paths, quiet, cpus, &password, &input_path, &p)?;
                }
                None => {
                    let current_dir = env::current_dir().unwrap();

                    extract(paths, quiet, cpus, &password, &input_path, current_dir.to_str().unwrap())?;
                }
            }
        }
    }

    Ok(0)
}

pub fn archive(paths: ExePaths, quiet: bool, cpus: usize, password: &str, best_compression: bool, input_paths: &Vec<String>, output_path: &str) -> Result<i32, String> {
    let format = match ArchiveFormat::get_archive_format_from_file_path(output_path) {
        Ok(f) => f,
        Err(err) => return Err(String::from(err))
    };

    let output_path_obj = Path::new(output_path);

    let output_folder = output_path_obj.parent().unwrap().to_str().unwrap();

    let threads = cpus.to_string();
    let threads = threads.as_str();

    match format {
        ArchiveFormat::Rar => {
            let password_arg = format!("-hp{}", create_cli_string(&password));
            let thread_arg = format!("-mt{}", threads);
            let mut cmd = vec![paths.rar_path.as_str(), "a", "-ep1"];

            cmd.push(thread_arg.as_str());

            if best_compression {
                cmd.push("-ma5");
                cmd.push("-m5");
                cmd.push("-s");
            }

            if !password.is_empty() {
                cmd.push(password_arg.as_str());
            }

            if quiet {
                cmd.push("-idq");
            }

            cmd.push(output_path);

            for input_path in input_paths {
                cmd.push(input_path);
            }

            if output_path_obj.exists() {
                if let Err(error) = fs::remove_file(output_path_obj) {
                    return Err(error.to_string());
                }
            }

            execute_one(&cmd, output_folder)
        }
        _ => Err(String::from("Cannot handle this format yet."))
    }
}

pub fn extract(paths: ExePaths, quiet: bool, cpus: usize, password: &str, input_path: &str, output_path: &str) -> Result<i32, String> {
    let format = match ArchiveFormat::get_archive_format_from_file_path(input_path) {
        Ok(f) => f,
        Err(err) => return Err(String::from(err))
    };

    let threads = cpus.to_string();
    let threads = threads.as_str();

    match format {
        ArchiveFormat::TarZ | ArchiveFormat::TarGzip => {
            if cpus > 1 {
                if let Ok(_) = check_executable(&vec![paths.pigz_path.as_str(), "-V"]) {
                    let cmd1 = vec![paths.pigz_path.as_str(), "-d", "-c", "-p", threads, input_path];

                    let mut cmd2 = vec![paths.tar_path.as_str(), "-x"];

                    if !quiet {
                        cmd2.push("-v");
                    }

                    cmd2.push("-f");
                    cmd2.push("-");

                    return execute_two(&cmd1, &cmd2, output_path);
                }
            }

            let mut cmd = vec![paths.tar_path.as_str(), "-z", "-x"];

            if !quiet {
                cmd.push("-v");
            }

            cmd.push("-f");
            cmd.push(input_path);

            execute_one(&cmd, output_path)
        }
        ArchiveFormat::TarBzip2 => {
            if cpus > 1 {
                if let Ok(_) = check_executable(&vec![paths.lbzip2_path.as_str(), "-V"]) {
                    let cmd1 = vec![paths.lbzip2_path.as_str(), "-d", "-c", "-n", threads, input_path];

                    let mut cmd2 = vec![paths.tar_path.as_str(), "-x"];

                    if !quiet {
                        cmd2.push("-v");
                    }

                    cmd2.push("-f");
                    cmd2.push("-");

                    return execute_two(&cmd1, &cmd2, output_path);
                } else if let Ok(_) = check_executable(&vec![paths.pbzip2_path.as_str(), "-V"]) {
                    let cmd1 = format!("-p{}", threads);
                    let cmd1 = vec![paths.pbzip2_path.as_str(), "-d", "-c", cmd1.as_str(), input_path];

                    let mut cmd2 = vec![paths.tar_path.as_str(), "-x"];

                    if !quiet {
                        cmd2.push("-v");
                    }

                    cmd2.push("-f");
                    cmd2.push("-");

                    return execute_two(&cmd1, &cmd2, output_path);
                }
            }

            let mut cmd = vec![paths.tar_path.as_str(), "-j", "-x"];

            if !quiet {
                cmd.push("-v");
            }

            cmd.push("-f");
            cmd.push(input_path);

            execute_one(&cmd, output_path)
        }
        ArchiveFormat::TarLz => {
            if cpus > 11 {
                if let Ok(_) = check_executable(&vec![paths.plzip_path.as_str(), "-V"]) {
                    let cmd1 = vec![paths.plzip_path.as_str(), "-d", "-c", "-n", threads, input_path];

                    let mut cmd2 = vec![paths.tar_path.as_str(), "-x"];

                    if !quiet {
                        cmd2.push("-v");
                    }

                    cmd2.push("-f");
                    cmd2.push("-");

                    return execute_two(&cmd1, &cmd2, output_path);
                }
            }

            if let Ok(_) = check_executable(&vec![paths.lunzip_path.as_str(), "-V"]) {
                let mut cmd = vec![paths.tar_path.as_str(), "-I", paths.lunzip_path.as_str(), "-x"];

                if !quiet {
                    cmd.push("-v");
                }

                cmd.push("-f");
                cmd.push(input_path);

                return execute_one(&cmd, output_path);
            }

            let mut cmd = vec![paths.tar_path.as_str(), "-I", paths.lzip_path.as_str(), "-x"];

            if !quiet {
                cmd.push("-v");
            }

            cmd.push("-f");
            cmd.push(input_path);

            execute_one(&cmd, output_path)
        }
        ArchiveFormat::TarXz => {
            if cpus > 1 {
                if let Ok(_) = check_executable(&vec![paths.pxz_path.as_str(), "-V"]) {
                    let cmd1 = vec![paths.pxz_path.as_str(), "-d", "-c", "-T", threads, input_path];

                    let mut cmd2 = vec![paths.tar_path.as_str(), "-x"];

                    if !quiet {
                        cmd2.push("-v");
                    }

                    cmd2.push("-f");
                    cmd2.push("-");

                    return execute_two(&cmd1, &cmd2, output_path);
                } else if let Ok(_) = check_executable(&vec![paths.unxz_path.as_str(), "-V"]) {
                    let cmd1 = vec![paths.unxz_path.as_str(), "-c", "-T", threads, input_path];

                    let mut cmd2 = vec![paths.tar_path.as_str(), "-x"];

                    if !quiet {
                        cmd2.push("-v");
                    }

                    cmd2.push("-f");
                    cmd2.push("-");

                    return execute_two(&cmd1, &cmd2, output_path);
                } else if let Ok(_) = check_executable(&vec![paths.xz_path.as_str(), "-V"]) {
                    let cmd1 = vec![paths.xz_path.as_str(), "-d", "-c", "-T", threads, input_path];

                    let mut cmd2 = vec![paths.tar_path.as_str(), "-x"];

                    if !quiet {
                        cmd2.push("-v");
                    }

                    cmd2.push("-f");
                    cmd2.push("-");

                    return execute_two(&cmd1, &cmd2, output_path);
                }
            }

            let mut cmd = vec![paths.tar_path.as_str(), "-J", "-x"];

            if !quiet {
                cmd.push("-v");
            }

            cmd.push("-f");
            cmd.push(input_path);

            execute_one(&cmd, output_path)
        }
        ArchiveFormat::TarLzma => {
            if cpus > 1 {
                if let Ok(_) = check_executable(&vec![paths.pxz_path.as_str(), "-V"]) {
                    let cmd1 = vec![paths.pxz_path.as_str(), "-d", "-c", "-T", threads, "-F", "lzma", input_path];

                    let mut cmd2 = vec![paths.tar_path.as_str(), "-x"];

                    if !quiet {
                        cmd2.push("-v");
                    }

                    cmd2.push("-f");
                    cmd2.push("-");

                    return execute_two(&cmd1, &cmd2, output_path);
                } else if let Ok(_) = check_executable(&vec![paths.unlzma_path.as_str(), "-V"]) {
                    let cmd1 = vec![paths.unlzma_path.as_str(), "-c", "-T", threads, input_path];

                    let mut cmd2 = vec![paths.tar_path.as_str(), "-x"];

                    if !quiet {
                        cmd2.push("-v");
                    }

                    cmd2.push("-f");
                    cmd2.push("-");

                    return execute_two(&cmd1, &cmd2, output_path);
                } else if let Ok(_) = check_executable(&vec![paths.lzma_path.as_str(), "-V"]) {
                    let cmd1 = vec![paths.lzma_path.as_str(), "-d", "-c", "-T", threads, input_path];

                    let mut cmd2 = vec![paths.tar_path.as_str(), "-x"];

                    if !quiet {
                        cmd2.push("-v");
                    }

                    cmd2.push("-f");
                    cmd2.push("-");

                    return execute_two(&cmd1, &cmd2, output_path);
                }
            }

            if let Ok(_) = check_executable(&vec![paths.unlzma_path.as_str(), "-V"]) {
                let mut cmd = vec![paths.tar_path.as_str(), "-I", paths.unlzma_path.as_str(), "-x"];

                if !quiet {
                    cmd.push("-v");
                }

                cmd.push("-f");
                cmd.push(input_path);

                return execute_one(&cmd, output_path);
            }

            let mut cmd = vec![paths.tar_path.as_str(), "-I", paths.lzma_path.as_str(), "-x"];

            if !quiet {
                cmd.push("-v");
            }

            cmd.push("-f");
            cmd.push(input_path);

            execute_one(&cmd, output_path)
        }
        ArchiveFormat::Tar7z => {
            let password_arg = format!("-p{}", create_cli_string(&password));
            let thread_arg = format!("-mmt{}", threads);
            let mut cmd1 = vec![paths.p7z_path.as_str(), "x", "-so", thread_arg.as_str()];

            if !password.is_empty() {
                cmd1.push(password_arg.as_str());
            }

            cmd1.push(input_path);

            let mut cmd2 = vec![paths.tar_path.as_str(), "-x"];

            if !quiet {
                cmd2.push("-v");
            }

            cmd2.push("-f");
            cmd2.push("-");

            return execute_two(&cmd1, &cmd2, output_path);
        }
        ArchiveFormat::Tar => {
            let mut cmd = vec![paths.tar_path.as_str(), "-x"];

            if !quiet {
                cmd.push("-v");
            }

            cmd.push("-f");
            cmd.push(input_path);

            execute_one(&cmd, output_path)
        }
        ArchiveFormat::Z | ArchiveFormat::Gzip => {
            if cpus > 1 {
                if let Ok(_) = check_executable(&vec![paths.pigz_path.as_str(), "-V"]) {
                    let cmd = vec![paths.pigz_path.as_str(), "-d", "-c", "-p", threads, input_path];

                    let file_path = Path::new(input_path);

                    match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                        Ok(_) => {
                            return Ok(0);
                        }
                        Err(error) => {
                            return Err(error);
                        }
                    }
                }
            }

            if let Ok(_) = check_executable(&vec![paths.gunzip_path.as_str(), "-V"]) {
                let cmd = vec![paths.gzip_path.as_str(), "-c", input_path];

                let file_path = Path::new(input_path);

                match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                    Ok(_) => {
                        return Ok(0);
                    }
                    Err(error) => {
                        return Err(error);
                    }
                }
            }

            let cmd = vec![paths.gzip_path.as_str(), "-d", "-c", input_path];

            let file_path = Path::new(input_path);

            match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                Ok(_) => {
                    return Ok(0);
                }
                Err(error) => {
                    return Err(error);
                }
            }
        }
        ArchiveFormat::Bzip2 => {
            if cpus > 1 {
                if let Ok(_) = check_executable(&vec![paths.lbzip2_path.as_str(), "-V"]) {
                    let cmd = vec![paths.lbzip2_path.as_str(), "-d", "-c", "-n", threads, input_path];

                    let file_path = Path::new(input_path);

                    match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                        Ok(_) => {
                            return Ok(0);
                        }
                        Err(error) => {
                            return Err(error);
                        }
                    }
                } else if let Ok(_) = check_executable(&vec![paths.pbzip2_path.as_str(), "-V"]) {
                    let cmd = format!("-p{}", threads);
                    let cmd = vec![paths.pbzip2_path.as_str(), "-d", "-c", cmd.as_str(), input_path];

                    let file_path = Path::new(input_path);

                    match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                        Ok(_) => {
                            return Ok(0);
                        }
                        Err(error) => {
                            return Err(error);
                        }
                    }
                }
            }

            if let Ok(_) = check_executable(&vec![paths.bunzip2_path.as_str(), "-V"]) {
                let cmd = vec![paths.bzip2_path.as_str(), "-c", input_path];

                let file_path = Path::new(input_path);

                match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                    Ok(_) => {
                        return Ok(0);
                    }
                    Err(error) => {
                        return Err(error);
                    }
                }
            }

            let cmd = vec![paths.bzip2_path.as_str(), "-d", "-c", input_path];

            let file_path = Path::new(input_path);

            match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                Ok(_) => {
                    return Ok(0);
                }
                Err(error) => {
                    return Err(error);
                }
            }
        }
        ArchiveFormat::Lz => {
            if cpus > 1 {
                if let Ok(_) = check_executable(&vec![paths.plzip_path.as_str(), "-V"]) {
                    let cmd = vec![paths.plzip_path.as_str(), "-d", "-c", "-n", threads, input_path];

                    let file_path = Path::new(input_path);

                    match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                        Ok(_) => {
                            return Ok(0);
                        }
                        Err(error) => {
                            return Err(error);
                        }
                    }
                }
            }

            if let Ok(_) = check_executable(&vec![paths.lunzip_path.as_str(), "-V"]) {
                let cmd = vec![paths.lunzip_path.as_str(), "-c", input_path];

                let file_path = Path::new(input_path);

                match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                    Ok(_) => {
                        return Ok(0);
                    }
                    Err(error) => {
                        return Err(error);
                    }
                }
            }

            let cmd = vec![paths.lzip_path.as_str(), "-d", "-c", input_path];

            let file_path = Path::new(input_path);

            match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                Ok(_) => {
                    return Ok(0);
                }
                Err(error) => {
                    return Err(error);
                }
            }
        }
        ArchiveFormat::Xz => {
            if cpus > 1 {
                if let Ok(_) = check_executable(&vec![paths.pxz_path.as_str(), "-V"]) {
                    let cmd = vec![paths.pxz_path.as_str(), "-d", "-c", "-T", threads, input_path];

                    let file_path = Path::new(input_path);

                    match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                        Ok(_) => {
                            return Ok(0);
                        }
                        Err(error) => {
                            return Err(error);
                        }
                    }
                } else if let Ok(_) = check_executable(&vec![paths.unxz_path.as_str(), "-V"]) {
                    let cmd = vec![paths.unxz_path.as_str(), "-c", "-T", threads, input_path];

                    let file_path = Path::new(input_path);

                    match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                        Ok(_) => {
                            return Ok(0);
                        }
                        Err(error) => {
                            return Err(error);
                        }
                    }
                } else if let Ok(_) = check_executable(&vec![paths.xz_path.as_str(), "-V"]) {
                    let cmd = vec![paths.xz_path.as_str(), "-d", "-c", "-T", threads, input_path];

                    let file_path = Path::new(input_path);

                    match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                        Ok(_) => {
                            return Ok(0);
                        }
                        Err(error) => {
                            return Err(error);
                        }
                    }
                }
            }

            if let Ok(_) = check_executable(&vec![paths.unxz_path.as_str(), "-V"]) {
                let cmd = vec![paths.xz_path.as_str(), "-c", input_path];

                let file_path = Path::new(input_path);

                match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                    Ok(_) => {
                        return Ok(0);
                    }
                    Err(error) => {
                        return Err(error);
                    }
                }
            }

            let cmd = vec![paths.xz_path.as_str(), "-d", "-c", input_path];

            let file_path = Path::new(input_path);

            match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                Ok(_) => {
                    return Ok(0);
                }
                Err(error) => {
                    return Err(error);
                }
            }
        }
        ArchiveFormat::Lzma => {
            if cpus > 1 {
                if let Ok(_) = check_executable(&vec![paths.pxz_path.as_str(), "-V"]) {
                    let cmd = vec![paths.pxz_path.as_str(), "-d", "-c", "-T", threads, "-F", "lzma", input_path];

                    let file_path = Path::new(input_path);

                    match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                        Ok(_) => {
                            return Ok(0);
                        }
                        Err(error) => {
                            return Err(error);
                        }
                    }
                } else if let Ok(_) = check_executable(&vec![paths.unlzma_path.as_str(), "-V"]) {
                    let cmd = vec![paths.unlzma_path.as_str(), "-c", "-T", threads, input_path];

                    let file_path = Path::new(input_path);

                    match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                        Ok(_) => {
                            return Ok(0);
                        }
                        Err(error) => {
                            return Err(error);
                        }
                    }
                } else if let Ok(_) = check_executable(&vec![paths.lzma_path.as_str(), "-V"]) {
                    let cmd = vec![paths.lzma_path.as_str(), "-d", "-c", "-T", threads, input_path];

                    let file_path = Path::new(input_path);

                    match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                        Ok(_) => {
                            return Ok(0);
                        }
                        Err(error) => {
                            return Err(error);
                        }
                    }
                }
            }

            if let Ok(_) = check_executable(&vec![paths.unlzma_path.as_str(), "-V"]) {
                let cmd = vec![paths.unlzma_path.as_str(), "-c", input_path];

                let file_path = Path::new(input_path);

                match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                    Ok(_) => {
                        return Ok(0);
                    }
                    Err(error) => {
                        return Err(error);
                    }
                }
            }

            let cmd = vec![paths.lzma_path.as_str(), "-d", "-c", input_path];

            let file_path = Path::new(input_path);

            match execute_one_stream_to_file(&cmd, output_path, file_path.file_stem().unwrap().to_str().unwrap()) {
                Ok(_) => {
                    return Ok(0);
                }
                Err(error) => {
                    return Err(error);
                }
            }
        }
        ArchiveFormat::P7z => {
            let output_path_arg = format!("-o{}", create_cli_string(output_path));
            let password_arg = format!("-p{}", create_cli_string(&password));
            let thread_arg = format!("-mmt{}", threads);
            let mut cmd = vec![paths.p7z_path.as_str(), "x", "-aoa", thread_arg.as_str(), output_path_arg.as_str()];

            if !password.is_empty() {
                cmd.push(password_arg.as_str());
            }

            cmd.push(input_path);

            if quiet {
                execute_one_quiet(&cmd, output_path)
            } else {
                execute_one(&cmd, output_path)
            }
        }
        ArchiveFormat::Zip => {
            let mut cmd = vec![paths.unzip_path.as_str()];

            if !password.is_empty() {
                cmd.push("-P");
                cmd.push(password);
            }

            if quiet {
                cmd.push("-qq");
            }

            cmd.push("-o");
            cmd.push(input_path);
            cmd.push("-d");
            cmd.push(output_path);

            execute_one(&cmd, output_path)
        }
        ArchiveFormat::Rar => {
            let password_arg = format!("-p{}", create_cli_string(&password));
            let thread_arg = format!("-mt{}", threads);

            if let Ok(_) = check_executable(&vec![paths.unrar_path.as_str(), "-?"]) {
                let mut cmd = vec![paths.unrar_path.as_str(), "x", "-o+"];

                cmd.push(thread_arg.as_str());

                if password.is_empty() {
                    cmd.push("-p-");
                } else {
                    cmd.push(password_arg.as_str());
                }

                if quiet {
                    cmd.push("-idq");
                }

                cmd.push(input_path);
                cmd.push(output_path);

                return execute_one(&cmd, output_path);
            }

            let mut cmd = vec![paths.rar_path.as_str(), "x", "-o+"];

            cmd.push(thread_arg.as_str());

            if password.is_empty() {
                cmd.push("-p-");
            } else {
                cmd.push(password_arg.as_str());
            }

            if quiet {
                cmd.push("-idq");
            }

            cmd.push(input_path);
            cmd.push(output_path);

            execute_one(&cmd, output_path)
        }
        // _ => Err(String::from("Cannot handle this format yet."))
    }
}

fn try_delete_file(file_path: &str) {
    match fs::remove_file(file_path) {
        _ => {}
    }
}

fn create_cli_string(string: &str) -> String {
    string.replace(" ", "\\ ")
}


// TODO -----Test START-----

#[cfg(test)]
mod test {
    // use super::*;
}

// TODO -----Test END-----