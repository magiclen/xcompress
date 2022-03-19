XCompress
====================

[![CI](https://github.com/magiclen/xcompress/actions/workflows/ci.yml/badge.svg)](https://github.com/magiclen/xcompress/actions/workflows/ci.yml)

XCompress is a free file archiver utility on Linux, providing multi-format archiving to and extracting from ZIP, Z, GZIP, BZIP2, LZ, XZ, LZMA, 7ZIP, TAR, RAR and ZSTD.

## Help

```
EXAMPLES:
xcompress a foo.wav                      # Archive foo.wav to foo.rar
xcompress a foo.wav /root/bar.txt        # Archive foo.wav and /root/bar.txt to foo.rar
xcompress a -o /tmp/out.7z foo.wav       # Archive foo.wav to /tmp/out.7z
xcompress a -b foo/bar                   # Archive foo/bar folder to bar.rar as small as possible
xcompress a -p password foo.wav          # Archive foo.wav to foo.rar with a password
xcompress x foo.rar                      # Extract foo.rar into current working directory
xcompress x foo.tar.gz /tmp/out_folder   # Extract foo.tar.gz into /tmp/out_folder
xcompress x -p password foo.rar          # Extract foo.rar with a password into current working directory

USAGE:
    xcompress [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -p, --password <PASSWORD>              Set password for your archive file. (Only supports 7Z, ZIP and RAR.) Set an empty string to read a password from stdin.
        --7z-path <7Z_PATH>                Specify the path of your 7z executable binary file. [default: 7z]
        --bunzip2-path <BUNZIP2_PATH>      Specify the path of your bunzip2 executable binary file. [default: bunzip2]
        --bzip2-path <BZIP2_PATH>          Specify the path of your bzip2 executable binary file. [default: bzip2]
        --compress-path <COMPRESS_PATH>    Specify the path of your compress executable binary file. [default: compress]
        --gunzip-path <GUNZIP_PATH>        Specify the path of your gunzip executable binary file. [default: gunzip]
        --gzip-path <GZIP_PATH>            Specify the path of your gzip executable binary file. [default: gzip]
    -h, --help                             Print help information
        --lbzip2-path <LBZIP2_PATH>        Specify the path of your lbzip2 executable binary file. [default: lbzip2]
        --lunzip-path <LUNZIP_PATH>        Specify the path of your lunzip executable binary file. [default: lunzip]
        --lzip-path <LZIP_PATH>            Specify the path of your lzip executable binary file. [default: lzip]
        --lzma-path <LZMA_PATH>            Specify the path of your lzma executable binary file. [default: lzma]
        --pbzip2-path <PBZIP2_PATH>        Specify the path of your pbzip2 executable binary file. [default: pbzip2]
        --pigz-path <PIGZ_PATH>            Specify the path of your pigz executable binary file. [default: pigz]
        --plzip-path <PLZIP_PATH>          Specify the path of your plzip executable binary file. [default: plzip]
        --pxz-path <PXZ_PATH>              Specify the path of your pxz executable binary file. [default: pxz]
        --pzstd-path <PZSTD_PATH>          Specify the path of your pzstd executable binary file. [default: pzstd]
    -q, --quiet                            Make programs not print anything on the screen.
        --rar-path <RAR_PATH>              Specify the path of your rar executable binary file. [default: rar]
    -s, --single-thread                    Use only one thread.
        --tar-path <TAR_PATH>              Specify the path of your tar executable binary file. [default: tar]
        --unlzma-path <UNLZMA_PATH>        Specify the path of your unlzma executable binary file. [default: unlzma]
        --unrar-path <UNRAR_PATH>          Specify the path of your unrar executable binary file. [default: unrar]
        --unxz-path <UNXZ_PATH>            Specify the path of your unxz executable binary file. [default: unxz]
        --unzip-path <UNZIP_PATH>          Specify the path of your unzip executable binary file. [default: unzip]
        --unzstd-path <UNZSTD_PATH>        Specify the path of your unzstd executable binary file. [default: unzstd]
    -V, --version                          Print version information
        --xz-path <XZ_PATH>                Specify the path of your xz executable binary file. [default: xz]
        --zip-path <ZIP_PATH>              Specify the path of your zip executable binary file. [default: zip]
        --zstd-path <ZSTD_PATH>            Specify the path of your zstd executable binary file. [default: zstd]

SUBCOMMANDS:
    a       Add files to archive. Excludes base directory from names. (e.g. add /path/to/folder, you can always get the "folder" in the root of the archive file, instead of /path/to/folder.)
    help    Print this message or the help of the given subcommand(s)
    x       Extract files with full path.
```

## License

[MIT](LICENSE)