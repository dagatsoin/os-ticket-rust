<?php
/*********************************************************************
    captcha.php

    Simply returns captcha image.
    
    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
// @implements FS-010.11: Captcha Image Generation — emits the PNG captcha (len 5/font 12) and records its hash in the session
// @implements FS-011.5: CAPTCHA Challenge (Anonymous Submitters) — the challenge image consumed by the anonymous new-ticket form
require_once('main.inc.php');
require(INCLUDE_DIR.'class.captcha.php');
$captcha = new Captcha(5,12,ROOT_DIR.'images/captcha/');
echo $captcha->getImage();
?>
