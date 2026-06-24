//! In-memory cache of ephemeral envelope keypairs, keyed by the envelope payload.
//!
//! Why this exists: the commit address shown to the signer ("COMMIT TX PREVIEW") and the
//! commit address actually funded/signed must be **identical**, or the on-device
//! verification of the hardware wallet is meaningless (issue #382). The commit address is a
//! pure function of `(envelope_keypair, payload, network)`. Previously the preview step and
//! the broadcast step each generated their *own* random ephemeral keypair, so the two
//! addresses never matched.
//!
//! This cache derives the keypair **once** for a given payload and reuses it across the
//! preview → broadcast lifecycle. The keypair stays in memory only — never persisted, never
//! derived from a seed — preserving the ephemeral-key property of
//! [`super::ephemeral_envelope_key`]. Entries expire after [`ENTRY_TTL`] so an abandoned
//! preview cannot pin a keypair indefinitely, and are explicitly evicted once the
//! transactions are signed.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use bitcoin::hashes::{sha256, Hash};
use bitcoin::key::UntweakedKeypair;

use super::ephemeral_envelope_key::generate_ephemeral_envelope_keypair;

/// How long a cached keypair survives without being broadcast. A preview that is never
/// followed by a broadcast (e.g. the user changes the fee and re-previews, or closes the
/// screen) is pruned after this window.
const ENTRY_TTL: Duration = Duration::from_secs(30 * 60);

struct CacheEntry {
    keypair: UntweakedKeypair,
    created: Instant,
}

/// Tauri-managed cache mapping an envelope payload to the ephemeral keypair used to derive
/// its commit address. Shared between the broadcast-preview and broadcast-submit commands.
#[derive(Default)]
pub struct EnvelopeKeyCache {
    inner: Mutex<HashMap<String, CacheEntry>>,
}

fn payload_key(payload: &[u8]) -> String {
    sha256::Hash::hash(payload).to_string()
}

impl EnvelopeKeyCache {
    /// Returns the ephemeral keypair for `payload`, generating and caching a fresh one on the
    /// first call and returning the same keypair on subsequent calls for an identical payload.
    ///
    /// A different payload (e.g. a new signature added before broadcast) yields a different
    /// key — which is correct, since the commit address must change with the payload.
    pub fn get_or_generate(&self, payload: &[u8]) -> UntweakedKeypair {
        let key = payload_key(payload);
        let mut map = self.inner.lock().expect("envelope key cache poisoned");
        map.retain(|_, e| e.created.elapsed() < ENTRY_TTL);
        let entry = map.entry(key).or_insert_with(|| CacheEntry {
            keypair: generate_ephemeral_envelope_keypair(),
            created: Instant::now(),
        });
        entry.keypair
    }

    /// Drops the cached keypair for `payload`. Called once the commit/reveal pair is signed
    /// so the entry does not linger after it is no longer needed.
    pub fn evict(&self, payload: &[u8]) {
        let key = payload_key(payload);
        self.inner
            .lock()
            .expect("envelope key cache poisoned")
            .remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_payload_returns_same_keypair() {
        let cache = EnvelopeKeyCache::default();
        let payload = b"envelope-payload-bytes";
        let first = cache.get_or_generate(payload);
        let second = cache.get_or_generate(payload);
        assert_eq!(
            first.secret_key().secret_bytes(),
            second.secret_key().secret_bytes(),
            "preview and broadcast must reuse the same ephemeral keypair for one payload"
        );
    }

    #[test]
    fn different_payloads_return_different_keypairs() {
        let cache = EnvelopeKeyCache::default();
        let a = cache.get_or_generate(b"payload-a");
        let b = cache.get_or_generate(b"payload-b");
        assert_ne!(
            a.secret_key().secret_bytes(),
            b.secret_key().secret_bytes(),
            "distinct payloads must derive distinct commit addresses"
        );
    }

    #[test]
    fn evict_forces_a_fresh_keypair() {
        let cache = EnvelopeKeyCache::default();
        let payload = b"evictable-payload";
        let before = cache.get_or_generate(payload);
        cache.evict(payload);
        let after = cache.get_or_generate(payload);
        assert_ne!(
            before.secret_key().secret_bytes(),
            after.secret_key().secret_bytes(),
            "after eviction a new broadcast attempt must mint a fresh keypair"
        );
    }
}
