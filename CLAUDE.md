# Claude Code Instructions

Read `AGENTS.md` for repo structure, build commands, architecture constraints, and code conventions.

## Environment Notes

`cargo` is not on the default PATH in the Claude Code sandbox. Prefix commands with:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

## Workflow Pointers

- `docs/adrs/` — architecture decision records
- `docs/plans/` — implementation plans
- `workers/gql-async-graphql/README.md` — worker-level dev guide
