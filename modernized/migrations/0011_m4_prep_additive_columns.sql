-- TS-M4-PREP-A — M4 additive admin columns on department / groups / staff.
--
-- @implements FS-091: reference data / data model (admin-CRUD columns).
-- @implements FS-030.4: department Email/Template selects (email_id, tpl_id).
-- @implements BS-030-02: department default email binding.
-- @implements BS-030-03: department default template-group binding.
-- @implements FS-031.10: staff Time Zone preference (timezone_id).
-- @implements FS-031.11: staff daylight-saving preference (daylight_saving).
-- @implements FS-032.1: net-new group permission flags (faq/premade/ban/stats).
--
-- Additive only: every column is nullable or defaulted, so existing rows survive
-- and every ADD COLUMN is guarded (information_schema check) for idempotency.
--
-- Columns that ALREADY EXIST are NOT re-added (they would error on a 2nd apply):
--   department: ispublic, ticket_auto_response, message_auto_response, updated,
--     dept_signature (the existing "signature"), sla_id (0006).
--   groups: notes, updated, plus the M1/M3 flag columns.
--   staff: isvisible, signature, isadmin (0001), show_assigned_only (0005).
--
-- Ordering note: runs AFTER 0010 (which creates email_account / template_group /
-- timezone that the FK columns below reference).

-- ---------------------------------------------------------------------------
-- ALTER department — net-new admin columns. FK columns are nullable per M4-D1.
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'department' AND column_name = 'email_id') THEN
        ALTER TABLE department ADD COLUMN email_id integer NULL REFERENCES email_account (id);
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'department' AND column_name = 'tpl_id') THEN
        ALTER TABLE department ADD COLUMN tpl_id integer NULL REFERENCES template_group (id);
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'department' AND column_name = 'manager_id') THEN
        ALTER TABLE department ADD COLUMN manager_id integer NULL REFERENCES staff (staff_id);
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'department' AND column_name = 'autoresp_email_id') THEN
        ALTER TABLE department ADD COLUMN autoresp_email_id integer NULL REFERENCES email_account (id);
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'department' AND column_name = 'group_membership') THEN
        ALTER TABLE department ADD COLUMN group_membership integer NOT NULL DEFAULT 0;
    END IF;
END $$;

-- ---------------------------------------------------------------------------
-- ALTER groups — the four net-new permission flags (default false). The 8
-- pre-existing M1/M3 flags are untouched.
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'groups' AND column_name = 'can_manage_faq') THEN
        ALTER TABLE groups ADD COLUMN can_manage_faq boolean NOT NULL DEFAULT false;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'groups' AND column_name = 'can_manage_premade') THEN
        ALTER TABLE groups ADD COLUMN can_manage_premade boolean NOT NULL DEFAULT false;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'groups' AND column_name = 'can_ban_emails') THEN
        ALTER TABLE groups ADD COLUMN can_ban_emails boolean NOT NULL DEFAULT false;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'groups' AND column_name = 'can_view_staff_stats') THEN
        ALTER TABLE groups ADD COLUMN can_view_staff_stats boolean NOT NULL DEFAULT false;
    END IF;
END $$;

-- ---------------------------------------------------------------------------
-- ALTER staff — net-new admin/preference columns. isvisible, signature, isadmin
-- (0001) and show_assigned_only (0005) already exist and are NOT re-added.
-- ---------------------------------------------------------------------------
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'staff' AND column_name = 'onvacation') THEN
        ALTER TABLE staff ADD COLUMN onvacation boolean NOT NULL DEFAULT false;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'staff' AND column_name = 'default_signature_type') THEN
        ALTER TABLE staff ADD COLUMN default_signature_type text NOT NULL DEFAULT 'none';
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'staff' AND column_name = 'default_paper_size') THEN
        ALTER TABLE staff ADD COLUMN default_paper_size text NOT NULL DEFAULT 'Letter';
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'staff' AND column_name = 'max_page_size') THEN
        ALTER TABLE staff ADD COLUMN max_page_size integer NOT NULL DEFAULT 0;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'staff' AND column_name = 'auto_refresh_rate') THEN
        ALTER TABLE staff ADD COLUMN auto_refresh_rate integer NOT NULL DEFAULT 0;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'staff' AND column_name = 'timezone_id') THEN
        ALTER TABLE staff ADD COLUMN timezone_id integer NULL REFERENCES timezone (id);
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'staff' AND column_name = 'daylight_saving') THEN
        ALTER TABLE staff ADD COLUMN daylight_saving boolean NOT NULL DEFAULT false;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'staff' AND column_name = 'change_passwd') THEN
        ALTER TABLE staff ADD COLUMN change_passwd boolean NOT NULL DEFAULT false;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'staff' AND column_name = 'passwdreset') THEN
        ALTER TABLE staff ADD COLUMN passwdreset timestamptz NULL;
    END IF;

    IF NOT EXISTS (SELECT 1 FROM information_schema.columns
        WHERE table_name = 'staff' AND column_name = 'notes') THEN
        ALTER TABLE staff ADD COLUMN notes text NULL;
    END IF;
END $$;
