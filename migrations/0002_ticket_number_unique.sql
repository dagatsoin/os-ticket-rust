-- TS-M1-B1 — globally-unique external ticket number (PostgreSQL dialect).
--
-- @implements FS-091.2: External ticket number generation — the application
--   convention that `ticketID` is *globally* unique (re-roll on collision) is
--   promoted to a DB guarantee so the modern core can rely on the UNIQUE
--   constraint + retry-on-conflict instead of a SELECT-then-INSERT race.
--
-- Modernisation note (ROADMAP Decisions): the legacy app enforced only the
-- composite UNIQUE (ticketID, email) (BS-091.1) and made ticketID globally
-- unique purely in application code via SELECT-then-recurse. TS-M1-B1 requires
-- collision safety via the UNIQUE constraint itself, so we add a standalone
-- UNIQUE on ticketID. The composite UNIQUE (ticketID, email) from 0001 remains
-- (it is now implied, but kept for spec fidelity / no destructive change).
--
-- IF NOT EXISTS keeps a re-run a safe no-op (AC-2 idempotency convention).

CREATE UNIQUE INDEX IF NOT EXISTS ticket_ticketid_unique ON ticket ("ticketID");
