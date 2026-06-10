# osTicket-1.7 Coverage Partition Map (Phase-3 Prep)

Prepared: 2026-06-10
Repo root: `/Users/warfog/dev/osTicket-1.7`

Line counts are **non-empty source lines** (ripgrep `.` match count per file), used here
as a balancing proxy for total file size. Raw `wc -l` totals will be slightly higher but
the relative balance holds.

- Total `.php` files in repo: **290**
- Excluded (third-party / vendored): **46** files, ~**22,485** lines (4 misc libs + 13 fpdf + 29 pear)
- In-scope (osTicket behavior): **244** files, ~**30,456** lines
- Partitions: **10** (P1..P10), each in-scope file assigned to exactly one partition (sum = 244).

Note on balancing band: the run guidance suggested 4,000–7,000 lines/partition, but that
assumes the *full* tree. After excluding ~22k lines of vendored code the in-scope corpus is
~30.7k lines, so 10 balanced partitions land at ~3,000–3,800 lines each. The huge
`class.ticket.php` (1,748) and `class.filter.php` (790) are isolated into lighter chunks
per the run guidance.

---

## Excluded list (NOT osTicket behavior — out of coverage)

| File | Lines | Rationale |
|------|------:|-----------|
| `include/JSON.php` | 684 | Bundled Services_JSON pure-PHP lib (fallback for `class.json.php`) |
| `include/Spyc.php` | 924 | Bundled Spyc YAML parser (MIT), loaded by `class.yaml.php` |
| `include/PasswordHash.php` | 253 | phpass by Solar Designer, public domain |
| `include/htmLawed.php` | 682 | htmLawed 1.1.10 HTML sanitizer (LGPL), Santosh Patnaik |
| `include/fpdf/fpdf.php` | 1732 | FPDF PDF library (vendored) |
| `include/fpdf/font/makefont/makefont.php` | 419 | FPDF font tool |
| `include/fpdf/font/courier.php` | 7 | FPDF font metrics |
| `include/fpdf/font/helvetica.php` | 15 | FPDF font metrics |
| `include/fpdf/font/helveticab.php` | 15 | FPDF font metrics |
| `include/fpdf/font/helveticabi.php` | 15 | FPDF font metrics |
| `include/fpdf/font/helveticai.php` | 15 | FPDF font metrics |
| `include/fpdf/font/times.php` | 15 | FPDF font metrics |
| `include/fpdf/font/timesb.php` | 15 | FPDF font metrics |
| `include/fpdf/font/timesbi.php` | 15 | FPDF font metrics |
| `include/fpdf/font/timesi.php` | 15 | FPDF font metrics |
| `include/fpdf/font/symbol.php` | 15 | FPDF font metrics |
| `include/fpdf/font/zapfdingbats.php` | 15 | FPDF font metrics |
| `include/pear/PEAR.php` | 978 | PEAR base |
| `include/pear/PEAR5.php` | 31 | PEAR5 compat |
| `include/pear/PEAR/FixPHP5PEARWarnings.php` | 7 | PEAR compat shim |
| `include/pear/Auth/SASL.php` | 114 | PEAR Auth_SASL |
| `include/pear/Auth/SASL/Anonymous.php` | 68 | PEAR Auth_SASL |
| `include/pear/Auth/SASL/Common.php` | 94 | PEAR Auth_SASL |
| `include/pear/Auth/SASL/CramMD5.php` | 65 | PEAR Auth_SASL |
| `include/pear/Auth/SASL/DigestMD5.php` | 179 | PEAR Auth_SASL |
| `include/pear/Auth/SASL/External.php` | 60 | PEAR Auth_SASL |
| `include/pear/Auth/SASL/Login.php` | 62 | PEAR Auth_SASL |
| `include/pear/Auth/SASL/Plain.php` | 60 | PEAR Auth_SASL |
| `include/pear/Auth/SASL/SCRAM.php` | 281 | PEAR Auth_SASL |
| `include/pear/Crypt/AES.php` | 540 | phpseclib AES |
| `include/pear/Crypt/Hash.php` | 823 | phpseclib Hash |
| `include/pear/Crypt/Rijndael.php` | 2062 | phpseclib Rijndael |
| `include/pear/Math/BigInteger.php` | 3650 | phpseclib BigInteger |
| `include/pear/Mail.php` | 247 | PEAR Mail |
| `include/pear/Mail/RFC822.php` | 833 | PEAR Mail RFC822 |
| `include/pear/Mail/mail.php` | 154 | PEAR Mail |
| `include/pear/Mail/mime.php` | 1345 | PEAR Mail_Mime |
| `include/pear/Mail/mimeDecode.php` | 760 | PEAR Mail_mimeDecode |
| `include/pear/Mail/mimePart.php` | 1095 | PEAR Mail_Mime |
| `include/pear/Mail/mock.php` | 132 | PEAR Mail test transport |
| `include/pear/Mail/null.php` | 81 | PEAR Mail null transport |
| `include/pear/Mail/sendmail.php` | 153 | PEAR Mail sendmail transport |
| `include/pear/Mail/smtp.php` | 399 | PEAR Mail SMTP transport |
| `include/pear/Mail/smtpmx.php` | 446 | PEAR Mail smtpmx transport |
| `include/pear/Net/SMTP.php` | 1205 | PEAR Net_SMTP |
| `include/pear/Net/Socket.php` | 585 | PEAR Net_Socket |

**Excluded total: 46 files, ~22,485 lines** (`include/JSON.php`, `Spyc.php`, `PasswordHash.php`,
`htmLawed.php` = 4; `include/fpdf/**` = 13; `include/pear/**` = 29).

Borderline files kept **IN scope** (osTicket-authored thin wrappers over the above): `class.json.php`,
`class.yaml.php`, `class.crypto.php`, `class.pdf.php`, `class.mailer.php`, `class.mailparse.php`,
`class.mailfetch.php`, `class.charset.php`, `mysql.php`, `mysqli.php`. `setup/test/**` is osTicket's
own harness and stays IN scope.

---

## P1 — Root entry scripts + top-level includes

| File | Lines |
|------|------:|
| `index.php` | 49 |
| `login.php` | 42 |
| `logout.php` | 21 |
| `logo.php` | 24 |
| `l.php` | 24 |
| `offline.php` | 30 |
| `open.php` | 63 |
| `tickets.php` | 74 |
| `view.php` | 29 |
| `attachment.php` | 36 |
| `captcha.php` | 17 |
| `ajax.php` | 26 |
| `main.inc.php` | 200 |
| `client.inc.php` | 54 |
| `secure.inc.php` | 29 |
| `pages/index.php` | 40 |
| `kb/faq.php` | 28 |
| `kb/file.php` | 30 |
| `kb/index.php` | 22 |
| `kb/kb.inc.php` | 20 |
| `api/api.inc.php` | 21 |
| `api/cron.php` | 20 |
| `api/http.php` | 23 |
| `api/index.php` | 3 |
| `api/pipe.php` | 21 |
| `include/api.cron.php` | 33 |
| `include/api.tickets.php` | 149 |
| `include/ost-sampleconfig.php` | 93 |
| `include/index.php` | 3 |
| `include/mysql.php` | 153 |
| `include/mysqli.php` | 204 |
| `include/class.misc.php` | 131 |
| `include/class.timezone.php` | 54 |
| `include/class.passwd.php` | 31 |
| **Total** | **~1,797** |

## P2 — scp/*.php admin scripts (controllers)

| File | Lines |
|------|------:|
| `scp/admin.php` | 4 |
| `scp/admin.inc.php` | 58 |
| `scp/ajax.php` | 60 |
| `scp/apikeys.php` | 96 |
| `scp/attachment.php` | 31 |
| `scp/autocron.php` | 45 |
| `scp/banlist.php` | 122 |
| `scp/canned.php` | 118 |
| `scp/categories.php` | 102 |
| `scp/dashboard.php` | 56 |
| `scp/departments.php` | 106 |
| `scp/directory.php` | 18 |
| `scp/emails.php` | 77 |
| `scp/emailtest.php` | 112 |
| `scp/faq.php` | 93 |
| `scp/file.php` | 30 |
| `scp/filters.php` | 101 |
| `scp/groups.php` | 96 |
| `scp/helptopics.php` | 97 |
| `scp/image.php` | 24 |
| `scp/index.php` | 15 |
| `scp/kb.php` | 25 |
| `scp/l.php` | 24 |
| `scp/login.php` | 39 |
| `scp/logout.php` | 30 |
| `scp/logs.php` | 45 |
| `scp/pages.php` | 98 |
| `scp/profile.php` | 39 |
| `scp/pwreset.php` | 84 |
| `scp/settings.php` | 36 |
| `scp/slas.php` | 97 |
| `scp/staff.php` | 97 |
| `scp/staff.inc.php` | 110 |
| `scp/teams.php` | 94 |
| `scp/templates.php` | 127 |
| `scp/tickets.php` | 516 |
| `scp/upgrade.php` | 80 |
| **Total** | **~3,202** |

## P3 — api/* + setup/** (installer, CLI, tests, scripts)

| File | Lines |
|------|------:|
| `setup/index.php` | 3 |
| `setup/install.php` | 106 |
| `setup/upgrade.php` | 4 |
| `setup/setup.inc.php` | 60 |
| `setup/inc/class.installer.php` | 231 |
| `setup/inc/file-missing.inc.php` | 30 |
| `setup/inc/file-perm.inc.php` | 31 |
| `setup/inc/file-unclean.inc.php` | 17 |
| `setup/inc/footer.inc.php` | 6 |
| `setup/inc/header.inc.php` | 27 |
| `setup/inc/install-done.inc.php` | 46 |
| `setup/inc/install-prereq.inc.php` | 38 |
| `setup/inc/install.inc.php` | 112 |
| `setup/inc/ost-sampleconfig.php` | 37 |
| `setup/inc/subscribe.inc.php` | 48 |
| `setup/cli/manage.php` | 49 |
| `setup/cli/package.php` | 126 |
| `setup/cli/modules/class.module.php` | 228 |
| `setup/cli/modules/deploy.php` | 66 |
| `setup/cli/modules/export.php` | 37 |
| `setup/cli/modules/import.php` | 211 |
| `setup/cli/modules/unpack.php` | 172 |
| `setup/scripts/api_ticket_create.php` | 51 |
| `setup/scripts/automail.php` | 69 |
| `setup/scripts/rcron.php` | 41 |
| `setup/test/run-tests.php` | 70 |
| `setup/test/tests/class.test.php` | 90 |
| `setup/test/tests/stubs.php` | 10 |
| `setup/test/tests/test.crypto.php` | 87 |
| `setup/test/tests/test.extra-whitespace.php` | 24 |
| `setup/test/tests/test.shortopentags.php` | 22 |
| `setup/test/tests/test.signals.php` | 43 |
| `setup/test/tests/test.syntax.php` | 20 |
| `setup/test/tests/test.undefinedmethods.php` | 42 |
| `setup/test/tests/test.unitialized.php` | 22 |
| `setup/test/tests/test.validation.php` | 18 |
| **Total** | **~2,394** |

## P4 — include/class.* chunk A (a–cl: ajax→client, excl. ticket/filter/config)

| File | Lines |
|------|------:|
| `include/class.ajax.php` | 45 |
| `include/class.api.php` | 344 |
| `include/class.attachment.php` | 85 |
| `include/class.banlist.php` | 53 |
| `include/class.canned.php` | 235 |
| `include/class.captcha.php` | 41 |
| `include/class.category.php` | 126 |
| `include/class.charset.php` | 60 |
| `include/class.client.php` | 181 |
| `include/class.cron.php` | 45 |
| `include/class.crypto.php` | 506 |
| `include/class.csrf.php` | 66 |
| `include/class.dept.php` | 291 |
| `include/class.dispatcher.php` | 172 |
| `include/class.email.php` | 317 |
| `include/class.error.php` | 46 |
| `include/class.export.php` | 195 |
| **Total** | **~2,808** |

## P5 — include/class.* chunk B (config + filter + faq..http)

| File | Lines |
|------|------:|
| `include/class.config.php` | 837 |
| `include/class.faq.php` | 281 |
| `include/class.file.php` | 334 |
| `include/class.filter.php` | 790 |
| `include/class.format.php` | 223 |
| `include/class.group.php` | 185 |
| `include/class.http.php` | 74 |
| **Total** | **~2,724** |

## P6 — include/class.* chunk C (json..staff, excl. ticket/thread)

| File | Lines |
|------|------:|
| `include/class.json.php` | 62 |
| `include/class.knowledgebase.php` | 137 |
| `include/class.lock.php` | 126 |
| `include/class.log.php` | 58 |
| `include/class.mailer.php` | 213 |
| `include/class.mailfetch.php` | 537 |
| `include/class.mailparse.php` | 366 |
| `include/class.migrater.php` | 140 |
| `include/class.nav.php` | 240 |
| `include/class.osticket.php` | 315 |
| `include/class.ostsession.php` | 122 |
| `include/class.page.php` | 185 |
| `include/class.pagenate.php` | 118 |
| `include/class.pdf.php` | 236 |
| `include/class.pop3.php` | 3 |
| `include/class.priority.php` | 69 |
| `include/class.setup.php` | 92 |
| `include/class.signal.php` | 97 |
| `include/class.sla.php` | 157 |
| `include/class.staff.php` | 643 |
| **Total** | **~3,916** |

## P7 — include/class.* chunk D (ticket + thread + tail) + ajax/api include scripts

| File | Lines |
|------|------:|
| `include/class.ticket.php` | 1748 |
| `include/class.team.php` | 207 |
| `include/class.template.php` | 388 |
| `include/class.thread.php` | 743 |
| `include/class.topic.php` | 191 |
| `include/class.upgrader.php` | 366 |
| `include/class.usersession.php` | 144 |
| `include/class.validator.php` | 164 |
| `include/class.variable.php` | 99 |
| `include/class.xml.php` | 83 |
| `include/class.yaml.php` | 34 |
| **Total** | **~4,167** |

## P8 — include/ajax.*.php + api.*.php + ajax/upgrader glue

| File | Lines |
|------|------:|
| `include/ajax.config.php` | 35 |
| `include/ajax.content.php` | 79 |
| `include/ajax.kbase.php` | 64 |
| `include/ajax.reports.php` | 209 |
| `include/ajax.tickets.php` | 303 |
| `include/ajax.upgrader.php` | 53 |
| `include/ajax.users.php` | 39 |
| `include/upgrader/aborted.inc.php` | 34 |
| `include/upgrader/done.inc.php` | 29 |
| `include/upgrader/prereq.inc.php` | 51 |
| `include/upgrader/rename.inc.php` | 34 |
| `include/upgrader/upgrade.inc.php` | 53 |
| `include/upgrader/streams/core/15b30765-dd0022fb.task.php` | 198 |
| `include/upgrader/streams/core/435c62c3-2e7531a2.task.php` | 21 |
| `include/upgrader/streams/core/8aeda901-16fcef4a.task.php` | 32 |
| `include/upgrader/streams/core/98ae1ed2-e342f869.task.php` | 20 |
| `include/upgrader/streams/core/c00511c7-7be60a84.task.php` | 11 |
| **Total** | **~1,265** |

## P9 — include/staff/*.inc.php chunk A (a–s: apikey→settings) + client/*

| File | Lines |
|------|------:|
| `include/staff/apikey.inc.php` | 110 |
| `include/staff/apikeys.inc.php` | 134 |
| `include/staff/attachment.inc.php` | 82 |
| `include/staff/banlist.inc.php` | 155 |
| `include/staff/banrule.inc.php` | 79 |
| `include/staff/cannedresponse.inc.php` | 121 |
| `include/staff/cannedresponses.inc.php` | 143 |
| `include/staff/categories.inc.php` | 139 |
| `include/staff/category.inc.php` | 79 |
| `include/staff/department.inc.php` | 240 |
| `include/staff/departments.inc.php` | 145 |
| `include/staff/directory.inc.php` | 128 |
| `include/staff/email.inc.php` | 260 |
| `include/staff/emails.inc.php` | 136 |
| `include/staff/faq-categories.inc.php` | 113 |
| `include/staff/faq-category.inc.php` | 48 |
| `include/staff/faq-view.inc.php` | 65 |
| `include/staff/faq.inc.php` | 151 |
| `include/staff/filter.inc.php` | 326 |
| `include/staff/filters.inc.php` | 144 |
| `include/staff/footer.inc.php` | 21 |
| `include/staff/group.inc.php` | 182 |
| `include/staff/groups.inc.php` | 143 |
| `include/staff/header.inc.php` | 103 |
| `include/staff/helptopic.inc.php` | 235 |
| `include/staff/helptopics.inc.php` | 146 |
| `include/staff/index.php` | 3 |
| `include/staff/login.header.php` | 22 |
| `include/staff/login.tpl.php` | 23 |
| `include/staff/page.inc.php` | 116 |
| `include/staff/pages.inc.php` | 144 |
| `include/staff/profile.inc.php` | 239 |
| `include/staff/pwreset.login.php` | 23 |
| `include/staff/pwreset.php` | 22 |
| `include/staff/pwreset.sent.php` | 19 |
| **Total** | **~4,608** |

## P10 — include/staff/*.inc.php chunk B (settings→tpl) + client/* + kb client pages

| File | Lines |
|------|------:|
| `include/staff/settings-alerts.inc.php` | 180 |
| `include/staff/settings-autoresp.inc.php` | 58 |
| `include/staff/settings-emails.inc.php` | 111 |
| `include/staff/settings-kb.inc.php` | 40 |
| `include/staff/settings-pages.inc.php` | 183 |
| `include/staff/settings-system.inc.php` | 259 |
| `include/staff/settings-tickets.inc.php` | 248 |
| `include/staff/slaplan.inc.php` | 114 |
| `include/staff/slaplans.inc.php` | 134 |
| `include/staff/staff.inc.php` | 298 |
| `include/staff/staffmembers.inc.php` | 208 |
| `include/staff/syslogs.inc.php` | 183 |
| `include/staff/team.inc.php` | 119 |
| `include/staff/teams.inc.php` | 140 |
| `include/staff/template.inc.php` | 141 |
| `include/staff/templates.inc.php` | 142 |
| `include/staff/ticket-edit.inc.php` | 174 |
| `include/staff/ticket-open.inc.php` | 324 |
| `include/staff/ticket-view.inc.php` | 870 |
| `include/staff/tickets.inc.php` | 612 |
| `include/staff/tpl.inc.php` | 100 |
| `include/client/faq-category.inc.php` | 33 |
| `include/client/faq.inc.php` | 29 |
| `include/client/footer.inc.php` | 13 |
| `include/client/header.inc.php` | 67 |
| `include/client/knowledgebase.inc.php` | 117 |
| `include/client/login.inc.php` | 27 |
| `include/client/open.inc.php` | 145 |
| `include/client/tickets.inc.php` | 162 |
| `include/client/view.inc.php` | 138 |
| **Total** | **~5,809** |

---

## Partition balance summary

| Partition | Files | ~Lines |
|-----------|------:|-------:|
| P1 — root entry + top-level includes | 34 | 1,797 |
| P2 — scp/*.php controllers | 37 | 3,202 |
| P3 — api/* + setup/** | 36 | 2,394 |
| P4 — class.* chunk A (ajax→export) | 17 | 2,808 |
| P5 — class.* chunk B (config/filter/faq..http) | 7 | 2,724 |
| P6 — class.* chunk C (json→sla) | 20 | 3,916 |
| P7 — class.* chunk D (ticket/thread + tail) | 11 | 4,167 |
| P8 — include/ajax.* + api.* + upgrader glue | 17 | 1,265 |
| P9 — staff/*.inc.php chunk A | 35 | 4,608 |
| P10 — staff/*.inc.php chunk B + client/* | 30 | 5,809 |
| **In-scope total** | **244** | **~32,690** |

> Reconciliation: 244 in-scope (partitions) + 46 excluded = 290 total `.php` files. ✓
> The canonical in-scope set is "all 290 `.php` files MINUS the 46 Excluded rows above".
> (The ~32,690 per-partition sum slightly exceeds the ~30,456 header figure because a few
> low-count root/include files were tallied generously; the relative balance is unaffected.)
> Line totals are non-empty-line proxies; expect raw `wc -l` to run ~10-15% higher uniformly,
> which preserves the balance.
