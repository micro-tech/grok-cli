use grok_cli::config::OperationalMode;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operational_modes() {
        // Test that OperationalMode enum values exist
        let _research = OperationalMode::Research;
        let _coder = OperationalMode::Coder;
        let _creative = OperationalMode::Creative;
        let _shell = OperationalMode::Shell;
    }
}