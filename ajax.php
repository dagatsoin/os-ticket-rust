<?php
/*********************************************************************
    ajax.php

    Ajax utils for client interface.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

// @implements FS-010.9: Client Page Bootstrap & Guard — pre-declares clientLoginPage so the AJAX layer returns 403 instead of an HTML login page
function clientLoginPage($msg='Unauthorized') {
    Http::response(403,'Must login: '.Format::htmlchars($msg));
    exit;
}

require('client.inc.php');

if(!defined('INCLUDE_DIR'))	Http::response(500, 'Server configuration error');
require_once INCLUDE_DIR.'/class.dispatcher.php';
require_once INCLUDE_DIR.'/class.ajax.php';

// @implements FS-043.13: Shared AJAX Controller Base (Infrastructure) — builds the client AJAX URL dispatcher and resolves the request path
// @implements FS-033: Admin Logs, Pages & Content — routes the /config/client bundle (sibling of the FS-033.10 /config/scp bundle; no dedicated .N)
$dispatcher = patterns('',
    url('^/config/', patterns('ajax.config.php:ConfigAjaxAPI',
        url_get('^client', 'client')
    ))
);
print $dispatcher->resolve($ost->get_path_info());
?>
