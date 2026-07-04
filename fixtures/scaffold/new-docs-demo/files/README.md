# docs-demo

A native runx skill: a `SKILL.md` contract, an `X.yaml` execution profile, and a
`run.mjs` script. No build step and no dependencies.

## Develop

For local development, `runx skill` and inline `runx harness` use
local-development receipts when no production signing env is configured.
Publishing and hosted verification still require real authority.

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
