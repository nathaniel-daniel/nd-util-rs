# nd-util-rs
A dumping ground of utilities I have needed for past projects. 
Utitlities are all feature-gated and default to disabled.
See Features section to see the available utilities.

## Features
`download-to-file`: A function to asynchronously preallocate and download to a `tokio` file via a `reqwest` client.
`drop-remove-file`: A Guard that wraps a `Path`, which tries to asynchronously delete the file it wraps when it drops. The user can specify for the path to persist as well.