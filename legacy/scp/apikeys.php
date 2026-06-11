<?php
/*********************************************************************
    apikeys.php

    API keys.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
require('admin.inc.php');
include_once(INCLUDE_DIR.'class.api.php');

// @implements FS-043.2: API Key Administration Screens — key id lookup, "Unknown or invalid API key ID." on miss
$api=null;
if($_REQUEST['id'] && !($api=API::lookup($_REQUEST['id'])))
    $errors['err']='Unknown or invalid API key ID.';

// @implements FS-043.2: API Key Administration Screens — POST dispatch
if($_POST){
    switch(strtolower($_POST['do'])){
        // @implements FS-043.1: API Key Entity & Auto-Generated Secret — edit key (status/flags/notes only; IP+secret immutable)
        case 'update':
            if(!$api){
                $errors['err']='Unknown or invalid API key.';
            }elseif($api->update($_POST,$errors)){
                $msg='API key updated successfully';
            }elseif(!$errors['err']){
                $errors['err']='Error updating API key. Try again!';
            }
            break;
        // @implements FS-043.1: API Key Entity & Auto-Generated Secret — add key with required bound IP + auto-generated secret
        case 'add':
            if(($id=API::add($_POST,$errors))){
                $msg='API key added successfully';
                $_REQUEST['a']=null;
            }elseif(!$errors['err']){
                $errors['err']='Unable to add an API key. Correct error(s) below and try again.';
            }
            break;
        // @implements FS-043.2: API Key Administration Screens — bulk enable/disable/delete keys
        case 'mass_process':
            if(!$_POST['ids'] || !is_array($_POST['ids']) || !count($_POST['ids'])) {
                $errors['err'] = 'You must select at least one API key';
            } else {
                $count=count($_POST['ids']);
                switch(strtolower($_POST['a'])) {
                    case 'enable':
                        $sql='UPDATE '.API_KEY_TABLE.' SET isactive=1 '
                            .' WHERE id IN ('.implode(',', db_input($_POST['ids'])).')';
                        if(db_query($sql) && ($num=db_affected_rows())) {
                            if($num==$count)
                                $msg = 'Selected API keys enabled';
                            else
                                $warn = "$num of $count selected API keys enabled";
                        } else {
                            $errors['err'] = 'Unable to enable selected API keys.';
                        }
                        break;
                    case 'disable':
                        $sql='UPDATE '.API_KEY_TABLE.' SET isactive=0 '
                            .' WHERE id IN ('.implode(',', db_input($_POST['ids'])).')';
                        if(db_query($sql) && ($num=db_affected_rows())) {
                            if($num==$count)
                                $msg = 'Selected API keys disabled';
                            else
                                $warn = "$num of $count selected API keys disabled";
                        } else {
                            $errors['err']='Unable to disable selected API keys';
                        }
                        break;
                    case 'delete':
                        $i=0;
                        foreach($_POST['ids'] as $k=>$v) {
                            if(($t=API::lookup($v)) && $t->delete())
                                $i++;
                        }
                        if($i && $i==$count)
                            $msg = 'Selected API keys deleted successfully';
                        elseif($i>0)
                            $warn = "$i of $count selected API keys deleted";
                        elseif(!$errors['err'])
                            $errors['err'] = 'Unable to delete selected API keys';
                        break;
                    default:
                        $errors['err']='Unknown action - get technical help';
                }
            }
            break;
        default:
            $errors['err']='Unknown action/command';
            break;
    }
}

// @implements FS-043.2: API Key Administration Screens — list vs. add/edit form routing
$page='apikeys.inc.php';
if($api || ($_REQUEST['a'] && !strcasecmp($_REQUEST['a'],'add')))
    $page='apikey.inc.php';

$nav->setTabActive('manage');
require(STAFFINC_DIR.'header.inc.php');
require(STAFFINC_DIR.$page);
include(STAFFINC_DIR.'footer.inc.php');
?>
