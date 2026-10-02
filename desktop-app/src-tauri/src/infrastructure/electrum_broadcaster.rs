//! [`TxBroadcaster`] implementation over the Electrum protocol (M3).
//!
//! Uses `bdk_electrum::electrum_client::{Client, ElectrumApi}` — the same dependency
//! as wallet sync and `ElectrumFeeEstimator`. Connection and broadcast both run
//! inside `spawn_blocking` (the Electrum client is blocking I/O), mirroring
//! `wallet_service.rs::do_sync`.

use async_trait::async_trait;

use bdk_electrum::electrum_client::Error as ElectrumError;

use crate::application::tx_broadcaster::{
    is_already_known, FailureKind, PairBroadcastError, TxBroadcastError, TxBroadcaster,
};

const SOURCE: &str = "Electrum";

/// Broadcasts commit+reveal via an Electrum server.
pub struct ElectrumBroadcaster {
    electrum_url: String,
}

impl ElectrumBroadcaster {
    pub fn new(electrum_url: impl Into<String>) -> Self {
        Self {
            electrum_url: electrum_url.into(),
        }
    }
}

fn err(kind: FailureKind, message: impl Into<String>) -> TxBroadcastError {
    TxBroadcastError {
        source_name: SOURCE,
        message: message.into(),
        kind,
    }
}

/// The bitcoind JSON-RPC error code an Electrum server relayed in its error object (#516), if it
/// carries one.
///
/// Electrum servers relay the daemon's refusal in different shapes, and only the code is read —
/// never the free text around it:
///
/// - the code as the error's own `code` (`{"code": -26, "message": "..."}`);
/// - Blockstream's electrs (esplora), JSON inside the message:
///   `sendrawtransaction RPC error: {"code":-26,"message":"..."}`;
/// - ElectrumX, the daemon error's repr: `daemon error: DaemonError({'code': -5, 'message': ...})`.
///
/// romanz/electrs (0.10, the regtest stack's) relays only the daemon's message, under its own
/// code 2, and ElectrumX's broadcast refusal carries no code either: both yield `None`.
pub(crate) fn relayed_bitcoind_code(error: &serde_json::Value) -> Option<i64> {
    const BITCOIND_CODES: std::ops::RangeInclusive<i64> = -28..=-1;
    let own = error
        .get("code")
        .and_then(serde_json::Value::as_i64)
        .filter(|code| BITCOIND_CODES.contains(code));
    own.or_else(|| {
        let message = error.get("message")?.as_str()?;
        embedded_codes(message).find(|code| BITCOIND_CODES.contains(code))
    })
}

/// Every integer written right after a `code` key in `text`: `"code":-26`, `'code': -5`,
/// `code -26`, `code=-25`.
fn embedded_codes(text: &str) -> impl Iterator<Item = i64> + '_ {
    text.match_indices("code").filter_map(move |(at, key)| {
        let rest = text[at + key.len()..].trim_start_matches(['"', '\'', ' ', ':', '=']);
        let digits_end = rest
            .char_indices()
            .find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && c == '-')))
            .map_or(rest.len(), |(i, _)| i);
        rest[..digits_end].parse().ok()
    })
}

/// bitcoind: the transaction was refused (`RPC_TRANSACTION_ERROR`, `RPC_TRANSACTION_REJECTED`).
const REFUSED: [i64; 2] = [-25, -26];
/// bitcoind: `RPC_TRANSACTION_ALREADY_IN_CHAIN`.
const ALREADY_IN_CHAIN: i64 = -27;

/// How a `blockchain.transaction.broadcast` call ended (#516). `Ok` is an acceptance ("already
/// known" included). Only an error that carries bitcoind's own refusal (-25/-26, see
/// [`relayed_bitcoind_code`]) is a rejection: any other server error — a daemon that timed out
/// or was unreachable, an index not ready, a refusal relayed without its code — and any
/// transport failure after the request went out leave the tx possibly live.
fn broadcast_outcome(e: &ElectrumError) -> Result<(), FailureKind> {
    if is_already_known(&e.to_string()) {
        return Ok(());
    }
    match e {
        ElectrumError::Protocol(error) => match relayed_bitcoind_code(error) {
            Some(ALREADY_IN_CHAIN) => Ok(()),
            Some(code) if REFUSED.contains(&code) => Err(FailureKind::Rejected),
            _ => Err(FailureKind::Ambiguous),
        },
        _ => Err(FailureKind::Ambiguous),
    }
}

/// The connection to the server never opened: nothing was delivered.
fn connect(url: &str) -> Result<bdk_electrum::electrum_client::Client, TxBroadcastError> {
    bdk_electrum::electrum_client::Client::new(url)
        .map_err(|e| err(FailureKind::NotDelivered, e.to_string()))
}

fn broadcast_one(
    client: &bdk_electrum::electrum_client::Client,
    hex: &str,
) -> Result<(), TxBroadcastError> {
    use bdk_electrum::electrum_client::ElectrumApi;

    // A tx that cannot be decoded is never sent.
    let tx_bytes = hex::decode(hex)
        .map_err(|e| err(FailureKind::NotDelivered, format!("hex decode failed: {e}")))?;
    let tx: bitcoin::Transaction = bitcoin::consensus::deserialize(&tx_bytes).map_err(|e| {
        err(
            FailureKind::NotDelivered,
            format!("tx deserialize failed: {e}"),
        )
    })?;
    match client.transaction_broadcast(&tx) {
        Ok(_) => Ok(()),
        Err(e) => broadcast_outcome(&e).map_err(|kind| err(kind, e.to_string())),
    }
}

/// A `spawn_blocking` task that did not finish: whatever it sent may have landed.
fn join_failure(e: tokio::task::JoinError) -> TxBroadcastError {
    err(FailureKind::Ambiguous, format!("spawn_blocking panic: {e}"))
}

#[async_trait]
impl TxBroadcaster for ElectrumBroadcaster {
    fn name(&self) -> &'static str {
        SOURCE
    }

    async fn broadcast_pair(
        &self,
        commit_hex: &str,
        reveal_hex: &str,
    ) -> Result<(), PairBroadcastError> {
        let url = self.electrum_url.clone();
        let commit = commit_hex.to_string();
        let reveal = reveal_hex.to_string();

        tokio::task::spawn_blocking(move || {
            let client = connect(&url).map_err(PairBroadcastError::at_commit)?;
            broadcast_one(&client, &commit).map_err(PairBroadcastError::at_commit)?;
            broadcast_one(&client, &reveal).map_err(PairBroadcastError::at_reveal)
        })
        .await
        .map_err(|e| PairBroadcastError::at_commit(join_failure(e)))?
    }

    async fn broadcast_one(&self, tx_hex: &str) -> Result<(), TxBroadcastError> {
        let url = self.electrum_url.clone();
        let hex = tx_hex.to_string();

        tokio::task::spawn_blocking(move || broadcast_one(&connect(&url)?, &hex))
            .await
            .map_err(join_failure)?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #516: only bitcoind's own refusal, relayed with its code, is a rejection; "already in
    /// chain" is an acceptance; every other answer — or no answer — may have left the tx live.
    /// The shapes are the ones real servers send (see `relayed_bitcoind_code`).
    #[test]
    fn broadcast_failures_are_classified_from_the_relayed_bitcoind_code() {
        use serde_json::json;
        let protocol = ElectrumError::Protocol;
        let cases: Vec<(ElectrumError, Result<(), FailureKind>)> = vec![
            // The daemon's code as the error's own code.
            (
                protocol(json!({"code": -26, "message": "min relay fee not met, 100 < 141"})),
                Err(FailureKind::Rejected),
            ),
            // Blockstream electrs (esplora): the daemon's JSON error inside the message.
            (
                protocol(json!({"code": 1, "message":
                    "sendrawtransaction RPC error: {\"code\":-25,\"message\":\"bad-txns-inputs-missingorspent\"}"})),
                Err(FailureKind::Rejected),
            ),
            // ElectrumX: the daemon error's repr.
            (
                protocol(json!({"code": 2, "message":
                    "daemon error: DaemonError({'code': -26, 'message': 'txn-mempool-conflict'})"})),
                Err(FailureKind::Rejected),
            ),
            (
                protocol(json!({"code": 2, "message":
                    "daemon error: DaemonError({'code': -27, 'message': 'Transaction outputs already in utxo set'})"})),
                Ok(()),
            ),
            // A daemon that is warming up or timed out has not refused anything.
            (
                protocol(json!({"code": 2, "message":
                    "daemon error: DaemonError({'code': -28, 'message': 'Loading block index...'})"})),
                Err(FailureKind::Ambiguous),
            ),
            // romanz/electrs 0.10 relays only the daemon's message, under its own code 2.
            (
                protocol(json!({"code": 2, "message": "min relay fee not met, 100 < 141"})),
                Err(FailureKind::Ambiguous),
            ),
            // ElectrumX's broadcast refusal names no code.
            (
                protocol(json!({"code": 1, "message":
                    "the transaction was rejected by network rules.\n\nmin relay fee not met\n[0200]"})),
                Err(FailureKind::Ambiguous),
            ),
            (
                protocol(json!({"code": -32603, "message": "unavailable index"})),
                Err(FailureKind::Ambiguous),
            ),
            (
                ElectrumError::IOError(std::io::Error::other("connection reset")),
                Err(FailureKind::Ambiguous),
            ),
            (
                ElectrumError::AllAttemptsErrored(vec![]),
                Err(FailureKind::Ambiguous),
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(broadcast_outcome(&error), expected, "{error:?}");
        }
    }

    /// #516: no server listening — the connection never opens, so nothing was delivered.
    #[tokio::test]
    async fn unreachable_server_is_not_delivered() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let e = ElectrumBroadcaster::new(format!("tcp://{addr}"))
            .broadcast_pair("00", "00")
            .await
            .unwrap_err();

        assert_eq!(
            (e.commit_accepted, e.error.kind),
            (false, FailureKind::NotDelivered)
        );
    }
}
