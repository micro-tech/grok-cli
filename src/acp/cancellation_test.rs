//! Simple test for the new ACP cancellation mechanism (Task 221)

#[cfg(test)]
mod tests {
    use crate::acp::GrokAcpAgent;
    use crate::config::Config;
    use std::sync::atomic::Ordering;
    use tokio::runtime::Runtime;

    #[test]
    fn cancellation_flag_roundtrip() {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let config = Config::default();
            let agent = GrokAcpAgent::new(config, None).await.unwrap();

            let flag = agent.get_cancellation_flag("test-sess").await;
            assert!(!flag.load(Ordering::SeqCst));

            agent.cancel_session("test-sess").await;
            assert!(flag.load(Ordering::SeqCst));

            agent.clear_cancellation_flag("test-sess").await;
            assert!(!flag.load(Ordering::SeqCst));
        });
    }
}