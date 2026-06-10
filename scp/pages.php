<?php
/*********************************************************************
    pages.php

    Site pages.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
require('admin.inc.php');
require_once(INCLUDE_DIR.'class.page.php');

// @implements FS-033.11: Site Pages List & Access Gate — page lookup, "Unknown or invalid page" on miss
// @implements FS-033.13: Create & Edit a Site Page — resolve page id for edit
$page = null;
if($_REQUEST['id'] && !($page=Page::lookup($_REQUEST['id'])))
   $errors['err']='Unknown or invalid page';

// @implements FS-033.13: Create & Edit a Site Page — POST dispatch (add/update)
if($_POST) {
    switch(strtolower($_POST['do'])) {
        // @implements FS-033.13: Create & Edit a Site Page — create new page
        case 'add':
            if(($pageId=Page::create($_POST, $errors))) {
                $_REQUEST['a'] = null;
                $msg='Page added successfully';
            } elseif(!$errors['err'])
                $errors['err'] = 'Unable to add page. Try again!';
        break;
        // @implements FS-033.13: Create & Edit a Site Page — update existing page
        case 'update':
            if(!$page)
                $errors['err'] = 'Invalid or unknown page';
            elseif($page->update($_POST, $errors)) {
                $msg='Page updated successfully';
                $_REQUEST['a']=null; //Go back to view
            } elseif(!$errors['err'])
                $errors['err'] = 'Unable to update page. Try again!';
            break;
        // @implements FS-033.15: Enable / Disable / Delete Pages (Bulk Actions) — mass enable/disable/delete with in-use protection
        case 'mass_process':
            if(!$_POST['ids'] || !is_array($_POST['ids']) || !count($_POST['ids'])) {
                $errors['err'] = 'You must select at least one page.';
            } elseif(array_intersect($_POST['ids'], $cfg->getDefaultPages()) && strcasecmp($_POST['a'], 'enable')) {
                 $errors['err'] = 'One or more of the selected pages is in-use and CANNOT be disabled/deleted.';
            } else {
                $count=count($_POST['ids']);
                switch(strtolower($_POST['a'])) {
                    case 'enable':
                        $sql='UPDATE '.PAGE_TABLE.' SET isactive=1 '
                            .' WHERE id IN ('.implode(',', db_input($_POST['ids'])).')';
                        if(db_query($sql) && ($num=db_affected_rows())) {
                            if($num==$count)
                                $msg = 'Selected pages enabled';
                            else
                                $warn = "$num of $count selected pages enabled";
                        } else {
                            $errors['err'] = 'Unable to enable selected pages';
                        }
                        break;
                    case 'disable':
                        $i = 0;
                        foreach($_POST['ids'] as $k=>$v) {
                            if(($p=Page::lookup($v)) && $p->disable())
                                $i++;
                        }

                        if($i && $i==$count)
                            $msg = 'Selected pages disabled';
                        elseif($i>0)
                            $warn = "$num of $count selected pages disabled";
                        elseif(!$errors['err'])
                            $errors['err'] = 'Unable to disable selected pages';
                        break;
                    case 'delete':
                        $i=0;
                        foreach($_POST['ids'] as $k=>$v) {
                            if(($p=Page::lookup($v)) && $p->delete())
                                $i++;
                        }

                        if($i && $i==$count)
                            $msg = 'Selected pages deleted successfully';
                        elseif($i>0)
                            $warn = "$i of $count selected pages deleted";
                        elseif(!$errors['err'])
                            $errors['err'] = 'Unable to delete selected pages';
                        break;
                    default:
                        $errors['err']='Unknown action - get technical help.';
                }
            }
            break;
        default:
            $errors['err']='Unknown action/command';
            break;
    }
}

// @implements FS-033.12: Site Pages Results Table, Sorting & Pagination — list view routing
// @implements FS-033.13: Create & Edit a Site Page — add/edit form routing
$inc='pages.inc.php';
if($page || $_REQUEST['a']=='add')
    $inc='page.inc.php';

$nav->setTabActive('manage');
require_once(STAFFINC_DIR.'header.inc.php');
require_once(STAFFINC_DIR.$inc);
require_once(STAFFINC_DIR.'footer.inc.php');
?>
