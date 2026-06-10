<?php
/*********************************************************************
    logo.php

    Simple logo to facilitate serving a customized client-side logo from
    osTicet. The logo is configurable in Admin Panel -> Settings -> Pages

    Peter Rotich <peter@osticket.com>
    Jared Hancock <jared@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

// @implements BS-010.10: Offline Gate Applies To All Client Pages Except The Logo — logo.php is the sole offline-exempt client endpoint
// @implements BS-010.15: Logo Endpoint Suppresses Session Writes (Client-Realm Mirror Of The API Realm) — installs a no-op session save handler + DISABLE_SESSION so inline-image fetches allocate no session record (client-realm mirror of FS-043 BS-436)
// Don't update the session for inline image fetches
if (!function_exists('noop')) { function noop() {} }
session_set_save_handler('noop','noop','noop','noop','noop','noop');
define('DISABLE_SESSION', true);

require('client.inc.php');

// @implements FS-032.6: Site Pages, Logos, Autoresponder, KB & Alerts Tabs — serves the admin-configured client logo, else the default asset
// @implements FS-022.10: Authorized Attachment Download & Inline Display — displays the stored logo file via the shared file store
if (($logo = $ost->getConfig()->getClientLogo())) {
    $logo->display();
} else {
    header('Location: '.ASSETS_PATH.'images/logo.png');
}

?>
