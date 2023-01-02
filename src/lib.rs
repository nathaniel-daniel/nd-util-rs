#[cfg(feature = "download-to-file")]
mod download_to_file;
#[cfg(feature = "download-to-file")]
pub use self::download_to_file::download_to_file;

#[cfg(feature = "drop-remove-path")]
mod drop_remove_path;
#[cfg(feature = "drop-remove-path")]
pub use self::drop_remove_path::DropRemovePath;

#[cfg(feature = "download-to-path")]
mod download_to_path;
#[cfg(feature = "download-to-path")]
pub use self::download_to_path::download_to_path;

use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;

/// Push an extension to a [`PathBuf`].
pub fn push_extension<S>(path: &mut PathBuf, extension: S)
where
    S: AsRef<OsStr>,
{
    let extension = extension.as_ref();

    // Bail out early if there is no extension, simply setting one.
    if path.extension().is_none() {
        path.set_extension(extension);
        return;
    }

    // Take the path memory, make it a string, push the extension, and restore the argument path.
    //
    // Ideally, I woudln't take ownership of the original string,
    // but there is no API to push arbitrary bytes to a [`PathBuf`].
    // Similarly, there is no api to access the underlying [`OsString`] of a [`PathBuf`].
    let mut path_string = OsString::from(std::mem::take(path));
    path_string.reserve(extension.len() + 1);
    path_string.push(".");
    path_string.push(extension);
    std::mem::swap(path, &mut path_string.into());
}

/// Push an extension to a [`Path`], returning a new [`PathBuf`].
pub fn with_push_extension<P, S>(path: P, extension: S) -> PathBuf
where
    P: AsRef<Path>,
    S: AsRef<OsStr>,
{
    let path = path.as_ref();
    let extension = extension.as_ref();

    // Bail out early if there is no extension, simply setting one.
    if path.extension().is_none() {
        return path.with_extension(extension);
    }

    // Change the path into an OsString so we can push arbitrary bytes to it,
    // then change it into a PathBuf so we can return it.
    let mut path_string = OsString::from(path);
    path_string.reserve(extension.len() + 1);
    path_string.push(".");
    path_string.push(extension);
    PathBuf::from(path_string)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn push_path_extension_works() {
        let base_path = PathBuf::from("file.txt");
        let extension = "part";
        let with_push_extension_path = with_push_extension(&base_path, extension);
        let mut push_extension_path = base_path;
        push_extension(&mut push_extension_path, extension);

        let expected = Path::new("file.txt.part");
        assert!(with_push_extension_path == expected);
        assert!(push_extension_path == expected);

        let base_path = PathBuf::from("file");
        let extension = "part";
        let with_push_extension_path = with_push_extension(&base_path, extension);
        let mut push_extension_path = base_path;
        push_extension(&mut push_extension_path, extension);

        let expected = Path::new("file.part");
        assert!(with_push_extension_path == expected);
        assert!(push_extension_path == expected);
    }
}
