//! Declare a general purpose "document ID type" for tracking which
//! documents we want and which we have.

// Code mostly copied from Arti.

use tor_netdoc::doc::{
    authcert::AuthCertKeyIds, microdesc::MdDigest, netstatus::ConsensusFlavor, routerdesc::RdDigest,
};

/// The identity of a single document, in enough detail to load it
/// from storage.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
#[non_exhaustive]
pub enum DocId {
    /// A request for the most recent consensus document.
    LatestConsensus {
        /// The flavor of consensus to request.
        flavor: ConsensusFlavor,
        /// Rules for loading this consensus from the cache.
        cache_usage: CacheUsage,
    },
    /// A request for an authority certificate, by the SHA1 digests of
    /// its identity key and signing key.
    AuthCert(AuthCertKeyIds),
    /// A request for a single microdescriptor, by SHA256 digest.
    Microdesc(MdDigest),
    /// A request for a router descriptor, by SHA1 digest.
    Routerdesc(RdDigest),
}

/// Description of how to start out a given bootstrap attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CacheUsage {
    /// The bootstrap attempt will only use the cache.  Therefore, don't
    /// load a pending consensus from the cache, since we won't be able
    /// to find enough information to make it usable.
    CacheOnly,
    /// The bootstrap attempt is willing to download information or to
    /// use the cache.  Therefore, we want the latest cached
    /// consensus, whether it is pending or not.
    CacheOkay,
    /// The bootstrap attempt is trying to fetch a new consensus. Therefore,
    /// we don't want a consensus from the cache.
    MustDownload,
}

/// A group of DocIds that can be downloaded or loaded from the database
/// together.
///
/// TODO: Perhaps this should be the same as ClientRequest?
#[derive(Clone, Debug)]
pub(crate) enum DocQuery {
    /// A request for the lastet consensus
    LatestConsensus {
        /// A desired flavor of consenus
        flavor: ConsensusFlavor,
        /// Whether we can or must use the cache
        cache_usage: CacheUsage,
    },
    /// A request for authority certificates
    AuthCert(Vec<AuthCertKeyIds>),
    /// A request for microdescriptors
    Microdesc(Vec<MdDigest>),
    /// A request for router descriptors
    Routerdesc(Vec<RdDigest>),
}

impl DocQuery {
    /// Construct an "empty" docquery from the given DocId
    pub fn empty_from_docid(id: &DocId) -> Self {
        match *id {
            DocId::LatestConsensus {
                flavor,
                cache_usage,
            } => Self::LatestConsensus {
                flavor,
                cache_usage,
            },
            DocId::AuthCert(_) => Self::AuthCert(Vec::new()),
            DocId::Microdesc(_) => Self::Microdesc(Vec::new()),
            DocId::Routerdesc(_) => Self::Routerdesc(Vec::new()),
        }
    }

    /// Add `id` to this query, if possible.
    fn push(&mut self, id: DocId) {
        match (self, id) {
            (Self::LatestConsensus { .. }, DocId::LatestConsensus { .. }) => {}
            (Self::AuthCert(ids), DocId::AuthCert(id)) => ids.push(id),
            (Self::Microdesc(ids), DocId::Microdesc(id)) => ids.push(id),
            (Self::Routerdesc(ids), DocId::Routerdesc(id)) => ids.push(id),
            (_, _) => panic!(),
        }
    }
}

impl From<DocId> for DocQuery {
    fn from(d: DocId) -> DocQuery {
        let mut result = DocQuery::empty_from_docid(&d);
        result.push(d);
        result
    }
}
