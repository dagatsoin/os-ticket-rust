<?php
// @implements FS-061.9: Upgrade wizard UX & access gating — legacy setup/upgrade.php entry redirects to staff-panel upgrade wizard
/* Simply redirect to Admin Panel - new upgrade home */
header('Location: ../scp/upgrade.php');
?>
