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

lock_version=$(grep -A1 'name = "justfetch"' Cargo.lock | sed -n 's/^version = "\(.*\)"/\1/p')
[ "$lock_version" = "$version" ] && note "Cargo.lock agrees" || bad "Cargo.lock says '$lock_version'"

grep -q "^## $version" CHANGELOG.md && note "CHANGELOG has an entry" || bad "CHANGELOG has no '## $version' entry"

if [ -x target/release/justfetch ]; then
  reported=$(target/release/justfetch --version | awk '{print $2}')
  [ "$reported" = "$version" ] && note "binary reports $reported" || bad "binary reports '$reported'"
fi

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

# The candidate/stable marker must follow the version, not be hardcoded: a
# `-rc.N` version published as stable, or a final version published as a
# prerelease, is worse than failing here.
case "$version" in
  *-*) kind="prerelease" ;;
  *)   kind="stable" ;;
esac
grep -q 'stage="--prerelease"' .github/workflows/release.yml \
  && grep -q '^            \*-\*) stage="--prerelease"' .github/workflows/release.yml \
  && note "release marker derives from the version ($version -> $kind)" \
  || bad "the workflow does not derive --prerelease from the version"

grep -q "github.event_name == 'push'" .github/workflows/release.yml \
  && note "release job requires a push event" || bad "release job does not require a push event"

# Targets the workflow builds, and the archive name it gives each.
targets=$(sed -n 's/^ *target: //p' .github/workflows/release.yml | tr -d '\r')
count=$(echo "$targets" | grep -c . || true)
[ "$count" -eq 5 ] && note "workflow builds 5 targets" || bad "workflow lists $count targets, expected 5"
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
      "justfetch-$tag-"*) ;;
      *) bad "local archive has the wrong name: $base"; continue ;;
    esac
    # The target must be one the workflow builds.
    target=$(echo "$base" | sed -e "s/^justfetch-$tag-//" -e 's/\.tar\.gz$//' -e 's/\.zip$//')
    if echo "$targets" | grep -qx "$target"; then
      note "local archive: $base"
    else
      bad "local archive names a target the workflow does not build: $target"
    fi
  done
  if [ -f dist/SHA256SUMS ]; then
    (cd dist && shasum -a 256 -c SHA256SUMS >/dev/null 2>&1 || sha256sum -c SHA256SUMS >/dev/null 2>&1) \
      && note "SHA256SUMS verifies" || bad "SHA256SUMS does not verify"
    for archive in dist/*.tar.gz dist/*.zip; do
      [ -e "$archive" ] || continue
      case "$archive" in
        *.tar.gz) contents=$(tar -tzf "$archive" | sed 's#.*/##' | sort | tr '\n' ' ') ;;
        *.zip)    contents=$(unzip -Z1 "$archive" 2>/dev/null | sed 's#.*/##' | sort | tr '\n' ' ') ;;
      esac
      echo "$contents" | grep -qE '(^| )justfetch(\.exe)?( |$)' \
        && echo "$contents" | grep -q ' LICENSE ' \
        && echo "$contents" | grep -q ' README.md ' \
        && note "archive contents: $(basename "$archive")" \
        || bad "archive is missing the binary, LICENSE or README: $(basename "$archive")"
    done
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
