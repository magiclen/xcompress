extern crate clap;
extern crate num_cpus;
extern crate subprocess;

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use subprocess::Exec;

use clap::{App, Arg, SubCommand};

// TODO -----Config START-----

const APP_NAME: &str = "XCompress";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const DEFAULT_UNCOMPRESS_PATH: &str = "uncompress";
const DEFAULT_UNZIP_PATH: &str = "unzip";
const DEFAULT_GUNZIP_PATH: &str = "gunzip";
const DEFAULT_BUNZIP2_PATH: &str = "bunzip2";
const DEFAULT_LBZIP2_PATH: &str = "lbzip2";
const DEFAULT_XZ_PATH: &str = "xz";
const DEFAULT_PXZ_PATH: &str = "pxz";
const DEFAULT_7Z_PATH: &str = "7z";
const DEFAULT_TAR_PATH: &str = "tar";
const DEFAULT_RAR_PATH: &str = "rar";
const DEFAULT_UNRAR_PATH: &str = "unrar";

#[derive(Debug)]
pub enum Mode {
    Archive(Box<[String]>, Option<String>),
    Extract(String, Option<String>),
}

#[derive(Debug)]
pub struct Config {
    pub uncompress_path: String,
    pub unzip_path: String,
    pub gunzip_path: String,
    pub bunzip2_path: String,
    pub lbzip2_path: String,
    pub xz_path: String,
    pub pxz_path: String,
    pub p7z_path: String,
    pub tar_path: String,
    pub rar_path: String,
    pub unrar_path: String,
    pub quiet: bool,
    pub mode: Mode,
}

impl Config {
    pub fn new() -> Result<Config, String> {
        let matches = App::new(APP_NAME)
            .version(CARGO_PKG_VERSION)
            .author(CARGO_PKG_AUTHORS)
            .about("XCompress is a free file archiver utility on Linux, providing multi-format archiving to ZIP, Z, GZIP, BZ2, XZ, 7Z, TAR and RAR.")
            .arg(Arg::with_name("quiet")
                .global(true)
                .long("quiet")
                .short("q")
                .help("Makes programs not print anything on the screen.")
            )
            .arg(Arg::with_name("UNCOMPRESS_PATH")
                .global(true)
                .long("uncompress-path")
                .help("Specifies the path of your uncompress executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_UNCOMPRESS_PATH)
            )
            .arg(Arg::with_name("UNZIP_PATH")
                .global(true)
                .long("unzip-path")
                .help("Specifies the path of your unzip executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_UNZIP_PATH)
            )
            .arg(Arg::with_name("GUNZIP_PATH")
                .global(true)
                .long("gunzip-path")
                .help("Specifies the path of your gunzip executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_GUNZIP_PATH)
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
            .arg(Arg::with_name("XZ_PATH")
                .global(true)
                .long("xz-path")
                .help("Specifies the path of your xz executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_XZ_PATH)
            )
            .arg(Arg::with_name("PXZ_PATH")
                .global(true)
                .long("pxz-path")
                .help("Specifies the path of your pxz executable binary file.")
                .takes_value(true)
                .default_value(DEFAULT_PXZ_PATH)
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
                .after_help("Enjoy it! https://magiclen.org")
            )
            .after_help("Enjoy it! https://magiclen.org")
            .get_matches();

        let uncompress_path;
        let unzip_path;
        let gunzip_path;
        let bunzip2_path;
        let lbzip2_path;
        let xz_path;
        let pxz_path;
        let p7z_path;
        let tar_path;
        let rar_path;
        let unrar_path;
        let quiet = false;

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

            uncompress_path = get_executable_path("UNCOMPRESS_PATH", DEFAULT_UNCOMPRESS_PATH)?;
            unzip_path = get_executable_path("UNZIP_PATH", DEFAULT_UNZIP_PATH)?;
            gunzip_path = get_executable_path("GUNZIP_PATH", DEFAULT_GUNZIP_PATH)?;
            bunzip2_path = get_executable_path("BUNZIP2_PATH", DEFAULT_BUNZIP2_PATH)?;
            lbzip2_path = get_executable_path("LBZIP2_PATH", DEFAULT_LBZIP2_PATH)?;
            xz_path = get_executable_path("XZ_PATH", DEFAULT_XZ_PATH)?;
            pxz_path = get_executable_path("PXZ_PATH", DEFAULT_PXZ_PATH)?;
            p7z_path = get_executable_path("7Z_PATH", DEFAULT_7Z_PATH)?;
            tar_path = get_executable_path("TAR_PATH", DEFAULT_TAR_PATH)?;
            rar_path = get_executable_path("RAR_PATH", DEFAULT_RAR_PATH)?;
            unrar_path = get_executable_path("UNRAR_PATH", DEFAULT_UNRAR_PATH)?;
        }

        let mode = if matches.is_present("x") {
            let input_path = matches.subcommand_matches("x").unwrap().value_of("INPUT_PATH").unwrap();

            let mut path = Path::new(input_path);

            match path.canonicalize() {
                Ok(path) => {
                    let path = path.to_str().unwrap();

                    let input_path = String::from(path);

                    let output_path = matches.value_of("OUTPUT_PATH");
                    let output_path = match output_path {
                        Some(p) => {
                            Some(String::from(p))
                        }
                        None => None
                    };

                    Ok(Mode::Extract(input_path, output_path))
                }
                Err(ref error) if error.kind() == ErrorKind::NotFound => {
                    Err(format!("{} does not exist.", input_path))
                }
                Err(_) => {
                    Err(format!("{} is incorrect.", input_path))
                }
            }
        } else {
            Ok(Mode::Extract(String::from(""), Some(String::from(""))))
        }?;

        Ok(Config {
            uncompress_path,
            unzip_path,
            gunzip_path,
            bunzip2_path,
            lbzip2_path,
            xz_path,
            pxz_path,
            p7z_path,
            tar_path,
            rar_path,
            unrar_path,
            quiet,
            mode,
        })
    }
}

// TODO -----Config END-----

// TODO -----Test START-----

#[cfg(test)]
mod test {
    // use super::*;
}

// TODO -----Test END-----