// rust-style-allow: large-file - launcher argument parity is centralized for CLI routing tests.
use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

use crate::config::ConfigPlan;
use crate::connect::ConnectPlan;
use crate::kernel::{KernelInputSource, KernelPlan};
use crate::mcp::McpPlan;
use crate::policy::{PolicyAction, PolicyPlan};
use crate::registry::{RegistryAction, RegistryPlan};

pub const DEFAULT_NPM_PACKAGE: &str = "@runxhq/cli@latest";

#[derive(Debug, Eq, PartialEq)]
pub enum LauncherAction {
    Delegate(CommandPlan),
    Error(String),
    RunDoctor(DoctorPlan),
    RunInit(InitPlan),
    RunList(ListPlan),
    RunMcp(McpPlan),
    RunNew(NewPlan),
    RunHistory(HistoryPlan),
    RunHarness(HarnessPlan),
    RunKernel(KernelPlan),
    RunConnect(ConnectPlan),
    RunConfig(ConfigPlan),
    RunPolicy(PolicyPlan),
    RunRegistry(RegistryPlan),
    RunTool(ToolPlan),
    PrintHelp,
    PrintVersion,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CommandPlan {
    pub program: OsString,
    pub args: Vec<OsString>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct HarnessPlan {
    pub fixture_path: OsString,
}

#[derive(Debug, Eq, PartialEq)]
pub struct HistoryPlan {
    pub args: Vec<OsString>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct DoctorPlan {
    pub path: Option<PathBuf>,
    pub json: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ListPlan {
    pub kind: ListKind,
    pub ok_only: bool,
    pub invalid_only: bool,
    pub json: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ListKind {
    All,
    Tools,
    Skills,
    Graphs,
    Packets,
    Overlays,
}

#[derive(Debug, Eq, PartialEq)]
pub struct NewPlan {
    pub name: String,
    pub directory: Option<PathBuf>,
    pub json: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub struct InitPlan {
    pub global: bool,
    pub prefetch_official: bool,
    pub json: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ToolPlan {
    pub action: ToolAction,
    pub path: Option<PathBuf>,
    pub ref_or_query: Option<String>,
    pub all: bool,
    pub source: Option<String>,
    pub json: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ToolAction {
    Build,
    Search,
    Inspect,
}

pub fn plan_launcher(
    args: Vec<OsString>,
    npm_package: Option<OsString>,
    js_bin: Option<OsString>,
) -> LauncherAction {
    plan_launcher_with_native_options(args, npm_package, js_bin, NativeLauncherOptions::default())
}

pub fn plan_launcher_with_rust_harness(
    args: Vec<OsString>,
    npm_package: Option<OsString>,
    js_bin: Option<OsString>,
    rust_harness: Option<OsString>,
) -> LauncherAction {
    plan_launcher_with_native_options(
        args,
        npm_package,
        js_bin,
        NativeLauncherOptions {
            rust_cli: None,
            rust_harness,
        },
    )
}

#[derive(Default)]
pub struct NativeLauncherOptions {
    pub rust_cli: Option<OsString>,
    pub rust_harness: Option<OsString>,
}

// rust-style-allow: long-function because launcher routing is the cutover gate:
// every native command branch and fallback delegation decision is reviewed here.
pub fn plan_launcher_with_native_options(
    args: Vec<OsString>,
    npm_package: Option<OsString>,
    js_bin: Option<OsString>,
    native: NativeLauncherOptions,
) -> LauncherAction {
    if has_arg(&args, "--shim-version") {
        return LauncherAction::PrintVersion;
    }

    if has_arg(&args, "--shim-help") {
        return LauncherAction::PrintHelp;
    }

    if native_signal_requested(native.rust_harness.as_deref()) && first_arg_is(&args, "harness") {
        return native_harness_plan(&args);
    }

    if native_signal_requested(native.rust_cli.as_deref()) {
        if first_arg_is(&args, "connect") {
            return crate::connect::parse_connect_plan(&args)
                .map_or_else(LauncherAction::Error, LauncherAction::RunConnect);
        }

        if first_arg_is(&args, "config") {
            return crate::config::parse_config_plan(&args)
                .map_or_else(LauncherAction::Error, LauncherAction::RunConfig);
        }

        if first_arg_is(&args, "policy") && policy_subcommand_is_native(&args) {
            return parse_policy_plan(&args)
                .map_or_else(LauncherAction::Error, LauncherAction::RunPolicy);
        }

        if first_arg_is(&args, "kernel") && kernel_subcommand_is_native(&args) {
            return parse_kernel_plan(&args)
                .map_or_else(LauncherAction::Error, LauncherAction::RunKernel);
        }

        if first_arg_is(&args, "doctor") {
            if doctor_deferred_to_js(&args) {
                return delegate_plan(args, npm_package, js_bin);
            }
            return parse_doctor_plan(&args)
                .map_or_else(LauncherAction::Error, LauncherAction::RunDoctor);
        }

        if first_arg_is(&args, "list") {
            if !list_shape_is_native(&args) {
                return delegate_plan(args, npm_package, js_bin);
            }
            return parse_list_plan(&args)
                .map_or_else(LauncherAction::Error, LauncherAction::RunList);
        }

        if first_arg_is(&args, "new") {
            return parse_new_plan(&args)
                .map_or_else(LauncherAction::Error, LauncherAction::RunNew);
        }

        if first_arg_is(&args, "init") {
            return parse_init_plan(&args)
                .map_or_else(LauncherAction::Error, LauncherAction::RunInit);
        }

        if first_arg_is(&args, "history") {
            return LauncherAction::RunHistory(HistoryPlan { args });
        }

        if first_arg_is(&args, "mcp") {
            if mcp_subcommand_is_native(&args) {
                return crate::mcp::parse_mcp_plan(&args)
                    .map_or_else(LauncherAction::Error, LauncherAction::RunMcp);
            }
            if mcp_args_request_runner_selection(&args) {
                return LauncherAction::Error(
                    "runx mcp --runner requires canonical form: runx mcp serve <skill-ref...> --runner <runner>"
                        .to_owned(),
                );
            }
        }

        if first_arg_is(&args, "tool") && tool_subcommand_is_native(&args) {
            return parse_tool_plan(&args)
                .map_or_else(LauncherAction::Error, LauncherAction::RunTool);
        }

        if first_arg_is(&args, "registry") && registry_subcommand_is_native(&args) {
            return parse_registry_plan(&args)
                .map_or_else(LauncherAction::Error, LauncherAction::RunRegistry);
        }
    }

    delegate_plan(args, npm_package, js_bin)
}

fn delegate_plan(
    args: Vec<OsString>,
    npm_package: Option<OsString>,
    js_bin: Option<OsString>,
) -> LauncherAction {
    if let Some(js_bin) = non_empty_os(js_bin) {
        let mut planned_args = Vec::with_capacity(args.len() + 1);
        planned_args.push(js_bin);
        planned_args.extend(args);
        return LauncherAction::Delegate(CommandPlan {
            program: node_command().into(),
            args: planned_args,
        });
    }

    let package = non_empty_os(npm_package).unwrap_or_else(|| DEFAULT_NPM_PACKAGE.into());
    let mut planned_args = vec![
        "exec".into(),
        "--yes".into(),
        "--package".into(),
        package,
        "--".into(),
        "runx".into(),
    ];
    planned_args.extend(args);

    LauncherAction::Delegate(CommandPlan {
        program: npm_command().into(),
        args: planned_args,
    })
}

pub fn shim_help() -> String {
    format!(
        "\
runx Cargo launcher

Usage:
  runx [runx CLI args]
  runx --shim-version
  runx --shim-help

Environment:
  RUNX_NPM_PACKAGE  npm package spec to execute, defaults to {DEFAULT_NPM_PACKAGE}
  RUNX_JS_BIN       local JavaScript runx entrypoint to execute with node
  RUNX_RUST_CLI     opt into native Rust candidate commands before the CLI cutover
  RUNX_RUST_HARNESS opt into native Rust `runx harness <fixture>` replay

Native commands:
  runx new <name> [--directory dir] [--json]
  runx init [-g|--global] [--prefetch official] [--json]
  runx history [query] [--skill s] [--status s] [--source s] [--actor a] [--artifact-type t] [--since iso] [--until iso] [--receipt-dir dir] [--json]
  runx list [tools|skills|graphs|packets|overlays] [--ok-only|--invalid-only] [--json]
  runx connect list|revoke <grant-id>|<provider> [--scope scope] [--scope-family family] [--authority-kind read_only|constructive|destructive] [--target-repo owner/repo] [--target-locator locator] [--json]
  runx config set|get|list [agent.provider|agent.model|agent.api_key] [value] [--json]
  runx policy inspect|lint <policy.json> [--json]
  runx kernel eval --input <file|-> --json
  runx doctor [path] [--json]
  runx mcp serve <skill-ref...> [--receipt-dir dir]
  runx tool build <tool-dir>|--all [--json]
  runx tool search <query> [--source source] [--json]
  runx tool inspect <ref> [--source source] [--json]
  runx registry search|read|resolve|install|publish ... --json
"
    )
}

pub fn npm_command() -> &'static str {
    if cfg!(windows) { "npm.cmd" } else { "npm" }
}

pub fn node_command() -> &'static str {
    if cfg!(windows) { "node.exe" } else { "node" }
}

fn has_arg(args: &[OsString], expected: &str) -> bool {
    args.iter().any(|arg| arg == OsStr::new(expected))
}

fn first_arg_is(args: &[OsString], expected: &str) -> bool {
    args.first().is_some_and(|arg| arg == OsStr::new(expected))
}

fn non_empty_os(value: Option<OsString>) -> Option<OsString> {
    value.filter(|value| !value.is_empty())
}

fn native_signal_requested(value: Option<&OsStr>) -> bool {
    value.is_some_and(|value| !value.is_empty() && value != OsStr::new("0"))
}

fn native_harness_plan(args: &[OsString]) -> LauncherAction {
    let mut fixture_path = None;
    let mut index = 1;

    while index < args.len() {
        let Some(token) = args.get(index).and_then(|arg| arg.to_str()) else {
            return LauncherAction::Error("harness arguments must be UTF-8".to_owned());
        };

        if !token.starts_with("--") {
            if fixture_path.is_some() {
                return LauncherAction::Error(
                    "runx harness requires exactly one fixture path when RUNX_RUST_HARNESS is set"
                        .to_owned(),
                );
            }
            fixture_path = Some(args[index].clone());
            index += 1;
            continue;
        }

        let (flag, inline_value) = split_flag(token);
        match flag {
            "--json" => {
                if inline_value.is_some() {
                    return LauncherAction::Error("--json does not take a value".to_owned());
                }
                index += 1;
            }
            _ => return LauncherAction::Error(format!("unknown harness flag {flag}")),
        }
    }

    let Some(fixture_path) = fixture_path else {
        return LauncherAction::Error(
            "runx harness requires exactly one fixture path when RUNX_RUST_HARNESS is set"
                .to_owned(),
        );
    };

    LauncherAction::RunHarness(HarnessPlan { fixture_path })
}

fn parse_new_plan(args: &[OsString]) -> Result<NewPlan, String> {
    let mut name = None;
    let mut directory = None;
    let mut json = false;
    let mut positional_directory = None;
    let mut extra_positionals = Vec::new();
    let mut index = 1;

    while index < args.len() {
        let token = os_arg(args, index, "new")?;
        if !token.starts_with("--") {
            if name.is_none() {
                name = Some(token.to_owned());
            } else if positional_directory.is_none() {
                positional_directory = Some(PathBuf::from(token));
            } else {
                extra_positionals.push(token.to_owned());
            }
            index += 1;
            continue;
        }

        let (flag, inline_value) = split_flag(token);
        match flag {
            "--json" => {
                if inline_value.is_some() {
                    return Err("--json does not take a value".to_owned());
                }
                json = true;
                index += 1;
            }
            "--directory" | "--dir" => {
                let (value, next_index) = flag_value(args, index, flag, inline_value, "new")?;
                directory = Some(PathBuf::from(value));
                index = next_index;
            }
            _ => return Err(format!("unknown new flag {flag}")),
        }
    }

    if !extra_positionals.is_empty() {
        return Err("runx new accepts at most one directory argument".to_owned());
    }

    Ok(NewPlan {
        name: name.ok_or_else(|| "runx new requires a package name".to_owned())?,
        directory: directory.or(positional_directory),
        json,
    })
}

fn parse_init_plan(args: &[OsString]) -> Result<InitPlan, String> {
    let mut global = false;
    let mut prefetch_official = false;
    let mut json = false;
    let mut index = 1;

    while index < args.len() {
        let token = os_arg(args, index, "init")?;
        if token == "-g" {
            global = true;
            index += 1;
            continue;
        }
        if !token.starts_with("--") {
            return Err(format!("unexpected init argument {token}"));
        }

        let (flag, inline_value) = split_flag(token);
        match flag {
            "--json" => {
                if inline_value.is_some() {
                    return Err("--json does not take a value".to_owned());
                }
                json = true;
                index += 1;
            }
            "--global" => {
                if inline_value.is_some() {
                    return Err("--global does not take a value".to_owned());
                }
                global = true;
                index += 1;
            }
            "--prefetch" | "--prefetchOfficial" | "--prefetch-official" => {
                if matches!(inline_value, Some("false") | Some("0")) {
                    prefetch_official = false;
                    index += 1;
                    continue;
                }
                let (value, next_index) = optional_flag_value(args, index, inline_value, "init")?;
                prefetch_official = value.is_none_or(|value| truthy(&value));
                index = next_index;
            }
            _ => return Err(format!("unknown init flag {flag}")),
        }
    }

    Ok(InitPlan {
        global,
        prefetch_official,
        json,
    })
}

fn doctor_deferred_to_js(args: &[OsString]) -> bool {
    args.iter().skip(1).any(|arg| {
        let Some(token) = arg.to_str() else {
            return false;
        };
        let (flag, _inline_value) = split_flag(token);
        matches!(
            flag,
            "--fix" | "--explain" | "--listDiagnostics" | "--list-diagnostics"
        )
    })
}

fn parse_doctor_plan(args: &[OsString]) -> Result<DoctorPlan, String> {
    let mut path = None;
    let mut json = false;
    let mut index = 1;

    while index < args.len() {
        let token = os_arg(args, index, "doctor")?;
        if !token.starts_with("--") {
            if path.is_some() {
                return Err("runx doctor accepts at most one path".to_owned());
            }
            path = Some(PathBuf::from(token));
            index += 1;
            continue;
        }

        let (flag, inline_value) = split_flag(token);
        match flag {
            "--json" => {
                if inline_value.is_some() {
                    return Err("--json does not take a value".to_owned());
                }
                json = true;
                index += 1;
            }
            _ => return Err(format!("unknown doctor flag {flag}")),
        }
    }

    Ok(DoctorPlan { path, json })
}

fn list_shape_is_native(args: &[OsString]) -> bool {
    let mut positionals = 0;
    for arg in args.iter().skip(1) {
        let Some(token) = arg.to_str() else {
            return true;
        };
        if !token.starts_with("--") {
            positionals += 1;
            if positionals > 1 || parse_list_kind(token).is_none() {
                return false;
            }
            continue;
        }

        let (flag, inline_value) = split_flag(token);
        if inline_value.is_some() {
            return false;
        }
        if !matches!(
            flag,
            "--json" | "--ok-only" | "--okOnly" | "--invalid-only" | "--invalidOnly"
        ) {
            return false;
        }
    }
    true
}

fn parse_list_plan(args: &[OsString]) -> Result<ListPlan, String> {
    let mut kind = ListKind::All;
    let mut ok_only = false;
    let mut invalid_only = false;
    let mut json = false;
    let mut saw_kind = false;
    let mut index = 1;

    while index < args.len() {
        let token = os_arg(args, index, "list")?;
        if !token.starts_with("--") {
            if saw_kind {
                return Err("runx list accepts at most one kind".to_owned());
            }
            kind = parse_list_kind(token).ok_or_else(|| {
                "runx list kind must be tools, skills, graphs, packets, or overlays".to_owned()
            })?;
            saw_kind = true;
            index += 1;
            continue;
        }

        let (flag, inline_value) = split_flag(token);
        if inline_value.is_some() {
            return Err(format!("{flag} does not take a value"));
        }
        match flag {
            "--json" => json = true,
            "--ok-only" | "--okOnly" => ok_only = true,
            "--invalid-only" | "--invalidOnly" => invalid_only = true,
            _ => return Err(format!("unknown list flag {flag}")),
        }
        index += 1;
    }

    if ok_only && invalid_only {
        return Err("runx list accepts either --ok-only or --invalid-only".to_owned());
    }

    Ok(ListPlan {
        kind,
        ok_only,
        invalid_only,
        json,
    })
}

fn parse_list_kind(value: &str) -> Option<ListKind> {
    match value {
        "tools" => Some(ListKind::Tools),
        "skills" => Some(ListKind::Skills),
        "graphs" => Some(ListKind::Graphs),
        "packets" => Some(ListKind::Packets),
        "overlays" => Some(ListKind::Overlays),
        _ => None,
    }
}

fn tool_subcommand_is_native(args: &[OsString]) -> bool {
    matches!(
        args.get(1).and_then(|arg| arg.to_str()),
        Some("build" | "search" | "inspect")
    )
}

fn mcp_subcommand_is_native(args: &[OsString]) -> bool {
    matches!(args.get(1).and_then(|arg| arg.to_str()), Some("serve"))
}

fn mcp_args_request_runner_selection(args: &[OsString]) -> bool {
    args.iter().skip(1).any(|arg| {
        arg.to_str()
            .map(|token| {
                let (flag, _inline_value) = split_flag(token);
                flag == "--runner"
            })
            .unwrap_or(false)
    })
}

fn policy_subcommand_is_native(args: &[OsString]) -> bool {
    matches!(
        args.get(1).and_then(|arg| arg.to_str()),
        Some("inspect" | "lint")
    )
}

fn kernel_subcommand_is_native(args: &[OsString]) -> bool {
    matches!(args.get(1).and_then(|arg| arg.to_str()), Some("eval"))
}

fn registry_subcommand_is_native(args: &[OsString]) -> bool {
    matches!(
        args.get(1).and_then(|arg| arg.to_str()),
        Some("search" | "read" | "resolve" | "install" | "publish")
    )
}

fn parse_kernel_plan(args: &[OsString]) -> Result<KernelPlan, String> {
    let subcommand = os_arg(args, 1, "kernel")?;
    if subcommand != "eval" {
        return Err(format!("unknown kernel subcommand {subcommand}"));
    }

    let mut input = None;
    let mut json = false;
    let mut index = 2;

    while index < args.len() {
        let token = os_arg(args, index, "kernel")?;
        if !token.starts_with("--") {
            return Err(format!("unexpected kernel eval argument {token}"));
        }

        let (flag, inline_value) = split_flag(token);
        match flag {
            "--json" => {
                if inline_value.is_some() {
                    return Err("--json does not take a value".to_owned());
                }
                json = true;
                index += 1;
            }
            "--input" => {
                if input.is_some() {
                    return Err("runx kernel eval accepts exactly one --input".to_owned());
                }
                let (value, next_index) = flag_value(args, index, flag, inline_value, "kernel")?;
                input = Some(if value == "-" {
                    KernelInputSource::Stdin
                } else {
                    KernelInputSource::Path(PathBuf::from(value))
                });
                index = next_index;
            }
            _ => return Err(format!("unknown kernel eval flag {flag}")),
        }
    }

    if !json {
        return Err("runx kernel eval requires --json".to_owned());
    }

    Ok(KernelPlan {
        input: input.ok_or_else(|| "runx kernel eval requires --input <file|->".to_owned())?,
        json,
    })
}

fn parse_policy_plan(args: &[OsString]) -> Result<PolicyPlan, String> {
    let subcommand = os_arg(args, 1, "policy")?;
    let action = match subcommand {
        "inspect" => PolicyAction::Inspect,
        "lint" => PolicyAction::Lint,
        _ => return Err(format!("unknown policy subcommand {subcommand}")),
    };
    let mut json = false;
    let mut positionals = Vec::new();
    let mut index = 2;

    while index < args.len() {
        let token = os_arg(args, index, "policy")?;
        if !token.starts_with("--") {
            positionals.push(PathBuf::from(token));
            index += 1;
            continue;
        }

        let (flag, inline_value) = split_flag(token);
        match flag {
            "--json" => {
                if inline_value.is_some() {
                    return Err("--json does not take a value".to_owned());
                }
                json = true;
                index += 1;
            }
            _ => return Err(format!("unknown policy flag {flag}")),
        }
    }

    let [path] = positionals.as_slice() else {
        return Err("runx policy inspect|lint requires exactly one policy path".to_owned());
    };
    Ok(PolicyPlan {
        action,
        path: path.clone(),
        json,
    })
}

fn parse_tool_plan(args: &[OsString]) -> Result<ToolPlan, String> {
    let subcommand = os_arg(args, 1, "tool")?;
    let action = match subcommand {
        "build" => ToolAction::Build,
        "search" => ToolAction::Search,
        "inspect" => ToolAction::Inspect,
        _ => return Err(format!("unknown tool subcommand {subcommand}")),
    };
    let mut json = false;
    let mut all = false;
    let mut source = None;
    let mut positionals = Vec::new();
    let mut index = 2;

    while index < args.len() {
        let token = os_arg(args, index, "tool")?;
        if !token.starts_with("--") {
            positionals.push(token.to_owned());
            index += 1;
            continue;
        }

        let (flag, inline_value) = split_flag(token);
        match flag {
            "--json" => {
                if inline_value.is_some() {
                    return Err("--json does not take a value".to_owned());
                }
                json = true;
                index += 1;
            }
            "--all" => {
                if inline_value.is_some() {
                    return Err("--all does not take a value".to_owned());
                }
                all = true;
                index += 1;
            }
            "--source" => {
                let (value, next_index) = flag_value(args, index, flag, inline_value, "tool")?;
                source = Some(value);
                index = next_index;
            }
            _ => return Err(format!("unknown tool flag {flag}")),
        }
    }

    match action {
        ToolAction::Build => build_tool_plan(positionals, all, source, json),
        ToolAction::Search => search_tool_plan(positionals, all, source, json),
        ToolAction::Inspect => inspect_tool_plan(positionals, all, source, json),
    }
}

fn build_tool_plan(
    positionals: Vec<String>,
    all: bool,
    source: Option<String>,
    json: bool,
) -> Result<ToolPlan, String> {
    if source.is_some() {
        return Err("runx tool build does not accept --source".to_owned());
    }
    if all && !positionals.is_empty() {
        return Err("runx tool build accepts either --all or one tool directory".to_owned());
    }
    if !all && positionals.len() != 1 {
        return Err("runx tool build requires a tool directory or --all".to_owned());
    }

    Ok(ToolPlan {
        action: ToolAction::Build,
        path: positionals.first().map(PathBuf::from),
        ref_or_query: None,
        all,
        source: None,
        json,
    })
}

fn search_tool_plan(
    positionals: Vec<String>,
    all: bool,
    source: Option<String>,
    json: bool,
) -> Result<ToolPlan, String> {
    if all {
        return Err("runx tool search does not accept --all".to_owned());
    }
    let query = positionals.join(" ");
    if query.is_empty() {
        return Err("runx tool search requires a query".to_owned());
    }

    Ok(ToolPlan {
        action: ToolAction::Search,
        path: None,
        ref_or_query: Some(query),
        all: false,
        source,
        json,
    })
}

fn inspect_tool_plan(
    positionals: Vec<String>,
    all: bool,
    source: Option<String>,
    json: bool,
) -> Result<ToolPlan, String> {
    if all {
        return Err("runx tool inspect does not accept --all".to_owned());
    }
    let tool_ref = positionals.join(" ");
    if tool_ref.is_empty() {
        return Err("runx tool inspect requires a tool reference".to_owned());
    }

    Ok(ToolPlan {
        action: ToolAction::Inspect,
        path: None,
        ref_or_query: Some(tool_ref),
        all: false,
        source,
        json,
    })
}

fn parse_registry_plan(args: &[OsString]) -> Result<RegistryPlan, String> {
    let subcommand = os_arg(args, 1, "registry")?;
    let action = parse_registry_action(subcommand)?;
    let mut state = RegistryParseState::default();
    parse_registry_args(args, &mut state)?;
    let subject = registry_subject(&action, subcommand, &mut state.positionals)?;

    Ok(RegistryPlan {
        action,
        subject,
        registry: state.registry,
        registry_dir: state.registry_dir,
        version: state.version,
        expected_digest: state.expected_digest,
        destination: state.destination,
        installation_id: state.installation_id,
        owner: state.owner,
        profile: state.profile,
        limit: state.limit,
        upsert: state.upsert,
        json: state.json,
    })
}

#[derive(Default)]
struct RegistryParseState {
    json: bool,
    upsert: bool,
    registry: Option<String>,
    registry_dir: Option<PathBuf>,
    version: Option<String>,
    expected_digest: Option<String>,
    destination: Option<PathBuf>,
    installation_id: Option<String>,
    owner: Option<String>,
    profile: Option<PathBuf>,
    limit: Option<usize>,
    positionals: Vec<String>,
}

fn parse_registry_action(subcommand: &str) -> Result<RegistryAction, String> {
    match subcommand {
        "search" => Ok(RegistryAction::Search),
        "read" => Ok(RegistryAction::Read),
        "resolve" => Ok(RegistryAction::Resolve),
        "install" => Ok(RegistryAction::Install),
        "publish" => Ok(RegistryAction::Publish),
        _ => Err(format!("unknown registry subcommand {subcommand}")),
    }
}

fn parse_registry_args(args: &[OsString], state: &mut RegistryParseState) -> Result<(), String> {
    let mut index = 2;
    while index < args.len() {
        let token = os_arg(args, index, "registry")?;
        if !token.starts_with("--") {
            state.positionals.push(token.to_owned());
            index += 1;
            continue;
        }

        let (flag, inline_value) = split_flag(token);
        index = parse_registry_flag(args, index, flag, inline_value, state)?;
    }
    Ok(())
}

fn parse_registry_flag(
    args: &[OsString],
    index: usize,
    flag: &str,
    inline_value: Option<&str>,
    state: &mut RegistryParseState,
) -> Result<usize, String> {
    match flag {
        "--json" => set_registry_bool_flag(flag, inline_value, &mut state.json, index),
        "--upsert" => set_registry_bool_flag(flag, inline_value, &mut state.upsert, index),
        "--registry" => {
            set_registry_string_flag(args, index, flag, inline_value, &mut state.registry)
        }
        "--registry-dir" | "--registryDir" => {
            set_registry_path_flag(args, index, flag, inline_value, &mut state.registry_dir)
        }
        "--version" => {
            set_registry_string_flag(args, index, flag, inline_value, &mut state.version)
        }
        "--digest" => {
            set_registry_string_flag(args, index, flag, inline_value, &mut state.expected_digest)
        }
        "--to" | "--destination" => {
            set_registry_path_flag(args, index, flag, inline_value, &mut state.destination)
        }
        "--installation-id" | "--installationId" => {
            set_registry_string_flag(args, index, flag, inline_value, &mut state.installation_id)
        }
        "--owner" => set_registry_string_flag(args, index, flag, inline_value, &mut state.owner),
        "--profile" => set_registry_path_flag(args, index, flag, inline_value, &mut state.profile),
        "--limit" => set_registry_limit_flag(args, index, flag, inline_value, state),
        _ => Err(format!("unknown registry flag {flag}")),
    }
}

fn set_registry_bool_flag(
    flag: &str,
    inline_value: Option<&str>,
    target: &mut bool,
    index: usize,
) -> Result<usize, String> {
    if inline_value.is_some() {
        return Err(format!("{flag} does not take a value"));
    }
    *target = true;
    Ok(index + 1)
}

fn set_registry_string_flag(
    args: &[OsString],
    index: usize,
    flag: &str,
    inline_value: Option<&str>,
    target: &mut Option<String>,
) -> Result<usize, String> {
    let (value, next_index) = flag_value(args, index, flag, inline_value, "registry")?;
    *target = Some(value);
    Ok(next_index)
}

fn set_registry_path_flag(
    args: &[OsString],
    index: usize,
    flag: &str,
    inline_value: Option<&str>,
    target: &mut Option<PathBuf>,
) -> Result<usize, String> {
    let (value, next_index) = flag_value(args, index, flag, inline_value, "registry")?;
    *target = Some(PathBuf::from(value));
    Ok(next_index)
}

fn set_registry_limit_flag(
    args: &[OsString],
    index: usize,
    flag: &str,
    inline_value: Option<&str>,
    state: &mut RegistryParseState,
) -> Result<usize, String> {
    let (value, next_index) = flag_value(args, index, flag, inline_value, "registry")?;
    state.limit = Some(
        value
            .parse::<usize>()
            .map_err(|_| "--limit must be a positive integer".to_owned())?,
    );
    Ok(next_index)
}

fn registry_subject(
    action: &RegistryAction,
    subcommand: &str,
    positionals: &mut Vec<String>,
) -> Result<String, String> {
    match action {
        RegistryAction::Search => {
            if positionals.is_empty() {
                return Err("runx registry search requires a query".to_owned());
            }
            Ok(positionals.join(" "))
        }
        RegistryAction::Read | RegistryAction::Resolve | RegistryAction::Install => {
            if positionals.len() != 1 {
                return Err(format!(
                    "runx registry {subcommand} requires exactly one ref"
                ));
            }
            Ok(positionals.remove(0))
        }
        RegistryAction::Publish => {
            if positionals.len() != 1 {
                return Err(
                    "runx registry publish requires exactly one skill markdown path".to_owned(),
                );
            }
            Ok(positionals.remove(0))
        }
    }
}

fn os_arg<'a>(args: &'a [OsString], index: usize, command: &str) -> Result<&'a str, String> {
    args.get(index)
        .and_then(|arg| arg.to_str())
        .ok_or_else(|| format!("{command} arguments must be UTF-8"))
}

fn split_flag(token: &str) -> (&str, Option<&str>) {
    token
        .split_once('=')
        .map_or((token, None), |(flag, value)| (flag, Some(value)))
}

fn flag_value(
    args: &[OsString],
    index: usize,
    flag: &str,
    inline_value: Option<&str>,
    command: &str,
) -> Result<(String, usize), String> {
    if let Some(value) = inline_value {
        return Ok((value.to_owned(), index + 1));
    }
    let value = os_arg(args, index + 1, command).map_err(|_| format!("{flag} requires a value"))?;
    if value.starts_with("--") {
        return Err(format!("{flag} requires a value"));
    }
    Ok((value.to_owned(), index + 2))
}

fn optional_flag_value(
    args: &[OsString],
    index: usize,
    inline_value: Option<&str>,
    command: &str,
) -> Result<(Option<String>, usize), String> {
    if let Some(value) = inline_value {
        return Ok((Some(value.to_owned()), index + 1));
    }
    let Some(value) = args.get(index + 1).and_then(|arg| arg.to_str()) else {
        return Ok((None, index + 1));
    };
    if value.starts_with('-') {
        return Ok((None, index + 1));
    }
    os_arg(args, index + 1, command)?;
    Ok((Some(value.to_owned()), index + 2))
}

fn truthy(value: &str) -> bool {
    matches!(value, "true" | "1" | "yes" | "official")
}
