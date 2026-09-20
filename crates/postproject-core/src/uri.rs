//! URI parsing and canonical serialization shared by domain values.

use url::Url;

use crate::{Error, ErrorKind, Result};

pub(crate) fn normalize_uri(value: impl Into<String>, label: &str) -> Result<String> {
    let value = value.into();
    let url = Url::parse(&value).map_err(|error| {
        Error::new(
            ErrorKind::InvalidArgument,
            format!("invalid {label} URI: {error}"),
        )
    })?;
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_uri_syntax() {
        assert_eq!(
            normalize_uri("FILE:///media/a%20b.mov", "locator").expect("valid URI"),
            "file:///media/a%20b.mov"
        );
    }

    #[test]
    fn rejects_native_paths_and_relative_references() {
        assert!(normalize_uri("relative/file.mov", "locator").is_err());
        assert!(normalize_uri("/native/path.mov", "locator").is_err());
    }
}
