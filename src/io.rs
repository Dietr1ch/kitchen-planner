use std::path::Path;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DataError {
	#[error("IO Error {0}")]
	IO(#[from] std::io::Error),
	#[error("RON Parsing Error {0}")]
	RonError(#[from] ron::error::SpannedError),
	#[error("JSON Parsing Error {0}")]
	JsonError(#[from] serde_json::Error),
	#[error("unknown data store error")]
	Unknown,
}

pub fn resolve_file_path(path: &Path) -> &Path {
	if path == Path::new("-") {
		Path::new("/dev/stdin")
	} else {
		path
	}
}

pub fn read_ron_file<T: serde::de::DeserializeOwned>(
	path: impl AsRef<Path>,
) -> Result<T, DataError> {
	let path = path.as_ref();
	let content = std::fs::read_to_string(resolve_file_path(path))?;
	let parsed: T = ron::from_str(&content)?;

	Ok(parsed)
}

pub fn read_ron_files<T, P>(paths: &[P]) -> Result<Vec<T>, DataError>
where
	T: serde::de::DeserializeOwned,
	P: AsRef<Path>,
{
	paths.iter().map(|p| read_ron_file::<T>(p)).collect()
}

pub fn read_json_file<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, DataError> {
	let content = std::fs::read_to_string(resolve_file_path(path))?;
	let parsed: T = serde_json::from_str(&content)?;

	Ok(parsed)
}
