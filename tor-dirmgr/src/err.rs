//! Declare an error type for the tor-dirmgr crate.

// Code mostly copied from Arti.

use thiserror::Error;

/// An error originated by the directory manager code
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// We received a document we didn't want at all.
    #[error("unwanted object: {0}")]
    Unwanted(&'static str),
    /// This DirMgr doesn't support downloads.
    #[error("tried to download information on a DirMgr with no download support")]
    NoDownloadSupport,
    /// A bad argument was provided to some configuration function.
    #[error("bad argument: {0}")]
    BadArgument(&'static str),
    /// We couldn't read something from disk that we should have been
    /// able to read.
    #[error("corrupt cache: {0}")]
    CacheCorruption(&'static str),
    /// rusqlite gave us an error.
    #[error("sqlite error: {0}")]
    SqliteError(#[from] rusqlite::Error),
    /// A schema version that says we can't read it.
    #[error("unrecognized data storage schema")]
    UnrecognizedSchema,
    /// An updater no longer has anything to update.
    #[error("directory updater has shut down")]
    UpdaterShutdown,
    /// We couldn't configure the network.
    #[error("bad network configuration")]
    BadNetworkConfig(&'static str),
    /// User requested an operation that required a usable
    /// bootstrapped directory, but we didn't have one.
    #[error("directory not present or not up-to-date")]
    DirectoryNotPresent,
    /// Another process has locked the store for writing.
    #[error("couldn't get write lock on directory cache")]
    CacheIsLocked,
    /// A consensus document is signed by an unrecognized authority set.
    #[error("authorities on consensus do not match what we expect.")]
    UnrecognizedAuthorities,
    /// A directory manager has been dropped; background tasks can exit too.
    #[error("dirmgr has been dropped; background tasks exiting")]
    ManagerDropped,
    /// We made a bunch of attempts, but weren't unable to advance the
    /// state of a download.
    #[error("unable to finish bootstrapping a directory")]
    CantAdvanceState,
}
