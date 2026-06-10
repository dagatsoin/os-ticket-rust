<?php
/*********************************************************************
    login.php

    Handles staff authentication/logins

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
require_once('../main.inc.php');
if(!defined('INCLUDE_DIR')) die('Fatal Error. Kwaheri!');

require_once(INCLUDE_DIR.'class.staff.php');
require_once(INCLUDE_DIR.'class.csrf.php');

// @implements FS-002.8: Post-login redirect to original destination — recover the stored auth destination + message
$dest = $_SESSION['_staff']['auth']['dest'];
$msg = $_SESSION['_staff']['auth']['msg'];
$msg = $msg?$msg:'Authentication Required';
// @implements FS-002.1: Staff login — credential check + post-login redirect
// @implements FS-002.7: CSRF protection on state-changing requests — verify + rotate the CSRF token on login POST
if($_POST) {
    // Check the CSRF token, and ensure that future requests will have to
    // use a different CSRF token. This will help ward off both parallel and
    // serial brute force attacks, because new tokens will have to be
    // requested for each attempt.
    if (!$ost->checkCSRFToken())
        Http::response(400, 'Valid CSRF Token Required');

    // Rotate the CSRF token (original cannot be reused)
    $ost->getCSRF()->rotate();

    //$_SESSION['_staff']=array(); #Uncomment to disable login strikes.
    if(($user=Staff::login($_POST['userid'], $_POST['passwd'], $errors))){
        $dest=($dest && (!strstr($dest,'login.php') && !strstr($dest,'ajax.php')))?$dest:'index.php';
        @header("Location: $dest");
        require_once('index.php'); //Just incase header is messed up.
        exit;
    }

    $msg = $errors['err']?$errors['err']:'Invalid login';
}
define("OSTSCPINC",TRUE); //Make includes happy!
include_once(INCLUDE_DIR.'staff/login.tpl.php');
?>
