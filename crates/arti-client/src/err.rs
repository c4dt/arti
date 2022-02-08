//! Declare tor client specific errors.

use std::fmt::{self, Display};
use std::sync::Arc;

use futures::task::SpawnError;

use thiserror::Error;
use tor_circmgr::TargetPorts;
use tor_error::{ErrorKind, HasKind};

/// Wrapper for definitions which need to vary according to `error_details`
macro_rules! define_according_to_cfg_error_details { { $vis:vis } => {
// We cheat with the indentation, a bit.  Happily rustfmt doesn't seem to mind.

/// Main high-level error type for the Arti Tor client
///
/// If you need to handle different types of errors differently, use the
/// [`kind`](`tor_error::HasKind::kind`) trait method to check what kind of
/// error it is.
///
/// Note that although this type implements that standard
/// [`Error`](std::error::Error) trait, the output of that trait's methods are
/// not covered by semantic versioning.  Specifically: you should not rely on
/// the specific output of `Display`, `Debug`, or `Error::source()` when run on
/// this type; it may change between patch versions without notification.
#[derive(Error, Clone, Debug)]
pub struct Error {
    /// The actual error.
    ///
    /// This field is exposed  via the `detail()` method only if the the
    /// `error_detail` feature is enabled. Using it will void your semver
    /// guarantee.
    #[source]
    detail: Box<ErrorDetail>,
}

impl From<ErrorDetail> for Error {
    fn from(detail: ErrorDetail) -> Error {
        Error {
            detail: detail.into(),
        }
    }
}

/// Represents errors that can occur while doing Tor operations.
///
/// This enumeration is the inner view of a
/// [`arti_client::Error`](crate::Error): we don't expose it unless the
/// `error_detail` feature is enabled.
///
/// The details of this enumeration are not stable: using the `error_detail`
/// feature will void your semver guarantee.
///
/// Instead of looking at the type, you try to should use the
/// [`kind`](`tor_error::HasKind::kind`) trait method to distinguish among
/// different kinds of [`Error`](crate::Error).  If that doesn't provide enough information
/// for your use case, please let us know.
#[derive(Error, Clone, Debug)]
#[non_exhaustive]
$vis enum ErrorDetail {
    /// Error setting up the circuit manager
    #[error("Error setting up the circuit manager {0}")]
    CircMgrSetup(#[source] tor_circmgr::Error), // TODO should this be its own type?

    /// Failed to obtain exit circuit
    #[error("Failed to obtain exit circuit for {exit_ports}")]
    ObtainExitCircuit {
        /// What for
        exit_ports: TargetPorts,

        /// What went wrong
        #[source]
        cause: tor_circmgr::Error,
    },

    /// Error while getting a circuit
    #[error("Directory state error {0}")]
    DirMgr(#[from] tor_dirmgr::Error),

    /// A protocol error while launching a stream
    #[error("Protocol error while launching a stream: {0}")]
    Proto(#[from] tor_proto::Error),

    /// An error while interfacing with the persistent data layer.
    #[error("Error from state manager: {0}")]
    Persist(#[from] tor_persist::Error),

    /// We asked an exit to do something, and waited too long for an answer..
    #[error("exit timed out")]
    ExitTimeout,

    /// Onion services not supported.
    #[error("Rejecting .onion address as unsupported.")]
    OnionAddressNotSupported,

    /// Unusable target address.
    #[error("Could not parse target address: {0}")]
    Address(#[from] crate::address::TorAddrError),

    /// Hostname not valid.
    #[error("Rejecting hostname as invalid.")]
    InvalidHostname,

    /// Address was local, and that's not allowed.
    #[error("Cannot connect to a local-only address without enabling allow_local_addrs")]
    LocalAddress,

    /// Building configuration for the client failed.
    #[error("Configuration failed: {0}")]
    Configuration(#[from] tor_config::ConfigBuildError),

    /// Unable to change configuration.
    #[error("Reconfiguration failed: {0}")]
    Reconfigure(#[from] tor_config::ReconfigureError),

    /// Unable to spawn task
    #[error("unable to spawn {spawning}")]
    Spawn {
        /// What we were trying to spawn.
        spawning: &'static str,
        /// What happened when we tried to spawn it.
        #[source]
        cause: Arc<SpawnError>
    }
}

// End of the use of $vis to refer to visibility according to `error_detail`
} }

#[cfg(feature = "error_detail")]
impl Error {
    /// Return the underlying error detail object for this error.
    ///
    /// In general, it's not a good idea to use this function.  Our
    /// `arti_client::ErrorDetail` objects are unstable, and matching on them is
    /// probably not the best way to achieve whatever you're trying to do.
    /// Instead, we recommend using  the [`kind`](`tor_error::HasKind::kind`)
    /// trait method if your program needs to distinguish among different types
    /// of errors.
    ///
    /// (If the above function don't meet your needs, please let us know!)
    ///
    /// This function is only available when `arti-client` is built with the
    /// `error_detail` feature.  Using this function will void your semver
    /// guarantees.
    pub fn detail(&self) -> &ErrorDetail {
        &self.detail
    }
}

impl ErrorDetail {
    /// Construct a new `Error` from a `SpawnError`.
    pub(crate) fn from_spawn(spawning: &'static str, err: SpawnError) -> ErrorDetail {
        ErrorDetail::Spawn {
            spawning,
            cause: Arc::new(err),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "tor: {}: {}", self.detail.kind(), &self.detail)
    }
}

impl tor_error::HasKind for Error {
    fn kind(&self) -> ErrorKind {
        self.detail.kind()
    }
}

impl tor_error::HasKind for ErrorDetail {
    fn kind(&self) -> ErrorKind {
        use ErrorDetail as E;
        use ErrorKind as EK;
        match self {
            E::ObtainExitCircuit { cause, .. } => cause.kind(),
            E::ExitTimeout => EK::ExitTimeout,
            _ => EK::TODO,
        }
    }
}

#[cfg(feature = "error_detail")]
define_according_to_cfg_error_details! { pub }

#[cfg(not(feature = "error_detail"))]
define_according_to_cfg_error_details! { pub(crate) }

#[cfg(test)]
mod test {
    use super::*;

    /// This code makes sure that our errors implement all the traits we want.
    #[test]
    fn traits_ok() {
        // I had intended to use `assert_impl`, but that crate can't check whether
        // a type is 'static.
        fn assert<
            T: Send + Sync + Clone + std::fmt::Debug + Display + std::error::Error + 'static,
        >() {
        }
        fn check() {
            assert::<Error>();
            assert::<ErrorDetail>();
        }
        check(); // doesn't do anything, but avoids "unused function" warnings.
    }
}
