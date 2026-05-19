//! Shared RPC timeouts for external calls (P-027).

use std::time::Duration;

use tokio::time::timeout;

pub const RPC_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn with_rpc_timeout<T, E>(
    label: &str,
    fut: impl std::future::Future<Output = Result<T, E>>,
) -> Result<T, String>
where
    E: std::fmt::Display,
{
    match timeout(RPC_TIMEOUT, fut).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(err)) => Err(format!("{label}: {err}")),
        Err(_) => Err(format!(
            "{label}: timed out after {}s",
            RPC_TIMEOUT.as_secs()
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration;

    #[tokio::test]
    async fn times_out_slow_future() {
        let result = with_rpc_timeout("test", async {
            tokio::time::sleep(StdDuration::from_secs(60)).await;
            Ok::<(), &str>(())
        })
        .await;
        assert!(result.unwrap_err().contains("timed out"));
    }
}
