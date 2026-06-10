use std::path::Path;

pub(crate) struct SkillResumeCommand<'a> {
    pub(crate) skill_ref: Option<&'a str>,
    pub(crate) run_id: &'a str,
    pub(crate) selected_runner: Option<&'a str>,
    pub(crate) receipt_dir: Option<&'a Path>,
    pub(crate) answers_path: Option<&'a Path>,
}

pub(crate) fn render_skill_resume_command(command: SkillResumeCommand<'_>) -> String {
    let mut parts = vec![
        "runx".to_owned(),
        "skill".to_owned(),
        shell_token(command.skill_ref.unwrap_or("SKILL.md")),
    ];
    if let Some(runner) = command.selected_runner.and_then(non_empty) {
        parts.push("--runner".to_owned());
        parts.push(shell_token(runner));
    }
    if let Some(receipt_dir) = command.receipt_dir {
        parts.push("--receipt-dir".to_owned());
        parts.push(shell_token(&receipt_dir.to_string_lossy()));
    }
    parts.extend([
        "--run-id".to_owned(),
        shell_token(command.run_id),
        "--answers".to_owned(),
    ]);
    parts.push(shell_token(
        &command
            .answers_path
            .map_or_else(|| "answers.json".into(), Path::to_string_lossy),
    ));
    parts.join(" ")
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn shell_token(value: &str) -> String {
    if value.is_empty() {
        return "''".to_owned();
    }
    if value.chars().all(|character| {
        character.is_ascii_alphanumeric() || matches!(character, '/' | '.' | '_' | '-' | ':' | '@')
    }) {
        return value.to_owned();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{SkillResumeCommand, render_skill_resume_command};

    #[test]
    fn resume_command_quotes_operator_supplied_tokens() {
        let command = render_skill_resume_command(SkillResumeCommand {
            skill_ref: Some("skills/support reply"),
            run_id: "run abc",
            selected_runner: Some("agent task"),
            receipt_dir: Some(Path::new("custom receipts")),
            answers_path: Some(Path::new("my answers.json")),
        });

        assert_eq!(
            command,
            "runx skill 'skills/support reply' --runner 'agent task' --receipt-dir 'custom receipts' --run-id 'run abc' --answers 'my answers.json'"
        );
    }

    #[test]
    fn resume_command_uses_safe_defaults_when_metadata_is_missing() {
        let command = render_skill_resume_command(SkillResumeCommand {
            skill_ref: None,
            run_id: "rx_123",
            selected_runner: None,
            receipt_dir: None,
            answers_path: None,
        });

        assert_eq!(
            command,
            "runx skill SKILL.md --run-id rx_123 --answers answers.json"
        );
    }
}
