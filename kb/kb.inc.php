<?php
/*********************************************************************
    kb.inc.php

    File included on every knowledgebase file.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
require_once('../client.inc.php');
require_once(INCLUDE_DIR.'class.faq.php');
/* Bail out if knowledgebase is disabled or if we have no public-published FAQs. */
// @implements FS-050.2: Public Knowledge-Base Entry & Guard — shared KB shell; redirects home unless KB is enabled and a published FAQ exists
// @implements BS-050.2: Public KB Reachability Requires Toggle AND At Least One Published Public FAQ — isKnowledgebaseEnabled + countPublishedFAQs gate
if(!$cfg || !$cfg->isKnowledgebaseEnabled() || !FAQ::countPublishedFAQs()) {
    header('Location: ../');
    exit;
}

$nav = new UserNav($thisclient, 'kb');
?>
