#!/usr/bin/env python3
"""PostToolUse hook: re-pin actions after an agent writes to a workflow file.

Reads the hook context from stdin. If the tool that just ran wrote to a file
under .github/workflows/, it runs the pin-actions skill script on that file.
"""

import json
import os
import subprocess
import sys

data = json.load(sys.stdin)

# PostToolUse payload has toolName + toolInput
tool_input = data.get("toolInput", {})
file_path = tool_input.get("filePath", "")

WORKFLOW_DIR = ".github/workflows/"

if not file_path.endswith((".yml", ".yaml")):
    sys.exit(0)

if WORKFLOW_DIR not in file_path and not file_path.startswith(WORKFLOW_DIR):
    sys.exit(0)

# Run the skill script against only the changed file
result = subprocess.run(  # noqa: S603
    [
        sys.executable,
        ".github/skills/pin-actions-to-sha/scripts/pin-actions.py",
    ],
    capture_output=True,
    text=True,
    env={**os.environ},
)

if result.stdout.strip():
    msg = f"[pin-actions-to-sha hook]\n{result.stdout}"
    print(json.dumps({"systemMessage": msg}))
