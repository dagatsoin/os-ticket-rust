<?php
require_once INCLUDE_DIR.'class.migrater.php';

// @implements FS-061.5: Resumable procedural migration tasks — single-shot v1.6 group department-access migration
// @implements BS-031-021: Group↔Department Access Matrix — populates the group↔dept access join table from the legacy CSV column
class MigrateGroupDeptAccess extends MigrationTask {
    var $description = "Migrate department access for groups from v1.6";

    // @implements FS-061.5: Resumable procedural migration tasks — run() explodes legacy dept_access CSV into join rows in one pass
    // @implements BS-031-021: Group↔Department Access Matrix — one INSERT per (group_id, dept_id) pair
    function run($max_time) {
        $this->setStatus("Migrating department access");

        $res = db_query('SELECT group_id, dept_access FROM '.GROUP_TABLE);
        if(!$res || !db_num_rows($res))
            return false;  //No groups??

        while(list($groupId, $access) = db_fetch_row($res)) {
            $depts=array_filter(array_map('trim', explode(',', $access)));
            foreach($depts as $deptId) {
                $sql='INSERT INTO '.GROUP_DEPT_TABLE
                    .' SET dept_id='.db_input($deptId).', group_id='.db_input($groupId);
                db_query($sql);
            }
        }
    }
}

return 'MigrateGroupDeptAccess';
?>
