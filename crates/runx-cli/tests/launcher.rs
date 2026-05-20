use runx_cli::config::{ConfigAction, ConfigPlan};
use runx_cli::connect::{ConnectAction, ConnectAuthorityKind, ConnectPlan};
use runx_cli::kernel::{KernelInputSource, KernelPlan};
use runx_cli::mcp::McpPlan;
use runx_cli::policy::{PolicyAction, PolicyPlan};
use runx_cli::registry::{RegistryAction, RegistryPlan};
use std::fs;
use std::path::{Path, PathBuf};

use runx_cli::launcher::{
    CommandPlan, DEFAULT_NPM_PACKAGE, DoctorPlan, HarnessPlan, HistoryPlan, InitPlan,
    LauncherAction, ListKind, ListPlan, NativeLauncherOptions, NewPlan, ToolAction, ToolPlan,
    node_command, npm_command, plan_launcher, plan_launcher_with_native_options,
    plan_launcher_with_rust_harness,
};

fn plan_with_rust_cli(args: Vec<std::ffi::OsString>) -> LauncherAction {
    plan_launcher_with_native_options(
        args,
        None,
        None,
        NativeLauncherOptions {
            rust_cli: Some("1".into()),
            rust_harness: None,
        },
    )
}

fn plan_with_rust_cli_and_js(args: Vec<std::ffi::OsString>) -> LauncherAction {
    plan_launcher_with_native_options(
        args,
        Some("@runxhq/cli@0.5.22".into()),
        Some("/repo/oss/packages/cli/bin/runx.js".into()),
        NativeLauncherOptions {
            rust_cli: Some("1".into()),
            rust_harness: None,
        },
    )
}

fn plan_with_rust_cli_signal_and_js(signal: &str, args: Vec<std::ffi::OsString>) -> LauncherAction {
    plan_launcher_with_native_options(
        args,
        Some("@runxhq/cli@0.5.22".into()),
        Some("/repo/oss/packages/cli/bin/runx.js".into()),
        NativeLauncherOptions {
            rust_cli: Some(signal.into()),
            rust_harness: None,
        },
    )
}

fn assert_delegates_to_js_bin_without_native_signals(args: Vec<std::ffi::OsString>) {
    let js_bin = std::ffi::OsString::from("/repo/oss/packages/cli/bin/runx.js");
    let mut expected_args = vec![js_bin.clone()];
    expected_args.extend(args.clone());

    assert_eq!(
        plan_launcher(
            args,
            Some("@runxhq/cli@0.5.22".into()),
            Some(js_bin.clone()),
        ),
        LauncherAction::Delegate(CommandPlan {
            program: node_command().into(),
            args: expected_args,
        })
    );
}

#[test]
fn pre_cutover_candidate_commands_delegate_without_native_signals() {
    for args in [
        vec!["connect".into(), "list".into(), "--json".into()],
        vec!["config".into(), "list".into(), "--json".into()],
        vec![
            "policy".into(),
            "inspect".into(),
            "fixtures/operational-policy/nitrosend-like.json".into(),
            "--json".into(),
        ],
        vec![
            "kernel".into(),
            "eval".into(),
            "--input".into(),
            "fixtures/kernel/policy/retry-admission-denies-mutating-without-key.json".into(),
            "--json".into(),
        ],
        vec!["doctor".into(), "--json".into()],
        vec!["list".into(), "tools".into(), "--json".into()],
        vec!["new".into(), "docs-demo".into(), "--json".into()],
        vec!["init".into(), "--json".into()],
        vec!["history".into(), "sourcey".into(), "--json".into()],
        vec!["mcp".into(), "serve".into(), "fixtures/skills/echo".into()],
        vec![
            "tool".into(),
            "search".into(),
            "echo".into(),
            "--json".into(),
        ],
    ] {
        assert_delegates_to_js_bin_without_native_signals(args);
    }
}

#[test]
fn rust_cli_signal_routes_mcp_serve_without_runner_to_native_lifecycle() {
    let action = plan_with_rust_cli(vec![
        "mcp".into(),
        "serve".into(),
        "fixtures/skills/echo".into(),
        "--receipt-dir=receipts".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::RunMcp(McpPlan {
            refs: vec![PathBuf::from("fixtures/skills/echo")],
            receipt_dir: Some(PathBuf::from("receipts")),
            runner: None,
        })
    );
}

#[test]
fn rust_cli_signal_routes_mcp_runner_selection_to_native_fail_closed_plan() {
    let action = plan_with_rust_cli_and_js(vec![
        "mcp".into(),
        "serve".into(),
        "fixtures/skills/echo".into(),
        "--runner".into(),
        "default".into(),
        "--receipt-dir=receipts".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::RunMcp(McpPlan {
            refs: vec![PathBuf::from("fixtures/skills/echo")],
            receipt_dir: Some(PathBuf::from("receipts")),
            runner: Some("default".to_owned()),
        })
    );
}

#[test]
fn rust_cli_signal_rejects_unknown_mcp_serve_flags_instead_of_delegating() {
    let action = plan_with_rust_cli_and_js(vec![
        "mcp".into(),
        "serve".into(),
        "fixtures/skills/echo".into(),
        "--legacy-js-only".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::Error("unknown mcp serve flag --legacy-js-only".to_owned())
    );
}

#[test]
fn rust_cli_signal_rejects_mcp_runner_before_serve_instead_of_delegating() {
    let action = plan_with_rust_cli_and_js(vec![
        "mcp".into(),
        "--runner=default".into(),
        "serve".into(),
        "fixtures/skills/echo".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::Error(
            "runx mcp --runner requires canonical form: runx mcp serve <skill-ref...> --runner <runner>"
                .to_owned(),
        )
    );
}

#[test]
fn native_mcp_runner_selection_fails_closed_without_js_fallback()
-> Result<(), Box<dyn std::error::Error>> {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_runx"))
        .args([
            "mcp",
            "serve",
            "fixtures/skills/mcp-echo",
            "--runner",
            "default",
        ])
        .env("RUNX_RUST_CLI", "1")
        .env("RUNX_JS_BIN", "/repo/oss/packages/cli/bin/runx.js")
        .env("RUNX_NPM_PACKAGE", "@runxhq/cli@0.5.22")
        .current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()?;

    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8(output.stderr)?
            .contains("runner selection 'default' is not supported by the native runtime yet")
    );
    Ok(())
}

#[test]
fn rust_cli_mcp_serve_requires_at_least_one_skill_ref() {
    assert_eq!(
        plan_with_rust_cli(vec!["mcp".into(), "serve".into()]),
        LauncherAction::Error("runx mcp serve requires at least one skill reference.".to_owned())
    );
}

#[test]
fn defaults_to_latest_npm_cli_package() {
    let action = plan_launcher(vec!["--help".into()], None, None);

    assert_eq!(
        action,
        LauncherAction::Delegate(CommandPlan {
            program: npm_command().into(),
            args: vec![
                "exec".into(),
                "--yes".into(),
                "--package".into(),
                DEFAULT_NPM_PACKAGE.into(),
                "--".into(),
                "runx".into(),
                "--help".into(),
            ],
        })
    );
}

#[test]
fn accepts_pinned_npm_package() {
    let action = plan_launcher(
        vec!["skill".into(), "sourcey".into()],
        Some("@runxhq/cli@0.5.22".into()),
        None,
    );

    assert_eq!(
        action,
        LauncherAction::Delegate(CommandPlan {
            program: npm_command().into(),
            args: vec![
                "exec".into(),
                "--yes".into(),
                "--package".into(),
                "@runxhq/cli@0.5.22".into(),
                "--".into(),
                "runx".into(),
                "skill".into(),
                "sourcey".into(),
            ],
        })
    );
}

#[test]
fn local_js_bin_overrides_npm_package() {
    let action = plan_launcher(
        vec!["--help".into()],
        Some("@runxhq/cli@0.5.22".into()),
        Some("/repo/oss/packages/cli/bin/runx.js".into()),
    );

    assert_eq!(
        action,
        LauncherAction::Delegate(CommandPlan {
            program: node_command().into(),
            args: vec!["/repo/oss/packages/cli/bin/runx.js".into(), "--help".into()],
        })
    );
}

#[test]
fn shim_flags_do_not_delegate() {
    assert_eq!(
        plan_launcher(vec!["--shim-version".into()], None, None),
        LauncherAction::PrintVersion
    );
    assert_eq!(
        plan_launcher(vec!["--shim-help".into()], None, None),
        LauncherAction::PrintHelp
    );
}

#[test]
fn rust_harness_signal_routes_harness_command_to_native_runner() {
    let action = plan_launcher_with_rust_harness(
        vec!["harness".into(), "fixtures/harness/echo-skill.yaml".into()],
        Some("@runxhq/cli@0.5.22".into()),
        Some("/repo/oss/packages/cli/bin/runx.js".into()),
        Some("1".into()),
    );

    assert_eq!(
        action,
        LauncherAction::RunHarness(HarnessPlan {
            fixture_path: "fixtures/harness/echo-skill.yaml".into(),
        })
    );
}

#[test]
fn rust_harness_signal_accepts_matrix_json_flag() {
    let action = plan_launcher_with_rust_harness(
        vec![
            "harness".into(),
            "fixtures/harness/echo-skill.yaml".into(),
            "--json".into(),
        ],
        Some("@runxhq/cli@0.5.22".into()),
        Some("/repo/oss/packages/cli/bin/runx.js".into()),
        Some("1".into()),
    );

    assert_eq!(
        action,
        LauncherAction::RunHarness(HarnessPlan {
            fixture_path: "fixtures/harness/echo-skill.yaml".into(),
        })
    );
}

#[test]
fn harness_without_rust_signal_still_delegates() {
    let action = plan_launcher(vec!["harness".into(), "fixture.yaml".into()], None, None);

    assert_eq!(
        action,
        LauncherAction::Delegate(CommandPlan {
            program: npm_command().into(),
            args: vec![
                "exec".into(),
                "--yes".into(),
                "--package".into(),
                DEFAULT_NPM_PACKAGE.into(),
                "--".into(),
                "runx".into(),
                "harness".into(),
                "fixture.yaml".into(),
            ],
        })
    );
}

#[test]
fn connect_delegates_without_rust_cli_signal_even_with_js_fallback_configured() {
    let action = plan_launcher(
        vec!["connect".into(), "list".into(), "--json".into()],
        Some("@runxhq/cli@0.5.22".into()),
        Some("/repo/oss/packages/cli/bin/runx.js".into()),
    );

    assert_eq!(
        action,
        LauncherAction::Delegate(CommandPlan {
            program: node_command().into(),
            args: vec![
                "/repo/oss/packages/cli/bin/runx.js".into(),
                "connect".into(),
                "list".into(),
                "--json".into(),
            ],
        })
    );
}

#[test]
fn rust_cli_zero_signal_still_delegates_to_js() {
    let action = plan_with_rust_cli_signal_and_js(
        "0",
        vec!["connect".into(), "list".into(), "--json".into()],
    );

    assert_eq!(
        action,
        LauncherAction::Delegate(CommandPlan {
            program: node_command().into(),
            args: vec![
                "/repo/oss/packages/cli/bin/runx.js".into(),
                "connect".into(),
                "list".into(),
                "--json".into(),
            ],
        })
    );
}

#[test]
fn rust_cli_empty_signal_still_delegates_to_js() {
    let action = plan_with_rust_cli_signal_and_js(
        "",
        vec!["connect".into(), "list".into(), "--json".into()],
    );

    assert_eq!(
        action,
        LauncherAction::Delegate(CommandPlan {
            program: node_command().into(),
            args: vec![
                "/repo/oss/packages/cli/bin/runx.js".into(),
                "connect".into(),
                "list".into(),
                "--json".into(),
            ],
        })
    );
}

#[test]
fn rust_harness_zero_and_empty_signals_still_delegate_to_js() {
    for signal in ["0", ""] {
        let action = plan_launcher_with_rust_harness(
            vec![
                "harness".into(),
                "fixtures/harness/echo-skill.yaml".into(),
                "--json".into(),
            ],
            Some("@runxhq/cli@0.5.22".into()),
            Some("/repo/oss/packages/cli/bin/runx.js".into()),
            Some(signal.into()),
        );

        assert_eq!(
            action,
            LauncherAction::Delegate(CommandPlan {
                program: node_command().into(),
                args: vec![
                    "/repo/oss/packages/cli/bin/runx.js".into(),
                    "harness".into(),
                    "fixtures/harness/echo-skill.yaml".into(),
                    "--json".into(),
                ],
            })
        );
    }
}

#[test]
fn rust_cli_signal_leaves_unknown_commands_delegated() {
    let action = plan_with_rust_cli_and_js(vec![
        "skill".into(),
        "run".into(),
        "sourcey".into(),
        "--json".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::Delegate(CommandPlan {
            program: node_command().into(),
            args: vec![
                "/repo/oss/packages/cli/bin/runx.js".into(),
                "skill".into(),
                "run".into(),
                "sourcey".into(),
                "--json".into(),
            ],
        })
    );
}

#[test]
fn rust_cli_signal_routes_connect_to_native_plan() {
    let action = plan_with_rust_cli(vec!["connect".into(), "list".into(), "--json".into()]);

    assert_eq!(
        action,
        LauncherAction::RunConnect(ConnectPlan {
            action: ConnectAction::List,
            provider: None,
            grant_id: None,
            scopes: Vec::new(),
            scope_family: None,
            authority_kind: None,
            target_repo: None,
            target_locator: None,
            json: true,
        })
    );
}

#[test]
fn connect_routes_provider_command_to_native_plan() {
    let action = plan_with_rust_cli_and_js(vec![
        "connect".into(),
        "github".into(),
        "--scope".into(),
        "repo:read,checks:read".into(),
        "--scope-family".into(),
        "github_repo".into(),
        "--authority-kind".into(),
        "read_only".into(),
        "--target-repo".into(),
        "runxhq/aster".into(),
        "--json".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::RunConnect(ConnectPlan {
            action: ConnectAction::Preprovision,
            provider: Some("github".to_owned()),
            grant_id: None,
            scopes: vec!["repo:read".to_owned(), "checks:read".to_owned()],
            scope_family: Some("github_repo".to_owned()),
            authority_kind: Some(ConnectAuthorityKind::ReadOnly),
            target_repo: Some("runxhq/aster".to_owned()),
            target_locator: None,
            json: true,
        })
    );
}

#[test]
fn connect_rejects_invalid_connect_shape() {
    let action = plan_with_rust_cli(vec!["connect".into(), "revoke".into()]);

    assert_eq!(
        action,
        LauncherAction::Error("runx connect revoke requires exactly one grant id".to_owned())
    );
}

#[test]
fn config_routes_to_native_cli_even_with_js_fallback_configured() {
    let action = plan_with_rust_cli_and_js(vec![
        "config".into(),
        "set".into(),
        "agent.model".into(),
        "gpt-test".into(),
        "--json".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::RunConfig(ConfigPlan {
            action: ConfigAction::Set,
            key: Some("agent.model".to_owned()),
            value: Some("gpt-test".to_owned()),
            json: true,
        })
    );
}

#[test]
fn config_delegates_without_rust_cli_signal() {
    let action = plan_launcher(
        vec!["config".into(), "list".into(), "--json".into()],
        Some("@runxhq/cli@0.5.22".into()),
        Some("/repo/oss/packages/cli/bin/runx.js".into()),
    );

    assert_eq!(
        action,
        LauncherAction::Delegate(CommandPlan {
            program: node_command().into(),
            args: vec![
                "/repo/oss/packages/cli/bin/runx.js".into(),
                "config".into(),
                "list".into(),
                "--json".into(),
            ],
        })
    );
}

#[test]
fn policy_routes_supported_shapes_to_native_cli() {
    let action = plan_with_rust_cli_and_js(vec![
        "policy".into(),
        "inspect".into(),
        "fixtures/operational-policy/nitrosend-like.json".into(),
        "--json".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::RunPolicy(PolicyPlan {
            action: PolicyAction::Inspect,
            path: PathBuf::from("fixtures/operational-policy/nitrosend-like.json"),
            json: true,
        })
    );
}

#[test]
fn policy_lint_routes_to_native_cli() {
    let action = plan_with_rust_cli(vec![
        "policy".into(),
        "lint".into(),
        "fixtures/operational-policy/nitrosend-like.json".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::RunPolicy(PolicyPlan {
            action: PolicyAction::Lint,
            path: PathBuf::from("fixtures/operational-policy/nitrosend-like.json"),
            json: false,
        })
    );
}

#[test]
fn policy_unsupported_subcommand_still_delegates() {
    let action = plan_with_rust_cli_and_js(vec!["policy".into(), "apply".into()]);

    assert_eq!(
        action,
        LauncherAction::Delegate(CommandPlan {
            program: node_command().into(),
            args: vec![
                "/repo/oss/packages/cli/bin/runx.js".into(),
                "policy".into(),
                "apply".into(),
            ],
        })
    );
}

#[test]
fn policy_rejects_invalid_native_shape() {
    let action = plan_with_rust_cli(vec!["policy".into(), "inspect".into(), "--json".into()]);

    assert_eq!(
        action,
        LauncherAction::Error(
            "runx policy inspect|lint requires exactly one policy path".to_owned()
        )
    );
}

#[test]
fn kernel_eval_routes_supported_shape_to_native_cli() {
    let action = plan_with_rust_cli_and_js(vec![
        "kernel".into(),
        "eval".into(),
        "--input".into(),
        "fixtures/kernel/policy/retry-admission-denies-mutating-without-key.json".into(),
        "--json".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::RunKernel(KernelPlan {
            input: KernelInputSource::Path(PathBuf::from(
                "fixtures/kernel/policy/retry-admission-denies-mutating-without-key.json",
            )),
            json: true,
        })
    );
}

#[test]
fn kernel_eval_accepts_stdin_input() {
    let action = plan_with_rust_cli(vec![
        "kernel".into(),
        "eval".into(),
        "--input=-".into(),
        "--json".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::RunKernel(KernelPlan {
            input: KernelInputSource::Stdin,
            json: true,
        })
    );
}

#[test]
fn kernel_unsupported_subcommand_still_delegates() {
    let action = plan_with_rust_cli_and_js(vec!["kernel".into(), "trace".into()]);

    assert_eq!(
        action,
        LauncherAction::Delegate(CommandPlan {
            program: node_command().into(),
            args: vec![
                "/repo/oss/packages/cli/bin/runx.js".into(),
                "kernel".into(),
                "trace".into(),
            ],
        })
    );
}

#[test]
fn kernel_eval_rejects_non_json_shape() {
    let action = plan_with_rust_cli(vec![
        "kernel".into(),
        "eval".into(),
        "--input".into(),
        "fixtures/kernel/state-machine/sequential-plan-first-step.json".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::Error("runx kernel eval requires --json".to_owned())
    );
}

#[test]
fn kernel_eval_rejects_missing_input() {
    let action = plan_with_rust_cli(vec!["kernel".into(), "eval".into(), "--json".into()]);

    assert_eq!(
        action,
        LauncherAction::Error("runx kernel eval requires --input <file|->".to_owned())
    );
}

#[test]
fn doctor_routes_to_native_cli_even_with_js_fallback_configured() {
    let action = plan_with_rust_cli_and_js(vec![
        "doctor".into(),
        "fixtures/doctor/empty-success/workspace".into(),
        "--json".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::RunDoctor(DoctorPlan {
            path: Some(PathBuf::from("fixtures/doctor/empty-success/workspace")),
            json: true,
        })
    );
}

#[test]
fn doctor_repair_semantics_still_delegate_to_js() {
    let action = plan_with_rust_cli_and_js(vec!["doctor".into(), "--fix".into()]);

    assert_eq!(
        action,
        LauncherAction::Delegate(CommandPlan {
            program: node_command().into(),
            args: vec![
                "/repo/oss/packages/cli/bin/runx.js".into(),
                "doctor".into(),
                "--fix".into(),
            ],
        })
    );
}

#[test]
fn history_routes_to_native_cli_even_with_js_fallback_configured() {
    let action = plan_with_rust_cli_and_js(vec![
        "history".into(),
        "sourcey".into(),
        "--json".into(),
        "--receipt-dir".into(),
        ".runx/receipts".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::RunHistory(HistoryPlan {
            args: vec![
                "history".into(),
                "sourcey".into(),
                "--json".into(),
                "--receipt-dir".into(),
                ".runx/receipts".into(),
            ],
        })
    );
}

#[test]
fn list_routes_native_supported_shape_even_with_js_fallback_configured() {
    let action = plan_with_rust_cli_and_js(vec![
        "list".into(),
        "packets".into(),
        "--ok-only".into(),
        "--json".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::RunList(ListPlan {
            kind: ListKind::Packets,
            ok_only: true,
            invalid_only: false,
            json: true,
        })
    );
}

#[test]
fn list_delegates_unsupported_shape() {
    let action = plan_with_rust_cli_and_js(vec![
        "list".into(),
        "skills".into(),
        "--source".into(),
        "registry".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::Delegate(CommandPlan {
            program: node_command().into(),
            args: vec![
                "/repo/oss/packages/cli/bin/runx.js".into(),
                "list".into(),
                "skills".into(),
                "--source".into(),
                "registry".into(),
            ],
        })
    );
}

#[test]
fn list_rejects_conflicting_status_filters() {
    let action = plan_with_rust_cli(vec![
        "list".into(),
        "--ok-only".into(),
        "--invalid-only".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::Error("runx list accepts either --ok-only or --invalid-only".to_owned())
    );
}

#[test]
fn registry_search_routes_to_native_cli_even_with_js_fallback_configured() {
    let action = plan_with_rust_cli_and_js(vec![
        "registry".into(),
        "search".into(),
        "echo".into(),
        "--registry-dir".into(),
        "/tmp/runx-registry".into(),
        "--limit".into(),
        "10".into(),
        "--json".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::RunRegistry(RegistryPlan {
            action: RegistryAction::Search,
            subject: "echo".to_owned(),
            registry: None,
            registry_dir: Some(PathBuf::from("/tmp/runx-registry")),
            version: None,
            expected_digest: None,
            destination: None,
            installation_id: None,
            owner: None,
            profile: None,
            limit: Some(10),
            upsert: false,
            json: true,
        })
    );
}

#[test]
fn registry_install_requires_one_ref() {
    let action = plan_with_rust_cli(vec!["registry".into(), "install".into()]);

    assert_eq!(
        action,
        LauncherAction::Error("runx registry install requires exactly one ref".to_owned())
    );
}

#[test]
fn new_routes_to_native_cli_even_with_js_fallback_configured() {
    let action = plan_with_rust_cli_and_js(vec![
        "new".into(),
        "docs-demo".into(),
        "--directory".into(),
        "tmp/docs-demo".into(),
        "--json".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::RunNew(NewPlan {
            name: "docs-demo".to_owned(),
            directory: Some(PathBuf::from("tmp/docs-demo")),
            json: true,
        })
    );
}

#[test]
fn new_accepts_positional_directory() {
    let action = plan_with_rust_cli(vec!["new".into(), "docs-demo".into(), "out".into()]);

    assert_eq!(
        action,
        LauncherAction::RunNew(NewPlan {
            name: "docs-demo".to_owned(),
            directory: Some(PathBuf::from("out")),
            json: false,
        })
    );
}

#[test]
fn init_routes_to_native_cli_even_with_js_fallback_configured() {
    let action = plan_with_rust_cli_and_js(vec![
        "init".into(),
        "-g".into(),
        "--prefetch".into(),
        "official".into(),
        "--json".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::RunInit(InitPlan {
            global: true,
            prefetch_official: true,
            json: true,
        })
    );
}

#[test]
fn tool_build_routes_to_native_cli_even_with_js_fallback_configured() {
    let action = plan_with_rust_cli_and_js(vec![
        "tool".into(),
        "build".into(),
        "tools/docs/echo".into(),
        "--json".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::RunTool(ToolPlan {
            action: ToolAction::Build,
            path: Some(PathBuf::from("tools/docs/echo")),
            ref_or_query: None,
            all: false,
            source: None,
            json: true,
        })
    );
}

#[test]
fn tool_build_all_routes_to_native_cli() {
    let action = plan_with_rust_cli(vec!["tool".into(), "build".into(), "--all".into()]);

    assert_eq!(
        action,
        LauncherAction::RunTool(ToolPlan {
            action: ToolAction::Build,
            path: None,
            ref_or_query: None,
            all: true,
            source: None,
            json: false,
        })
    );
}

#[test]
fn tool_search_routes_to_native_cli_even_with_js_fallback_configured() {
    let action = plan_with_rust_cli_and_js(vec![
        "tool".into(),
        "search".into(),
        "echo".into(),
        "writer".into(),
        "--source".into(),
        "fixture-mcp".into(),
        "--json".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::RunTool(ToolPlan {
            action: ToolAction::Search,
            path: None,
            ref_or_query: Some("echo writer".to_owned()),
            all: false,
            source: Some("fixture-mcp".to_owned()),
            json: true,
        })
    );
}

#[test]
fn tool_inspect_routes_to_native_cli_even_with_js_fallback_configured() {
    let action = plan_with_rust_cli_and_js(vec![
        "tool".into(),
        "inspect".into(),
        "fixture.echo".into(),
        "--source=fixture-mcp".into(),
    ]);

    assert_eq!(
        action,
        LauncherAction::RunTool(ToolPlan {
            action: ToolAction::Inspect,
            path: None,
            ref_or_query: Some("fixture.echo".to_owned()),
            all: false,
            source: Some("fixture-mcp".to_owned()),
            json: false,
        })
    );
}

#[test]
fn tool_unknown_subcommand_still_delegates() {
    let action = plan_launcher(
        vec!["tool".into(), "publish".into(), "fixture.echo".into()],
        None,
        None,
    );

    assert_eq!(
        action,
        LauncherAction::Delegate(CommandPlan {
            program: npm_command().into(),
            args: vec![
                "exec".into(),
                "--yes".into(),
                "--package".into(),
                DEFAULT_NPM_PACKAGE.into(),
                "--".into(),
                "runx".into(),
                "tool".into(),
                "publish".into(),
                "fixture.echo".into(),
            ],
        })
    );
}

#[test]
fn init_accepts_truthy_prefetch_forms() {
    assert_eq!(
        plan_with_rust_cli(vec!["init".into(), "--prefetch".into()]),
        LauncherAction::RunInit(InitPlan {
            global: false,
            prefetch_official: true,
            json: false,
        })
    );
    assert_eq!(
        plan_with_rust_cli(vec!["init".into(), "--prefetchOfficial".into()]),
        LauncherAction::RunInit(InitPlan {
            global: false,
            prefetch_official: true,
            json: false,
        })
    );
}

#[test]
fn rust_harness_signal_rejects_unsupported_argument_shape() {
    let action =
        plan_launcher_with_rust_harness(vec!["harness".into()], None, None, Some("1".into()));

    assert_eq!(
        action,
        LauncherAction::Error(
            "runx harness requires exactly one fixture path when RUNX_RUST_HARNESS is set"
                .to_owned(),
        )
    );
}

#[test]
fn native_launcher_argument_errors_exit_with_usage_code() -> Result<(), Box<dyn std::error::Error>>
{
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_runx"))
        .args(["policy", "inspect", "--json"])
        .env("RUNX_RUST_CLI", "1")
        .env_remove("RUNX_JS_BIN")
        .env_remove("RUNX_NPM_PACKAGE")
        .output()?;

    assert_eq!(output.status.code(), Some(64));
    assert!(
        String::from_utf8(output.stderr)?
            .contains("runx policy inspect|lint requires exactly one policy path")
    );
    Ok(())
}

#[test]
fn packaged_node_cli_does_not_enable_rust_candidate_dispatch()
-> Result<(), Box<dyn std::error::Error>> {
    let package_json = fs::read_to_string(repo_root()?.join("packages/cli/package.json"))?;
    let node_bin = fs::read_to_string(repo_root()?.join("packages/cli/bin/runx.js"))?;

    assert_not_contains(&package_json, "RUNX_RUST_CLI");
    assert_not_contains(&package_json, "RUNX_RUST_HARNESS");
    assert_not_contains(&package_json, "crates/runx-cli");
    assert_not_contains(&node_bin, "RUNX_RUST_CLI");
    assert_not_contains(&node_bin, "RUNX_RUST_HARNESS");
    assert_not_contains(&node_bin, "CARGO_BIN_EXE_runx");
    Ok(())
}

fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?)
}

fn assert_not_contains(contents: &str, needle: &str) {
    assert!(
        !contents.contains(needle),
        "packaged CLI must not contain {needle}"
    );
}
