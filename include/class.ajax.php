<?php
/*********************************************************************
    class.ajax.php

    AjaxController class that is an extension of the ApiController class. It
    will be used to provide functionality common to all Ajax API calls

    Jared Hancock 
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

require_once (INCLUDE_DIR.'class.api.php');

/**
 * AjaxController Class
 * A simple extension of the ApiController class that will assist in
 * providing functionality common to all Ajax call controllers. Any Ajax
 * call controller should inherit from this class in order to maintain
 * consistency.
 */
// @implements FS-043.13: Shared AJAX Controller Base (Infrastructure) — extends ApiController, no API key
class AjaxController extends ApiController {
    function AjaxController() {
    
    }
    // @implements FS-043.13: Shared AJAX Controller Base (Infrastructure) — staffOnly guard → 401 when no valid staff
    function staffOnly() {
        global $thisstaff;
        if(!$thisstaff || !$thisstaff->isValid()) {
            Http::response(401,'Access Denied. IP '.$_SERVER['REMOTE_ADDR']);
        }
    }
    /**
     * Convert a PHP array into a JSON-encoded string
     */
    // @implements FS-043.13: Shared AJAX Controller Base (Infrastructure) — JSON-encoding helper
    function json_encode($what) {
        require_once (INCLUDE_DIR.'class.json.php');
        $encoder = new JsonDataEncoder();
        return $encoder->encode($what);
    }

    // @implements FS-043.13: Shared AJAX Controller Base (Infrastructure) — encode alias
    function encode($what) {
        return $this->json_encode($what);
    }

    // @implements FS-043.13: Shared AJAX Controller Base (Infrastructure) — GET param reader with default
    function get($var, $default=null) {
        return (isset($_GET[$var])) ? $_GET[$var] : $default;
    }
}
