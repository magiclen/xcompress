use std::path::PathBuf;

use clap::{Args, CommandFactory, FromArgMatches, Parser, Subcommand};
use concat_with::concat_line;
use terminal_size::terminal_size;

const APP_NAME: &str = "XCompress";
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");

const AFTER_HELP: &str = "Enjoy it! https://magiclen.org";

const APP_ABOUT: &str = concat!(
    "XCompress is a free file archiver utility on Linux, providing multi-format archiving to and \
     extracting from ZIP, Z, GZIP, BZIP2, LZ, XZ, LZMA, 7ZIP, TAR and RAR.\n\nEXAMPLES:\n",
    concat_line!(prefix "xcompress ",
        "a foo.wav                      # Archive foo.wav to foo.rar",
        "a foo.wav /root/bar.txt        # Archive foo.wav and /root/bar.txt to foo.rar",
        "a -o /tmp/out.7z foo.wav       # Archive foo.wav to /tmp/out.7z",
        "a -b foo/bar                   # Archive foo/bar folder to bar.rar as small as possible",
        "a -f foo/bar -r 5              # Archive foo/bar folder to bar.rar as fast as possible and add 5% recovery record",
        "a -p password foo.wav          # Archive foo.wav to foo.rar with a password",
        "x foo.rar                      # Extract foo.rar into current working directory",
        "x foo.tar.gz /tmp/out_folder   # Extract foo.tar.gz into /tmp/out_folder",
        "x -p password foo.rar          # Extract foo.rar with a password into current working directory"
    )
);

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

#[derive(Debug, Parser)]
#[command(name = APP_NAME)]
#[command(term_width = terminal_size().map(|(width, _)| width.0 as usize).unwrap_or(0))]
#[command(version = CARGO_PKG_VERSION)]
#[command(author = CARGO_PKG_AUTHORS)]
#[command(after_help = AFTER_HELP)]
pub struct CLIArgs {
    #[command(subcommand)]
    pub command: CLICommands,

    #[arg(short, long)]
    #[arg(global = true)]
    #[arg(help = "Make programs not print anything on the screen")]
    pub quiet: bool,

    #[arg(short, long)]
    #[arg(global = true)]
    #[arg(help = "Use only one thread")]
    pub single_thread: bool,

    #[arg(short, long)]
    #[arg(global = true)]
    #[arg(num_args = 0..=1, default_missing_value = "")]
    #[arg(help = "Set password for your archive file. (Only supports 7Z, ZIP and RAR) Set an \
                  empty string to read a password from stdin")]
    pub password: Option<String>,

    #[command(flatten)]
    pub executable_paths: ExecutablePaths,
}

#[derive(Debug, Args)]
pub struct ExecutablePaths {
    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_COMPRESS_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your compress executable binary file")]
    pub compress_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_ZIP_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your zip executable binary file")]
    pub zip_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_UNZIP_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your unzip executable binary file")]
    pub unzip_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_GZIP_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your gzip executable binary file")]
    pub gzip_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_GUNZIP_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your gunzip executable binary file")]
    pub gnuzip_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_PIGZ_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your pigz executable binary file")]
    pub pigz_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_BZIP2_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your bzip2 executable binary file")]
    pub bzip2_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_BUNZIP2_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your bunzip2 executable binary file")]
    pub bunzip2_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_LBZIP2_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your lbzip2 executable binary file")]
    pub lbzip2_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_PBZIP2_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your pbzip2 executable binary file")]
    pub pbzip2_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_LZIP_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your lzip executable binary file")]
    pub lzip_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_LUNZIP_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your lunzip executable binary file")]
    pub lunzip_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_PLZIP_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your plzip executable binary file")]
    pub plzip_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_XZ_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your xz executable binary file")]
    pub xz_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_UNXZ_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your unxz executable binary file")]
    pub unxz_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_PXZ_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your pxz executable binary file")]
    pub pxz_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_LZMA_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your lzma executable binary file")]
    pub lzma_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_UNLZMA_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your unlzma executable binary file")]
    pub unlzma_path: String,

    #[arg(name = "7z-path")]
    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_7Z_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your 7z executable binary file")]
    pub p7z_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_TAR_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your tar executable binary file")]
    pub tar_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_RAR_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your rar executable binary file")]
    pub rar_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_UNRAR_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your unrar executable binary file")]
    pub unrar_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_ZSTD_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your zstd executable binary file")]
    pub zstd_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_UNZSTD_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your unzstd executable binary file")]
    pub unzstd_path: String,

    #[arg(long)]
    #[arg(global = true)]
    #[arg(default_value = DEFAULT_PZSTD_PATH)]
    #[arg(value_hint = clap::ValueHint::CommandName)]
    #[arg(help = "Specify the path of your pzstd executable binary file")]
    pub pzstd_path: String,
}

#[derive(Debug, Subcommand)]
pub enum CLICommands {
    #[command(about = "Extract files with full path")]
    #[command(after_help = AFTER_HELP)]
    X {
        #[arg(value_hint = clap::ValueHint::FilePath)]
        #[arg(
            help = "Assign the source of your original files. It should be at least one file path"
        )]
        input_path:  PathBuf,
        #[arg(value_hint = clap::ValueHint::DirPath)]
        #[arg(conflicts_with = "output")]
        #[arg(help = "Assign a destination of your extracted files. It should be a directory path")]
        output_path: Option<PathBuf>,
        #[arg(short, long)]
        #[arg(value_hint = clap::ValueHint::DirPath)]
        #[arg(conflicts_with = "output_path")]
        #[arg(help = "Assign a destination of your extracted files. It should be a directory path")]
        output:      Option<PathBuf>,
    },
    #[command(about = "Add files to archive. Excludes base directory from names (e.g. add \
                       /path/to/folder, you can always get the \"folder\" in the root of the \
                       archive file, instead of /path/to/folder)")]
    #[command(after_help = AFTER_HELP)]
    A {
        #[arg(required = true)]
        #[arg(value_hint = clap::ValueHint::AnyPath)]
        #[arg(
            help = "Assign the source of your original files. It should be at least one file path"
        )]
        input_paths:         Vec<PathBuf>,
        #[arg(short, long)]
        #[arg(value_hint = clap::ValueHint::FilePath)]
        #[arg(help = "Assign a destination of your extracted files. It should be a file path. \
                      Specify the file extension name in order to determine which archive \
                      format you want to use. [default archive format: RAR]")]
        output_path:         Option<PathBuf>,
        #[arg(short, long, visible_alias = "best")]
        #[arg(conflicts_with = "fastest_compression")]
        #[arg(help = "If you are OK about the compression and depression time and want to save \
                      more disk space and network traffic, it will make the archive file as \
                      small as possible")]
        best_compression:    bool,
        #[arg(short, long, alias = "fast-compression", visible_alias = "fast")]
        #[arg(conflicts_with = "best_compression")]
        #[arg(help = "If you are OK about using more disk space and network traffic, and want \
                      to get the fastest compression and depression time, it will make the \
                      compression as minimal as possible (even not use compression at all)")]
        fastest_compression: bool,
        #[arg(short = 'd', long)]
        #[arg(help = "Split the archive file into volumes with a specified size. The unit of \
                      value is byte. You can also use KB, MB, KiB, MiB, etc, as a suffix. The \
                      minimum volume is 64 KiB (Only supports 7Z, ZIP and RAR)")]
        split:               Option<String>,
        #[arg(short, long = "recovery-record", visible_alias = "rr")]
        #[arg(value_parser = clap::value_parser!(u8).range(1..=100))]
        #[arg(help = "Add data recovery record (Only supports RAR)")]
        recovery_record:     Option<u8>,
    },
}

pub fn get_args() -> CLIArgs {
    let args = CLIArgs::command();

    let about = format!("{APP_NAME} {CARGO_PKG_VERSION}\n{CARGO_PKG_AUTHORS}\n{APP_ABOUT}");

    let args = args.about(about);

    let matches = args.get_matches();

    match CLIArgs::from_arg_matches(&matches) {
        Ok(args) => args,
        Err(err) => {
            err.exit();
        },
    }
}
