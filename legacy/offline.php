<?php
/*********************************************************************
    offline.php

    Offline page...modify to fit your needs.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
// @implements FS-001.15: Offline Page — serves the offline notice to client visitors; redirects home if the system came back online
// @implements FS-001.7: System-State Predicates (Online / Offline / Upgrade-Pending) — isSystemOnline drives the online-recheck redirect
require_once('client.inc.php');
if(is_object($ost) && $ost->isSystemOnline()) {
    @header('Location: index.php'); //Redirect if the system is online.
    include('index.php');
    exit;
}
$nav=null;
require(CLIENTINC_DIR.'header.inc.php');
?>
<div id="landing_page">
<?php
// @implements FS-001.15: Offline Page — renders the configured offline-page body, else the default "Support Ticket System Offline" heading
// @implements FS-032.6: Site Pages, Logos, Autoresponder, KB & Alerts Tabs — sources the offline page body from admin Pages settings (getOfflinePage)
if(($page=$cfg->getOfflinePage())) {
    echo $page->getBody();
} else {
    echo '<h1>Support Ticket System Offline</h1>';
}
?>
</div>
<?php require(CLIENTINC_DIR.'footer.inc.php'); ?>
