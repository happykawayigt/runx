use crate::dev::types::{DevFixtureStatus, DevReport, DevReportStatus};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DevRenderTheme {
    pub success: &'static str,
    pub failure: &'static str,
    pub skipped: &'static str,
    pub needs_approval: &'static str,
}

impl Default for DevRenderTheme {
    fn default() -> Self {
        Self {
            success: "ok",
            failure: "x",
            skipped: "-",
            needs_approval: "!",
        }
    }
}

#[must_use]
pub fn render_dev_result(result: &DevReport) -> String {
    render_dev_result_with_theme(result, &DevRenderTheme::default())
}

#[must_use]
pub fn render_dev_result_with_theme(result: &DevReport, theme: &DevRenderTheme) -> String {
    let mut lines = vec![
        String::new(),
        format!(
            "  {}  dev  {} fixture(s)",
            status_icon(&result.status, theme),
            result.fixtures.len()
        ),
    ];
    for fixture in &result.fixtures {
        lines.push(format!(
            "  {}  {:<14} {}  {}ms",
            fixture_status_icon(&fixture.status, theme),
            fixture.lane,
            fixture.name,
            fixture.duration_ms
        ));
        for assertion in fixture.assertions.iter().take(3) {
            lines.push(format!("     {}: {}", assertion.path, assertion.message));
        }
    }
    if let Some(receipt_id) = &result.receipt_id {
        lines.push(format!("  receipt  {receipt_id}"));
    }
    lines.push(String::new());
    lines.join("\n")
}

fn status_icon(status: &DevReportStatus, theme: &DevRenderTheme) -> &'static str {
    match status {
        DevReportStatus::Success => theme.success,
        DevReportStatus::Failure => theme.failure,
        DevReportStatus::Skipped => theme.skipped,
        DevReportStatus::NeedsApproval => theme.needs_approval,
    }
}

fn fixture_status_icon(status: &DevFixtureStatus, theme: &DevRenderTheme) -> &'static str {
    match status {
        DevFixtureStatus::Success => theme.success,
        DevFixtureStatus::Failure => theme.failure,
        DevFixtureStatus::Skipped => theme.skipped,
    }
}
