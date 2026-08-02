use std::{
    collections::HashSet,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::{Context, anyhow};
use path_absolutize::Absolutize;

pub fn file_name(path: &Path) -> anyhow::Result<&OsStr> {
    path.file_name().ok_or_else(|| anyhow!("{path:?} has no file name."))
}

pub fn resolve_output_path(path: &Path) -> anyhow::Result<PathBuf> {
    match path.canonicalize() {
        Ok(path) => Ok(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if fs::symlink_metadata(path).is_ok() {
                return Err(error).with_context(|| anyhow!("{path:?}"));
            }

            let path = path.absolutize()?;
            let parent = path.parent().ok_or_else(|| anyhow!("{path:?} has no parent."))?;
            let parent = parent.canonicalize().with_context(|| anyhow!("{parent:?}"))?;

            Ok(parent.join(file_name(path.as_ref())?))
        },
        Err(error) => Err(error).with_context(|| anyhow!("{path:?}")),
    }
}

pub fn validate_compression_paths(
    output_path: &Path,
    input_paths: &[PathBuf],
) -> anyhow::Result<()> {
    for input_path in input_paths {
        if output_path == input_path {
            return Err(anyhow!("The output path {output_path:?} is also an input path."));
        }

        if output_path.starts_with(input_path) {
            return Err(anyhow!(
                "The output path {output_path:?} is inside the input path {input_path:?}."
            ));
        }
    }

    Ok(())
}

pub fn validate_unique_file_names(input_paths: &[PathBuf]) -> anyhow::Result<()> {
    let mut file_names = HashSet::with_capacity(input_paths.len());

    for input_path in input_paths {
        let file_name = file_name(input_path)?;

        if !file_names.insert(file_name) {
            return Err(anyhow!("More than one input path uses the file name {file_name:?}."));
        }
    }

    Ok(())
}

pub fn decompressed_file_path(output_path: &Path, input_path: &Path) -> anyhow::Result<PathBuf> {
    let file_stem =
        input_path.file_stem().ok_or_else(|| anyhow!("{input_path:?} has no file stem."))?;
    let file_path = output_path.join(file_stem);

    if file_path == input_path {
        return Err(anyhow!("The output path {file_path:?} is also the input path."));
    }

    Ok(file_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compression_path_collisions() {
        let inputs = vec![PathBuf::from("/input/file"), PathBuf::from("/input/directory")];

        assert!(validate_compression_paths(Path::new("/output/archive.tar"), &inputs).is_ok());
        assert!(validate_compression_paths(Path::new("/input/file"), &inputs).is_err());
        assert!(
            validate_compression_paths(Path::new("/input/directory/archive.tar"), &inputs).is_err()
        );
    }

    #[test]
    fn duplicate_file_names() {
        let inputs = vec![PathBuf::from("/first/file"), PathBuf::from("/second/file")];

        assert!(validate_unique_file_names(&inputs).is_err());
    }

    #[test]
    fn decompression_path_collision() {
        assert_eq!(
            Path::new("/output/archive"),
            decompressed_file_path(Path::new("/output"), Path::new("/input/archive.gz")).unwrap()
        );
        assert!(decompressed_file_path(Path::new("/input"), Path::new("/input/.gz")).is_err());
    }
}
