<?php
/*********************************************************************
    secure.inc.php

    File included on every client's "secure" pages

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
if(!strcasecmp(basename($_SERVER['SCRIPT_NAME']),basename(__FILE__))) die('Kwaheri!');
if(!file_exists('client.inc.php')) die('Fatal Error.');
require_once('client.inc.php');

//Client Login page: Ajax interface can pre-declare the function to trap logins.
// @implements FS-010.9: Client Page Bootstrap & Guard — declares clientLoginPage (login.php) unless the AJAX layer pre-declared its 403 trap
if(!function_exists('clientLoginPage')) {
    function clientLoginPage($msg ='') {
        global $ost;
        require('./login.php');
        exit;
    }
}

//User must be logged in!
// @implements FS-010.6: Client Session Lifecycle & Validation — requires a valid $thisclient on secure pages; routes to login + refreshes the sliding token
// @implements FS-001.8: Client-Realm Gate — the authenticated-client requirement for the portal's "secure" pages
if(!$thisclient || !$thisclient->getId() || !$thisclient->isValid()){
    clientLoginPage();
    exit;
}
$thisclient->refreshSession();
?>
