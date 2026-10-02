//! [`TxLookup`] adapters (#516): the Bitcoin node and the Electrum server, the same two sources
//! the broadcasters use.
//!
//! Each answer is typed. "Not found" is only ever a source's explicit answer — bitcoind's JSON-RPC
//! code -5, or an Electrum history that lists the transaction's output script without it — never
//! a match on message text. Everything else is `Unanswered`.

use std::sync::Arc;

use async_trait::async_trait;
use bdk_electrum::electrum_client::{Error as ElectrumError, GetHistoryRes};
use bitcoin::{Transaction, Txid};

use crate::application::tx_settle::{Lookup, TrackedTx, TxLookup};
use crate::infrastructure::bitcoin_rpc::{BitcoinRpcClient, RpcError, RPC_INVALID_ADDRESS_OR_KEY};
use crate::infrastructure::electrum_broadcaster::relayed_bitcoind_code;

/// Looks a transaction up with `getrawtransaction <txid> true`.
///
/// Without `-txindex` the node only finds mempool transactions and answers -5 for a mined one;
/// the Electrum source, which indexes the chain, still finds it, so the combined answer is never
/// `Absent` for a mined transaction while Electrum answers.
pub struct NodeTxLookup {
    rpc: Arc<dyn BitcoinRpcClient>,
}

impl NodeTxLookup {
    pub fn new(rpc: Arc<dyn BitcoinRpcClient>) -> Self {
        Self { rpc }
    }
}

fn node_answer(result: Result<u32, RpcError>) -> Lookup {
    match result {
        Ok(confirmations) => Lookup::Found {
            confirmed: confirmations >= 1,
        },
        Err(e) if e.code == Some(RPC_INVALID_ADDRESS_OR_KEY) => Lookup::NotFound,
        Err(_) => Lookup::Unanswered,
    }
}

#[async_trait]
impl TxLookup for NodeTxLookup {
    fn name(&self) -> &'static str {
        "Bitcoin node"
    }

    async fn lookup(&self, tracked: &TrackedTx) -> Lookup {
        node_answer(
            self.rpc
                .get_transaction_depth(&tracked.txid.to_string())
                .await,
        )
    }
}

/// Looks a transaction up on the Electrum server.
///
/// With the transaction at hand, the history of its first spendable output script is read
/// (`blockchain.scripthash.get_history`, which every Electrum server implements and which lists
/// mempool and mined transactions): the txid listed → found (mined when its height is positive);
/// not listed → not found. That is an answer, not an error, whatever the server's error format.
///
/// With only the txid, `blockchain.transaction.get` is used: a transaction → found (depth
/// unknown); an error relaying bitcoind's -5 → not found (see `relayed_bitcoind_code`); any other
/// error → unanswered. romanz/electrs relays that -5 without its code, so there only the history
/// answers "not found".
pub struct ElectrumTxLookup {
    electrum_url: String,
}

impl ElectrumTxLookup {
    pub fn new(electrum_url: impl Into<String>) -> Self {
        Self {
            electrum_url: electrum_url.into(),
        }
    }
}

/// The output script whose history can list `tx`: the first output that is not `OP_RETURN`.
fn watched_script(tx: &Transaction) -> Option<bitcoin::ScriptBuf> {
    tx.output
        .iter()
        .map(|out| out.script_pubkey.clone())
        .find(|script| !script.is_op_return() && !script.is_empty())
}

fn history_answer(txid: Txid, history: &[GetHistoryRes]) -> Lookup {
    match history.iter().find(|entry| entry.tx_hash == txid) {
        Some(entry) => Lookup::Found {
            confirmed: entry.height > 0,
        },
        None => Lookup::NotFound,
    }
}

fn transaction_get_failure(e: &ElectrumError) -> Lookup {
    match e {
        ElectrumError::Protocol(error)
            if relayed_bitcoind_code(error) == Some(RPC_INVALID_ADDRESS_OR_KEY) =>
        {
            Lookup::NotFound
        }
        _ => Lookup::Unanswered,
    }
}

fn electrum_lookup(url: &str, tracked: &TrackedTx) -> Lookup {
    use bdk_electrum::electrum_client::{Client, ElectrumApi};

    let Ok(client) = Client::new(url) else {
        return Lookup::Unanswered;
    };
    match tracked.tx.as_ref().and_then(watched_script) {
        Some(script) => match client.script_get_history(&script) {
            Ok(history) => history_answer(tracked.txid, &history),
            Err(_) => Lookup::Unanswered,
        },
        None => match client.transaction_get(&tracked.txid) {
            Ok(_) => Lookup::Found { confirmed: false },
            Err(e) => transaction_get_failure(&e),
        },
    }
}

#[async_trait]
impl TxLookup for ElectrumTxLookup {
    fn name(&self) -> &'static str {
        "Electrum"
    }

    async fn lookup(&self, tracked: &TrackedTx) -> Lookup {
        let url = self.electrum_url.clone();
        let tracked = tracked.clone();
        // The Electrum client is blocking I/O, like the broadcaster and the wallet sync.
        tokio::task::spawn_blocking(move || electrum_lookup(&url, &tracked))
            .await
            .unwrap_or(Lookup::Unanswered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::hashes::Hash;
    use serde_json::json;

    /// #516: only the node's -5 says "not held"; any other failure is no answer.
    #[test]
    fn node_answers_are_typed() {
        let cases = [
            (Ok(0), Lookup::Found { confirmed: false }),
            (Ok(3), Lookup::Found { confirmed: true }),
            (
                Err(RpcError::answered(
                    Some(-5),
                    "No such mempool or blockchain transaction",
                )),
                Lookup::NotFound,
            ),
            (
                Err(RpcError::answered(Some(-28), "Loading block index...")),
                Lookup::Unanswered,
            ),
            (
                Err(RpcError::not_connected("tcp connect error")),
                Lookup::Unanswered,
            ),
            (
                Err(RpcError::unknown("operation timed out")),
                Lookup::Unanswered,
            ),
        ];
        for (result, expected) in cases {
            assert_eq!(node_answer(result.clone()), expected, "{result:?}");
        }
    }

    fn txid(n: u8) -> Txid {
        Txid::from_byte_array([n; 32])
    }

    /// #516: an Electrum history that lists the script without the tx is an answer: not found.
    #[test]
    fn electrum_history_says_whether_the_tx_is_held() {
        let entry = |n: u8, height: i32| GetHistoryRes {
            height,
            tx_hash: txid(n),
            fee: None,
        };
        let cases = [
            (vec![entry(1, 0)], Lookup::Found { confirmed: false }),
            (
                vec![entry(2, 800), entry(1, -1)],
                Lookup::Found { confirmed: false },
            ),
            (vec![entry(1, 800)], Lookup::Found { confirmed: true }),
            (vec![entry(2, 800)], Lookup::NotFound),
            (vec![], Lookup::NotFound),
        ];
        for (history, expected) in cases {
            assert_eq!(history_answer(txid(1), &history), expected);
        }
    }

    /// #516: `transaction.get` says "not found" only through bitcoind's relayed -5, in the shapes
    /// `relayed_bitcoind_code` reads; romanz/electrs' bare message is no answer.
    #[test]
    fn electrum_transaction_get_errors_are_typed() {
        let cases = [
            (
                ElectrumError::Protocol(json!({"code": 2, "message":
                    "daemon error: DaemonError({'code': -5, 'message': 'No such mempool or blockchain transaction.'})"})),
                Lookup::NotFound,
            ),
            (
                ElectrumError::Protocol(
                    json!({"code": -5, "message": "No such mempool transaction"}),
                ),
                Lookup::NotFound,
            ),
            (
                ElectrumError::Protocol(json!({"code": 2, "message":
                    "No such mempool or blockchain transaction. Use gettransaction for wallet transactions."})),
                Lookup::Unanswered,
            ),
            (
                ElectrumError::IOError(std::io::Error::other("connection reset")),
                Lookup::Unanswered,
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(transaction_get_failure(&error), expected, "{error:?}");
        }
    }

    /// No server listening: no answer, never "not found".
    #[tokio::test]
    async fn unreachable_electrum_server_is_unanswered() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let answer = ElectrumTxLookup::new(format!("tcp://{addr}"))
            .lookup(&TrackedTx {
                txid: txid(1),
                tx: None,
            })
            .await;

        assert_eq!(answer, Lookup::Unanswered);
    }
}
