use std::{fs, path::Path, process::Command};

fn assert_success(mut command: Command) {
    let description = format!("{command:?}");
    let output = command.output().unwrap();

    assert!(
        output.status.success(),
        "{description}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_archive(input_paths: &[&Path], output_path: &Path, split: Option<&str>) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_xcompress"));
    command.args(["--quiet", "a", "--fastest-compression"]);

    if let Some(split) = split {
        command.args(["--split", split]);
    }

    command.args(input_paths);
    command.arg("--output-path");
    command.arg(output_path);

    assert_success(command);
}

fn extract_archive(input_path: &Path, output_path: &Path) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_xcompress"));
    command.args(["--quiet", "x"]);
    command.arg(input_path);
    command.arg(output_path);

    assert_success(command);
}

#[test]
#[ignore = "requires Ubuntu 24.04 external tools"]
fn single_stream_round_trips() {
    let directory = tempfile::tempdir().unwrap();
    let input_path = directory.path().join("source");
    // `compress` exits with 2 when its output would be larger, so the payload has to be worth compressing
    let payload = b"single stream payload".repeat(512);
    fs::write(input_path.as_path(), payload.as_slice()).unwrap();

    for extension in ["Z", "gz", "bz2", "lz", "xz", "lzma", "zst"] {
        let archive_path = directory.path().join(format!("source.{extension}"));
        let output_path = directory.path().join(format!("output-{extension}"));
        fs::create_dir(output_path.as_path()).unwrap();

        create_archive(&[input_path.as_path()], archive_path.as_path(), None);
        extract_archive(archive_path.as_path(), output_path.as_path());

        assert_eq!(payload.as_slice(), fs::read(output_path.join("source")).unwrap().as_slice());
    }
}

#[test]
#[ignore = "requires Ubuntu 24.04 external tools"]
fn tar_stream_round_trips() {
    let directory = tempfile::tempdir().unwrap();
    let input_path = directory.path().join("tree");
    fs::create_dir(input_path.as_path()).unwrap();
    fs::write(input_path.join("file"), b"tar stream payload").unwrap();

    for extension in ["tar.Z", "tar.gz", "tar.bz2", "tar.lz", "tar.xz", "tar.lzma", "tar.zst"] {
        let archive_path = directory.path().join(format!("tree.{extension}"));
        let output_path = directory.path().join(format!("output-{}", extension.replace('.', "-")));
        fs::create_dir(output_path.as_path()).unwrap();

        create_archive(&[input_path.as_path()], archive_path.as_path(), None);
        extract_archive(archive_path.as_path(), output_path.as_path());

        assert_eq!(
            b"tar stream payload",
            fs::read(output_path.join("tree/file")).unwrap().as_slice()
        );
    }
}

#[test]
#[ignore = "requires Ubuntu 24.04 external tools"]
fn archive_inputs_use_base_names() {
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("first/file-a");
    let second_path = directory.path().join("second/file-b");
    fs::create_dir_all(first_path.parent().unwrap()).unwrap();
    fs::create_dir_all(second_path.parent().unwrap()).unwrap();
    fs::write(first_path.as_path(), b"first").unwrap();
    fs::write(second_path.as_path(), b"second").unwrap();

    for extension in ["7z", "zip", "rar"] {
        let archive_path = directory.path().join(format!("archive.{extension}"));
        let output_path = directory.path().join(format!("output-{extension}"));
        fs::create_dir(output_path.as_path()).unwrap();

        create_archive(
            &[first_path.as_path(), second_path.as_path()],
            archive_path.as_path(),
            None,
        );
        extract_archive(archive_path.as_path(), output_path.as_path());

        assert_eq!(b"first", fs::read(output_path.join("file-a")).unwrap().as_slice());
        assert_eq!(b"second", fs::read(output_path.join("file-b")).unwrap().as_slice());
        assert!(!output_path.join("first").exists());
        assert!(!output_path.join("second").exists());
    }
}

#[test]
#[ignore = "requires Ubuntu 24.04 external tools"]
fn split_volume_sets() {
    let directory = tempfile::tempdir().unwrap();
    let input_path = directory.path().join("source");
    let mut data = Vec::with_capacity(140 * 1024);
    let mut state = 0x1234_5678u32;

    for _ in 0..140 * 1024 {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        data.push((state >> 24) as u8);
    }

    fs::write(input_path.as_path(), data.as_slice()).unwrap();

    let seven_zip_path = directory.path().join("archive.7z");
    create_archive(&[input_path.as_path()], seven_zip_path.as_path(), Some("64KiB"));
    assert!(directory.path().join("archive.7z.001").is_file());
    assert!(directory.path().join("archive.7z.002").is_file());

    let zip_path = directory.path().join("archive.zip");
    create_archive(&[input_path.as_path()], zip_path.as_path(), Some("64KiB"));
    assert!(directory.path().join("archive.z01").is_file());
    assert!(zip_path.is_file());

    let rar_path = directory.path().join("archive.rar");
    create_archive(&[input_path.as_path()], rar_path.as_path(), Some("64KiB"));
    assert!(directory.path().join("archive.part1.rar").is_file());
    assert!(directory.path().join("archive.part2.rar").is_file());

    let tar_seven_zip_path = directory.path().join("archive.tar.7z");
    create_archive(&[input_path.as_path()], tar_seven_zip_path.as_path(), Some("64KiB"));
    assert!(directory.path().join("archive.tar.7z.001").is_file());
    assert!(directory.path().join("archive.tar.7z.002").is_file());

    // a split ZIP can only be read back by 7z, so `unzip` alone would not be enough
    let output_path = directory.path().join("output-zip");
    fs::create_dir(output_path.as_path()).unwrap();
    extract_archive(zip_path.as_path(), output_path.as_path());

    assert_eq!(data.as_slice(), fs::read(output_path.join("source")).unwrap().as_slice());

    for archive_path in [
        directory.path().join("archive.7z.001"),
        zip_path,
        directory.path().join("archive.tar.7z.001"),
    ] {
        let mut command = Command::new("7z");
        command.arg("t");
        command.arg(archive_path);
        assert_success(command);
    }

    let mut command = Command::new("rar");
    command.arg("t");
    command.arg("-idq");
    command.arg(directory.path().join("archive.part1.rar"));
    assert_success(command);
}
