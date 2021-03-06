//! Manage a set of channels on the Tor network.
//!
//! In Tor, a Channel is a connection to a Tor relay.  It can be
//! direct via TLS, or indirect via TLS over a pluggable transport.
//! (For now, only direct channels are supported.)
//!
//! Since a channel can be used for more than one circuit, it's
//! important to reuse channels when possible.  This crate implements
//! a [ChanMgr] type that can be used to do that.

#![deny(missing_docs)]
#![deny(clippy::await_holding_lock)]
#![warn(clippy::clone_on_ref_ptr)]
#![warn(clippy::cognitive_complexity)]
#![deny(clippy::debug_assert_with_mut_call)]
#![deny(clippy::exhaustive_enums)]
#![deny(clippy::exhaustive_structs)]
#![deny(clippy::expl_impl_clone_on_copy)]
#![deny(clippy::missing_docs_in_private_items)]
#![warn(clippy::rc_buffer)]

mod builder;
mod err;
mod mgr;

use tor_linkspec::ChanTarget;
use tor_proto::channel::Channel;

pub use err::Error;
use std::sync::Arc;

use tor_rtcompat::Runtime;

/// A Result as returned by this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// A Type that remembers a set of live channels, and launches new
/// ones on request.
///
/// Use the [ChanMgr::get_or_launch] function to craete a new channel, or
/// get one if it exists.
pub struct ChanMgr<R: Runtime> {
    /// Internal channel manager object that does the actual work.
    mgr: mgr::AbstractChanMgr<builder::ChanBuilder<R>>,
}

impl<R: Runtime> ChanMgr<R> {
    /// Construct a new channel manager.
    pub fn new(runtime: R) -> Self {
        let builder = builder::ChanBuilder::new(runtime);
        let mgr = mgr::AbstractChanMgr::new(builder);
        ChanMgr { mgr }
    }

    /// Try to get a suitable channel to the provided `target`,
    /// launching one if one does not exist.
    ///
    /// If there is already a channel launch attempt in progress, this
    /// function will wait until that launch is complete, and succeed
    /// or fail depending on its outcome.
    pub async fn get_or_launch<T: ChanTarget + ?Sized>(&self, target: &T) -> Result<Arc<Channel>> {
        let ed_identity = target.ed_identity();
        let targetinfo = builder::TargetInfo::from_chan_target(target);

        let chan = self.mgr.get_or_launch(*ed_identity, targetinfo).await?;
        // Double-check the match to make sure that the RSA identity is
        // what we wanted too.
        chan.check_match(target)?;
        Ok(chan)
    }
}
