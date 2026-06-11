<?php
/*********************************************************************
    pages/index.php

    Custom pages servlet

    Peter Rotich <peter@osticket.com>
    Jared Hancock <jared@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
@chdir(realpath(dirname(__file__).'/../'));

require_once('client.inc.php');
require_once(INCLUDE_DIR.'class.format.php');
require_once(INCLUDE_DIR.'class.page.php');

// @implements FS-033: Admin Logs, Pages & Content — public servlet resolving a slug to an active 'other'-type content Page and printing its body (no dedicated public-render .N)
// @implements FS-003.15: File-size, phone, slug, elapsed-time & array formatting — Format::slugify maps the path-info to a comparable page slug
// @implements FS-010.9: Client Page Bootstrap & Guard — boots the client shell so the page renders inside the standard client header/footer
// Determine the requested page
// - Strip extension
$slug = Format::slugify($ost->get_path_info());

// Get the part before the first dash
$first_word = explode('-', $slug);
$first_word = $first_word[0];

$sql = 'SELECT id, name FROM '.PAGE_TABLE
    .' WHERE name LIKE '.db_input("$first_word%");
$page_id = null;

$res = db_query($sql);
while (list($id, $name) = db_fetch_row($res)) {
    if (Format::slugify($name) == $slug) {
        $page_id = $id;
        break;
    }
}

if (!$page_id || !($page = Page::lookup($page_id)))
    Http::response(404, 'Page Not Found');

if (!$page->isActive() || $page->getType() != 'other')
    Http::response(404, 'Page Not Found');

require(CLIENTINC_DIR.'header.inc.php');

print $page->getBody();

require(CLIENTINC_DIR.'footer.inc.php');
?>
