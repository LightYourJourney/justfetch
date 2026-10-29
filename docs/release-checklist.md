# Release checklist

What has to be true before the candidate is published, and the commands that do
it. The gates themselves are in [roadmap](roadmap.md); this file is the order of
operations.

## Verdict as of 2026-10-30

| Gate A condition | State |
| --- | --- |
| CI green on the exact commit to be tagged | Met — `d21356e`, run 40911175646: all six jobs succeeded |
| No known P0 or P1 | Met |
| Release notes and matrix state what was and was not executed | Met |
| The tag creates a draft prerelease | In place in the workflow; publishing stays a maintainer action |

**Go** for the tag. Step 1 below is already done; start at step 2.

## Publishing the candidate

```sh
# 1. The commit that will be tagged must exist on GitHub and pass CI.
#    Done for d21356e: run 40911175646, six jobs green.
git status                       # clean, nothing left to commit
gh run list --limit 1 --jq '.[0] | "\(.headSha[0:7]) \(.conclusion)"'

# 2. The tag. This is the point of no return for the version number.
git tag -a v1.0.0-rc.1 -m "justfetch v1.0.0-rc.1"
git push origin v1.0.0-rc.1

# 3. The workflow builds five targets, smoke-tests each packaged binary, writes
#    SHA256SUMS and creates a DRAFT prerelease. Check it before publishing.
gh release view v1.0.0-rc.1
gh release download v1.0.0-rc.1 --dir /tmp/rc-assets
(cd /tmp/rc-assets && shasum -a 256 -c SHA256SUMS)

# 4. Each archive: binary, LICENSE, README; the binary reports the tag version.
for archive in /tmp/rc-assets/*.tar.gz; do tar -tzf "$archive"; done
./scripts/check-release.sh

# 5. Publish. A maintainer does this deliberately; nothing else does.
gh release edit v1.0.0-rc.1 --draft=false
```

If step 3 produced anything unexpected, delete the draft and the tag, fix, and
start again — a published version number cannot be reused.

## After publishing

- Pin the release in `docs/validation.md` with its date and the CI run id.
- Ask for one run of `scripts/verify-linux.sh` and one of
  `scripts/verify-windows.ps1` on real machines, and paste the verdict tables
  into `docs/validation.md`. That is Gate B item 2.
- Regenerate the Homebrew and Scoop checksums from the published assets before
  anyone submits those manifests. They are placeholders today.

## Gate B — turning the candidate into v1.0

1. Public trial done and every defect it surfaced closed.
2. The two external verification runs above.
3. Intel macOS exercised on hardware, or its absence accepted in writing.
4. A crates.io decision.
5. `--draft --prerelease` removed from the workflow.

Until then the project is a release candidate, and the release notes say so.
