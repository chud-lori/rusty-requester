# Git Workspace Format

Rusty Requester's Git workspace format is a deterministic directory export for
reviewable request collections. It is separate from the single-file JSON/YAML
backup export: normal imports still regenerate IDs to avoid collisions, while
Git workspace imports preserve IDs so branches can round-trip cleanly.

## Layout

```text
workspace.json
requests/
  001-collection-name-collection-id/
    001-request-name-request-id.rr
    002-other-request-other-request-id.rr
    001-nested-folder-folder-id/
      001-nested-request-request-id.rr
environments/
  001-local-env-local.rrenv
.gitignore
```

- `workspace.json` is the manifest. It records the format name, version, secret
  policy, ordered folder tree, request file paths, and environment file paths.
- Each request lives in its own readable `.rr` file under `requests/`. `.rr` is
  a compact Rusty Requester text format: scalar metadata at the top, table-like
  row sections for params / headers / cookies, and fenced blocks only for raw
  bodies or structured extension data.
- Each environment lives in its own `.rrenv` file under `environments/`.
  Variables are row-based and cookies remain structured data. Default exports
  mask secret-looking values.
- Export writes a `.gitignore` that excludes future local secret overlays:
  `secrets/` and `*.rrsecret`.
- Imports remain backward-compatible with older workspaces whose manifest
  points at `.json` request files.
- Folder and request file names include a 1-based order prefix, a readable slug,
  and the stable object ID. The manifest is the source of truth for order.
- Export rewrites the managed `requests/` directory so stale request files do
  not survive after deletes or moves.

## Determinism

For the same workspace data and export options, Rusty Requester writes the same
manifest content, request/environment file paths, field order, block delimiters,
and trailing newlines. The manifest preserves the app's visible collection,
folder, and request order; files are written in sorted path order.

## IDs

Folder, request, and environment IDs are preserved in both the manifest and
native files. Import rejects a request or environment file if its `id` does not
match the manifest entry that points at it. This catches common merge mistakes
such as resolving a manifest path to the wrong file.

## Secrets

The default export policy is `masked`.

- Sensitive auth values, known sensitive row keys, cookies, form fields, and
  OAuth cached tokens are masked with the shared privacy helpers.
- Environment variable values and cookies are masked by default.
- Collection Settings can add custom comma-separated mask patterns for keys
  such as `x-api-key`, and allow patterns for safe shared keys such as
  `platform` or `env`. Allow patterns win over built-in and custom mask rules.
- URL query strings and fragments are redacted by default.
- Raw bodies are not parsed for secrets; keep credentials in auth, params,
  headers, cookies, or form fields if you want automatic masking.

Use the `include` policy only for private repositories or local-only sync when
you need a fully lossless export. Imports preserve whatever values are present
in the files, including masked placeholders.

## Merge Conflicts

Expected conflict shape:

- Two people edit different request files: Git usually merges cleanly.
- Two people edit the same request file: resolve the `.rr` sections like any
  other small source file, keeping the original `id`.
- Two people reorder or move requests: resolve `workspace.json` first because it
  owns ordering and file paths.
- A request file and manifest entry disagree on ID after conflict resolution:
  import fails; fix the manifest path or the request file so both IDs match.
- Deleted request files should also be removed from `workspace.json`. Re-export
  after resolving conflicts if you want to normalize paths and ordering.

The safest manual resolution rule is: preserve IDs, keep one manifest entry per
request/environment, and make every manifest `path` point to a `.rr`,
`.rrenv`, or legacy `.json` file with the same `id`.
