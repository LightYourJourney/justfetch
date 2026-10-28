#!/bin/sh
# Check that the version, release notes, archive names and checksums agree.
#
# Run before tagging, and again after downloading a release. Exit status 0 means
# everything it could check agrees.
set -u
fail=0
note() { printf '  %s\n' "$1"; }
bad() { printf 'FAIL %s\n' "$1"; fail=1; }

version=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
tag="v$version"
printf 'version: %s (tag %s)\n' "$version" "$tag"

notes="docs/release-notes/$tag.md"
if [ -f "$notes" ]; then note "release notes: $notes"; else bad "missing $notes"; fi

# The release job reads exactly this path.
workflow=$(grep -c 'notes-file "docs/release-notes/${GITHUB_REF_NAME}.md"' .github/workflows/release.yml)
[ "$workflow" -ge 1 ] && note "workflow notes path matches" || bad "workflow notes path does not match"

# The version gate must exist, and publication must be a draft.
grep -q "Check the tag matches the crate version" .github/workflows/release.yml \
  && note "tag/version gate present" || bad "no tag/version gate"
grep -q -- "--draft" .github/workflows/release.yml \
  && note "release is a draft" || bad "release is not a draft"

# Targets the workflow builds, and the archive name it gives each.
targets=$(sed -n 's/^ *target: //p' .github/workflows/release.yml | tr -d '\r')
for target in $targets; do
  case "$target" in
    *windows*) name="justfetch-$tag-$target.zip" ;;
    *)         name="justfetch-$tag-$target.tar.gz" ;;
  esac
  note "asset name: $name"
done

# If local archives exist, their names and checksums must match.
if [ -d dist ]; then
  for archive in dist/*.tar.gz dist/*.zip; do
    [ -e "$archive" ] || continue
    base=$(basename "$archive")
    case "$base" in
      "justfetch-$tag-"*) note "local archive: $base" ;;
      *) bad "local archive has the wrong name: $base" ;;
    esac
  done
  if [ -f dist/SHA256SUMS ]; then
    (cd dist && shasum -a 256 -c SHA256SUMS >/dev/null 2>&1 || sha256sum -c SHA256SUMS >/dev/null 2>&1) \
      && note "SHA256SUMS verifies" || bad "SHA256SUMS does not verify"
    listed=$(wc -l < dist/SHA256SUMS | tr -d ' ')
    archives=$(find dist -maxdepth 1 -type f \( -name '*.tar.gz' -o -name '*.zip' \) | wc -l | tr -d ' ')
    [ "$listed" = "$archives" ] && note "SHA256SUMS covers all $archives archives" \
      || bad "SHA256SUMS lists $listed of $archives archives"
  else
    bad "dist/ has no SHA256SUMS"
  fi
fi

if [ "$fail" -ne 0 ]; then echo "release check: FAILED"; exit 1; fi
echo "release check: OK"
