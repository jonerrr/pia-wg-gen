#[cfg(test)]
mod tests {
    use assert_cmd::prelude::*;
    use predicates::prelude::*;
    use std::process::Command;

    #[test]
    fn test_cli_help() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        cmd.arg("--help");

        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Usage: "));
    }

    #[test]
    fn test_invalid_args() {
        let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
        cmd.arg("p111111");
        cmd.arg("u111111");

        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Failed to get token"));
    }

    //TODO: test valid pia creds
}
