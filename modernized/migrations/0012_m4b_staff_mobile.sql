-- TS-M4-B1 — staff Mobile Number column (FS-031.3 / FS-031.9 directory).
--
-- @implements FS-031.3: staff create/edit collects a Mobile Number.
-- @implements FS-031.9: the staff directory exposes a Mobile Number column.
-- @implements BS-031-030: directory numeric-term search matches phone/ext/mobile.
--
-- Additive + guarded (information_schema check) so a re-apply is a safe no-op,
-- per the 0001/0010/0011 conventions. NOT NULL DEFAULT '' keeps existing rows
-- valid (parity with the existing `phone` / `phone_ext` columns).

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'staff' AND column_name = 'mobile'
    ) THEN
        ALTER TABLE staff ADD COLUMN mobile text NOT NULL DEFAULT '';
    END IF;
END $$;
