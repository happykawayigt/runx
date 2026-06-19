// Native cli-tool skill scaffold: SKILL.md + X.yaml + run.mjs + README + .gitignore.
// The output has zero dependencies and no build step, so `runx new` produces a
// skill that runs and harnesses immediately, with nothing pinned that can drift.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScaffoldFile {
    pub relative_path: String,
    pub contents: String,
}

pub fn scaffold_package_files(name: &str) -> Vec<ScaffoldFile> {
    vec![
        file("SKILL.md", skill_md(name)),
        file("X.yaml", x_yaml(name)),
        file("run.mjs", run_mjs()),
        file("README.md", readme(name)),
        file(".gitignore", "node_modules/\n.runx/\n*.tgz\n".to_owned()),
    ]
}

fn file(relative_path: &str, contents: String) -> ScaffoldFile {
    ScaffoldFile {
        relative_path: relative_path.to_owned(),
        contents,
    }
}

fn skill_md(name: &str) -> String {
    format!(
        r#"---
name: {name}
description: {name} runx skill. Replace this with what the skill does and returns.
source:
  type: cli-tool
  command: node
  args:
    - run.mjs
  timeout_seconds: 30
  sandbox:
    profile: readonly
    cwd_policy: skill-directory
inputs:
  message:
    type: string
    required: true
    description: Input the skill acts on. Replace with the real inputs.
runx:
  input_resolution:
    required:
      - message
---

# {name}

Describe what this skill does, when an agent should reach for it, and what it
returns. Replace the echo in `run.mjs` with the real work, and add cases to
`X.yaml` so the behaviour is locked by the harness.
"#
    )
}

fn x_yaml(name: &str) -> String {
    format!(
        r#"skill: {name}
version: "0.1.0"

catalog:
  kind: skill
  audience: public
  visibility: public
  role: canonical

harness:
  cases:
    - name: {name}-smoke
      runner: default
      inputs:
        message: hello
      expect:
        status: sealed
        receipt:
          schema: runx.receipt.v1
          state: sealed
          disposition: closed
          reason_code: process_closed
    - name: {name}-empty-message-fails
      runner: default
      inputs:
        message: ""
      expect:
        status: failure
        receipt:
          schema: runx.receipt.v1
          state: sealed
          disposition: closed
          reason_code: process_failed

runners:
  default:
    default: true
    type: cli-tool
    command: node
    args:
      - run.mjs
    inputs:
      message:
        type: string
        required: true
        description: Input the skill acts on.
"#
    )
}

fn run_mjs() -> String {
    r#"// Inputs arrive as RUNX_INPUT_<NAME> environment variables. Do the work and
// write the result to stdout. Replace this echo with the real logic.
const message = process.env.RUNX_INPUT_MESSAGE ?? "";
if (message.trim().length === 0) {
  process.stderr.write("message is required\n");
  process.exit(64);
}
process.stdout.write(`${message}\n`);
"#
    .to_owned()
}

fn readme(name: &str) -> String {
    format!(
        r#"# {name}

A native runx skill: a `SKILL.md` contract, an `X.yaml` execution profile, and a
`run.mjs` script. No build step and no dependencies.

## Develop

```bash
runx harness . --json                       # run the harness cases in X.yaml
runx skill . --input message=hello --json   # run the skill once
runx history                                # inspect the signed receipt
```

Edit `run.mjs` to do the real work, and keep both harness classes in `X.yaml`:
one happy path and one stop, error, or refusal case.

## Publish

```bash
runx login --provider github --for publish
runx registry publish .   # the registry runs the harness as the publish gate
```
"#
    )
}
