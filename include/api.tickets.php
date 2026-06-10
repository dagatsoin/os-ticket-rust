<?php

include_once INCLUDE_DIR.'class.api.php';
include_once INCLUDE_DIR.'class.ticket.php';

// @implements FS-043.4: External Ticket-Create API Endpoint — API controller exposing remote ticket creation over HTTP/pipe
class TicketApiController extends ApiController {

    // @implements FS-041.5.1: Permitted Email-Request Field Set — declares the supported request fields, extended for the 'email' format
    # Supported arguments -- anything else is an error. These items will be
    # inspected _after_ the fixup() method of the ApiXxxDataParser classes
    # so that all supported input formats should be supported
    function getRequestStructure($format) {
        $supported = array(
            "alert", "autorespond", "source", "topicId",
            "name", "email", "subject", "phone", "phone_ext",
            "attachments" => array("*" =>
                array("name", "type", "data", "encoding")
            ),
            "message", "ip", "priorityId"
        );

        if(!strcasecmp($format, 'email'))
            $supported = array_merge($supported, array('header', 'mid',
                'emailId', 'ticketId', 'reply-to', 'reply-to-name',
                'in-reply-to', 'references'));

        return $supported;
    }

    /*
     Validate data - overwrites parent's validator for additional validations.
    */
    // @implements FS-043.5: API Attachment Intake & Validation — strips attachments when disallowed, decodes base64, soft-fails on type/size errors
    // @implements BS-437: Soft Attachment Validation — sets per-attachment error and passes the request on rather than rejecting
    // @implements FS-001.16: System-Object Request & Environment Utilities — isFileTypeAllowed checks each attachment extension against config
    function validate(&$data, $format) {
        global $ost;

        //Call parent to Validate the structure
        if(!parent::validate($data, $format))
            $this->exerr(400, 'Unexpected or invalid data received');

        //Nuke attachments IF API files are not allowed.
        if(!$ost->getConfig()->allowAPIAttachments())
            $data['attachments'] = array();

        //Validate attachments: Do error checking... soft fail - set the error and pass on the request.
        if($data['attachments'] && is_array($data['attachments'])) {
            foreach($data['attachments'] as &$attachment) {
                if(!$ost->isFileTypeAllowed($attachment))
                    $attachment['error'] = 'Invalid file type (ext) for '.Format::htmlchars($attachment['name']);
                elseif ($attachment['encoding'] && !strcasecmp($attachment['encoding'], 'base64')) {
                    if(!($attachment['data'] = base64_decode($attachment['data'], true)))
                        $attachment['error'] = sprintf('%s: Poorly encoded base64 data', Format::htmlchars($attachment['name']));
                }
                if (!$attachment['error']
                        && ($size = $ost->getConfig()->getMaxFileSize())
                        && ($fsize = $attachment['size'] ? $attachment['size'] : strlen($attachment['data']))
                        && $fsize > $size) {
                    $attachment['error'] = sprintf('File %s (%s) is too big. Maximum of %s allowed',
                            Format::htmlchars($attachment['name']),
                            Format::file_size($fsize),
                            Format::file_size($size));
                }
            }
            unset($attachment);
        }

        return true;
    }


    // @implements FS-043.4: External Ticket-Create API Endpoint — authenticates the API key, then creates a ticket (or processes an email) and returns 201 + ext id
    // @implements FS-043.6: API Key Authentication & IP Binding — requireApiKey + canCreateTickets permission gate, 401 otherwise
    function create($format) {

        if(!($key=$this->requireApiKey()) || !$key->canCreateTickets())
            return $this->exerr(401, 'API key not authorized');

        $ticket = null;
        if(!strcasecmp($format, 'email')) {
            # Handle remote piped emails - could be a reply...etc.
            $ticket = $this->processEmail();
        } else {
            # Parse request body
            $ticket = $this->createTicket($this->getRequest($format));
        }

        if(!$ticket)
            return $this->exerr(500, "Unable to create new ticket: unknown error");

        $this->response(201, $ticket->getExtId());
    }

    /* private helper functions */

    // @implements FS-043.4: External Ticket-Create API Endpoint — pulls alert/autorespond/source meta then delegates to Ticket::create, mapping errors to API codes
    // @implements FS-011.9: Ticket Creation, Routing, and Reference Assignment — shared Ticket::create routing path used by the API channel
    function createTicket($data) {

        # Pull off some meta-data
        $alert          = array_key_exists('alert',       $data) ? $data['alert']       : true;
        $autorespond    = array_key_exists('autorespond', $data) ? $data['autorespond'] : true;
        $data['source'] = array_key_exists('source',      $data) ? $data['source']      : 'API';

        # Create the ticket with the data (attempt to anyway)
        $errors = array();
        $ticket = Ticket::create($data, $errors, $data['source'], $autorespond, $alert);
        # Return errors (?)
        if (count($errors)) {
            if(isset($errors['errno']) && $errors['errno'] == 403)
                return $this->exerr(403, 'Ticket denied');
            else
                return $this->exerr(
                        400,
                        "Unable to create new ticket: validation errors:\n"
                        .Format::array_implode(": ", "\n", $errors)
                        );
        } elseif (!$ticket) {
            return $this->exerr(500, "Unable to create new ticket: unknown error");
        }

        return $ticket;
    }

    // @implements FS-041.6: Threading Detection & Create-or-Append Flow — appends to an existing ticket/thread by id or email headers, else creates a new ticket
    // @implements BS-041.7: Thread-Match Precedence — explicit ticketId, then Message-Id header match, then new-ticket fallback
    function processEmail() {

        $data = $this->getEmailRequest();
        if($data['ticketId'] && ($ticket=Ticket::lookup($data['ticketId']))) {
            if(($msgid=$ticket->postMessage($data, 'Email')))
                return $ticket;
        }

        if (($thread = ThreadEntry::lookupByEmailHeaders($data))
                && $thread->postEmail($data)) {
            return $thread->getTicket();
        }
        return $this->createTicket($data);
    }

}

//Local email piping controller - no API key required!
// @implements FS-041.1: Local Pipe Intake — local MTA pipe controller; no API key required for local piping
// @implements BS-041.3: Local-Only Pipe — only reachable via the local pipe.php CLI entry point
class PipeApiController extends TicketApiController {

    //Overwrite grandparent's (ApiController) response method.
    // @implements FS-041.10: Outcome Signaling to the MTA (pipe channel) — maps HTTP-style codes to postfix exit codes
    // @implements BS-041.2: Pipe Exit-Code Mapping — 201→0, 400→66, 401/403→77, 415-417/501→65, 503→69, else 75 (temp/retry)
    function response($code, $resp) {

        //Use postfix exit codes - instead of HTTP
        switch($code) {
            case 201: //Success
                $exitcode = 0;
                break;
            case 400:
                $exitcode = 66;
                break;
            case 401: /* permission denied */
            case 403:
                $exitcode = 77;
                break;
            case 415:
            case 416:
            case 417:
            case 501:
                $exitcode = 65;
                break;
            case 503:
                $exitcode = 69;
                break;
            case 500: //Server error.
            default: //Temp (unknown) failure - retry
                $exitcode = 75;
        }

        //echo "$code ($exitcode):$resp";
        //We're simply exiting - MTA will take care of the rest based on exit code!
        exit($exitcode);
    }

    // @implements FS-041.1: Local Pipe Intake — static entry invoked by api/pipe.php: processes the piped email and signals the MTA via exit code
    // @implements BS-041.2: Pipe Exit-Code Mapping — 201 on success, 416 (retry, exit 75) when processing fails
    function  process() {
        $pipe = new PipeApiController();
        if(($ticket=$pipe->processEmail()))
           return $pipe->response(201, $ticket->getNumber());

        return $pipe->exerr(416, 'Request failed - retry again!');
    }
}

?>
