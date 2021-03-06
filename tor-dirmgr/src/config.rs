//! Types for managing directory configuration.
//!
//! Directory configuration tells us where to load and store directory
//! information, where to fetch it from, and how to validate it.

// Code mostly copied from Arti.

use crate::retry::RetryConfig;
use crate::Authority;
use crate::{Error, Result};
use tor_netdir::fallback::FallbackDir;
use tor_netdoc::doc::netstatus;

use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Configuration information about the Tor network; used as part of
/// Arti's configuration.
// TODO: move this?
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct NetworkConfig {
    /// List of locations to look in when downloading directory information,
    /// if we don't actually have a directory yet.
    ///
    /// (If we do have a chached directory, we use directory caches
    /// listed there instead.)
    #[serde(default = "fallbacks::default_fallbacks")]
    fallback_cache: Vec<FallbackDir>,

    /// List of directory authorities which we expect to sign
    /// consensus documents.
    #[serde(default = "crate::authority::default_authorities")]
    authority: Vec<Authority>,

    /// A map of network parameters that we're overriding from their
    /// setttings in the consensus.
    #[serde(default)]
    override_net_params: netstatus::NetParams<i32>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        NetworkConfig {
            fallback_cache: fallbacks::default_fallbacks(),
            authority: crate::authority::default_authorities(),
            override_net_params: Default::default(),
        }
    }
}

/// Configuration information for how exactly we download things from the
/// Tor directory caches.
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct DownloadScheduleConfig {
    /// Top-level configuration for how to retry our initial bootstrap attempt.
    #[serde(default = "default_retry_bootstrap")]
    retry_bootstrap: RetryConfig,

    /// Configuration for how to retry a consensus download.
    #[serde(default)]
    retry_consensus: RetryConfig,

    /// Configuration for how to retry an authority cert download.
    #[serde(default)]
    retry_certs: RetryConfig,

    /// Configuration for how to retry a microdescriptor download.
    #[serde(default)]
    retry_microdescs: RetryConfig,

    /// Number of microdescriptor downloads to attempt in parallel
    #[serde(default = "default_microdesc_parallelism")]
    microdesc_parallelism: u8,
}

/// Default value for retry_bootstrap in DownloadScheduleConfig.
fn default_retry_bootstrap() -> RetryConfig {
    RetryConfig::new(128, std::time::Duration::new(1, 0))
}
/// Default value for microdesc_parallelism in DownloadScheduleConfig.
fn default_microdesc_parallelism() -> u8 {
    4
}

impl Default for DownloadScheduleConfig {
    fn default() -> Self {
        DownloadScheduleConfig {
            retry_bootstrap: default_retry_bootstrap(),
            retry_consensus: Default::default(),
            retry_certs: Default::default(),
            retry_microdescs: Default::default(),
            microdesc_parallelism: default_microdesc_parallelism(),
        }
    }
}

/// Builder for a NetDirConfig.
///
/// To create a directory configuration, create one of these,
/// configure it, then call its finalize function.
///
/// # Examples
///
/// ```
/// # use tor_dirmgr::*;
/// # fn x() -> anyhow::Result<()> {
/// let mut builder = NetDirConfigBuilder::new();
/// builder.use_default_cache_path()?;
/// let config: NetDirConfig = builder.finalize()?;
/// # Ok(()) }
/// # x().unwrap()
/// ```
#[derive(Debug, Clone, Default)]
pub struct NetDirConfigBuilder {
    /// The directory from which to read legacy directory information.
    ///
    /// This has to be the directory used by a Tor instance
    /// that downloads microdesc info, and has been running fairly
    /// recently.
    legacy_cache_path: Option<PathBuf>,

    /// Path to use for current (sqlite) directory information.
    cache_path: Option<PathBuf>,

    /// Configuration information about the network.
    network: NetworkConfig,

    /// Configuration information about when to download stuff.
    timing: DownloadScheduleConfig,
}

/// Configuration type for network directory operations.
///
/// This type is immutable once constructed.
///
/// To create an object of this type, use NetDirConfigBuilder.
#[derive(Debug, Clone)]
pub struct NetDirConfig {
    /// The directory from which to read legacy directory information.
    ///
    /// This has to be the directory used by a Tor instance
    /// that downloads microdesc info, and has been running fairly
    /// recently.
    legacy_cache_path: Option<PathBuf>,

    /// Location to use for storing and reading current-format
    /// directory information.
    pub cache_path: PathBuf,

    /// Configuration information about the network.
    network: NetworkConfig,

    /// Configuration information about when we download things.
    timing: DownloadScheduleConfig,
}

impl NetDirConfigBuilder {
    /// Construct a new NetDirConfig.
    ///
    /// To use this, call at least one method to set a cache directory,
    /// then call load().
    pub fn new() -> Self {
        NetDirConfigBuilder::default()
    }

    /// Set the network information (authorities and fallbacks) from `config`.
    pub fn set_network_config(&mut self, config: NetworkConfig) {
        self.network = config;
    }

    /// Set the timining information that we use for deciding when to
    /// attempt and retry downloads.
    pub fn set_timing_config(&mut self, timing: DownloadScheduleConfig) {
        self.timing = timing;
    }

    /// Use `path` as the directory to search for legacy directory files.
    ///
    /// This path must contain `cached-certs`, `cached-microdesc-consensus`,
    /// and at least one of `cached-microdescs` and `cached-microdescs.new`.
    pub fn set_legacy_cache_path(&mut self, path: &Path) {
        self.legacy_cache_path = Some(path.to_path_buf());
    }

    /// Use `path` as the directory to use for current directory files.
    pub fn set_cache_path(&mut self, path: &Path) {
        self.cache_path = Some(path.to_path_buf());
    }

    /// Set directory authorities.
    pub fn set_authorities(&mut self, authorities: &[Authority]) {
        self.network.authority = authorities.to_vec();
    }

    /// Try to use the default cache path.
    ///
    /// This will be ~/.cache/arti on unix, and in other suitable
    /// locations on other platforms.
    pub fn use_default_cache_path(&mut self) -> Result<()> {
        let pd = directories::ProjectDirs::from("org", "torproject", "Arti")
            .ok_or(Error::DirectoryNotPresent)?;

        self.cache_path = Some(pd.cache_dir().into());

        Ok(())
    }

    /// Consume this builder and return a NetDirConfig that can be used
    /// to load directories
    pub fn finalize(mut self) -> Result<NetDirConfig> {
        if self.legacy_cache_path.is_none() {
            if let Some(home) = std::env::var_os("HOME") {
                let mut pb: PathBuf = home.into();
                pb.push(".tor");
                self.legacy_cache_path = Some(pb);
            }
        };

        let cache_path = self
            .cache_path
            .ok_or(Error::BadNetworkConfig("No cache path configured"))?;

        if self.network.authority.is_empty() {
            return Err(Error::BadNetworkConfig("No authorities configured").into());
        }
        if self.network.fallback_cache.is_empty() {
            return Err(Error::BadNetworkConfig("No fallback caches configured").into());
        }

        Ok(NetDirConfig {
            legacy_cache_path: self.legacy_cache_path,
            cache_path,
            network: self.network,
            timing: self.timing,
        })
    }
}

impl NetDirConfig {
    #[cfg(feature = "legacy-storage")]
    /// Read directory information from the configured storage location.
    pub fn load_legacy(&self) -> Result<tor_netdir::PartialNetDir> {
        let path = self
            .legacy_cache_path
            .as_ref()
            .ok_or(Error::BadNetworkConfig("No legacy cache path available"))?;
        let store = LegacyStore::new(path.clone());
        store.load_legacy(&self.authorities[..])
    }

    /// Return a slice of the configured authorities
    pub fn authorities(&self) -> &[Authority] {
        &self.network.authority[..]
    }

    /// Return the configured set of fallback directories
    pub fn fallbacks(&self) -> &[FallbackDir] {
        &self.network.fallback_cache[..]
    }

    /// Return set of configured networkstatus parameter overrides.
    pub fn override_net_params(&self) -> &netstatus::NetParams<i32> {
        &self.network.override_net_params
    }

    /// Return the timing configuration we should use to decide when to
    /// attemppt and retry downloads.
    pub fn timing(&self) -> &DownloadScheduleConfig {
        &self.timing
    }
}

impl DownloadScheduleConfig {
    /// Return configuration for retrying our entire bootstrap
    /// operation at startup.
    pub fn retry_bootstrap(&self) -> &RetryConfig {
        &self.retry_bootstrap
    }

    /// Return configuration for retrying a consensus download.
    pub fn retry_consensus(&self) -> &RetryConfig {
        &self.retry_consensus
    }

    /// Return configuration for retrying an authority certificate download
    pub fn retry_certs(&self) -> &RetryConfig {
        &self.retry_certs
    }

    /// Return configuration for retrying an authority certificate download
    pub fn retry_microdescs(&self) -> &RetryConfig {
        &self.retry_microdescs
    }

    /// Number of microdescriptor fetches to attemppt in parallel
    pub fn microdesc_parallelism(&self) -> usize {
        self.microdesc_parallelism.max(1).into()
    }
}

/// Helpers for fallbacksx
mod fallbacks {
    use tor_llcrypto::pk::{ed25519::Ed25519Identity, rsa::RsaIdentity};
    use tor_netdir::fallback::FallbackDir;
    /// Return a list of the default fallback directories shipped with
    /// arti.
    pub(crate) fn default_fallbacks() -> Vec<super::FallbackDir> {
        /// Build a fallback directory; panic if input is bad.
        fn fallback(rsa: &str, ed: &str, ports: Vec<&str>) -> FallbackDir {
            let rsa = hex::decode(rsa).expect("Bad hex in built-in fallback list");
            let rsa =
                RsaIdentity::from_bytes(&rsa).expect("Wrong length in built-in fallback list");
            let ed = base64::decode_config(ed, base64::STANDARD_NO_PAD)
                .expect("Bad hex in built-in fallback list");
            let ed =
                Ed25519Identity::from_bytes(&ed).expect("Wrong length in built-in fallback list");
            let ports = ports
                .iter()
                .map(|s| s.parse().expect("Bad socket address in fallbacklist"))
                .collect();
            FallbackDir::new(rsa, ed, ports)
        }
        include!("fallback_dirs.inc")
    }
}
