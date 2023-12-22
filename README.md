# nd-util-rs
A dumping ground of utilities I have needed for past projects. 
Utitlities are all feature-gated and default to disabled.
See the Features section to see the available utilities.

## Features
| Feature            | Description                                                                                                                                                                      |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `download-to-file` | A function to asynchronously preallocate and download to a `tokio` file via a `reqwest` client.                                                                                  |
| `drop-remove-path` | A Guard that wraps a `Path`, which tries to asynchronously delete the file it wraps when it drops. The user can specify for the path to persist as well.                         |
| `download-to-path` | A function to asynchronously download a file to a path using `download-to-file`. It uses temp files and locking to ensure that the file at the given path is valid and complete. |
| `arc-anyhow-error` | A wrapper for an `anyhow::Error` that is clonable and can itself be nested into an `anyhow::Error`.                                                                              |

## License
Licensed under either of
 * Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)
at your option.

## Contributing
Unless you explicitly state otherwise, 
any contribution intentionally submitted for inclusion in the work by you, 
as defined in the Apache-2.0 license, 
shall be dual licensed as above, 
without any additional terms or conditions.