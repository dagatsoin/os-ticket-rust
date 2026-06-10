# Pre-Tag PHP Backup Runbook (Phase-3 Prep)

Status: **PENDING SHELL EXECUTION** — the prep agent had no shell/Bash tool available,
so the tarball below has NOT yet been produced. Run these commands from the repo root
(`/Users/warfog/dev/osTicket-1.7`) before any `[FS-XXX]` tagging begins.

## 1. Create the tarball

```bash
cd /Users/warfog/dev/osTicket-1.7
mkdir -p work/backups
tar czf work/backups/pre-tag-php-$(date +%Y%m%d).tar.gz \
    $(find . -name '*.php' -not -path './work/*')
```

## 2. Verify it is non-empty and has a plausible file count

```bash
ls -l work/backups/pre-tag-php-*.tar.gz          # size must be > 0
tar tzf work/backups/pre-tag-php-*.tar.gz | wc -l # expect 290 (.php files in repo)
```

Expected file count: **290** `.php` files (verified by the prep agent via ripgrep).

## 3. Record git cleanliness (baseline for comment-only verification)

```bash
git status --porcelain | head -30
```

The repo IS a git repository. Capture this output so that, after Phase-3 tagging,
`git diff` can confirm only additive comment lines were introduced to in-scope files.
Tracked-file cleanliness at baseline is what makes that later verification trustworthy.

## Why this is a runbook and not a completed step

The preparation agent's toolset (Read/Write/Edit/Glob/Grep) does not include a Bash/shell
executor. Creating a `.tar.gz` and running `git` require shell access. The partition map and
exclusion analysis (which only need file reads) are complete; this backup must be executed by
a shell-capable runner or the operator before tagging.
