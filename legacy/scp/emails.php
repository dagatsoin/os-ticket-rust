<?php
/*********************************************************************
    emails.php

    Emails

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
require('admin.inc.php');
include_once(INCLUDE_DIR.'class.email.php');

// @implements FS-040.1: Email Account Listing — email account id lookup, "Unknown or invalid email ID." on miss
$email=null;
if($_REQUEST['id'] && !($email=Email::lookup($_REQUEST['id'])))
    $errors['err']='Unknown or invalid email ID.';

// @implements FS-040.3: Email Account Save Validation & Provisioning — POST dispatch
if($_POST){
    switch(strtolower($_POST['do'])){
        // @implements FS-040.3: Email Account Save Validation & Provisioning — update existing account
        case 'update':
            if(!$email){
                $errors['err']='Unknown or invalid email.';
            }elseif($email->update($_POST,$errors)){
                $msg='Email updated successfully';
            }elseif(!$errors['err']){
                $errors['err']='Error updating email. Try again!';
            }
            break;
        // @implements FS-040.3: Email Account Save Validation & Provisioning — create new account
        case 'create':
            if(($id=Email::create($_POST,$errors))){
                $msg='Email address added successfully';
                $_REQUEST['a']=null;
            }elseif(!$errors['err']){
                $errors['err']='Unable to add email. Correct error(s) below and try again.';
            }
            break;
        // @implements FS-040.4: Email Account Bulk Delete — refuse if dept-associated, skip default, delete the rest
        // @implements BS-040.3: Department Association Blocks Deletion
        case 'mass_process':
            if(!$_POST['ids'] || !is_array($_POST['ids']) || !count($_POST['ids'])) {
                $errors['err'] = 'You must select at least one email address';
            } else {
                $count=count($_POST['ids']);

                $sql='SELECT count(dept_id) FROM '.DEPT_TABLE.' dept '
                    .' WHERE email_id IN ('.implode(',', db_input($_POST['ids'])).') '
                    .' OR autoresp_email_id IN ('.implode(',', db_input($_POST['ids'])).')';

                list($depts)=db_fetch_row(db_query($sql));
                if($depts>0) {
                    $errors['err'] = 'One or more of the selected emails is being used by a department. Remove association first!';
                } elseif(!strcasecmp($_POST['a'], 'delete')) {
                    $i=0;
                    foreach($_POST['ids'] as $k=>$v) {
                        if($v!=$cfg->getDefaultEmailId() && ($e=Email::lookup($v)) && $e->delete())
                            $i++;
                    }

                    if($i && $i==$count)
                        $msg = 'Selected emails deleted successfully';
                    elseif($i>0)
                        $warn = "$i of $count selected emails deleted";
                    elseif(!$errors['err'])
                        $errors['err'] = 'Unable to delete selected emails';
                    
                } else {
                    $errors['err'] = 'Unknown action - get technical help';
                }
            }
            break;
        default:
            $errors['err'] = 'Unknown action/command';
            break;
    }
}

// @implements FS-040.1: Email Account Listing — list partial routing
// @implements FS-040.2: Email Account Create / Edit Form — form partial routing
$page='emails.inc.php';
if($email || ($_REQUEST['a'] && !strcasecmp($_REQUEST['a'],'add')))
    $page='email.inc.php';

$nav->setTabActive('emails');
require(STAFFINC_DIR.'header.inc.php');
require(STAFFINC_DIR.$page);
include(STAFFINC_DIR.'footer.inc.php');
?>
