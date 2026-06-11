<?php
/*********************************************************************
    ost-config.php

    Static osTicket configuration file. Mainly useful for mysql login info.
    Created during installation process and shouldn't change even on upgrades.
   
    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2010 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
    $Id: $
**********************************************************************/

// @implements BS-060-02: Runtime config self-redirects to installer until installed — direct-access guard dies kwaheri rafiki
// @implements EC-060-13: Two divergent config templates — this copy guards on ROOT_PATH (setup/inc variant)
#Disable direct access.
if(!strcasecmp(basename($_SERVER['SCRIPT_NAME']),basename(__FILE__)) || !defined('ROOT_PATH')) die('kwaheri rafiki!');

// @implements BS-060-02: Runtime config self-redirects to installer until installed — redirect to setup/install.php while OSTINSTALLED FALSE
// @implements FS-060.9: Re-run protection (already-installed detection) — OSTINSTALLED flag is the template substitution target
#Install flag
define('OSTINSTALLED',FALSE);
if(OSTINSTALLED!=TRUE){
    if(!file_exists(ROOT_PATH.'setup/install.php')) die('Error: Contact system admin.'); //Something is really wrong!
    //Invoke the installer.
    header('Location: '.ROOT_PATH.'setup/install.php');
    exit;
}

// @implements BS-060-17: A secret salt is generated per install — SECRET_SALT placeholder (%CONFIG-SIRI → 32-char randCode at install)
# Encrypt/Decrypt secret key - randomly generated during installation.
define('SECRET_SALT','%CONFIG-SIRI');

#Default admin email. Used only on db connection issues and related alerts.
define('ADMIN_EMAIL','%ADMIN-EMAIL');

// @implements FS-060.7: Schema load, default seeding, admin & config provisioning — %CONFIG-* DB credential placeholders filled at install
// @implements KL-060-01: Installer is MySQL-only and version checks are coarse — DBTYPE hard-coded mysql
#Mysql Login info
define('DBTYPE','mysql');
define('DBHOST','%CONFIG-DBHOST'); 
define('DBNAME','%CONFIG-DBNAME');
define('DBUSER','%CONFIG-DBUSER');
define('DBPASS','%CONFIG-DBPASS');

#Table prefix
define('TABLE_PREFIX','%CONFIG-PREFIX');

?>
