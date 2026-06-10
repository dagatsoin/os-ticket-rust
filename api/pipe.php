#!/usr/bin/php -q
<?php
/*********************************************************************
    pipe.php

    Converts piped emails to ticket. Just local - remote must use /api/tickets.email

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
ini_set('memory_limit', '256M'); //The concern here is having enough mem for emails with attachments.
@chdir(realpath(dirname(__FILE__)).'/'); //Change dir.
require('api.inc.php');

//Only local piping supported via pipe.php
// @implements FS-041.1: Local Pipe Intake — local MTA email-pipe entry point; refuses non-CLI invocation
// @implements BS-041.3: Local-Only Pipe — pipe.php supports only local piping (remote senders must use the HTTP email API)
// @implements FS-001.16: System-Object Request & Environment Utilities — is_cli() guards the pipe to command-line execution
if (!osTicket::is_cli())
    die('pipe.php only supports local piping - use http -> api/tickets.email');

require_once(INCLUDE_DIR.'api.tickets.php');
PipeApiController::process();
?>
