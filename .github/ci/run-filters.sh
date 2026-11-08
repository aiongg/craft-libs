#!/usr/bin/env bash
# Runs each line of .github/ci/filters.txt (<package> <test-binary|lib> <filter>) with cargo test.
# Output goes to test.log; the exit status is 1 when any line failed.
: > test.log
fail=0
while read -r pkg target filter; do
  [ -n "$pkg" ] || continue
  feat=()
  if [ "$pkg" = craft-ui ]; then
    feat=(--features testing)
  else
    # A domain crate: its check-crates.txt features plus screen-testing when it has one, as check.yml does.
    fl=$(awk -v p="$pkg" '$1 == p { print $2 }' migration/check-crates.txt 2>/dev/null)
    fs=()
    IFS=, read -ra listed <<< "$fl"
    for f in "${listed[@]}"; do [ -n "$f" ] && [ "$f" != screen-testing ] && fs+=("$f"); done
    if cargo metadata --no-deps --format-version 1 --locked 2>/dev/null \
      | jq -e --arg p "$pkg" '.packages[] | select(.name == $p) | .features | has("screen-testing")' > /dev/null; then
      fs+=(screen-testing)
    fi
    [ "${#fs[@]}" -gt 0 ] && feat=(--features "$(IFS=,; echo "${fs[*]}")")
  fi
  if [ "$target" = lib ]; then sel=(--lib); else sel=(--test "$target"); fi
  echo "=== $pkg $target $filter" | tee -a test.log
  cargo test -p "$pkg" "${feat[@]}" --locked "${sel[@]}" -- $filter 2>&1 | tee -a test.log
  [ "${PIPESTATUS[0]}" -eq 0 ] || fail=1
done < .github/ci/filters.txt
exit $fail
