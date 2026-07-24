-- TS-M3-D2 — add sla_id to department table.
--
-- @implements FS-091 Entity #8: Department references SLA (sla_id→SLA).
-- @implements TS-M3-D2 AC-3: Support department has Standard SLA assignment.
--
-- The M1 schema (0001) omitted this column; adding it now for M3 SLA assignment.

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'department' AND column_name = 'sla_id'
    ) THEN
        ALTER TABLE department ADD COLUMN sla_id integer NULL REFERENCES sla (id);
    END IF;
END $$;
