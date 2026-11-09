#!/usr/bin/env bash
# usage: check-summary.sh <check-log>   (markdown on stdout)
echo "### compile check"
echo '```'
grep -E '(^|: )(error|warning)' "$1" 2>/dev/null | head -200
echo '```'
exit 0
