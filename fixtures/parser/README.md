# Rust parser parity fixtures

Parser fixtures are the cross-language contract between the CLI-local
TypeScript parser in `packages/cli/src/cli-parser` and the Rust `runx-parser`
crate.

Fixture categories:

- `skills`: `SKILL.md` markdown parsing and validated skill output.
- `graphs`: graph YAML parsing and validated graph output.
- `runner-manifests`: runner manifest parsing and validation.
- `tool-manifests`: tool manifest YAML/JSON parsing and validation.
- `installs`: skill-install parsing and validation.
- `rejections`: shared parser rejection cases when a case is not tied to one
  category.

Each fixture stores a typed input envelope plus either `expected.validated` or
`expected.rejection`. Skill fixtures use `input.markdown`. Parsed raw skill
fields live under `expected.raw.frontmatter`, `expected.raw.rawFrontmatter`,
and `expected.raw.body`. Raw object subtrees use the shared
`runx_contracts::JsonValue` model and stable sorted-key JSON.

The YAML scalar subset intentionally excludes host-divergent forms until a
separate compatibility spec proves them across TypeScript and Rust:
sexagesimal values, implicit `yes`/`no`/`on`/`off` booleans, octal/hex integer
forms, timestamps, unquoted date-like strings, and special floats.
