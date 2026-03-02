use clap::Parser;

/// Command-line interface for nv-swaptop.
#[derive(Parser, Debug)]
#[command(name = "nv-swaptop", version)]
#[command(about = "Real-time TUI monitor for swap, NUMA topology, and GPU memory (Linux)")]
pub struct Cli {
    /// Run with synthetic demo data instead of real system data
    #[arg(long)]
    pub demo: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_demo_flag() {
        let cli = Cli::try_parse_from(["nv-swaptop", "--demo"]).unwrap();
        assert!(cli.demo);
    }

    #[test]
    fn test_cli_parse_no_args() {
        let cli = Cli::try_parse_from(["nv-swaptop"]).unwrap();
        assert!(!cli.demo);
    }

    #[test]
    fn test_cli_rejects_unknown_flag() {
        let result = Cli::try_parse_from(["nv-swaptop", "--unknown"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cli_version_matches_cargo_toml() {
        let result = Cli::try_parse_from(["nv-swaptop", "-V"]);
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
        let output = err.to_string();
        assert!(
            output.contains(env!("CARGO_PKG_VERSION")),
            "Version output {output:?} should contain {}",
            env!("CARGO_PKG_VERSION"),
        );
    }

    #[test]
    fn test_cli_short_help_does_not_panic() {
        let result = Cli::try_parse_from(["nv-swaptop", "--help"]);
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayHelp);
    }

    #[test]
    fn test_cli_short_version_does_not_panic() {
        let result = Cli::try_parse_from(["nv-swaptop", "-V"]);
        let err = result.unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::DisplayVersion);
    }
}
