<?php
/*********************************************************************
    ajax.upgrader.php

    AJAX interface for Upgrader

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

if(!defined('INCLUDE_DIR')) die('403');
require_once INCLUDE_DIR.'class.upgrader.php';

// @implements FS-061.11: AJAX progress protocol with manual fallback — the polled upgrade endpoint
class UpgraderAjaxAPI extends AjaxController {

    // @implements FS-061.11: AJAX progress protocol with manual fallback — 200 progress / 201 done / 416 aborted poll responses
    // @implements FS-061.12: Abort, error capture & alerting — 416 short-circuit when aborted or errors accumulated
    // @implements FS-061.14: Post-upgrade completion actions — setState('done') + session write-close on no pending upgrade
    // @implements BS-061-03: Only an authenticated admin may run the upgrade — 403 Access Denied otherwise
    function upgrade() {
        global $thisstaff, $ost;

        if(!$thisstaff or !$thisstaff->isAdmin() or !$ost)
            Http::response(403, 'Access Denied');

        $upgrader = new Upgrader(TABLE_PREFIX, UPGRADE_DIR.'streams/');

        if($upgrader->isAborted()) {
            Http::response(416, "We have a problem ... wait a sec.");
            exit;
        }

        if($upgrader->getTask() && $upgrader->doTask()) {
            //More pending tasks - doTasks returns the number of pending tasks
            Http::response(200, $upgrader->getNextAction());
            exit;
        } elseif($ost->isUpgradePending()) {
            if($upgrader->isUpgradable()) {
                $version = $upgrader->getNextVersion();
                if($upgrader->upgrade()) {
                    //We're simply reporting progress here - call back will report next action'
                    Http::response(200, "Upgraded to $version ... post-upgrade checks!");
                    exit;
                }
            } else {
                //Abort: Upgrade pending but NOT upgradable - invalid or wrong hash.
                $upgrader->abort(sprintf('Upgrade Failed: Invalid or wrong hash [%s]',$ost->getDBSignature()));
            }
        } elseif(!$ost->isUpgradePending()) {
            $upgrader->setState('done');
            session_write_close();
            Http::response(201, "We're done!");
            exit;
        }

        if($upgrader->isAborted() || $upgrader->getErrors()) {
            Http::response(416, "We have a problem ... wait a sec.");
            exit;
        }

        Http::response(200, $upgrader->getNextAction());
    }
}
?>
