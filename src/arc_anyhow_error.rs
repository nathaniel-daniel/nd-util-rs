use std::fmt::Debug;
use std::fmt::Display;
use std::sync::Arc;

/// An Arc'ed anyhow error.
///
/// It is not intended as a replacement for anyhow's error,
/// just a way to share it as well as a way to provide an error impl.
/// This means that this error can itself be placed into an anyhow::Error.
///
/// # Use Cases
/// This error is intended to be used where multiple tasks want the result of a singular, shared operation.
/// Consider a web request to a server to a static resource X.
/// It is not desired behavior to send multiple parallel requests to resource X at once.
/// Instead one would send 1 request and have all tasks that need it wait of the result.
/// However, this means that both X and the error must be clonable.
/// X can be make Clone with Arc<X>, and the error can be made clone with ArcAnyhowError.
#[derive(Clone)]
pub struct ArcAnyhowError(Arc<anyhow::Error>);

impl ArcAnyhowError {
    /// Make a new [`ArcAnyhowError`].
    pub fn new(error: anyhow::Error) -> Self {
        Self(Arc::new(error))
    }
}

impl Debug for ArcAnyhowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&*self.0, f)
    }
}

impl Display for ArcAnyhowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&*self.0, f)
    }
}

// We allow deprecated functions as this is just a wrapper,
// we want to emulate anyhow::error's choices,
// even if they use deprecated code
impl std::error::Error for ArcAnyhowError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.as_ref().source()
    }

    #[allow(deprecated)]
    fn description(&self) -> &str {
        self.0.as_ref().description()
    }

    #[allow(deprecated)]
    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.0.as_ref().cause()
    }
}
