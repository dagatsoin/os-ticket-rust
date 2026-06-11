<?php
/*********************************************************************
    open.php

    New tickets handle.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
require('client.inc.php');
define('SOURCE','Web'); //Ticket source.
$ticket = null;
$errors=array();
// @implements FS-011.1: New-Ticket Request Handler — processes the web new-ticket POST, forcing deptId/emailId to 0 per BS-011.1
// @implements FS-011.5: CAPTCHA Challenge (Anonymous Submitters) — verifies the session captcha hash for anonymous submitters
// @implements FS-011.7: Attachment Submission — formats $_FILES attachments when online attachments are allowed
// @implements FS-011.9: Ticket Creation, Routing, and Reference Assignment — Ticket::create on validated web-form input
if($_POST):
    $vars = $_POST;
    $vars['deptId']=$vars['emailId']=0; //Just Making sure we don't accept crap...only topicId is expected.
    if($thisclient) {
        $vars['name']=$thisclient->getName();
        $vars['email']=$thisclient->getEmail();
    } elseif($cfg->isCaptchaEnabled()) {
        if(!$_POST['captcha'])
            $errors['captcha']='Enter text shown on the image';
        elseif(strcmp($_SESSION['captcha'], md5(strtoupper($_POST['captcha']))))
            $errors['captcha']='Invalid - try again!';
    }

    if(!$errors && $cfg->allowOnlineAttachments() && $_FILES['attachments'])
        $vars['files'] = AttachmentFile::format($_FILES['attachments'], true);

    //Ticket::create...checks for errors..
    if(($ticket=Ticket::create($vars, $errors, SOURCE))){
        $msg='Support ticket request created';
        //Logged in...simply view the newly created ticket.
        if($thisclient && $thisclient->isValid()) {
            if(!$cfg->showRelatedTickets())
                $_SESSION['_client']['key']= $ticket->getExtId(); //Resetting login Key to the current ticket!
            session_write_close();
            session_regenerate_id();
            @header('Location: tickets.php?id='.$ticket->getExtId());
        }
    }else{
        $errors['err']=$errors['err']?$errors['err']:'Unable to create a ticket. Please correct errors below and try again!';
    }
endif;

//page
// @implements FS-011.10: Success / Thank-You Response — renders the topic/thank-you page with the ticket number masked as XXXXXX
// @implements BS-011.6: Ticket Number Is Never Displayed On-Screen at Creation — masks %{ticket.number}/extId vars before display
// @implements FS-090.4: Client Portal Navigation Links — sets the 'new' (Open New Ticket) nav tab active
$nav->setActiveNav('new');
require(CLIENTINC_DIR.'header.inc.php');
if($ticket
        && (
            (($topic = $ticket->getTopic()) && ($page = $topic->getPage()))
            || ($page = $cfg->getThankYouPage())
            )) { //Thank the user and promise speedy resolution!
    //Hide ticket number -  it should only be delivered via email for security reasons.
    echo Format::safe_html($ticket->replaceVars(str_replace(
                    array('%{ticket.number}', '%{ticket.extId}', '%{ticket}'), //ticket number vars.
                    array_fill(0, 3, 'XXXXXX'),
                    $page->getBody()
                    )));
} else {
    require(CLIENTINC_DIR.'open.inc.php');
}
require(CLIENTINC_DIR.'footer.inc.php');
?>
