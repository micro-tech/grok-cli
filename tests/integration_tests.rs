/// Smoke integration tests — verify key public-API items are reachable.
///
/// NOTE: `OperationalMode` was removed in the "Switch grok_api path; drop OperationalMode"
/// commit.  This file previously tested that enum and has been updated accordingly.
#[cfg(test)]
mod tests {
    #[test]
    fn config_module_is_accessible() {
        // Ensure the config module compiles and exposes the expected types.
        use grok_cli::config::AcpConfig;
        let _cfg = AcpConfig::default();
    }

    #[test]
    fn router_module_is_accessible() {
        // Verify the AppRouter shim type is publicly reachable.
        use grok_cli::router::AppRouter;
        let _ = std::mem::size_of::<AppRouter>();
    }
}
