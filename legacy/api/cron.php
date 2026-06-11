<?php
/*********************************************************************
    cron.php

    File to handle LOCAL cron job calls.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
@chdir(realpath(dirname(__FILE__)).'/'); //Change dir.
require('api.inc.php');

// @implements FS-043.10: Local (Command-Line) Cron Execution — entry point for local cron; refuses non-CLI invocation
// @implements FS-001.16: System-Object Request & Environment Utilities — is_cli() distinguishes shell execution from HTTP requests
if (!osTicket::is_cli())
    die('cron.php only supports local cron calls - use http -> api/tasks/cron');

@chdir(realpath(dirname(__FILE__)).'/'); //Change dir.
require('api.inc.php');
require_once(INCLUDE_DIR.'api.cron.php');
LocalCronApiController::call();
?>
