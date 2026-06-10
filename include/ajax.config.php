<?php
/*********************************************************************
    ajax.content.php

    AJAX interface for content fetching...allowed methods.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

if(!defined('INCLUDE_DIR')) die('!');
	    
// @implements FS-033.10: Staff Configuration Bundle (Config AJAX — /config/scp) — read-only config projection for the staff/client UI
class ConfigAjaxAPI extends AjaxController {

    //config info UI might need.
    // @implements FS-033.10: Staff Configuration Bundle (Config AJAX — /config/scp) — emits lock_time (seconds), date_format, staff max_file_uploads
    function scp() {
        global $cfg;

        $config=array(
                      'lock_time'       => ($cfg->getLockTime()*3600),
                      'date_format'     => ($cfg->getDateFormat()),
                      'max_file_uploads'=> (int) $cfg->getStaffMaxFileUploads()
                      );
        return $this->json_encode($config);
    }

    // @implements FS-033.10: Staff Configuration Bundle (Config AJAX) — companion client projection co-located in ConfigAjaxAPI
    // @implements FS-011.7: Attachment Submission — supplies file_types / max_file_size / client max_file_uploads to the public new-ticket form
    // @implements FS-010.7: Client Ticket View (Thread, Header, Reply) — same upload-config bundle for the client reply form
    function client() {
        global $cfg;

        $config=array(
                      'file_types'      => $cfg->getAllowedFileTypes(),
                      'max_file_size'   => (int) $cfg->getMaxFileSize(),
                      'max_file_uploads'=> (int) $cfg->getClientMaxFileUploads()
                      );

        return $this->json_encode($config);
    }
}
?>
