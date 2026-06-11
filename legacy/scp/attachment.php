<?php
/*********************************************************************
    attachment.php

    Handles attachment downloads & access validation.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
require('staff.inc.php');
require_once(INCLUDE_DIR.'class.attachment.php');

//Basic checks
// @implements FS-022.10: Authorized Attachment Download & Inline Display — staff ticket-attachment resolve (id + file)
if(!$thisstaff || !$_GET['id'] || !$_GET['h'] 
        || !($attachment=Attachment::lookup($_GET['id'])) 
        || !($file=$attachment->getFile()))
    die('Unknown attachment!');

//Validate session access hash - we want to make sure the link is FRESH! and the user has access to the parent ticket!!
// @implements FS-022.10: Authorized Attachment Download & Inline Display — session-bound hash + parent-ticket staff-access check
// @implements BS-022.8: Download Access Requires a Fresh Session-Bound Hash
$vhash=md5($attachment->getFileId().session_id().$file->getHash());
if(strcasecmp(trim($_GET['h']),$vhash) || !($ticket=$attachment->getTicket()) || !$ticket->checkStaffAccess($thisstaff)) die('Access Denied');

//Download the file..
// @implements FS-022.11: Download vs Display Delivery Semantics — forced download delivery
$file->download();
?>
