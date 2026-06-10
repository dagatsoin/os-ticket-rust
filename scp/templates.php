<?php
/*********************************************************************
    templates.php

    Email Templates

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
require('admin.inc.php');
include_once(INCLUDE_DIR.'class.template.php');
// @implements FS-040.5: Email Template Set Listing — template group lookup, "Unknown or invalid template group ID." on miss
// @implements FS-040.7: Per-Message Template Editing — individual template lookup, "Unknown or invalid template ID." on miss
$template=null;
if($_REQUEST['tpl_id'] &&
        !($template=EmailTemplateGroup::lookup($_REQUEST['tpl_id'])))
    $errors['err']='Unknown or invalid template group ID.';
elseif($_REQUEST['id'] &&
        !($template=EmailTemplate::lookup($_REQUEST['id'])))
    $errors['err']='Unknown or invalid template ID.';

// @implements FS-040.7: Per-Message Template Editing — POST dispatch
if($_POST){
    switch(strtolower($_POST['do'])){
        // @implements FS-040.7: Per-Message Template Editing — update an existing message template
        case 'updatetpl':
            if(!$template){
                $errors['err']='Unknown or invalid template';
            }elseif($template->update($_POST,$errors)){
                $template->reload();
                $msg='Message template updated successfully';
            }elseif(!$errors['err']){
                $errors['err']='Error updating message template. Try again!';
            }
            break;
        // @implements FS-040.7: Per-Message Template Editing — implement (add) a not-yet-customized message template
        case 'implement':
            if(!$template){
                $errors['err']='Unknown or invalid template';
            }elseif($new = EmailTemplate::add($_POST,$errors)){
                $template = $new;
                $msg='Message template updated successfully';
            }elseif(!$errors['err']){
                $errors['err']='Error updating message template. Try again!';
            }
            break;
        // @implements FS-040.5: Email Template Set Listing — update template group properties
        case 'update':
            if(!$template){
                $errors['err']='Unknown or invalid template';
            }elseif($template->update($_POST,$errors)){
                $msg='Template updated successfully';
            }elseif(!$errors['err']){
                $errors['err']='Error updating template. Try again!';
            }
            break;
        // @implements FS-040.8: Template Set Create (Clone-Based) — add a new template set cloned from an existing one
        case 'add':
            if(($new=EmailTemplateGroup::add($_POST,$errors))){
                $template=$new;
                $msg='Template added successfully';
                $_REQUEST['a']=null;
            }elseif(!$errors['err']){
                $errors['err']='Unable to add template. Correct error(s) below and try again.';
            }
            break;
        // @implements FS-040.9: Template Set Bulk Actions (Enable / Disable / Delete) — in-use sets skipped on disable/delete
        case 'mass_process':
            if(!$_POST['ids'] || !is_array($_POST['ids']) || !count($_POST['ids'])) {
                $errors['err']='You must select at least one template to process.';
            } else {
                $count=count($_POST['ids']);
                switch(strtolower($_POST['a'])) {
                    case 'enable':
                        $sql='UPDATE '.EMAIL_TEMPLATE_GRP_TABLE.' SET isactive=1 '
                            .' WHERE tpl_id IN ('.implode(',', db_input($_POST['ids'])).')';
                        if(db_query($sql) && ($num=db_affected_rows())){
                            if($num==$count)
                                $msg = 'Selected templates enabled';
                            else
                                $warn = "$num of $count selected templates enabled";
                        } else {
                            $errors['err'] = 'Unable to enable selected templates';
                        }
                        break;
                    case 'disable':
                        $i=0;
                        foreach($_POST['ids'] as $k=>$v) {
                            if(($t=EmailTemplateGroup::lookup($v)) && !$t->isInUse() && $t->disable())
                                $i++;
                        }
                        if($i && $i==$count)
                            $msg = 'Selected templates disabled';
                        elseif($i)
                            $warn = "$i of $count selected templates disabled (in-use templates can't be disabled)";
                        else
                            $errors['err'] = "Unable to disable selected templates (in-use or default template can't be disabled)";
                        break;
                    case 'delete':
                        $i=0;
                        foreach($_POST['ids'] as $k=>$v) {
                            if(($t=EmailTemplateGroup::lookup($v)) && !$t->isInUse() && $t->delete())
                                $i++;
                        }

                        if($i && $i==$count)
                            $msg = 'Selected templates deleted successfully';
                        elseif($i>0)
                            $warn = "$i of $count selected templates deleted";
                        elseif(!$errors['err'])
                            $errors['err'] = 'Unable to delete selected templates';
                        break;
                    default:
                        $errors['err']='Unknown template action';
                }
            }
            break;
        default:
            $errors['err']='Unknown action';
            break;
    }
}

// @implements FS-040.5: Email Template Set Listing — list partial routing
// @implements FS-040.6: Template Set Manage View (Message List) — manage/implement view routing
$page='templates.inc.php';
if($template && !strcasecmp($_REQUEST['a'],'manage')){
    $page='tpl.inc.php';
}elseif($template && !strcasecmp($_REQUEST['a'],'implement')){
    $page='tpl.inc.php';
}elseif($template || !strcasecmp($_REQUEST['a'],'add')){
    $page='template.inc.php';
}

$nav->setTabActive('emails');
require(STAFFINC_DIR.'header.inc.php');
require(STAFFINC_DIR.$page);
include(STAFFINC_DIR.'footer.inc.php');
?>
