<?php
/*********************************************************************
    class.cron.php

    Nothing special...just a central location for all cron calls.
    
    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    TODO: The plan is to make cron jobs db based.
    
    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
//TODO: Make it DB based!
// @implements FS-043.7: Cron Job Inventory — fixed background maintenance job set
// @implements KL-433: Cron is not DB-scheduled
class Cron {

    // @implements FS-043.8: Mail-Fetch as Cron Job #1 — invoke MailFetcher::run
    // @implements BS-434: Mail-Fetch Is Cron Job #1
    function MailFetcher() {
        require_once(INCLUDE_DIR.'class.mailfetch.php');
        MailFetcher::run(); //Fetch mail..frequency is limited by email account setting.
    }

    // @implements FS-043.7: Cron Job Inventory — ticket monitor (overdue sweep + expired-lock cleanup)
    // @implements BS-432: Ticket Overdue Qualification
    function TicketMonitor() {
        require_once(INCLUDE_DIR.'class.ticket.php');
        require_once(INCLUDE_DIR.'class.lock.php');
        Ticket::checkOverdue(); //Make stale tickets overdue
        TicketLock::cleanup(); //Remove expired locks 
    }

    // @implements FS-043.7: Cron Job Inventory — purge aged system-log entries
    function PurgeLogs() {
        global $ost;
        if($ost) $ost->purgeLogs();
    }

    // @implements FS-043.7: Cron Job Inventory — delete orphaned ticket-type files + chunks
    function CleanOrphanedFiles() {
        require_once(INCLUDE_DIR.'class.file.php');
        AttachmentFile::deleteOrphans();
    }

    // @implements FS-043.7: Cron Job Inventory — full cycle, upgrade-gated, fixed order
    // @implements BS-433: Full Cron Cycle Is Upgrade-Gated and Ordered
    // @implements EC-441: Cron during pending upgrade — returns immediately
    function run(){ //called by outside cron NOT autocron
        global $ost;
        if (!$ost || $ost->isUpgradePending())
            return;

        self::MailFetcher();
        self::TicketMonitor();
        self::PurgeLogs();
        self::CleanOrphanedFiles();
    }
}
?>
