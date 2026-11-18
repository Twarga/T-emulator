#!/usr/bin/env bash
# T-Emulator autosync: commit + push any changes every run.
cd "$(dirname "$0")" || exit 1
git add -A
if git diff --cached --quiet; then
    exit 0   # nothing changed
fi
git commit -m "autosync $(date '+%Y-%m-%d %H:%M')" >/dev/null
git push -q origin main || true
