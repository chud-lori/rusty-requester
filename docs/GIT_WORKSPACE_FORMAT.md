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
    001-request-name-request-id.json
    002-other-request-other-request-id.json
    001-nested-folder-folder-id/
      001-nested-request-request-id.json
```

- `workspace.json` is the manifest. It records the format name, version, secret
  policy, ordered folder tree, and each request file path.
- Each request lives in its own readable JSON file under `requests/`.
- Folder and request file names include a 1-based order prefix, a readable slug,
  and the stable object ID. The manifest is the source of truth for order.
- Export rewrites the managed `requests/` directory so stale request files do
  not survive after deletes or moves.

## Determinism

For the same workspace data and export options, Rusty Requester writes the same
manifest content, request file paths, JSON field order, and trailing newlines.
The manifest preserves the app's visible collection, folder, and request order;
request files are written in sorted path order.

## IDs

Folder and request IDs are preserved in both the manifest and request files.
Import rejects a request file if its `id` does not match the manifest entry that
points at it. This catches common merge mistakes such as resolving a manifest
path to the wrong request body.

## Secrets

The default export policy is `masked`.

- Sensitive auth values, known sensitive row keys, cookies, form fields, and
  OAuth cached tokens are masked with the shared privacy helpers.
- URL query strings and fragments are redacted by default.
- Raw bodies are not parsed for secrets; keep credentials in auth, params,
  headers, cookies, or form fields if you want automatic masking.

Use the `include` policy only for private repositories or local-only sync when
you need a fully lossless export. Imports preserve whatever values are present
in the files, including masked placeholders.

## Merge Conflicts

Expected conflict shape:

- Two people edit different request files: Git usually merges cleanly.
- Two people edit the same request file: resolve the JSON object like any other
  small source file, keeping the original `id`.
- Two people reorder or move requests: resolve `workspace.json` first because it
  owns ordering and file paths.
- A request file and manifest entry disagree on ID after conflict resolution:
  import fails; fix the manifest path or the request file so both IDs match.
- Deleted request files should also be removed from `workspace.json`. Re-export
  after resolving conflicts if you want to normalize paths and ordering.

The safest manual resolution rule is: preserve IDs, keep one manifest entry per
request, and make every manifest `path` point to a JSON file with the same `id`.
