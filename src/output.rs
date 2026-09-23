use std::io;
use std::path::Path;

use crate::error::AVWriterError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OutputPolicy {
    #[default]
    FailIfExists,
    ReplaceFile,
}

pub(crate) fn prepare_output(
    path: &Path,
    policy: OutputPolicy,
    source: Option<&Path>,
) -> Result<(), AVWriterError> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(AVWriterError::InvalidArgument(format!(
                "cannot inspect output path {}: {error}",
                path.display()
            )))
        }
    };
    if policy == OutputPolicy::FailIfExists {
        return Err(AVWriterError::OutputExists(path.display().to_string()));
    }
    if !metadata.file_type().is_file() {
        return Err(AVWriterError::OutputExists(format!(
            "{} is not a regular file, so it is never replaced",
            path.display()
        )));
    }
    if source.is_some_and(|source| is_same_file(path, source)) {
        return Err(AVWriterError::InvalidArgument(format!(
            "output path {} is the source asset",
            path.display()
        )));
    }
    std::fs::remove_file(path).map_err(|error| {
        AVWriterError::InvalidState(format!("cannot replace {}: {error}", path.display()))
    })
}

fn is_same_file(left: &Path, right: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;

    match (std::fs::metadata(left), std::fs::metadata(right)) {
        (Ok(left), Ok(right)) => left.dev() == right.dev() && left.ino() == right.ino(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{prepare_output, OutputPolicy};
    use crate::error::AVWriterError;

    fn scratch(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/test-artifacts")
            .join(format!("output-policy-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    #[test]
    fn missing_output_is_accepted_under_both_policies() {
        let dir = scratch("missing");
        let path = dir.join("absent.mp4");

        assert_eq!(
            prepare_output(&path, OutputPolicy::FailIfExists, None),
            Ok(())
        );
        assert_eq!(
            prepare_output(&path, OutputPolicy::ReplaceFile, None),
            Ok(())
        );
        std::fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn existing_file_is_kept_unless_replacement_is_requested() {
        let dir = scratch("file");
        let path = dir.join("existing.mp4");
        std::fs::write(&path, b"keep me").expect("seed file");

        assert!(matches!(
            prepare_output(&path, OutputPolicy::FailIfExists, None),
            Err(AVWriterError::OutputExists(_))
        ));
        assert_eq!(std::fs::read(&path).expect("file kept"), b"keep me");

        assert_eq!(
            prepare_output(&path, OutputPolicy::ReplaceFile, None),
            Ok(())
        );
        assert!(!path.exists());
        std::fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn directories_and_symlinks_are_never_removed() {
        let dir = scratch("directory");
        let target_dir = dir.join("movies");
        std::fs::create_dir_all(&target_dir).expect("seed directory");
        std::fs::write(target_dir.join("clip.mov"), b"precious").expect("seed file");
        let link = dir.join("link.mp4");
        std::os::unix::fs::symlink(target_dir.join("clip.mov"), &link).expect("seed symlink");

        for policy in [OutputPolicy::FailIfExists, OutputPolicy::ReplaceFile] {
            assert!(matches!(
                prepare_output(&target_dir, policy, None),
                Err(AVWriterError::OutputExists(_))
            ));
            assert!(matches!(
                prepare_output(&link, policy, None),
                Err(AVWriterError::OutputExists(_))
            ));
        }
        assert_eq!(
            std::fs::read(target_dir.join("clip.mov")).expect("directory contents kept"),
            b"precious"
        );
        assert!(link.symlink_metadata().is_ok());
        std::fs::remove_dir_all(dir).expect("cleanup");
    }

    #[test]
    fn replacement_refuses_to_delete_the_source_asset() {
        let dir = scratch("source");
        let path = dir.join("source.mov");
        std::fs::write(&path, b"source").expect("seed file");

        assert!(matches!(
            prepare_output(
                &path,
                OutputPolicy::ReplaceFile,
                Some(&dir.join("./source.mov"))
            ),
            Err(AVWriterError::InvalidArgument(_))
        ));
        assert_eq!(std::fs::read(&path).expect("source kept"), b"source");
        std::fs::remove_dir_all(dir).expect("cleanup");
    }
}
