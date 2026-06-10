<?php
/*********************************************************************
    upgrade.php

    osTicket Upgrade Wizard

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
require_once 'admin.inc.php';
require_once INCLUDE_DIR.'class.upgrader.php';

//$_SESSION['ost_upgrader']=null;
// @implements FS-061.17: Time-limit removal at stream-upgrader construction — instantiate the stream upgrader
$upgrader = new Upgrader(TABLE_PREFIX, UPGRADE_DIR.'streams/');
$errors=array();
// @implements FS-061.9: Upgrade wizard UX & access gating — manual POST step dispatch (non-AJAX fallback)
if($_POST && $_POST['s'] && !$upgrader->isAborted()) {
    switch(strtolower($_POST['s'])) {
        // @implements FS-061.16: Minimum-requirement check semantics — prereq step (upgradable + prereqs + config-file rename)
        // @implements FS-061.13: Config-file rename precondition
        case 'prereq':
            if(!$ost->isUpgradePending()) {
                $errors['err']=' Nothing to do! System already upgraded to the current version';
            } elseif(!$upgrader->isUpgradable()) {
                $errors['err']='The upgrader does NOT support upgrading from the current vesion!';
            } elseif(!$upgrader->check_prereq()) {
                $errors['prereq']='Minimum requirements not met! Refer to Release Notes for more information';
            } elseif(!strcasecmp(basename(CONFIG_FILE), 'settings.php')) {
                $errors['err']='Config file rename required to continue!';
            } else {
                $upgrader->setState('upgrade');
            }
            break;
        // @implements FS-061.4: Per-patch batched application with time-boxing — manual upgrade step (task/patch advancement)
        // @implements FS-061.5: Resumable procedural migration tasks
        case 'upgrade': //Manual upgrade.... when JS (ajax) is not supported.
            if($upgrader->getPendingTask()) {
                $upgrader->doTask();
            } elseif($ost->isUpgradePending() && $upgrader->isUpgradable()) {
                $upgrader->upgrade();
            } elseif(!$ost->isUpgradePending()) {
                $upgrader->setState('done');
            }

            if(($errors=$upgrader->getErrors()))  {
                $upgrader->setState('aborted');
            }
            break;
        default:
            $errors['err']='Unknown action!';
    }
}

// @implements FS-061.9: Upgrade wizard UX & access gating — select wizard view by upgrader state
// @implements FS-061.15: Persistence of upgrade run state across requests — branch on persisted run state
switch(strtolower($upgrader->getState())) {
    case 'aborted':
        $inc='aborted.inc.php';
        break;
    case 'upgrade':
        $inc='upgrade.inc.php';
        break;
    case 'done':
        $inc='done.inc.php';
        break;
    default:
        $inc='prereq.inc.php';
        if($upgrader->isAborted())
            $inc='aborted.inc.php';
        elseif(!strcasecmp(basename(CONFIG_FILE), 'settings.php'))
            $inc='rename.inc.php';
        elseif(!$ost->isUpgradePending())
            $errors['err']='Nothing to do! System already upgraded to <b>'.$ost->getVersion().'</b> with no pending patches to apply.';
        elseif(!$upgrader->isUpgradable())
            $errors['err']=sprintf('The upgrader does NOT support upgrading from the current patch [%s]!', $cfg->getSchemaSignature());

}

$nav = new AdminNav($thisstaff);
$nav->setTabActive('dashboard');
$nav->addSubMenu(array('desc'=>'Upgrader',
                           'title'=>'Upgrader',
                           'href'=>'upgrade.php',
                           'iconclass'=>'preferences'),
                        true);
$ost->addExtraHeader('<script type="text/javascript" src="./js/upgrader.js"></script>');
require(STAFFINC_DIR.'header.inc.php');
require(UPGRADE_DIR.$inc);
require(STAFFINC_DIR.'footer.inc.php');
?>
