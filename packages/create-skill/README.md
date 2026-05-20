# @runxhq/create-skill

Initializer package behind:

```bash
npm create @runxhq/skill@latest my-skill
```

The canonical runx command remains:

```bash
runx new my-skill
```

This package is intentionally thin. It invokes the `runx` binary from
`@runxhq/cli` so the scaffolding logic stays in one native CLI path.

## Rust takeover boundary

`@runxhq/create-skill` remains a thin npm bootstrapper. After the Rust CLI
cutover it continues to wrap `runx new` through the bundled CLI rather than
reimplementing scaffolding logic.

See the [TypeScript interop boundary](../../docs/ts-interop-boundary.md) for
the package disposition and ownership rules.
