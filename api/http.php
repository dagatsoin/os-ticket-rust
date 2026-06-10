<?php
/*********************************************************************
    http.php

    HTTP controller for the osTicket API

    Jared Hancock
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
require 'api.inc.php';

# Include the main api urls
require_once INCLUDE_DIR."class.dispatcher.php";

// @implements FS-043.3: HTTP API URL Dispatcher — defines the HTTP API routes and resolves the request path-info
// @implements FS-043.4: External Ticket-Create API Endpoint — POST /tickets.(xml|json|email) → TicketApiController::create
// @implements FS-043.9: Remote HTTP Cron Execution — POST /tasks/cron → CronApiController::execute
// @implements BS-438: Only Ticket-Create and Cron Are Exposed Over HTTP API — the dispatcher exposes exactly these two routes
// @implements FS-041.2: Remote HTTP Email Intake — the email format of the ticket-create route is the remote HTTP email intake channel
$dispatcher = patterns('',
        url_post("^/tickets\.(?P<format>xml|json|email)$", array('api.tickets.php:TicketApiController','create')),
        url('^/tasks/', patterns('',
                url_post("^cron$", array('api.cron.php:CronApiController', 'execute'))
         ))
        );

# Call the respective function
print $dispatcher->resolve($ost->get_path_info());
?>
