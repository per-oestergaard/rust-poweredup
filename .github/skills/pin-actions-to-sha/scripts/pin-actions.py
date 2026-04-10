#!/usr/bin/env python3
"""Pin GitHub Actions workflow steps to exact commit SHAs.

Usage:
    python3 pin-actions.py [--dry-run]

Set GITHUB_TOKEN to avoid hitting the unauthenticated API rate limit (60 req/h).
"""

import json
import os
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path

# Matches a full 40-char hex SHA (already pinned)
_SHA_RE = re.compile(r"^[0-9a-f]{40}$")

# Matches a `uses:` line: captures (indent+key, owner/repo, ref, trailing text)
# Examples:
#   "      uses: actions/checkout@v4"
#   "      uses: actions/checkout@v4 # comment"
_USES_RE = re.compile(
    r"^(\s+uses:\s+)"             # group 1: leading whitespace + "uses: "
    r"([A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)"  # group 2: owner/repo
    r"@([^\s#]+)"                  # group 3: ref (tag / branch / SHA)
    r"([ \t]*(?:#.*)?)$"           # group 4: optional trailing comment
)


def _api_get(url: str, token: str | None) -> dict:
    req = urllib.request.Request(url)
    req.add_header("Accept", "application/vnd.github+json")
    req.add_header("X-GitHub-Api-Version", "2022-11-28")
    if token:
        req.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(req) as resp:  # noqa: S310 (safe: built-in URL)
        return json.loads(resp.read())


def resolve_sha(owner_repo: str, ref: str, token: str | None) -> str | None:
    """Return the commit SHA for *ref* in *owner_repo*, or None on failure."""
    # 1. Try commits endpoint (works for branches and full/shortened SHAs)
    try:
        data = _api_get(
            f"https://api.github.com/repos/{owner_repo}/commits/{ref}", token
        )
        sha = data.get("sha")
        if sha and _SHA_RE.match(sha):
            return sha
    except urllib.error.HTTPError:
        pass

    # 2. Try the git/ref endpoint for tags (handles lightweight and annotated)
    for ref_path in (f"tags/{ref}", f"heads/{ref}"):
        try:
            data = _api_get(
                f"https://api.github.com/repos/{owner_repo}/git/ref/{ref_path}",
                token,
            )
            obj = data.get("object", {})
            if obj.get("type") == "commit":
                sha = obj.get("sha")
                if sha and _SHA_RE.match(sha):
                    return sha
            if obj.get("type") == "tag":
                # Annotated tag — follow to the tagged commit
                tag_data = _api_get(obj["url"], token)
                sha = tag_data.get("object", {}).get("sha")
                if sha and _SHA_RE.match(sha):
                    return sha
        except urllib.error.HTTPError:
            continue

    return None


def pin_file(path: Path, token: str | None, dry_run: bool) -> bool:
    """Pin all unpinned actions in *path*. Returns True if any changes were made."""
    original = path.read_text(encoding="utf-8")
    lines = original.splitlines(keepends=True)
    new_lines: list[str] = []
    changed = False

    for line in lines:
        m = _USES_RE.match(line.rstrip("\n"))
        if not m:
            new_lines.append(line)
            continue

        prefix, owner_repo, ref, trailing = m.group(1, 2, 3, 4)

        if _SHA_RE.match(ref):
            # Already pinned to a SHA — leave it alone
            new_lines.append(line)
            continue

        # Strip any existing comment so we don't double-up
        trailing_clean = re.sub(r"\s*#.*", "", trailing).rstrip()

        print(f"  Resolving {owner_repo}@{ref} ...", end=" ", flush=True)
        sha = resolve_sha(owner_repo, ref, token)

        if sha:
            print(sha[:12] + "...")
            new_line = f"{prefix}{owner_repo}@{sha} # {ref}{trailing_clean}\n"
            new_lines.append(new_line)
            changed = True
        else:
            print("FAILED (could not resolve SHA — leaving unchanged)")
            new_lines.append(line)

    if changed and not dry_run:
        path.write_text("".join(new_lines), encoding="utf-8")

    return changed


def main() -> None:
    dry_run = "--dry-run" in sys.argv
    token = os.environ.get("GITHUB_TOKEN")

    if not token:
        print(
            "Warning: GITHUB_TOKEN not set. "
            "Unauthenticated requests are limited to 60/hour.\n"
        )

    workflow_dir = Path(".github/workflows")
    if not workflow_dir.is_dir():
        print(f"No directory found at {workflow_dir}. Run from the repository root.")
        sys.exit(1)

    files = sorted(
        [*workflow_dir.glob("*.yml"), *workflow_dir.glob("*.yaml")]
    )
    if not files:
        print("No workflow files found.")
        sys.exit(0)

    if dry_run:
        print("=== DRY RUN — files will NOT be modified ===\n")

    any_changed = False
    for f in files:
        print(f"\nProcessing {f}:")
        changed = pin_file(f, token, dry_run)
        if changed:
            any_changed = True
            status = "(dry-run, not written)" if dry_run else "Updated"
            print(f"  {status}: {f}")
        else:
            print(f"  No changes needed.")

    if not any_changed:
        print("\nAll actions are already pinned to SHA hashes.")


if __name__ == "__main__":
    main()
