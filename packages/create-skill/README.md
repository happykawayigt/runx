# @runxhq/create-skill

Deprecated compatibility wrapper behind the old npm initializer:

```bash
npm create @runxhq/skill@latest my-skill
```

The supported runx command is:

```bash
runx new my-skill
```

This package is intentionally thin and should not grow new behaviour. It invokes
the `runx` binary from `@runxhq/cli` so the scaffolding logic stays in the
native CLI path.

## Rust takeover boundary

`@runxhq/create-skill` is compatibility-only after the Rust CLI cutover. It
continues to wrap `runx new` through the bundled CLI rather than reimplementing
scaffolding logic.

See the [TypeScript interop boundary](../../docs/ts-interop-boundary.md) for
the package disposition and ownership rules.
