<?php
require_once INCLUDE_DIR.'class.migrater.php';

// @implements FS-061.5: Resumable procedural migration tasks — single-shot switch to DB-backed sessions
// @implements FS-002.14: Database-backed session store — persists the current session into the new store via osTicketSession::write
class MigrateDbSession extends MigrationTask {
    var $description = "Migrate to database-backed sessions";

    // @implements FS-002.14: Database-backed session store — writes session_id()/session_encode() into the DB session table
    function run() {
        # How about 'dis for a hack?
        osTicketSession::write(session_id(), session_encode());
    }
}

return 'MigrateDbSession';
?>
