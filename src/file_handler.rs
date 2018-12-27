// Copyright 2018 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use error::Error;
use fs2::FileExt;
use global_mutex;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{from_reader, to_string_pretty};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

lazy_static! {
    static ref ADDITIONAL_SEARCH_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);
}

/// Set an additional search path. This, if set, will be tried before the other default ones.
pub fn set_additional_search_path<P: AsRef<OsStr> + ?Sized>(path: &P) {
    *unwrap!(ADDITIONAL_SEARCH_PATH.lock()) = Some(From::from(path));
}

/// Struct for reading and writing config files.
///
/// # Thread- and Process-Safety
///
/// It is safe to read and write the same file using `FileHandler`concurrently
/// in multiple threads and/or processes.
pub struct FileHandler<T> {
    path: PathBuf,
    _ph: PhantomData<T>,
}

impl<T> FileHandler<T> {
    /// Constructor taking the required file name (not the full path)
    /// This function will return an error if the file does not exist.
    ///
    /// This function tests whether it has write access to the file in the following locations in
    /// this order (see also [an example config file flowchart]
    /// (https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf)):
    ///
    ///   1. [`current_bin_dir()`](fn.current_bin_dir.html)
    ///   2. [`user_app_dir()`](fn.user_app_dir.html)
    ///   3. [`system_cache_dir()`](fn.system_cache_dir.html)
    ///
    /// Parameter `assert_writable` dictates if the file should be writable or not.
    ///
    /// See [Thread- and Process-Safety](#thread--and-process-safety) for notes on thread- and
    /// process-safety.
    pub fn open<S: AsRef<OsStr> + ?Sized>(
        name: &S,
        assert_writable: bool,
    ) -> Result<FileHandler<T>, Error> {
        let name = name.as_ref();

        if let Some(mut path) = unwrap!(ADDITIONAL_SEARCH_PATH.lock()).clone() {
            path.push(name);
            if OpenOptions::new()
                .read(true)
                .write(assert_writable)
                .open(&path)
                .is_ok()
            {
                return Ok(FileHandler {
                    path,
                    _ph: PhantomData,
                });
            }
        }

        if let Ok(mut path) = current_bin_dir() {
            path.push(name);
            if OpenOptions::new()
                .read(true)
                .write(assert_writable)
                .open(&path)
                .is_ok()
            {
                return Ok(FileHandler {
                    path,
                    _ph: PhantomData,
                });
            }
        }

        if let Ok(mut path) = bundle_resource_dir() {
            path.push(name);
            if OpenOptions::new()
                .read(true)
                .write(assert_writable)
                .open(&path)
                .is_ok()
            {
                return Ok(FileHandler {
                    path,
                    _ph: PhantomData,
                });
            }
        }

        if let Ok(mut path) = user_app_dir() {
            path.push(name);
            if OpenOptions::new()
                .read(true)
                .write(assert_writable)
                .open(&path)
                .is_ok()
            {
                return Ok(FileHandler {
                    path,
                    _ph: PhantomData,
                });
            }
        }

        let mut path = system_cache_dir()?;
        path.push(name);
        match OpenOptions::new()
            .read(true)
            .write(assert_writable)
            .open(&path)
        {
            Ok(_) => Ok(FileHandler {
                path,
                _ph: PhantomData,
            }),
            Err(e) => Err(From::from(e)),
        }
    }

    /// Get the full path to the file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl<T> FileHandler<T>
where
    T: Default + Serialize,
{
    /// Constructor taking the required file name (not the full path)
    /// The config file will be initialised to a default if it does not exist.
    ///
    /// This function tests whether it has read access to the file in the following locations in
    /// this order (see also [an example config file flowchart]
    /// (https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf)):
    ///
    ///   1. [`current_bin_dir()`](fn.current_bin_dir.html)
    ///   2. [`user_app_dir()`](fn.user_app_dir.html)
    ///   3. [`system_cache_dir()`](fn.system_cache_dir.html)
    ///
    /// Parameter `is_existing_file_writable` will assert that if the file pre-exists should it be
    /// also writable or not. (E.g. it is enough for `crust-config` file to merely exist as
    /// readable, but `bootstrap-cache` must be writable too if it exists, else no updation can
    /// happen).
    ///
    /// See [Thread- and Process-Safety](#thread--and-process-safety) for notes on thread- and
    /// process-safety.
    #[allow(clippy::new_ret_no_self)]
    pub fn new<S: AsRef<OsStr> + ?Sized>(
        name: &S,
        is_existing_file_writable: bool,
    ) -> Result<FileHandler<T>, Error> {
        if let Ok(fh) = Self::open(name, is_existing_file_writable) {
            return Ok(fh);
        }

        let contents = to_string_pretty(&T::default())?.into_bytes();
        let name = name.as_ref();

        let _guard = global_mutex::get_mutex()
            .lock()
            .expect("Could not lock mutex");

        if let Some(mut path) = unwrap!(ADDITIONAL_SEARCH_PATH.lock()).clone() {
            path.push(name);
            if let Ok(mut f) = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)
            {
                write_with_lock(&mut f, &contents)?;
                return Ok(FileHandler {
                    path,
                    _ph: PhantomData,
                });
            }
        }

        if let Ok(mut path) = current_bin_dir() {
            path.push(name);
            if let Ok(mut f) = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)
            {
                write_with_lock(&mut f, &contents)?;
                return Ok(FileHandler {
                    path,
                    _ph: PhantomData,
                });
            }
        }

        if let Ok(mut path) = user_app_dir() {
            let avoid = if path.is_dir() {
                false
            } else {
                fs::create_dir(&path).is_err()
            };
            if !avoid {
                path.push(name);
                if let Ok(mut f) = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&path)
                {
                    write_with_lock(&mut f, &contents)?;
                    return Ok(FileHandler {
                        path,
                        _ph: PhantomData,
                    });
                }
            }
        }

        let mut path = system_cache_dir()?;
        if !path.is_dir() {
            fs::create_dir(&path)?;
        }
        path.push(name);
        match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
        {
            Ok(mut f) => {
                write_with_lock(&mut f, &contents)?;
                Ok(FileHandler {
                    path,
                    _ph: PhantomData,
                })
            }
            Err(e) => Err(From::from(e)),
        }
    }
}

impl<T> FileHandler<T>
where
    T: DeserializeOwned,
{
    /// Read the contents of the file and decode it as JSON.
    #[allow(clippy::redundant_closure)] // because of lifetimes
    pub fn read_file(&self) -> Result<T, Error> {
        let mut file = File::open(&self.path)?;
        let contents = shared_lock(&mut file, |file| from_reader(file))?;
        Ok(contents)
    }
}

impl<T> FileHandler<T>
where
    T: Serialize,
{
    /// Write `contents` to the file as JSON.
    pub fn write_file(&self, contents: &T) -> Result<(), Error> {
        let contents = to_string_pretty(contents)?.into_bytes();

        let _guard = global_mutex::get_mutex()
            .lock()
            .expect("Could not lock mutex");

        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.path)?;
        write_with_lock(&mut file, &contents)?;
        Ok(())
    }
}

/// Remove the file from every location where it can be read.
pub fn cleanup<S: AsRef<OsStr>>(name: &S) -> io::Result<()> {
    let name = name.as_ref();
    let i1 = current_bin_dir().into_iter();
    let i2 = user_app_dir().into_iter();
    let i3 = system_cache_dir().into_iter();

    let dirs = i1.chain(i2.chain(i3));

    for mut path in dirs {
        path.push(name);
        if path.exists() {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn exclusive_lock<F, R, E>(file: &mut File, f: F) -> Result<R, Error>
where
    F: FnOnce(&mut File) -> Result<R, E>,
    Error: From<E>,
{
    file.lock_exclusive()?;
    let result = f(file);
    file.unlock()?;
    result.map_err(From::from)
}

fn shared_lock<F, R, E>(file: &mut File, f: F) -> Result<R, Error>
where
    F: FnOnce(&mut File) -> Result<R, E>,
    Error: From<E>,
{
    file.lock_shared()?;
    let result = f(file);
    file.unlock()?;
    result.map_err(From::from)
}

fn write_with_lock(file: &mut File, contents: &[u8]) -> Result<(), Error> {
    exclusive_lock(file, |file| file.write_all(contents))
}

/// The full path to the directory containing the currently-running binary. See also [an example
/// config file flowchart][1].
///
/// [1]: https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf
pub fn current_bin_dir() -> Result<PathBuf, Error> {
    match env::current_exe()?.parent() {
        Some(path) => Ok(path.to_path_buf()),
        None => Err(Error::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "Current bin dir",
        ))),
    }
}

/// The full path to the directory containing the resources to currently-running binary.
/// For OSX this is special directory. For others it's an error.
#[cfg(not(target_os = "macos"))]
pub fn bundle_resource_dir() -> Result<PathBuf, Error> {
    Err(Error::Io(io::Error::new(
        io::ErrorKind::NotFound,
        "Bundle resource directory only applicable to MacOs",
    )))
}

/// The full path to the directory containing the resources to currently-running binary.
/// For OSX this is special directory. For others it's an error.
#[cfg(target_os = "macos")]
pub fn bundle_resource_dir() -> Result<PathBuf, Error> {
    let mut bundle_dir = env::current_exe()?
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Bundle resources directory"))?
        .to_path_buf();

    let is_inside_bundle = bundle_dir
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Path is not unicode"))?
        .ends_with(".app/Contents/MacOS");

    if !is_inside_bundle {
        return Err(Error::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "Not inside an Application Bundle",
        )));
    }

    bundle_dir = bundle_dir
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Bundle resource directory"))?
        .to_path_buf();
    bundle_dir.push("Resources");

    Ok(bundle_dir)
}

/// The full path to an application support directory for the current user.  See also [an example
/// config file flowchart][1].
///
/// [1]: https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf
#[cfg(windows)]
pub fn user_app_dir() -> Result<PathBuf, Error> {
    let path = env::var("APPDATA")?;
    let app_dir = Path::new(&path);

    if app_dir.is_dir() {
        Ok(join_exe_file_stem(app_dir)?)
    } else {
        Err(Error::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "Global user app directory not found.",
        )))
    }
}

/// The full path to an application support directory for the current user.  See also [an example
/// config file flowchart][1].
///
/// [1]: https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf
#[cfg(all(unix, not(target_os = "macos")))]
pub fn user_app_dir() -> Result<PathBuf, Error> {
    let mut home_dir = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found."))?;
    home_dir.push(".config");

    if home_dir.is_dir() {
        Ok(join_exe_file_stem(&home_dir)?)
    } else {
        Err(Error::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "Global user app directory not found.",
        )))
    }
}

/// The full path to an application support directory for the current user.  See also [an example
/// config file flowchart][1].
///
/// [1]: https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf
#[cfg(target_os = "macos")]
pub fn user_app_dir() -> Result<PathBuf, Error> {
    let mut app_dir = dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found."))?;
    app_dir.push("Library/Application Support");

    if app_dir.is_dir() {
        Ok(join_exe_file_stem(&app_dir)?)
    } else {
        Err(Error::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "Global user app directory not found.",
        )))
    }
}

/// The full path to a system cache directory available for all users. See also [an example config
/// file flowchart][1].
///
/// [1]: https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf
#[cfg(windows)]
pub fn system_cache_dir() -> Result<PathBuf, Error> {
    let path = env::var("ALLUSERSPROFILE")?;
    let sys_cache_dir = Path::new(&path);

    if sys_cache_dir.is_dir() {
        Ok(join_exe_file_stem(sys_cache_dir)?)
    } else {
        Err(Error::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "Global system cache directory not found.",
        )))
    }
}

/// The full path to a system cache directory available for all users. See also [an example config
/// file flowchart][1].
///
/// [1]: https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf
#[cfg(all(unix, not(target_os = "macos")))]
pub fn system_cache_dir() -> Result<PathBuf, Error> {
    let sys_cache_dir = Path::new("/var/cache");

    if sys_cache_dir.is_dir() {
        Ok(join_exe_file_stem(sys_cache_dir)?)
    } else {
        Err(Error::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "Global system cache directory not found.",
        )))
    }
}

/// The full path to a system cache directory available for all users. See also [an example config
/// file flowchart][1].
///
/// [1]: https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf
#[cfg(target_os = "macos")]
pub fn system_cache_dir() -> Result<PathBuf, Error> {
    let sys_cache_dir = Path::new("/Library/Application Support");

    if sys_cache_dir.is_dir() {
        Ok(join_exe_file_stem(sys_cache_dir)?)
    } else {
        Err(Error::Io(io::Error::new(
            io::ErrorKind::NotFound,
            "Global system cache directory not found.",
        )))
    }
}

/// The file name of the currently-running binary without any suffix or extension.  For example, if
/// the binary is "C:\\Abc.exe" this function will return `Ok("Abc")`.
pub fn exe_file_stem() -> Result<OsString, Error> {
    if let Ok(exe_path) = env::current_exe() {
        let file_stem = exe_path.file_stem();
        Ok(file_stem
            .ok_or_else(|| not_found_error(&exe_path))?
            .to_os_string())
    } else {
        Ok(From::from("default"))
    }
}

/// RAII object which removes the [`user_app_dir()`](fn.user_app_dir.html) when an instance is
/// dropped.
///
/// Since the `user_app_dir` is frequently created by tests or examples which use Crust, this is a
/// convenience object which tries to remove the directory when it is destroyed.
///
/// # Examples
///
/// ```
/// use config_file_handler::{FileHandler, ScopedUserAppDirRemover};
///
/// {
///     let _cleaner = ScopedUserAppDirRemover;
///     let file_handler = FileHandler::new("test.json", true).unwrap();
///     // User app dir is possibly created by this call.
///     let _ = file_handler.write_file(&111u64);
/// }
/// // User app dir is now removed since '_cleaner' has gone out of scope.
/// ```
pub struct ScopedUserAppDirRemover;

impl ScopedUserAppDirRemover {
    fn remove_dir(&mut self) {
        let _ = user_app_dir()
            .and_then(|user_app_dir| fs::remove_dir_all(user_app_dir).map_err(Error::Io));
    }
}

impl Drop for ScopedUserAppDirRemover {
    fn drop(&mut self) {
        self.remove_dir();
    }
}

fn not_found_error(file_name: &Path) -> io::Error {
    let mut msg: String = From::from("No file name component: ");
    msg.push_str(&file_name.to_string_lossy());
    io::Error::new(io::ErrorKind::NotFound, msg)
}

fn join_exe_file_stem(path: &Path) -> Result<PathBuf, Error> {
    Ok(path.join(exe_file_stem()?))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_write_file_test() {
        let _cleaner = ScopedUserAppDirRemover;
        let file_handler = match FileHandler::new("test0.json", true) {
            Ok(result) => result,
            Err(err) => panic!("failed accessing file with error {:?}", err),
        };
        let test_value = 123_456_789u64;

        let _ = file_handler.write_file(&test_value);
        let read_value = match file_handler.read_file() {
            Ok(result) => result,
            Err(err) => panic!("failed reading file with error {:?}", err),
        };
        assert_eq!(test_value, read_value);
    }

    #[test]
    fn existing_file_is_overwritten() {
        let _cleaner = ScopedUserAppDirRemover;
        let file_handler = FileHandler::new("test1.json", true).expect("failed accessing file");

        let write_value0 = vec![1, 2, 3];
        file_handler
            .write_file(&write_value0)
            .expect("failed writing file");

        let write_value1 = vec![4, 5, 6];
        file_handler
            .write_file(&write_value1)
            .expect("failed writing file");

        let read_value = file_handler.read_file().expect("failed reading file");
        assert_eq!(read_value, write_value1);
    }

    #[test]
    fn concurrent_writes() {
        use std::iter;
        use std::sync::{Arc, Barrier};
        use std::thread;

        const NUM_THREADS: usize = 100;
        const DATA_SIZE: usize = 10_000;
        const FILE_NAME: &str = "test2.json";

        let _cleaner = ScopedUserAppDirRemover;
        let barrier = Arc::new(Barrier::new(NUM_THREADS));

        let handles = (0..NUM_THREADS)
            .map(|i| {
                let barrier = Arc::clone(&barrier);

                thread::spawn(move || {
                    let data = iter::repeat(i).take(DATA_SIZE).collect::<Vec<_>>();

                    let _ = barrier.wait();

                    let file_handler =
                        FileHandler::new(FILE_NAME, true).expect("failed accessing file");
                    file_handler.write_file(&data).expect("failed writing file");
                })
            })
            .collect::<Vec<_>>();

        for handle in handles {
            unwrap!(handle.join());
        }

        let file_handler = FileHandler::new(FILE_NAME, true).expect("failed accessing file");
        let mut data: Vec<usize> = file_handler.read_file().expect("failed reading file");

        // Test that all elements in the vector are the same, to verify no
        // interleaving took place.
        data.sort();
        data.dedup();
        assert_eq!(data.len(), 1);
    }

    // Run as `cargo test -- --ignored --nocapture` to print the paths
    #[test]
    #[ignore]
    #[allow(clippy::ifs_same_cond)]
    fn print_paths() {
        let os = if cfg!(target_os = "macos") {
            "macOS".to_string()
        } else if cfg!(target_os = "linux") {
            "Linux".to_string()
        } else if cfg!(unix) {
            "Unix (family)".to_string()
        } else if cfg!(windows) {
            "Windows".to_string()
        } else {
            "Unknown".to_string()
        };

        let current_bin_dir = match current_bin_dir() {
            Ok(x) => format!("{:?}", x),
            Err(x) => format!("{:?}", x),
        };

        let bundle_resource_dir = match bundle_resource_dir() {
            Ok(x) => format!("{:?}", x),
            Err(x) => format!("{:?}", x),
        };

        let user_app_dir = match user_app_dir() {
            Ok(x) => format!("{:?}", x),
            Err(x) => format!("{:?}", x),
        };

        let system_cache_dir = match system_cache_dir() {
            Ok(x) => format!("{:?}", x),
            Err(x) => format!("{:?}", x),
        };

        println!("=================================");
        println!("Current bin dir in {}: {}", os, current_bin_dir);
        println!("Current bin resource in {}: {}", os, bundle_resource_dir);
        println!("Current use-app-dir in {}: {}", os, user_app_dir);
        println!("Current system-cache-dir in {}: {}", os, system_cache_dir);
        println!("=================================");
    }
}
