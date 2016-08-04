// Copyright 2016 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under (1) the MaidSafe.net Commercial License,
// version 1.0 or later, or (2) The General Public License (GPL), version 3, depending on which
// licence you accepted on initial access to the Software (the "Licences").
//
// By contributing code to the SAFE Network Software, or to this project generally, you agree to be
// bound by the terms of the MaidSafe Contributor Agreement, version 1.0.  This, along with the
// Licenses can be found in the root directory of this project at LICENSE, COPYING and CONTRIBUTOR.
//
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.
//
// Please review the Licences for the specific language governing permissions and limitations
// relating to use of the SAFE Network Software.

use fs2::FileExt;
use rustc_serialize::{Decodable, Encodable};
use rustc_serialize::json::{self, Json, Decoder};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, Once, ONCE_INIT};

use error::Error;

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
    /// See [Thread- and Process-Safety](#thread--and-process-safety) for notes on thread- and
    /// process-safety.
    pub fn open<S: AsRef<OsStr> + ?Sized>(name: &S) -> Result<FileHandler<T>, Error> {
        let name = name.as_ref();
        if let Ok(mut path) = current_bin_dir()
            .or_else(|_| user_app_dir())
            .or_else(|_| system_cache_dir()) {
            path.push(name);
            if let Ok(_) = OpenOptions::new().create(false).write(true).open(&path) {
                return Ok(FileHandler {
                    path: path,
                    _ph: PhantomData,
                });
            }
        }
        Err(Error::Io(io::Error::new(io::ErrorKind::NotFound, "config dir")))
    }

    /// Get the full path to the file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl<T> FileHandler<T>
    where T: Default + Encodable
{
    /// Constructor taking the required file name (not the full path)
    /// The config file will be initialised to a default if it does not exist.
    ///
    /// This function tests whether it has write access to the file in the following locations in
    /// this order (see also [an example config file flowchart]
    /// (https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf)):
    ///
    ///   1. [`current_bin_dir()`](fn.current_bin_dir.html)
    ///   2. [`user_app_dir()`](fn.user_app_dir.html)
    ///   3. [`system_cache_dir()`](fn.system_cache_dir.html)
    ///
    /// See [Thread- and Process-Safety](#thread--and-process-safety) for notes on thread- and
    /// process-safety.
    pub fn new<S: AsRef<OsStr> + ?Sized>(name: &S) -> Result<FileHandler<T>, Error> {
        if let Ok(fh) = FileHandler::open(name) {
            return Ok(fh);
        }

        let contents = format!("{}", json::as_pretty_json(&T::default())).into_bytes();
        let name = name.as_ref();
        if let Ok(mut path) = current_bin_dir()
            .or_else(|_| user_app_dir())
            .or_else(|_| system_cache_dir()) {
            path.push(name);
            let mutex = global_mutex();
            let _guard = mutex.lock().unwrap();
            if let Ok(mut f) = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path) {
                try!(f.lock_exclusive());
                let write_result = f.write_all(&contents);
                try!(f.unlock());
                try!(write_result);
                return Ok(FileHandler {
                    path: path,
                    _ph: PhantomData,
                });
            }
        }
        Err(Error::Io(io::Error::new(io::ErrorKind::NotFound, "config dir")))
    }
}

impl<T> FileHandler<T>
    where T: Decodable
{
    /// Read the contents of the file and decode it as JSON.
    pub fn read_file(&self) -> Result<T, Error> {
        let mut file = try!(OpenOptions::new().create(false).read(true).open(&self.path));

        try!(file.lock_shared());
        let json_result = Json::from_reader(&mut file);
        try!(file.unlock());
        Ok(try!(T::decode(&mut Decoder::new(try!(json_result)))))
    }
}

impl<T> FileHandler<T>
    where T: Encodable
{
    /// Write `contents` to the file as JSON.
    pub fn write_file(&self, contents: &T) -> Result<(), Error> {
        let contents = format!("{}", json::as_pretty_json(contents)).into_bytes();
        let mutex = global_mutex();
        let _guard = mutex.lock().unwrap();
        let mut file =
            try!(OpenOptions::new().write(true).create(true).truncate(true).open(&self.path));

        try!(file.lock_exclusive());
        let write_result = file.write_all(&contents);
        try!(file.unlock());
        try!(write_result);

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
            try!(fs::remove_file(path));
        }
    }

    Ok(())
}

struct GlobalMutex(Arc<Mutex<()>>);

#[allow(unsafe_code)]
fn global_mutex() -> Arc<Mutex<()>> {
    static mut GLOBAL_MUTEX: *const GlobalMutex = 0 as *const GlobalMutex;
    static ONCE: Once = ONCE_INIT;
    unsafe {
        ONCE.call_once(|| {
            let mutex = GlobalMutex(Arc::new(Mutex::new(())));
            GLOBAL_MUTEX = Box::into_raw(Box::new(mutex));
        });

        (*GLOBAL_MUTEX).0.clone()
    }
}

fn test_file_creation(mut path: PathBuf) -> bool {
    path.set_file_name("sample_0123498765.txt");
    let mutex = global_mutex();
    let _guard = mutex.lock().unwrap();
    match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path) {
        Ok(_) => fs::remove_file(path).is_ok(),
        Err(e) => {
            println!("Error: {:?}", e);
            false
        }
    }
}

/// The full path to the directory containing the currently-running binary.  See also [an example
/// config file flowchart]
/// (https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf).
pub fn current_bin_dir() -> Result<PathBuf, Error> {
    let path = match try!(env::current_exe()).parent() {
        Some(path) => path.to_path_buf(),
        None => return Err(Error::Io(io::Error::new(io::ErrorKind::NotFound, "Current bin dir"))),
    };

    if test_file_creation(path.clone()) {
        Ok(path)
    } else {
        Err(Error::Io(io::Error::new(io::ErrorKind::Other, "Permission denied")))
    }
}

// /// The full path to an application support directory for the current user.  See also [an example
// /// config file flowchart]
// /// (https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf).
// #[cfg(windows)]
// pub fn user_app_dir() -> Result<PathBuf, Error> {
//     Ok(try!(join_exe_file_stem(Path::new(&try!(env::var("APPDATA"))))))
// }
//
// /// The full path to an application support directory for the current user.  See also [an example
// /// config file flowchart]
// /// (https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf).
// #[cfg(unix)]
// pub fn user_app_dir() -> Result<PathBuf, Error> {
//     let home_dir = try!(env::home_dir()
//         .ok_or(io::Error::new(io::ErrorKind::NotFound, "User home directory not found.")));
//     Ok(try!(join_exe_file_stem(&home_dir)).join(".config"))
// }

/// The full path to an application support directory for the current user.  See also [an example
/// config file flowchart]
/// (https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf).
#[cfg(windows)]
pub fn user_app_dir() -> Result<PathBuf, Error> {
    let home_dir = try!(join_exe_file_stem(Path::new(&try!(env::var("APPDATA")))));
    if test_file_creation(home_dir.clone()) {
        Ok(home_dir)
    } else {
        Err(Error::Io(io::Error::new(io::ErrorKind::Other, "Permission denied")))
    }
}

/// The full path to an application support directory for the current user.  See also [an example
/// config file flowchart]
/// (https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf).
#[cfg(target_os="macos")]
pub fn user_app_dir() -> Result<PathBuf, Error> {
    let mut home_dir = try!(env::home_dir()
        .ok_or(io::Error::new(io::ErrorKind::NotFound, "User home directory not found.")));
    home_dir.push("Library/Application Support");
    home_dir = try!(join_exe_file_stem(&home_dir));
    if test_file_creation(home_dir.clone()) {
        Ok(home_dir)
    } else {
        Err(Error::Io(io::Error::new(io::ErrorKind::Other, "Permission denied")))
    }
}

/// The full path to an application support directory for the current user.  See also [an example
/// config file flowchart]
/// (https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf).
#[cfg(all(unix, not(target_os="macos")))]
pub fn user_app_dir() -> Result<PathBuf, Error> {
    let mut home_dir = try!(env::home_dir()
        .ok_or(io::Error::new(io::ErrorKind::NotFound, "User home directory not found.")));
    home_dir.push(".config");
    home_dir = try!(join_exe_file_stem(&home_dir));
    if test_file_creation(home_dir.clone()) {
        Ok(home_dir)
    } else {
        Err(Error::Io(io::Error::new(io::ErrorKind::Other, "Permission denied")))
    }
}

/// The full path to a system cache directory available for all users.  See also [an example config
/// file flowchart]
/// (https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf).
#[cfg(windows)]
pub fn system_cache_dir() -> Result<PathBuf, Error> {
    Ok(try!(join_exe_file_stem(Path::new(&try!(env::var("ALLUSERSPROFILE"))))))
}

/// The full path to a system cache directory available for all users.  See also [an example config
/// file flowchart]
/// (https://github.com/maidsafe/crust/blob/master/docs/vault_config_file_flowchart.pdf).
#[cfg(unix)]
pub fn system_cache_dir() -> Result<PathBuf, Error> {
    join_exe_file_stem(Path::new("/var/cache"))
}

/// The file name of the currently-running binary without any suffix or extension.  For example, if
/// the binary is "C:\\Abc.exe" this function will return `Ok("Abc")`.
pub fn exe_file_stem() -> Result<OsString, Error> {
    let exe_path = try!(env::current_exe());
    let file_stem = exe_path.file_stem();
    Ok(try!(file_stem.ok_or(not_found_error(&exe_path))).to_os_string())
}

fn not_found_error(file_name: &Path) -> io::Error {
    let mut msg: String = From::from("No file name component: ");
    msg.push_str(&file_name.to_string_lossy());
    io::Error::new(io::ErrorKind::NotFound, msg)
}

fn join_exe_file_stem(path: &Path) -> Result<PathBuf, Error> {
    Ok(path.join(try!(exe_file_stem())))
}

#[cfg(test)]
mod test {
    use super::*;
    use error::Error;
    use std::fs;

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
    ///     let file_handler = FileHandler::new("test.json").unwrap();
    ///     // User app dir is possibly created by this call.
    ///     let _ = file_handler.write_file(&111u64);
    /// }
    /// // User app dir is now removed since '_cleaner' has gone out of scope.
    /// ```
    struct ScopedUserAppDirRemover;

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

    #[test]
    fn read_write_file_test() {
        let _cleaner = ScopedUserAppDirRemover;
        let file_handler = match FileHandler::new("test0.json") {
            Ok(result) => result,
            Err(err) => panic!("failed accessing file with error {:?}", err),
        };
        let test_value = 123456789u64;

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
        let file_handler = FileHandler::new("test1.json").expect("failed accessing file");

        let write_value0 = vec![1, 2, 3];
        file_handler.write_file(&write_value0).expect("failed writing file");

        let write_value1 = vec![4, 5, 6];
        file_handler.write_file(&write_value1).expect("failed writing file");

        let read_value = file_handler.read_file().expect("failed reading file");
        assert_eq!(read_value, write_value1);
    }

    #[test]
    fn concurrent_writes() {
        use std::iter;
        use std::sync::{Arc, Barrier};
        use std::thread;

        const NUM_THREADS: usize = 100;
        const DATA_SIZE: usize = 10000;
        const FILE_NAME: &'static str = "test2.json";

        let _cleaner = ScopedUserAppDirRemover;
        let barrier = Arc::new(Barrier::new(NUM_THREADS));

        let handles = (0..NUM_THREADS)
            .map(|i| {
                let barrier = barrier.clone();

                thread::spawn(move || {
                    let data = iter::repeat(i).take(DATA_SIZE).collect::<Vec<_>>();

                    let _ = barrier.wait();

                    let file_handler = FileHandler::new(FILE_NAME).expect("failed accessing file");
                    file_handler.write_file(&data).expect("failed writing file");
                })
            })
            .collect::<Vec<_>>();

        for handle in handles {
            handle.join().unwrap();
        }

        let file_handler = FileHandler::new(FILE_NAME).expect("failed accessing file");
        let mut data: Vec<usize> = file_handler.read_file().expect("failed reading file");

        // Test that all elements in the vector are the same, to verify no
        // interleaving took place.
        data.sort();
        data.dedup();
        assert_eq!(data.len(), 1);
    }

    // To be run as `cargo test -- --ignored --nocapture` to find out path in current OS
    #[test]
    #[ignore]
    fn print_path() {
        println!("1st Preference: File pathe in current binary directory: {:?}",
                 current_bin_dir().unwrap());
        println!("2nd Preference: File path in user app directory: {:?}",
                 user_app_dir().unwrap());
    }
}
