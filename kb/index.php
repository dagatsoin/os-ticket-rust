<?php
/*********************************************************************
    index.php

    Knowledgebase Index.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
// @implements FS-050.2: Public Knowledge-Base Entry & Guard — public KB index entry point booting through kb.inc.php
// @implements FS-050.4: Public Landing Page — Category Listing & Search Form — renders knowledgebase.inc.php as the KB landing page
require('kb.inc.php');
require_once(INCLUDE_DIR.'class.category.php');
$inc='knowledgebase.inc.php';
require(CLIENTINC_DIR.'header.inc.php');
require(CLIENTINC_DIR.$inc);
require(CLIENTINC_DIR.'footer.inc.php');
?>
