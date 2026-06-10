<?php
/*********************************************************************
    class.json.php

    Parses JSON text data to PHP associative array. Useful mainly for API
    JSON requests. The module will attempt to use the json_* functions
    builtin to PHP5.2+ if they exist and will fall back to a pure-php
    implementation included in JSON.php.

    Jared Hancock
    Copyright (c)  2006-2010 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
    $Id: $
**********************************************************************/

include_once "JSON.php";

// @implements FS-043.4: External Ticket-Create API Endpoint — JSON-format request body parser
class JsonDataParser {
    // @implements FS-043.4: External Ticket-Create API Endpoint — reads stream then decodes JSON body
    function parse($stream) {
        $contents = '';
        while (!feof($stream)) {
            $contents .= fread($stream, 8192);
        }
        return self::decode($contents);
    }

    // @implements FS-043.4: External Ticket-Create API Endpoint — native json_decode with pure-PHP fallback
    function decode($contents) {
        if (function_exists("json_decode")) {
            return json_decode($contents, true);
        } else {
            # Create associative arrays rather than 'objects'
            $decoder = new Services_JSON(SERVICES_JSON_LOOSE_TYPE);
            return $decoder->decode($contents);
        }
    }
    // @implements FS-043.4: External Ticket-Create API Endpoint — parser last-error message for HTTP 400 body
    function lastError() {
        if (function_exists("json_last_error")) {
            $errors = array(
            JSON_ERROR_NONE => 'No errors',
            JSON_ERROR_DEPTH => 'Maximum stack depth exceeded',
            JSON_ERROR_STATE_MISMATCH => 'Underflow or the modes mismatch',
            JSON_ERROR_CTRL_CHAR => 'Unexpected control character found',
            JSON_ERROR_SYNTAX => 'Syntax error, malformed JSON',
            JSON_ERROR_UTF8 => 'Malformed UTF-8 characters, possibly incorrectly encoded'
            );
            if ($message = $errors[json_last_error()])
                return $message;
            return "Unknown error";
        } else {
            # Doesn't look like Servies_JSON supports errors for decode()
            return "Unknown JSON parsing error";
        }
    }
}

// @implements FS-090.22: JSON Exporter Format — shared JSON encoder reused by exporter, report views, backup writer
class JsonDataEncoder {
    // @implements FS-090.22: JSON Exporter Format — native json_encode with pure-PHP fallback
    function encode($var) {
        if (function_exists('json_encode'))
            return json_encode($var);
        else {
            $decoder = new Services_JSON();
            return $decoder->encode($var);
        }
    }
}
