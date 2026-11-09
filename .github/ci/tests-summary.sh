#!/usr/bin/env bash
# usage: tests-summary.sh <test-log>   (markdown on stdout)
log=$1
echo "### named tests"
grep -E '^(=== |test result:)' "$log" | sed 's/^/- /'
echo
echo "Failing tests:"
grep -E ' \.\.\. FAILED$' "$log" | sed -E 's/ \.\.\. FAILED$//; s/^test //; s/^/- /' | sort -u
echo
echo '```'
grep -E -A4 'panicked at' "$log" | head -80
echo '```'
grep -E '^error(\[|:)' "$log" | head -5 | sed 's/^/- BUILD /'
exit 0
