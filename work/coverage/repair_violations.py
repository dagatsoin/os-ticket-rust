#!/usr/bin/env python3
"""Repair real violations left by phase-3 tag agents, preserving all [FS-] tags.

Cases:
 A. 33 files: existing first line `<?php` mutated to `<?php /* [FS-xxx] */`.
    Fix: restore HEAD line 1, insert `// [FS-xxx]` comment line after it.
 B. setup/inc/install-done.inc.php + subscribe.inc.php: `// [FS-060]...`
    appended to existing code line 1. Fix: restore HEAD line 1, move the
    comment to its own inserted line 2 (still inside the PHP block).
 C. setup/inc/footer.inc.php: original blank line 1 replaced by an HTML
    comment. Fix: re-insert the blank line after the comment.
 D. include/ajax.users.php: pre-existing blank line (HEAD line 17) deleted.
    Fix: re-insert it after the header-comment closing line.
 E. include/staff/tickets.inc.php: 4 mid-PHP insertions of
    `<?php /* ... */ ?>` + `<?php` (parse error inside an open PHP block).
    Fix: collapse each pair into a single pure `/* ... */` PHP comment line.
"""
import re, subprocess, sys

REPO = "/Users/warfog/dev/osTicket-1.7"

def head(f):
    return subprocess.run(["git", "show", f"HEAD:{f}"], cwd=REPO,
                          capture_output=True, check=True).stdout.split(b'\n')

def load(f):
    with open(f"{REPO}/{f}", "rb") as fh:
        return fh.read().split(b'\n')

def save(f, lines):
    with open(f"{REPO}/{f}", "wb") as fh:
        fh.write(b'\n'.join(lines))

fixed = []

# ---- Case A: first-line `<?php` -> `<?php /* TAGS */` in 33 files
CASE_A_RE = re.compile(rb'^<\?php\s+/\*\s*(.*?)\s*\*/\s*(\r?)$')
import json
rep = json.load(open(sys.argv[1] if len(sys.argv) > 1 else "/tmp/tag_dry3.json"))
case_a_files = sorted({v["file"] for v in rep["violations"]
                       if v["kind"] == "replace"
                       and v["orig_line_nos"] == [1, 1]
                       and len(v["orig_lines"]) == 1
                       and v["orig_lines"][0].rstrip() == "<?php"})
for f in case_a_files:
    cur = load(f); orig = head(f)
    m = CASE_A_RE.match(cur[0])
    assert m, (f, cur[0])
    inner, cr = m.group(1), m.group(2)
    assert b'[FS-' in inner, (f, inner)
    cur[0:1] = [orig[0], b'// ' + inner + cr]
    save(f, cur); fixed.append(("A", f, "line1 restored; tags moved to new '// %s' line" % inner.decode()))

# ---- Case B: setup/inc/{install-done,subscribe}.inc.php
for f in ("setup/inc/install-done.inc.php", "setup/inc/subscribe.inc.php"):
    cur = load(f); orig = head(f)
    o0 = orig[0].rstrip(b' \t\r')
    assert cur[0].startswith(o0 + b' // [FS-'), (f, cur[0])
    comment = cur[0][len(o0):].strip()           # "// [FS-060] ..."
    cur[0:1] = [orig[0], comment]
    save(f, cur); fixed.append(("B", f, "line1 restored; comment moved to own line"))

# ---- Case C: setup/inc/footer.inc.php blank line restored
f = "setup/inc/footer.inc.php"
cur = load(f); orig = head(f)
assert orig[0].rstrip(b' \t\r') == b'' and cur[0].startswith(b'<!-- [FS-'), (f, cur[0])
cur[1:1] = [orig[0]]
save(f, cur); fixed.append(("C", f, "blank HEAD line 1 re-inserted after comment"))

# ---- Case D: include/ajax.users.php deleted blank line (HEAD line 17)
f = "include/ajax.users.php"
cur = load(f); orig = head(f)
anchor = orig[15]   # line 16: the ****/ header close; HEAD line 17 was blank
assert orig[16].rstrip(b' \t\r') == b''
idx = cur.index(anchor)
assert cur[idx + 1].rstrip(b' \t\r') != b''     # blank really missing
cur[idx + 1:idx + 1] = [orig[16]]
save(f, cur); fixed.append(("D", f, "blank HEAD line 17 re-inserted"))

# ---- Case E: include/staff/tickets.inc.php stray <?php pairs
f = "include/staff/tickets.inc.php"
cur = load(f)
PAIR_RE = re.compile(rb'^<\?php\s+(/\*.*\*/)\s*\?>\s*$')
# i > 2 excludes the legitimate top-of-file pattern (inserted comment line 1
# BEFORE the pre-existing `<?php` at line 2 — pure insertion, valid HTML-mode)
positions = [i for i, l in enumerate(cur) if l == b'<?php' and i > 2 and PAIR_RE.match(cur[i - 1])]
assert len(positions) == 4, positions
for i in sorted(positions, reverse=True):
    m = PAIR_RE.match(cur[i - 1])
    cur[i - 1:i + 1] = [m.group(1)]             # one pure PHP comment line
save(f, cur); fixed.append(("E", f, f"4 stray '<?php /* */ ?>'+'<?php' pairs collapsed to pure /* */ comments at former lines {[p+1 for p in positions]}"))

for c, f, msg in fixed:
    print(f"[{c}] {f}: {msg}")
print(f"total files repaired: {len(set(f for _, f, _ in fixed))}")
