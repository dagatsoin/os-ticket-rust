<?php
/*********************************************************************
    logout.php

    Destroy clients session.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

// @implements FS-010.6: Client Session Lifecycle & Validation — destroys $_SESSION['_client'] and the PHP session on logout
// @implements BS-010.14: Logout Link-Token Guard Is Non-Blocking (Defect) — validateLinkToken is checked but the missing exit lets logout proceed without a valid token (cf. FS-002 KL-002-10)
// @implements EC-010.13: Tokenless / Forged Logout GET — tokenless/forged logout still destroys the session (inert guard)
// @implements KL-010.10: Logout Link-Token Guard Is Inert — logout is effectively unauthenticated (low-severity logout-CSRF)
require('client.inc.php');
//Check token: Make sure the user actually clicked on the link to logout.
if(!$_GET['auth'] || !$ost->validateLinkToken($_GET['auth']))
   @header('Location: index.php');

$_SESSION['_client']=array();
session_unset();
session_destroy();
header('Location: index.php');
require('index.php');
?>
