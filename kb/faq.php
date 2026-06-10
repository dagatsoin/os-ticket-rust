<?php
/*********************************************************************
    faq.php

    FAQs Clients' interface..

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
require('kb.inc.php');
require_once(INCLUDE_DIR.'class.faq.php');

// @implements FS-050.3: Public Article / Category Routing (kb/faq.php) — resolves id→FAQ or cid→Category and selects the view template
// @implements FS-050.7: Public Single-Article View — routes to faq.inc.php only when the FAQ is published
// @implements FS-050.6: Public Category View — routes to faq-category.inc.php only when the category is public
// @implements BS-050.1: Public Visibility Requires Published Article AND Public Category — published/public guards on the routing branches
$faq=$category=null;
if($_REQUEST['id'] && !($faq=FAQ::lookup($_REQUEST['id'])))
   $errors['err']='Unknown or invalid FAQ';

if(!$faq && $_REQUEST['cid'] && !($category=Category::lookup($_REQUEST['cid'])))
    $errors['err']='Unknown or invalid FAQ category';


$inc='knowledgebase.inc.php'; //FAQs landing page.
if($faq && $faq->isPublished()) {
    $inc='faq.inc.php';
} elseif($category && $category->isPublic() && $_REQUEST['a']!='search') {
    $inc='faq-category.inc.php';
}
require_once(CLIENTINC_DIR.'header.inc.php');
require_once(CLIENTINC_DIR.$inc);
require_once(CLIENTINC_DIR.'footer.inc.php');
?>
