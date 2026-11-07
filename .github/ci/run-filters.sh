#!/usr/bin/env bash
# Runs each line of .github/ci/filters.txt (<package> <test-binary|lib> <filter>) with cargo test.
# Output goes to test.log; the exit status is 1 when any line failed.
: > test.log
fail=0
while read -r pkg target filter; do
  [ -n "$pkg" ] || continue
  feat=()
  [ "$pkg" = craft-ui ] && feat=(--features testing)
  if [ "$target" = lib ]; then sel=(--lib); else sel=(--test "$target"); fi
  echo "=== $pkg $target $filter" | tee -a test.log
  cargo test -p "$pkg" "${feat[@]}" --locked "${sel[@]}" -- $filter 2>&1 | tee -a test.log
  [ "${PIPESTATUS[0]}" -eq 0 ] || fail=1
done < .github/ci/filters.txt
exit $fail
