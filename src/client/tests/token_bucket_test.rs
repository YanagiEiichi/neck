#[cfg(test)]
use std::time::Duration;
use tokio;

#[cfg(test)]
use tokio::time::timeout;

#[cfg(test)]
use super::super::token_bucket::TokenBucket;

#[tokio::test]
async fn basic_test() {
    let tb = TokenBucket::new(2);

    // Acquire the first token will be successful.
    let _t1 = tb.acquire().await;

    let _ = {
        // Acquire the second token will be successful too.
        let t2 = tb.acquire().await;

        // Acquire the third token will block.
        // Because the t2 still being held and has not been put back yet, leaving the TokenBucket is currently empty.
        assert!(timeout(Duration::from_secs(1), tb.acquire()).await.is_err());
        t2
    };

    // Now, t2 is not used in after, it will be dropped, or say "put back".

    // Acquire the new second token will be successful.
    let _new_t2 = tb.acquire().await;
}
