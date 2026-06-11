<?php
/*********************************************************************
    tickets.php

    Main client/user interface.
    Note that we are using external ID. The real (local) ids are hidden from user.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
require('secure.inc.php');
if(!is_object($thisclient) || !$thisclient->isValid()) die('Access denied'); //Double check again.
require_once(INCLUDE_DIR.'class.ticket.php');
// @implements FS-010.7: Client Ticket View (Thread, Header, Reply) — resolves the requested ticket by external id for the logged-in client
// @implements BS-010.4: Client Ticket Access Is Scoped By Email (Or Login Ticket) — checkClientAccess gate, generic error on mismatch
$ticket=null;
if($_REQUEST['id']) {
    if(!($ticket=Ticket::lookupByExtId($_REQUEST['id']))) {
        $errors['err']='Unknown or invalid ticket ID.';
    }elseif(!$ticket->checkClientAccess($thisclient)) {
        $errors['err']='Unknown or invalid ticket ID.'; //Using generic message on purpose!
        $ticket=null;
    }
}

//Process post...depends on $ticket object above.
// @implements FS-010.7: Client Ticket View (Thread, Header, Reply) — handles the a=reply POST: re-checks access, validates message, posts the reply
// @implements BS-010.4: Client Ticket Access Is Scoped By Email (Or Login Ticket) — re-verifies checkClientAccess before posting
if($_POST && is_object($ticket) && $ticket->getId()):
    $errors=array();
    switch(strtolower($_POST['a'])){
    case 'reply':
        if(!$ticket->checkClientAccess($thisclient)) //double check perm again!
            $errors['err']='Access Denied. Possibly invalid ticket ID';

        if(!$_POST['message'])
            $errors['message']='Message required';

        if(!$errors) {
            //Everything checked out...do the magic.
            $vars = array('message'=>$_POST['message']);
            if($cfg->allowOnlineAttachments() && $_FILES['attachments'])
                $vars['files'] = AttachmentFile::format($_FILES['attachments'], true);

            if(($msgid=$ticket->postMessage($vars, 'Web'))) {
                $msg='Message Posted Successfully';
            } else {
                $errors['err']='Unable to post the message. Try again';
            }

        } elseif(!$errors['err']) {
            $errors['err']='Error(s) occurred. Please try again';
        }
        break;
    default:
        $errors['err']='Unknown action';
    }
    $ticket->reload();
endif;
// @implements FS-010.7: Client Ticket View (Thread, Header, Reply) — selects view.inc.php when the client has access to the resolved ticket
// @implements FS-010.8: "My Tickets" List (Related Tickets by Email) — falls back to the related-tickets list when enabled and the client has tickets
// @implements FS-090.4: Client Portal Navigation Links — sets the 'tickets'/'new' nav tab active for the chosen view
$nav->setActiveNav('tickets');
if($ticket && $ticket->checkClientAccess($thisclient)) {
    $inc='view.inc.php';
} elseif($cfg->showRelatedTickets() && $thisclient->getNumTickets()) {
    $inc='tickets.inc.php';
} else {
    $nav->setActiveNav('new');
    $inc='open.inc.php';
}
include(CLIENTINC_DIR.'header.inc.php');
include(CLIENTINC_DIR.$inc);
include(CLIENTINC_DIR.'footer.inc.php');
?>
