---
name: pin-actions-to-sha
description: "Pin GitHub Actions to SHA hashes for supply-chain security. Use when: auditing workflow files, replacing version tags with SHA pins, fixing unpinned actions, securing GitHub Actions, pinning actions to commit hash."
argument-hint: "Optional: --dry-run to preview changes without writing files"
---

# Pin GitHub Actions to SHA Hashes

Scans every workflow file under `.github/workflows/` and replaces floating
version references (e.g. `actions/checkout@v4`, `actions/checkout@main`) with
their exact commit SHA, keeping the original tag as a comment.

**Before:**

```yaml
uses: actions/checkout@v4
```

**After:**

```yaml
uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683 # v4
```

## When to Use

- Auditing or hardening a repository's CI pipeline
- Reviewing a PR that adds or updates GitHub Actions
- Any request to "pin actions", "use SHA hashes", or "fix unpinned actions"

## Prerequisites

- Python 3 available on `PATH`
- `GITHUB_TOKEN` environment variable set (recommended to avoid rate limits)

## Procedure

1. **Run the pinning script** from the workspace root:

   ```bash
   python3 .github/skills/pin-actions-to-sha/scripts/pin-actions.py
   ```

   Pass `--dry-run` to preview changes without modifying files:

   ```bash
   python3 .github/skills/pin-actions-to-sha/scripts/pin-actions.py --dry-run
   ```

2. **Review the output.** The script prints each resolved action and its new
   SHA. Any action whose SHA cannot be resolved is listed as a warning.

3. **Verify the changes** by inspecting the diff:

   ```bash
   git diff .github/workflows/
   ```

4. **Commit** the pinned workflow files.

## Notes

- Actions that already reference a full 40-character SHA are skipped.
- The script preserves existing inline comments after replacing the tag.
- Set `GITHUB_TOKEN` to a fine-grained token with **read-only public repo**
  access to avoid hitting the GitHub API unauthenticated rate limit (60 req/h).
