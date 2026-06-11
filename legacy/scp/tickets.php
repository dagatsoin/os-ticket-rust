<?php
/*************************************************************************
    tickets.php

    Handles all tickets related actions.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

require('staff.inc.php');
require_once(INCLUDE_DIR.'class.ticket.php');
require_once(INCLUDE_DIR.'class.dept.php');
require_once(INCLUDE_DIR.'class.filter.php');
require_once(INCLUDE_DIR.'class.canned.php');


$page='';
$ticket=null; //clean start.
//LOCKDOWN...See if the id provided is actually valid and if the user has access.
// @implements FS-021.1: Ticket Lookup, Access Gate & Page Routing — resolve ticket by internal id, "Unknown or invalid ticket ID"
// @implements FS-021.2: Staff Access Check (checkStaffAccess) — clear ticket + access-denied error on failed access
if($_REQUEST['id']) {
    if(!($ticket=Ticket::lookup($_REQUEST['id'])))
         $errors['err']='Unknown or invalid ticket ID';
    elseif(!$ticket->checkStaffAccess($thisstaff)) {
        $errors['err']='Access denied. Contact admin if you believe this is in error';
        $ticket=null; //Clear ticket obj.
    }
}
//At this stage we know the access status. we can process the post.
// @implements FS-021.1: Ticket Lookup, Access Gate & Page Routing — dispatch POST to per-action handlers once access is established
if($_POST && !$errors):

    // @implements FS-021.1: Ticket Lookup, Access Gate & Page Routing — per-ticket action switch (reply/transfer/assign/...)
    if($ticket && $ticket->getId()) {
        //More coffee please.
        $errors=array();
        $lock=$ticket->getLock(); //Ticket lock if any
        $statusKeys=array('open'=>'Open','Reopen'=>'Open','Close'=>'Closed');
        switch(strtolower($_POST['a'])):
        // @implements FS-021.3: Post Reply to Requester (reply -> postReply) — lock/ban guards, email-reply, close/reopen-on-reply
        // @implements BS-021.18: Reply Status Checkbox Can Close or Reopen On Reply
        case 'reply':
            if(!$thisstaff->canPostReply())
                $errors['err'] = 'Action denied. Contact admin for access';
            else {

                if(!$_POST['response'])
                    $errors['response']='Response required';
                //Use locks to avoid double replies
                if($lock && $lock->getStaffId()!=$thisstaff->getId())
                    $errors['err']='Action Denied. Ticket is locked by someone else!';

                //Make sure the email is not banned
                if(!$errors['err'] && TicketFilter::isBanned($ticket->getEmail()))
                    $errors['err']='Email is in banlist. Must be removed to reply.';
            }

            $wasOpen =($ticket->isOpen());

            //If no error...do the do.
            $vars = $_POST;
            if(!$errors && $_FILES['attachments'])
                $vars['files'] = AttachmentFile::format($_FILES['attachments']);

            if(!$errors && ($response=$ticket->postReply($vars, $errors, isset($_POST['emailreply'])))) {
                $msg='Reply posted successfully';
                $ticket->reload();
                if($ticket->isClosed() && $wasOpen)
                    $ticket=null;

            } elseif(!$errors['err']) {
                $errors['err']='Unable to post the reply. Correct the errors below and try again!';
            }
            break;
        // @implements FS-021.10: Transfer Between Departments (transfer -> transfer) — dept change, SLA re-select, access re-check
        case 'transfer': /** Transfer ticket **/
            //Check permission
            if(!$thisstaff->canTransferTickets())
                $errors['err']=$errors['transfer'] = 'Action Denied. You are not allowed to transfer tickets.';
            else {

                //Check target dept.
                if(!$_POST['deptId'])
                    $errors['deptId'] = 'Select department';
                elseif($_POST['deptId']==$ticket->getDeptId())
                    $errors['deptId'] = 'Ticket already in the department';
                elseif(!($dept=Dept::lookup($_POST['deptId'])))
                    $errors['deptId'] = 'Unknown or invalid department';

                //Transfer message - required.
                if(!$_POST['transfer_comments'])
                    $errors['transfer_comments'] = 'Transfer comments required';
                elseif(strlen($_POST['transfer_comments'])<5)
                    $errors['transfer_comments'] = 'Transfer comments too short!';

                //If no errors - them attempt the transfer.
                if(!$errors && $ticket->transfer($_POST['deptId'], $_POST['transfer_comments'])) {
                    $msg = 'Ticket transferred successfully to '.$ticket->getDeptName();
                    //Check to make sure the staff still has access to the ticket
                    if(!$ticket->checkStaffAccess($thisstaff))
                        $ticket=null;

                } elseif(!$errors['transfer']) {
                    $errors['err'] = 'Unable to complete the ticket transfer';
                    $errors['transfer']='Correct the error(s) below and try again!';
                }
            }
            break;
        // @implements FS-021.7: Assign / Reassign Ticket (assign -> assign) — staff/team/claim prefix decode, comments, alerts
        // @implements BS-021.9: No Self-Assignment Alerts
        case 'assign':

             if(!$thisstaff->canAssignTickets())
                 $errors['err']=$errors['assign'] = 'Action Denied. You are not allowed to assign/reassign tickets.';
             else {

                 $id = preg_replace("/[^0-9]/", "",$_POST['assignId']);
                 $claim = (is_numeric($_POST['assignId']) && $_POST['assignId']==$thisstaff->getId());

                 if(!$_POST['assignId'] || !$id)
                     $errors['assignId'] = 'Select assignee';
                 elseif($_POST['assignId'][0]!='s' && $_POST['assignId'][0]!='t' && !$claim)
                     $errors['assignId']='Invalid assignee ID - get technical support';
                 elseif($ticket->isAssigned()) {
                     if($_POST['assignId'][0]=='s' && $id==$ticket->getStaffId())
                         $errors['assignId']='Ticket already assigned to the staff.';
                     elseif($_POST['assignId'][0]=='t' && $id==$ticket->getTeamId())
                         $errors['assignId']='Ticket already assigned to the team.';
                 }

                 //Comments are not required on self-assignment (claim)
                 if($claim && !$_POST['assign_comments'])
                     $_POST['assign_comments'] = 'Ticket claimed by '.$thisstaff->getName();
                 elseif(!$_POST['assign_comments'])
                     $errors['assign_comments'] = 'Assignment comments required';
                 elseif(strlen($_POST['assign_comments'])<5)
                         $errors['assign_comments'] = 'Comment too short';

                 if(!$errors && $ticket->assign($_POST['assignId'], $_POST['assign_comments'], !$claim)) {
                     if($claim) {
                         $msg = 'Ticket is NOW assigned to you!';
                     } else {
                         $msg='Ticket assigned successfully to '.$ticket->getAssigned();
                         TicketLock::removeStaffLocks($thisstaff->getId(), $ticket->getId());
                         $ticket=null;
                     }
                 } elseif(!$errors['assign']) {
                     $errors['err'] = 'Unable to complete the ticket assignment';
                     $errors['assign'] = 'Correct the error(s) below and try again!';
                 }
             }
            break;
        // @implements FS-021.4: Post Internal Note (postnote -> postNote) — note body + optional manager-gated state change
        // @implements BS-021.13: Manual Overdue/Answered/State Flags Are Manager-Only
        case 'postnote': /* Post Internal Note */
            //Make sure the staff can set desired state
            if($_POST['state']) {
                if($_POST['state']=='closed' && !$thisstaff->canCloseTickets())
                    $errors['state'] = "You don't have permission to close tickets";
                elseif(in_array($_POST['state'], array('overdue', 'notdue', 'unassigned'))
                        && (!($dept=$ticket->getDept()) || !$dept->isManager($thisstaff)))
                    $errors['state'] = "You don't have permission to set the state";
            }

            $wasOpen = ($ticket->isOpen());

            $vars = $_POST;
            if($_FILES['attachments'])
                $vars['files'] = AttachmentFile::format($_FILES['attachments']);

            if(($note=$ticket->postNote($vars, $errors, $thisstaff))) {

                $msg='Internal note posted successfully';
                if($wasOpen && $ticket->isClosed())
                    $ticket = null; //Going back to main listing.

            } else {

                if(!$errors['err'])
                    $errors['err'] = 'Unable to post internal note - missing or invalid data.';

                $errors['postnote'] = 'Unable to post the note. Correct the error(s) below and try again!';
            }
            break;
        // @implements FS-021.15: Edit Ticket Properties (a=edit -> update) — validated field edit + access re-check
        case 'edit':
        case 'update':
            if(!$ticket || !$thisstaff->canEditTickets())
                $errors['err']='Perm. Denied. You are not allowed to edit tickets';
            elseif($ticket->update($_POST,$errors)) {
                $msg='Ticket updated successfully';
                $_REQUEST['a'] = null; //Clear edit action - going back to view.
                //Check to make sure the staff STILL has access post-update (e.g dept change).
                if(!$ticket->checkStaffAccess($thisstaff))
                    $ticket=null;
            } elseif(!$errors['err']) {
                $errors['err']='Unable to update the ticket. Correct the errors below and try again!';
            }
            break;
        // @implements FS-021.1: Ticket Lookup, Access Gate & Page Routing — process sub-action dispatcher (close/reopen/...)
        case 'process':
            switch(strtolower($_POST['do'])):
                // @implements FS-021.11: Close Ticket (process/close -> close) — close, clear overdue/due, credit closer
                // @implements BS-021.4: Closing Clears Overdue & Due Date and Credits the Closer
                case 'close':
                    if(!$thisstaff->canCloseTickets()) {
                        $errors['err'] = 'Perm. Denied. You are not allowed to close tickets.';
                    } elseif($ticket->isClosed()) {
                        $errors['err'] = 'Ticket is already closed!';
                    } elseif($ticket->close()) {
                        $msg='Ticket #'.$ticket->getExtId().' status set to CLOSED';
                        //Log internal note
                        if($_POST['ticket_status_notes'])
                            $note = $_POST['ticket_status_notes'];
                        else
                            $note='Ticket closed (without comments)';

                        $ticket->logNote('Ticket Closed', $note, $thisstaff);

                        //Going back to main listing.
                        TicketLock::removeStaffLocks($thisstaff->getId(), $ticket->getId());
                        $page=$ticket=null;

                    } else {
                        $errors['err']='Problems closing the ticket. Try again';
                    }
                    break;
                // @implements FS-021.12: Reopen Ticket (process/reopen -> reopen) — reopen, annul prior close event
                // @implements BS-021.14: Reopen Annuls the Prior Close Event
                case 'reopen':
                    //if staff can close or create tickets ...then assume they can reopen.
                    if(!$thisstaff->canCloseTickets() && !$thisstaff->canCreateTickets()) {
                        $errors['err']='Perm. Denied. You are not allowed to reopen tickets.';
                    } elseif($ticket->isOpen()) {
                        $errors['err'] = 'Ticket is already open!';
                    } elseif($ticket->reopen()) {
                        $msg='Ticket REOPENED';

                        if($_POST['ticket_status_notes'])
                            $note = $_POST['ticket_status_notes'];
                        else
                            $note='Ticket reopened (without comments)';

                        $ticket->logNote('Ticket Reopened', $note, $thisstaff);

                    } else {
                        $errors['err']='Problems reopening the ticket. Try again';
                    }
                    break;
                // @implements FS-021.9: Release / Unassign Ticket (process/release -> release/unassign) — manager-only unassign
                case 'release':
                    if(!$ticket->isAssigned() || !($assigned=$ticket->getAssigned())) {
                        $errors['err'] = 'Ticket is not assigned!';
                    } elseif($ticket->release()) {
                        $msg='Ticket released (unassigned) from '.$assigned;
                        $ticket->logActivity('Ticket unassigned',$msg.' by '.$thisstaff->getName());
                    } else {
                        $errors['err'] = 'Problems releasing the ticket. Try again';
                    }
                    break;
                // @implements FS-021.8: Claim Ticket (process/claim) — self-assign an open, unassigned ticket (no alert)
                case 'claim':
                    if(!$thisstaff->canAssignTickets()) {
                        $errors['err'] = 'Perm. Denied. You are not allowed to assign/claim tickets.';
                    } elseif(!$ticket->isOpen()) {
                        $errors['err'] = 'Only open tickets can be assigned';
                    } elseif($ticket->isAssigned()) {
                        $errors['err'] = 'Ticket is already assigned to '.$ticket->getAssigned();
                    } elseif($ticket->assignToStaff($thisstaff->getId(), ('Ticket claimed by '.$thisstaff->getName()), false)) {
                        $msg = 'Ticket is now assigned to you!';
                    } else {
                        $errors['err'] = 'Problems assigning the ticket. Try again';
                    }
                    break;
                // @implements FS-021.13: Due Date, SLA Selection & Overdue Marking — manual mark-overdue (manager-only, idempotent)
                // @implements BS-021.21: Mark-Overdue / Mark-Answered Are Idempotent
                case 'overdue':
                    $dept = $ticket->getDept();
                    if(!$dept || !$dept->isManager($thisstaff)) {
                        $errors['err']='Perm. Denied. You are not allowed to flag tickets overdue';
                    } elseif($ticket->markOverdue()) {
                        $msg='Ticket flagged as overdue';
                        $ticket->logActivity('Ticket Marked Overdue',($msg.' by '.$thisstaff->getName()));
                    } else {
                        $errors['err']='Problems marking the the ticket overdue. Try again';
                    }
                    break;
                // @implements FS-021.13: Due Date, SLA Selection & Overdue Marking — manager-only mark-answered (idempotent)
                case 'answered':
                    $dept = $ticket->getDept();
                    if(!$dept || !$dept->isManager($thisstaff)) {
                        $errors['err']='Perm. Denied. You are not allowed to flag tickets';
                    } elseif($ticket->markAnswered()) {
                        $msg='Ticket flagged as answered';
                        $ticket->logActivity('Ticket Marked Answered',($msg.' by '.$thisstaff->getName()));
                    } else {
                        $errors['err']='Problems marking the the ticket answered. Try again';
                    }
                    break;
                // @implements FS-021.13: Due Date, SLA Selection & Overdue Marking — manager-only mark-unanswered (idempotent)
                case 'unanswered':
                    $dept = $ticket->getDept();
                    if(!$dept || !$dept->isManager($thisstaff)) {
                        $errors['err']='Perm. Denied. You are not allowed to flag tickets';
                    } elseif($ticket->markUnAnswered()) {
                        $msg='Ticket flagged as unanswered';
                        $ticket->logActivity('Ticket Marked Unanswered',($msg.' by '.$thisstaff->getName()));
                    } else {
                        $errors['err']='Problems marking the the ticket unanswered. Try again';
                    }
                    break;
                // @implements FS-021.14: Ban / Unban Requester E-mail (process/banemail) — per-ticket ban action
                // @implements FS-042.12: Per-Ticket Ban / Unban Staff Action — add owner email to SYSTEM BAN LIST
                case 'banemail':
                    if(!$thisstaff->canBanEmails()) {
                        $errors['err']='Perm. Denied. You are not allowed to ban emails';
                    } elseif(BanList::includes($ticket->getEmail())) {
                        $errors['err']='Email already in banlist';
                    } elseif(Banlist::add($ticket->getEmail(),$thisstaff->getName())) {
                        $msg='Email ('.$ticket->getEmail().') added to banlist';
                    } else {
                        $errors['err']='Unable to add the email to banlist';
                    }
                    break;
                // @implements FS-021.14: Ban / Unban Requester E-mail (process/unbanemail) — per-ticket unban action
                // @implements FS-042.12: Per-Ticket Ban / Unban Staff Action — remove owner email from SYSTEM BAN LIST
                case 'unbanemail':
                    if(!$thisstaff->canBanEmails()) {
                        $errors['err'] = 'Perm. Denied. You are not allowed to remove emails from banlist.';
                    } elseif(Banlist::remove($ticket->getEmail())) {
                        $msg = 'Email removed from banlist';
                    } elseif(!BanList::includes($ticket->getEmail())) {
                        $warn = 'Email is not in the banlist';
                    } else {
                        $errors['err']='Unable to remove the email from banlist. Try again.';
                    }
                    break;
                // @implements FS-021.19: Delete Ticket (process/delete -> delete) — permanent delete + thread/attachment cascade
                // @implements BS-021.16: Deletion Is Permanent and Cascades to Thread & Attachments
                case 'delete': // Dude what are you trying to hide? bad customer support??
                    if(!$thisstaff->canDeleteTickets()) {
                        $errors['err']='Perm. Denied. You are not allowed to DELETE tickets!!';
                    } elseif($ticket->delete()) {
                        $msg='Ticket #'.$ticket->getNumber().' deleted successfully';
                        //Log a debug note
                        $ost->logDebug('Ticket #'.$ticket->getNumber().' deleted',
                                sprintf('Ticket #%s deleted by %s',
                                    $ticket->getNumber(), $thisstaff->getName())
                                );
                        $ticket=null; //clear the object.
                    } else {
                        $errors['err']='Problems deleting the ticket. Try again';
                    }
                    break;
                default:
                    $errors['err']='You must select action to perform';
            endswitch;
            break;
        default:
            $errors['err']='Unknown action';
        endswitch;
        if($ticket && is_object($ticket))
            $ticket->reload();//Reload ticket info following post processing
    }elseif($_POST['a']) {

        switch($_POST['a']) {
            // @implements FS-020.9: Mass / Bulk Actions From the Queue — selection + gating + queue→action mapping
            // @implements FS-021.21: Mass Ticket Actions From the View Script (mass_process)
            case 'mass_process':
                if(!$thisstaff->canManageTickets())
                    $errors['err']='You do not have permission to mass manage tickets. Contact admin for such access';
                elseif(!$_POST['tids'] || !is_array($_POST['tids']))
                    $errors['err']='No tickets selected. You must select at least one ticket.';
                else {
                    $count=count($_POST['tids']);
                    $i = 0;
                    switch(strtolower($_POST['do'])) {
                        // @implements FS-021.21: Mass Ticket Actions From the View Script — bulk reopen (close-or-create perm)
                        // @implements BS-020.11: Per-Action Permission Re-Check On Bulk
                        case 'reopen':
                            if($thisstaff->canCloseTickets() || $thisstaff->canCreateTickets()) {
                                $note='Ticket reopened by '.$thisstaff->getName();
                                foreach($_POST['tids'] as $k=>$v) {
                                    if(($t=Ticket::lookup($v)) && $t->isClosed() && @$t->reopen()) {
                                        $i++;
                                        $t->logNote('Ticket Reopened', $note, $thisstaff);
                                    }
                                }

                                if($i==$count)
                                    $msg = "Selected tickets ($i) reopened successfully";
                                elseif($i)
                                    $warn = "$i of $count selected tickets reopened";
                                else
                                    $errors['err'] = 'Unable to reopen selected tickets';
                            } else {
                                $errors['err'] = 'You do not have permission to reopen tickets';
                            }
                            break;
                        // @implements FS-021.21: Mass Ticket Actions From the View Script — bulk close (close perm)
                        case 'close':
                            if($thisstaff->canCloseTickets()) {
                                $note='Ticket closed without response by '.$thisstaff->getName();
                                foreach($_POST['tids'] as $k=>$v) {
                                    if(($t=Ticket::lookup($v)) && $t->isOpen() && @$t->close()) {
                                        $i++;
                                        $t->logNote('Ticket Closed', $note, $thisstaff);
                                    }
                                }

                                if($i==$count)
                                    $msg ="Selected tickets ($i) closed succesfully";
                                elseif($i)
                                    $warn = "$i of $count selected tickets closed";
                                else
                                    $errors['err'] = 'Unable to close selected tickets';
                            } else {
                                $errors['err'] = 'You do not have permission to close tickets';
                            }
                            break;
                        // @implements FS-021.21: Mass Ticket Actions From the View Script — bulk mark-overdue
                        case 'mark_overdue':
                            $note='Ticket flagged as overdue by '.$thisstaff->getName();
                            foreach($_POST['tids'] as $k=>$v) {
                                if(($t=Ticket::lookup($v)) && !$t->isOverdue() && $t->markOverdue()) {
                                    $i++;
                                    $t->logNote('Ticket Marked Overdue', $note, $thisstaff);
                                }
                            }

                            if($i==$count)
                                $msg = "Selected tickets ($i) marked overdue";
                            elseif($i)
                                $warn = "$i of $count selected tickets marked overdue";
                            else
                                $errors['err'] = 'Unable to flag selected tickets as overdue';
                            break;
                        // @implements FS-021.21: Mass Ticket Actions From the View Script — bulk delete (delete perm) + warning log
                        case 'delete':
                            if($thisstaff->canDeleteTickets()) {
                                foreach($_POST['tids'] as $k=>$v) {
                                    if(($t=Ticket::lookup($v)) && @$t->delete()) $i++;
                                }

                                //Log a warning
                                if($i) {
                                    $log = sprintf('%s (%s) just deleted %d ticket(s)',
                                            $thisstaff->getName(), $thisstaff->getUserName(), $i);
                                    $ost->logWarning('Tickets deleted', $log, false);

                                }

                                if($i==$count)
                                    $msg = "Selected tickets ($i) deleted successfully";
                                elseif($i)
                                    $warn = "$i of $count selected tickets deleted";
                                else
                                    $errors['err'] = 'Unable to delete selected tickets';
                            } else {
                                $errors['err'] = 'You do not have permission to delete tickets';
                            }
                            break;
                        default:
                            $errors['err']='Unknown or unsupported action - get technical help';
                    }
                }
                break;
            // @implements FS-021.20: Staff-Initiated (Phone) New Ticket (a=open -> Ticket::open) — staff create path
            // @implements BS-021.15: Staff-Created Tickets Default to No Auto-Response
            case 'open':
                $ticket=null;
                if(!$thisstaff || !$thisstaff->canCreateTickets()) {
                     $errors['err']='You do not have permission to create tickets. Contact admin for such access';
                } else {
                    $vars = $_POST;
                    if($_FILES['attachments'])
                        $vars['files'] = AttachmentFile::format($_FILES['attachments']);

                    if(($ticket=Ticket::open($vars, $errors))) {
                        $msg='Ticket created successfully';
                        $_REQUEST['a']=null;
                        if(!$ticket->checkStaffAccess($thisstaff) || $ticket->isClosed())
                            $ticket=null;
                    } elseif(!$errors['err']) {
                        $errors['err']='Unable to create the ticket. Correct the error(s) and try again';
                    }
                }
                break;
        }
    }
    if(!$errors)
        $thisstaff ->resetStats(); //We'll need to reflect any changes just made!
endif;

/*... Quick stats ...*/
// @implements FS-020.11: Quick Ticket Stats — compute the acting staff's quick ticket counts
$stats= $thisstaff->getTicketsStats();

//Navigation
// @implements FS-020.2: Predefined Queues (Status Tabs) — assemble queue sub-menu from quick stats
// @implements BS-020.14: Quick-Stat Warning & Notice Thresholds — >10 assigned/overdue warnings
$nav->setTabActive('tickets');
if($cfg->showAnsweredTickets()) {
    $nav->addSubMenu(array('desc'=>'Open ('.number_format($stats['open']+$stats['answered']).')',
                            'title'=>'Open Tickets',
                            'href'=>'tickets.php',
                            'iconclass'=>'Ticket'),
                        (!$_REQUEST['status'] || $_REQUEST['status']=='open'));
} else {

    if($stats) {
        $nav->addSubMenu(array('desc'=>'Open ('.number_format($stats['open']).')',
                               'title'=>'Open Tickets',
                               'href'=>'tickets.php',
                               'iconclass'=>'Ticket'),
                            (!$_REQUEST['status'] || $_REQUEST['status']=='open'));
    }

    if($stats['answered']) {
        $nav->addSubMenu(array('desc'=>'Answered ('.number_format($stats['answered']).')',
                               'title'=>'Answered Tickets',
                               'href'=>'tickets.php?status=answered',
                               'iconclass'=>'answeredTickets'),
                            ($_REQUEST['status']=='answered'));
    }
}

if($stats['assigned']) {
    if(!$ost->getWarning() && $stats['assigned']>10)
        $ost->setWarning($stats['assigned'].' tickets assigned to you! Do something about it!');

    $nav->addSubMenu(array('desc'=>'My Tickets ('.number_format($stats['assigned']).')',
                           'title'=>'Assigned Tickets',
                           'href'=>'tickets.php?status=assigned',
                           'iconclass'=>'assignedTickets'),
                        ($_REQUEST['status']=='assigned'));
}

if($stats['overdue']) {
    $nav->addSubMenu(array('desc'=>'Overdue ('.number_format($stats['overdue']).')',
                           'title'=>'Stale Tickets',
                           'href'=>'tickets.php?status=overdue',
                           'iconclass'=>'overdueTickets'),
                        ($_REQUEST['status']=='overdue'));

    if(!$sysnotice && $stats['overdue']>10)
        $sysnotice=$stats['overdue'] .' overdue tickets!';
}

if($thisstaff->showAssignedOnly() && $stats['closed']) {
    $nav->addSubMenu(array('desc'=>'My Closed Tickets ('.number_format($stats['closed']).')',
                           'title'=>'My Closed Tickets',
                           'href'=>'tickets.php?status=closed',
                           'iconclass'=>'closedTickets'),
                        ($_REQUEST['status']=='closed'));
} else {

    $nav->addSubMenu(array('desc'=>'Closed Tickets ('.number_format($stats['closed']).')',
                           'title'=>'Closed Tickets',
                           'href'=>'tickets.php?status=closed',
                           'iconclass'=>'closedTickets'),
                        ($_REQUEST['status']=='closed'));
}

if($thisstaff->canCreateTickets()) {
    $nav->addSubMenu(array('desc'=>'New Ticket',
                           'href'=>'tickets.php?a=open',
                           'iconclass'=>'newTicket'),
                        ($_REQUEST['a']=='open'));
}


// @implements FS-021.1: Ticket Lookup, Access Gate & Page Routing — view-routing (view/edit/print vs listing)
$inc = 'tickets.inc.php';
if($ticket) {
    // @implements FS-021.1: Ticket Lookup, Access Gate & Page Routing — resolved ticket renders ticket-view/edit; print -> PDF export
    // @implements FS-021.17: Print Ticket to PDF (a=print -> pdfExport) — PDF export trigger
    $ost->setPageTitle('Ticket #'.$ticket->getNumber());
    $nav->setActiveSubMenu(-1);
    $inc = 'ticket-view.inc.php';
    if($_REQUEST['a']=='edit' && $thisstaff->canEditTickets())
        $inc = 'ticket-edit.inc.php';
    elseif($_REQUEST['a'] == 'print' && !$ticket->pdfExport($_REQUEST['psize'], $_REQUEST['notes']))
        $errors['err'] = 'Internal error: Unable to export the ticket to PDF for print.';
} else {
    // @implements FS-020.1: Queue Entry & Default Landing — no ticket resolved renders the queue listing
    $inc = 'tickets.inc.php';
    // @implements FS-021.20: Staff-Initiated (Phone) New Ticket — a=open renders the new-ticket form
    if($_REQUEST['a']=='open' && $thisstaff->canCreateTickets())
        $inc = 'ticket-open.inc.php';
    // @implements FS-020.10: Export Current Query to CSV — token-backed queue export trigger
    // @implements FS-090.23: Ticket-Queue CSV Export Flow (Token-Backed) — session query-token lookup + CSV dump
    elseif($_REQUEST['a'] == 'export') {
        require_once(INCLUDE_DIR.'class.export.php');
        $ts = strftime('%Y%m%d');
        if (!($token=$_REQUEST['h']))
            $errors['err'] = 'Query token required';
        elseif (!($query=$_SESSION['search_'.$token]))
            $errors['err'] = 'Query token not found';
        elseif (!Export::saveTickets($query, "tickets-$ts.csv", 'csv'))
            $errors['err'] = 'Internal error: Unable to dump query results';
    }

    //Clear active submenu on search with no status
    if($_REQUEST['a']=='search' && !$_REQUEST['status'])
        $nav->setActiveSubMenu(-1);

    //set refresh rate if the user has it configured
    if(!$_POST && !$_REQUEST['a'] && ($min=$thisstaff->getRefreshRate()))
        $ost->addExtraHeader('<meta http-equiv="refresh" content="'.($min*60).'" />');
}

require_once(STAFFINC_DIR.'header.inc.php');
require_once(STAFFINC_DIR.$inc);
require_once(STAFFINC_DIR.'footer.inc.php');
?>
