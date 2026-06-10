<?php
/*********************************************************************
    class.banlist.php

    Banned email addresses handle.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

require_once "class.filter.php";

// @implements FS-042.11: Banlist — Banned Email Address Interface — facade over the reserved SYSTEM BAN LIST filter
// @implements BS-042-13: Banlist is a reserved, special-cased filter
class Banlist {
    
    // @implements FS-042.12: Per-Ticket Ban / Unban Staff Action — add an email-equals ban rule
    function add($email,$submitter='') {
        return self::getSystemBanList()->addRule('email','equal',$email);
    }
    
    // @implements FS-042.12: Per-Ticket Ban / Unban Staff Action — remove an email-equals ban rule
    function remove($email) {
        return self::getSystemBanList()->removeRule('email','equal',$email);
    }
    
    // @implements FS-042.8: Fast Ban Pre-Screen (`isBanned`) — delegate to the fast ban pre-screen
    function isbanned($email) {
        return TicketFilter::isBanned($email);
    }

    // @implements FS-042.11: Banlist — Banned Email Address Interface — membership check (duplicate-add guard)
    function includes($email) {
        return self::getSystemBanList()->containsRule('email','equal',$email);
    }

    // @implements FS-042.11: Banlist — Banned Email Address Interface — resolve banlist filter id, auto-create on demand
    function ensureSystemBanList() {

        if (!($id=Filter::getIdByName('SYSTEM BAN LIST')))
            $id=self::createSystemBanList();

        return $id;
    }

    // @implements FS-042.11: Banlist — Banned Email Address Interface — create reserved filter (execorder 99, match-any, reject)
    // @implements BS-042-24: Banlist filter may exist with zero rules
    function createSystemBanList() {
        # XXX: Filter::create should return the ID!!!
        $errors=array();
        return Filter::create(array(
            'execorder'     => 99,
            'name'          => 'SYSTEM BAN LIST',
            'isactive'      => 1,
            'match_all_rules' => false,
            'reject_ticket'  => true,
            'rules'         => array(),
            'notes'         => 'Internal list for email banning. Do not remove'
        ), $errors);
    }

    // @implements FS-042.11: Banlist — Banned Email Address Interface — load the reserved filter object
    function getSystemBanList() {
        return new Filter(self::ensureSystemBanList());
    }

    // @implements FS-042.11: Banlist — Banned Email Address Interface — filter accessor alias
    function getFilter() {
        return self::getSystemBanList();
    }
}
