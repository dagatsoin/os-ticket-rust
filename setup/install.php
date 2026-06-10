<?php
/*********************************************************************
    install.php

    osTicket Installer.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
// @implements FS-060.1: Installer bootstrap & step state machine — wizard entry point; loads setup bootstrap
require('setup.inc.php');

require_once INC_DIR.'class.installer.php';


//define('OSTICKET_CONFIGFILE','../include/ost-config.php'); //osTicket config file full path.
define('OSTICKET_CONFIGFILE','../include/ost-config.php'); //XXX: Make sure the path is corrent b4 releasing.


$installer = new Installer(OSTICKET_CONFIGFILE); //Installer instance.
$wizard=array();
$wizard['title']='osTicket Installer';
$wizard['tagline']='Installing osTicket '.$installer->getVersionVerbose();
$wizard['logo']='logo.png';
$wizard['menu']=array('Installation Guide'=>'http://osticket.com/wiki/Installation',
        'Get Professional Help'=>'http://osticket.com/support');

// @implements FS-060.1: Installer bootstrap & step state machine — POST step dispatcher advances the session step pointer
if($_POST && $_POST['s']) {
    $errors = array();
    $_SESSION['ost_installer']['s']=$_POST['s'];
    switch(strtolower($_POST['s'])) {
        // @implements FS-060.2: Prerequisite check (step `prereq`) — on pass advance pointer to config
        case 'prereq':
            if($installer->check_prereq())
                $_SESSION['ost_installer']['s']='config';
            else
                $errors['prereq']='Minimum requirements not met!';
            break;
        // @implements FS-060.3: Configuration-file check (step `config`) — re-run exists+writable checks, advance to install
        case 'config':
            if(!$installer->config_exists())
                $errors['err']='Configuration file does NOT exist. Follow steps below to add one.';
            elseif(!$installer->config_writable())
                $errors['err']='Write access required to continue';
            else
                $_SESSION['ost_installer']['s']='install';
            break;
        // @implements FS-060.5: Install-form field validation — run install(), outer fallback error when no err registered
        // @implements FS-060.7: Schema load, default seeding, admin & config provisioning — drives the master install routine
        // @implements FS-060.8: Completion screen & post-install hardening (step `done`) — captures $_SESSION['info'] on success
        case 'install':
            if($installer->install($_POST)) {
                $_SESSION['info']=array('name'  =>ucfirst($_POST['fname'].' '.$_POST['lname']),
                                        'email' =>$_POST['admin_email'],
                                        'URL'=>URL);
                //TODO: Go to subscribe step.
                $_SESSION['ost_installer']['s']='done';
            } elseif(!($errors=$installer->getErrors()) || !$errors['err']) {
                $errors['err']='Error installing osTicket - correct the errors below and try again.';
            }
            break;
        // @implements FS-060.1: Installer bootstrap & step state machine — subscribe POST handler (name/email/notify validation)
        // @implements KL-060-02: `subscribe` step is dead/RC scaffolding — unreachable in normal flow
        case 'subscribe':
            if(!trim($_POST['name']))
                $errors['name'] = 'Required';

            if(!$_POST['email'])
                $errors['email'] = 'Required';
            elseif(!Validator::is_email($_POST['email']))
                $errors['email'] = 'Invalid';

            if(!$_POST['alerts'] && !$_POST['news'])
                $errors['notify'] = 'Check one or more';

            if(!$errors)
                $_SESSION['ost_installer']['s'] = 'done';
            break;
    }

// @implements FS-060.1: Installer bootstrap & step state machine — only GET-driven transition: "No thanks." subscribe → done
// @implements KL-060-02: `subscribe` step is dead/RC scaffolding — dead subscribe → done GET branch
}elseif($_GET['s'] && $_GET['s']=='ns' && $_SESSION['ost_installer']['s']=='subscribe') {
    $_SESSION['ost_installer']['s']='done';
}

// @implements FS-060.1: Installer bootstrap & step state machine — view selector keyed off the *current* pointer value
switch(strtolower($_SESSION['ost_installer']['s'])) {
    // @implements FS-060.3: Configuration-file check (step `config`) — four-way view select on config file state
    // @implements BS-060-03: Install-step views re-validate clean config — mid-flow re-check, clearstatcache before file-perm
    case 'config':
    case 'install':
        if(!$installer->config_exists()) {
            $inc='file-missing.inc.php';
        } elseif(!($cFile=file_get_contents($installer->getConfigFile()))
                || preg_match("/define\('OSTINSTALLED',TRUE\)\;/i",$cFile)) { //osTicket already installed or empty config file?
            $inc='file-unclean.inc.php';
        } elseif(!$installer->config_writable()) { //writable config file??
            clearstatcache();
            $inc='file-perm.inc.php';
        } else { //Everything checked out show install form.
            $inc='install.inc.php';
        }
        break;
    // @implements KL-060-02: `subscribe` step is dead/RC scaffolding — dead subscribe view (TODO RC1)
    case 'subscribe': //TODO: Prep for v1.7 RC1
       $inc='subscribe.inc.php';
        break;
    // @implements FS-060.8: Completion screen & post-install hardening (step `done`) — done view + defensive fallback to prereq when config gone
    case 'done':
        $inc='install-done.inc.php';
        if(!$installer->config_exists())
            $inc='install-prereq.inc.php';
        break;
    // @implements FS-060.9: Re-run protection (already-installed detection) — default branch routes installed markers to file-unclean
    // @implements BS-060-01: Already-installed detection blocks fresh install — settings.php / ostconfig.php / OSTINSTALLED=TRUE markers
    default:
        //Fail IF any of the old config files exists.
        if(file_exists(INCLUDE_DIR.'settings.php')
                || file_exists(ROOT_DIR.'ostconfig.php')
                || (file_exists(OSTICKET_CONFIGFILE)
                    && preg_match("/define\('OSTINSTALLED',TRUE\)\;/i",
                        file_get_contents(OSTICKET_CONFIGFILE)))
                )
            $inc='file-unclean.inc.php';
        else
            $inc='install-prereq.inc.php';
}

require(INC_DIR.'header.inc.php');
require(INC_DIR.$inc);
require(INC_DIR.'footer.inc.php');
?>
