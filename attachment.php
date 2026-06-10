<?php
/*********************************************************************
    attachment.php

    Attachments interface for clients.
    Clients should never see the dir paths.
    
    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
// @implements FS-022.10: Authorized Attachment Download & Inline Display — client-side attachment download with session-bound hash validation
// @implements BS-022.8: Download Access Requires a Fresh Session-Bound Hash — verifies md5(fileId+session_id+fileHash) before serving
// @implements BS-010.4: Client Ticket Access Is Scoped By Email (Or Login Ticket) — checkClientAccess on the parent ticket gates the download
require('secure.inc.php');
require_once(INCLUDE_DIR.'class.attachment.php');
//Basic checks
if(!$thisclient 
        || !$_GET['id'] 
        || !$_GET['h']
        || !($attachment=Attachment::lookup($_GET['id']))
        || !($file=$attachment->getFile()))
    die('Unknown attachment!');

//Validate session access hash - we want to make sure the link is FRESH! and the user has access to the parent ticket!!
$vhash=md5($attachment->getFileId().session_id().$file->getHash());
if(strcasecmp(trim($_GET['h']),$vhash) 
        || !($ticket=$attachment->getTicket()) 
        || !$ticket->checkClientAccess($thisclient)) 
    die('Unknown or invalid attachment');
//Download the file..
$file->download();

?>
