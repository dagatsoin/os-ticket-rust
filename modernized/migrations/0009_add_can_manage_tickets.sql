-- Add can_manage_tickets permission flag to groups table.
--
-- @implements BS-021.1: mass manage permission (FS-021.21).
--
-- This permission controls who can perform bulk/mass actions on tickets.

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'groups' AND column_name = 'can_manage_tickets'
    ) THEN
        ALTER TABLE groups ADD COLUMN can_manage_tickets boolean NOT NULL DEFAULT false;
    END IF;
END $$;
