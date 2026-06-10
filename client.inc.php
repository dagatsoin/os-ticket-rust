<?php
/*********************************************************************
    client.inc.php

    File included on every client page

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
if(!strcasecmp(basename($_SERVER['SCRIPT_NAME']),basename(__FILE__))) die('kwaheri rafiki!');

$thisdir=str_replace('\\', '/', realpath(dirname(__FILE__))).'/';
if(!file_exists($thisdir.'main.inc.php')) die('Fatal Error.');

require_once($thisdir.'main.inc.php');

if(!defined('INCLUDE_DIR')) die('Fatal error');

/*Some more include defines specific to client only */
define('CLIENTINC_DIR',INCLUDE_DIR.'client/');
define('OSTCLIENTINC',TRUE);

define('ASSETS_PATH',ROOT_PATH.'assets/default/');

// @implements FS-010.9: Client Page Bootstrap & Guard — offline gate shows offline.php and exits for all client pages except logo.php
// @implements BS-010.10: Offline Gate Applies To All Client Pages Except The Logo — logo.php exemption from the offline short-circuit
// @implements FS-001.8: Client-Realm Gate — enforces the offline state on the client realm after master bootstrap
//Check the status of the HelpDesk.
if (!in_array(strtolower(basename($_SERVER['SCRIPT_NAME'])), array('logo.php',))
        && !(is_object($ost) && $ost->isSystemOnline())) {
    include(ROOT_DIR.'offline.php');
    exit;
}

/* include what is needed on client stuff */
require_once(INCLUDE_DIR.'class.client.php');
require_once(INCLUDE_DIR.'class.ticket.php');
require_once(INCLUDE_DIR.'class.dept.php');

//clear some vars
$errors=array();
$msg='';
$thisclient=$nav=null;
//Make sure the user is valid..before doing anything else.
// @implements FS-010.6: Client Session Lifecycle & Validation — reconstructs ClientSession from $_SESSION['_client'], refreshing on valid, dropping otherwise
// @implements FS-010.9: Client Page Bootstrap & Guard — resolves the optional client session and seeds $thisclient for downstream pages
// @implements FS-001.8: Client-Realm Gate — optional client-session resolution step of the client-realm gate
if($_SESSION['_client']['userID'] && $_SESSION['_client']['key'])
    $thisclient = new ClientSession($_SESSION['_client']['userID'],$_SESSION['_client']['key']);

//is the user logged in?
if($thisclient && $thisclient->getId() && $thisclient->isValid()){
     $thisclient->refreshSession();
} else {
    $thisclient = null;
}

/******* CSRF Protectin *************/
// @implements FS-001.11: Cross-Site Request Forgery Protection — blanket CSRF guard on client POSTs; redirect to index + "Action denied (400)!" on failure
// @implements BS-009: CSRF Required On All POST Requests — rejects forged/stale client POSTs before any state change
// @implements FS-010.9: Client Page Bootstrap & Guard — the client-realm CSRF-on-POST gate
// Enforce CSRF protection for POSTS
if ($_POST  && !$ost->checkCSRFToken()) {
    @header('Location: index.php');
    //just incase redirect fails
    die('Action denied (400)!');
}

/* Client specific defaults */
define('PAGE_LIMIT', DEFAULT_PAGE_LIMIT);

$nav = new UserNav($thisclient, 'home');
?>
