#!/usr/bin/env bash
# usage: summary.sh <cargo-test-log> <title>   (markdown on stdout)
log=$1; title=$2
echo "### $title"
grep -E '^test result:' "$log" | sed 's/^/- /'
echo
echo "Failing tests:"
grep -E ' \.\.\. FAILED$' "$log" | sed -E 's/ \.\.\. FAILED$//; s/^test //; s/^ +//; s/^/- /' | sort -u
grep -E '^error(\[|:)' "$log" | head -5 | sed 's/^/- BUILD /'
exit 0
