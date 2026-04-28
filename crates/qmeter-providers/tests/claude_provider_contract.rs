use std::time::Duration;

use qmeter_core::types::NormalizedErrorType;
use qmeter_providers::claude::{
    ClaudeProvider, ClaudeProviderConfig, ClaudeScreenRunner,
};
use qmeter_providers::provider::AcquireContext;

struct FakeClaudeRunner {
    result: Result<String, String>,
}

impl ClaudeScreenRunner for FakeClaudeRunner {
    fn capture_usage_screen(&self, _config: &ClaudeProviderConfig) -> Result<String, String> {
        self.result.clone()
    }
}

fn provider() -> ClaudeProvider {
    ClaudeProvider::new(ClaudeProviderConfig {
        bash_command: "bash-test".to_string(),
        timeout: Duration::from_secs(1),
    })
}

#[test]
fn claude_provider_parses_runner_screen() {
    let runner = FakeClaudeRunner {
        result: Ok(
            [
                "Settings: Usage",
                "Current session",
                "  90% used",
                "  Resets 3am",
                "",
                "Current week (all models)",
                "  21% used",
                "  Resets Feb 28, 10am",
            ]
            .join("\n"),
        ),
    };

    let result = provider().acquire_with_runner(&runner, AcquireContext {
        refresh: true,
        debug: true,
    });

    assert_eq!(result.errors, vec![]);
    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.rows[0].window, "claude:session");
    assert_eq!(result.rows[0].used_percent, Some(90.0));
    assert!(result.debug.is_some());
}

#[test]
fn claude_provider_maps_runner_timeout() {
    let runner = FakeClaudeRunner {
        result: Err("claude /usage timed out after 25000ms".to_string()),
    };

    let result = provider().acquire_with_runner(&runner, AcquireContext {
        refresh: true,
        debug: false,
    });

    assert_eq!(result.rows, vec![]);
    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0].error_type, NormalizedErrorType::Timeout);
}
