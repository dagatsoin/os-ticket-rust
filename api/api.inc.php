<?php
/*********************************************************************
    api.inc.php

    File included on every API page...handles common includes.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
file_exists('../main.inc.php') or die('System Error');

// @implements BS-436: Stateless API — No Sessions — installs a no-op session save handler and sets DISABLE_SESSION before booting
// @implements FS-043.13: Shared AJAX Controller Base (Infrastructure) — common API include boot loading the dispatcher + ApiController base
// @implements FS-001.1: Master Bootstrap Include — routes API entry points through main.inc.php before any feature logic
// Disable sessions for the API. API should be considered stateless and
// shouldn't chew up database records to store sessions
if (!function_exists('noop')) { function noop() {} }
session_set_save_handler('noop','noop','noop','noop','noop','noop');
define('DISABLE_SESSION', true);

require_once('../main.inc.php');
require_once(INCLUDE_DIR.'class.http.php');
require_once(INCLUDE_DIR.'class.api.php');

?>
