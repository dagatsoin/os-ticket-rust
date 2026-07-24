#!/usr/bin/env python3
"""Classify every changed line in modified tracked files of osTicket-1.7.

Method: byte-level lines (split on b'\n'), diffed with SequenceMatcher over
TRAILING-WHITESPACE-STRIPPED lines (strip b' \t\r' — same semantics as
`git diff --ignore-space-at-eol`).

Classification:
  - equal (stripped) but raw bytes differ  -> trailing-whitespace-only change
                                              (repairable: restore HEAD bytes)
  - insert of comment lines (// # /* * <!-- <?php /*) -> allowed addition;
    counted; tag lines contain [FS-
  - insert of NON-comment lines, any delete, any replace -> REAL violation

Usage: verify_tags.py [--repair] [output.json]
"""
import subprocess, sys, re, difflib, json, os

REPO = "/Users/warfog/dev/osTicket-1.7"
REPAIR = "--repair" in sys.argv

COMMENT_RE = re.compile(rb'^\s*(//|#|/\*|\*|<!--|<\?php\s*/\*)')
TAG_RE = re.compile(rb'\[FS-')
WS = b' \t\r'

def git(*args, binary=False):
    r = subprocess.run(["git"] + list(args), cwd=REPO, capture_output=True)
    if r.returncode != 0:
        raise RuntimeError(f"git {args}: {r.stderr.decode()}")
    return r.stdout if binary else r.stdout.decode()

files = [f for f in git("diff", "--name-only").splitlines() if f]

report = {
    "files_modified": len(files),
    "added_comment_lines": 0,
    "added_tag_lines": 0,
    "ws_repairs": {},   # file -> count of trailing-ws-only lines
    "violations": [],
}

for f in files:
    orig = git("show", f"HEAD:{f}", binary=True).split(b'\n')
    path = os.path.join(REPO, f)
    with open(path, "rb") as fh:
        cur = fh.read().split(b'\n')

    orig_s = [l.rstrip(WS) for l in orig]
    cur_s = [l.rstrip(WS) for l in cur]

    sm = difflib.SequenceMatcher(a=orig_s, b=cur_s, autojunk=False)
    new_cur = list(cur)
    ws_count = 0

    for tag, i1, i2, j1, j2 in sm.get_opcodes():
        if tag == "equal":
            for k in range(i2 - i1):
                if orig[i1 + k] != cur[j1 + k]:
                    ws_count += 1
                    new_cur[j1 + k] = orig[i1 + k]
            continue
        if tag == "insert":
            for j in range(j1, j2):
                line = cur[j]
                if COMMENT_RE.match(line):
                    report["added_comment_lines"] += 1
                    if TAG_RE.search(line):
                        report["added_tag_lines"] += 1
                else:
                    report["violations"].append({"file": f, "kind": "non-comment-insert",
                        "cur_line_no": j + 1, "line": line.decode("utf-8", "replace")})
            continue
        if tag == "delete":
            for i in range(i1, i2):
                report["violations"].append({"file": f, "kind": "deleted-line",
                    "orig_line_no": i + 1, "line": orig[i].decode("utf-8", "replace")})
            continue
        # replace = real content change
        report["violations"].append({"file": f, "kind": "replace",
            "orig_line_nos": [i1 + 1, i2], "cur_line_nos": [j1 + 1, j2],
            "orig_lines": [l.decode("utf-8", "replace") for l in orig[i1:i2]],
            "cur_lines": [l.decode("utf-8", "replace") for l in cur[j1:j2]]})

    if ws_count:
        report["ws_repairs"][f] = ws_count
        if REPAIR:
            with open(path, "wb") as fh:
                fh.write(b'\n'.join(new_cur))

report["total_ws_lines"] = sum(report["ws_repairs"].values())
report["violation_count"] = len(report["violations"])
out = sys.argv[-1] if sys.argv[-1].endswith(".json") else "/tmp/tag_report.json"
with open(out, "w") as fh:
    json.dump(report, fh, indent=1)
print("files:", report["files_modified"], "| comments:", report["added_comment_lines"],
      "| tags:", report["added_tag_lines"], "| ws files:", len(report["ws_repairs"]),
      "| ws lines:", report["total_ws_lines"], "| violations:", report["violation_count"])
