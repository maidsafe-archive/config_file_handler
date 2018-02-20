# Config File Handler - Change Log

## [0.10.0]
- Use rust 1.24.0 stable / 2018-02-05 nightly
- rustfmt 0.9.0 and clippy-0.0.186

## [0.9.0]
- Use rust 1.22.1 stable / 2017-11-23 nightly
- rustfmt 0.9.0 and clippy-0.0.174

## [0.8.3]
- `exe_file_stem` falls back to a default value in case of error

## [0.8.2]
- Feature to specify additional search paths externally.

## [0.8.1]
- change inline style link to reference style link definitions

## [0.8.0]
- Use rust 1.19 stable / 2017-07-20 nightly
- rustfmt 0.9.0 and clippy-0.0.144
- Replace -Zno-trans with cargo check
- Make appveyor script using fixed version of stable

## [0.7.0]
- Use rust 1.17 stable
- Update serde serialisations
- Update CI script to run cargo_install from QA.

## [0.6.0]
- Switch to serde insted of rustc-serialize
- rustfmt 0.8.1 and clippy-0.0.120

## [0.5.0]
- Cleaned up and improved CI scripts and README.md.
- Renamed some public error variants.

## [0.4.0]
- Modify file search paths for various paltforms. The path returned would either be the potential path where files can be read from or created, or will contain the default file already created by this crate, depending on function invoked.

## [0.3.1]
- Migrate quick-error to 1.1.0.
- various docs update

## [0.3.0]
- Implemented std::fmt::Display and std::error::Error for Error.

## [0.2.1]
- Fix: existing files are now overwritten, not appended to.
- Added file locks to protect concurrent access.

## [0.2.0]
- Added `open` function and made `cleanup` function public.

## [0.1.0]
- Removed dependency on CBOR.
- Updated dependencies.

## [0.0.1]
- Initial implementation.
