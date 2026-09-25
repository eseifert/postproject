//! Repository-level policy checks for published documentation.

use std::fs;
use std::path::{Path, PathBuf};

fn collect_markdown(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read documentation directory") {
        let path = entry.expect("read documentation entry").path();
        if path.is_dir() {
            collect_markdown(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "md") {
            files.push(path);
        }
    }
}

#[test]
fn published_documentation_uses_versioned_vocabulary() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let mut files = vec![
        repository.join("README.md"),
        repository.join("CHANGELOG.md"),
        repository.join("STEWARDSHIP.md"),
    ];
    collect_markdown(&repository.join("docs"), &mut files);

    let planning_term = ["iter", "ation"].concat();
    for path in files {
        let relative = path.strip_prefix(&repository).expect("repository path");
        assert!(
            !relative
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains(&planning_term),
            "published documentation path uses a planning label: {}",
            relative.display()
        );

        let contents = fs::read_to_string(&path).expect("read published documentation");
        assert!(
            !contents.to_ascii_lowercase().contains(&planning_term),
            "published documentation uses a planning label: {}",
            relative.display()
        );
    }
}
