<?php
/*********************************************************************
    class.client.php

    Handles everything about client

    XXX: Please note that osTicket uses email address and ticket ID to authenticate the user*!
          Client is modeled on the info of the ticket used to login .

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

// @implements FS-010.6: Client Session Lifecycle & Validation — client model built from ticket info
// @implements BS-010.1: Identity Is Ticket-Number + Email, Validated In Order
class Client {

    var $id;
    var $fullname;
    var $username;
    var $email;

    var $ticket_id;
    var $ticketID;

    var $ht;


    // @implements FS-010.6: Client Session Lifecycle & Validation — construct client from ticket id + email
    function Client($id, $email=null) {
        $this->id =0;
        $this->load($id,$email);
    }

    // @implements FS-010.6: Client Session Lifecycle & Validation — load client row from ticket by number (+email)
    // @implements BS-010.1: Identity Is Ticket-Number + Email, Validated In Order
    function load($id=0, $email=null) {

        if(!$id && !($id=$this->getId()))
            return false;

        $sql='SELECT ticket_id, ticketID, name, email, phone, phone_ext '
            .' FROM '.TICKET_TABLE
            .' WHERE ticketID='.db_input($id);
        if($email)
            $sql.=' AND email='.db_input($email);

        if(!($res=db_query($sql)) || !db_num_rows($res))
            return NULL;

        $this->ht = db_fetch_array($res);
        $this->id         = $this->ht['ticketID']; //placeholder
        $this->ticket_id  = $this->ht['ticket_id'];
        $this->ticketID   = $this->ht['ticketID'];
        $this->fullname   = ucfirst($this->ht['name']);
        $this->username   = $this->ht['email'];
        $this->email      = $this->ht['email'];

        $this->stats = array();
      
        return($this->id);
    }

    // @implements FS-010.6: Client Session Lifecycle & Validation — reload client model
    function reload() {
        return $this->load();
    }

    // @implements FS-010.6: Client Session Lifecycle & Validation — client-role marker
    function isClient() {
        return TRUE;
    }

    // @implements FS-010.6: Client Session Lifecycle & Validation — id accessor (placeholder = ticketID)
    function getId() {
        return $this->id;
    }

    // @implements FS-010.8: "My Tickets" List (Related Tickets by Email) — email is the scoping key
    function getEmail() {
        return $this->email;
    }

    // @implements FS-010.6: Client Session Lifecycle & Validation — username (email) accessor
    function getUserName() {
        return $this->username;
    }

    // @implements FS-010.10: Client Page Shell — Header, Footer & Navigation — display name accessor
    function getName() {
        return $this->fullname;
    }

    // @implements FS-010.7: Client Ticket View (Thread, Header, Reply) — phone accessor for header block
    function getPhone() {
        return $this->ht['phone'];
    }

    // @implements FS-010.7: Client Ticket View (Thread, Header, Reply) — phone extension accessor
    function getPhoneExt() {
        return $this->ht['phone_ext'];
    }
    
    // @implements FS-010.6: Client Session Lifecycle & Validation — external ticket number accessor
    function getTicketID() {
        return $this->ticketID;
    }

    // @implements FS-010.8: "My Tickets" List (Related Tickets by Email) — per-email open/closed aggregate
    function getTicketStats() {

        if(!$this->stats['tickets'])
            $this->stats['tickets'] = Ticket::getClientStats($this->getEmail());

        return $this->stats['tickets'];
    }

    // @implements FS-010.10: Client Page Shell — Header, Footer & Navigation — "My Tickets (N)" total count
    function getNumTickets() {
        return ($stats=$this->getTicketStats())?($stats['open']+$stats['closed']):0;
    }

    // @implements FS-010.8: "My Tickets" List (Related Tickets by Email) — open ticket count
    function getNumOpenTickets() {
        return ($stats=$this->getTicketStats())?$stats['open']:0;
    }

    // @implements FS-010.8: "My Tickets" List (Related Tickets by Email) — closed ticket count
    function getNumClosedTickets() {
        return ($stats=$this->getTicketStats())?$stats['closed']:0;
    }

    /* ------------- Static ---------------*/
    // @implements FS-010.8: "My Tickets" List (Related Tickets by Email) — email-only ticket# resolver
    // @implements KL-010.11: Email-Only Client Resolver Returns The OLDEST Ticket Despite Its "Last" Name — ORDER BY created lacks DESC, so despite the "Last"/"most-recent" naming this returns the oldest ticket for the email
    function getLastTicketIdByEmail($email) {
        $sql='SELECT ticketID FROM '.TICKET_TABLE
            .' WHERE email='.db_input($email)
            .' ORDER BY created '
            .' LIMIT 1';
        if(($res=db_query($sql)) && db_num_rows($res))
            list($tid) = db_fetch_row($res);

        return $tid;
    }

    // @implements FS-010.6: Client Session Lifecycle & Validation — load client by ticket # (+email)
    function lookup($id, $email=null) {
        return ($id && is_numeric($id) && ($c=new Client($id,$email)) && $c->getId()==$id)?$c:null;
    }

    // @implements FS-010.8: "My Tickets" List (Related Tickets by Email) — resolve client from email alone
    function lookupByEmail($email) {
        return (($id=self::getLastTicketIdByEmail($email)))?self::lookup($id, $email):null;
    }

    // @implements FS-010.3: Interactive Login (Ticket ID + Email) — POST login by ticket # + email
    // @implements FS-010.4: Access-Link (Auto) Login — GET auto-login via per-ticket auth token
    // @implements FS-010.5: Brute-Force Throttling & Lockout — per-session strike counter + lockout window
    // @implements BS-010.1: Identity Is Ticket-Number + Email, Validated In Order
    // @implements BS-010.2: Auth Token Is Mandatory For Link (GET) Login, Forbidden For Interactive (POST) Login
    /* static */ function login($ticketID, $email, $auth=null, &$errors=array()) {
        global $ost;


        $cfg = $ost->getConfig();
        $auth = trim($auth);
        $email = trim($email);
        $ticketID = trim($ticketID);

        # Only consider auth token for GET requests, and for GET requests,
        # REQUIRE the auth token
        $auto_login = ($_SERVER['REQUEST_METHOD'] == 'GET');

        //Check time for last max failed login attempt strike.
        if($_SESSION['_client']['laststrike']) {
            if((time()-$_SESSION['_client']['laststrike'])<$cfg->getClientLoginTimeout()) {
                $errors['login'] = 'Excessive failed login attempts';
                $errors['err'] = 'You\'ve reached maximum failed login attempts allowed. Try again later or <a href="open.php">open a new ticket</a>';
                $_SESSION['_client']['laststrike'] = time(); //renew the strike.
            } else { //Timeout is over.
                //Reset the counter for next round of attempts after the timeout.
                $_SESSION['_client']['laststrike'] = null;
                $_SESSION['_client']['strikes'] = 0;
            }
        }

        if($auto_login && !$auth)
            $errors['login'] = 'Invalid method';
        elseif(!$ticketID || !Validator::is_email($email))
            $errors['login'] = 'Valid email and ticket number required';

        //Bail out on error.
        if($errors) return false;

        //See if we can fetch local ticket id associated with the ID given
        if(($ticket=Ticket::lookupByExtId($ticketID, $email)) && $ticket->getId()) {
            //At this point we know the ticket ID is valid.
            //TODO: 1) Check how old the ticket is...3 months max?? 2) Must be the latest 5 tickets?? 
            //Check the email given.

            # Require auth token for automatic logins (GET METHOD).
            if (!strcasecmp($ticket->getEmail(), $email) && (!$auto_login || $auth === $ticket->getAuthToken())) {
                    
                //valid match...create session goodies for the client.
                $user = new ClientSession($email,$ticket->getExtId());
                $_SESSION['_client'] = array(); //clear.
                $_SESSION['_client']['userID'] = $ticket->getEmail(); //Email
                $_SESSION['_client']['key'] = $ticket->getExtId(); //Ticket ID --acts as password when used with email. See above.
                $_SESSION['_client']['token'] = $user->getSessionToken();
                $_SESSION['TZ_OFFSET'] = $cfg->getTZoffset();
                $_SESSION['TZ_DST'] = $cfg->observeDaylightSaving();
                $user->refreshSession(); //set the hash.
                //Log login info...
                $msg=sprintf('%s/%s logged in [%s]', $ticket->getEmail(), $ticket->getExtId(), $_SERVER['REMOTE_ADDR']);
                $ost->logDebug('User login', $msg);
        
                //Regenerate session ID.
                $sid=session_id(); //Current session id.
                session_regenerate_id(TRUE); //get new ID.
                if(($session=$ost->getSession()) && is_object($session) && $sid!=session_id())
                    $session->destroy($sid);

                return $user;

            } 
        }

        //If we get to this point we know the login failed.
        $errors['login'] = 'Invalid login';
        $_SESSION['_client']['strikes']+=1;
        if(!$errors && $_SESSION['_client']['strikes']>$cfg->getClientMaxLogins()) {
            $errors['login'] = 'Access Denied';
            $errors['err'] = 'Forgot your login info? Please <a href="open.php">open a new ticket</a>.';
            $_SESSION['_client']['laststrike'] = time();
            $alert='Excessive login attempts by a user.'."\n".
                    'Email: '.$email."\n".'Ticket#: '.$ticketID."\n".
                    'IP: '.$_SERVER['REMOTE_ADDR']."\n".'Time:'.date('M j, Y, g:i a T')."\n\n".
                    'Attempts #'.$_SESSION['_client']['strikes'];
            $ost->logError('Excessive login attempts (user)', $alert, ($cfg->alertONLoginError()));
        } elseif($_SESSION['_client']['strikes']%2==0) { //Log every other failed login attempt as a warning.
            $alert='Email: '.$email."\n".'Ticket #: '.$ticketID."\n".'IP: '.$_SERVER['REMOTE_ADDR'].
                   "\n".'TIME: '.date('M j, Y, g:i a T')."\n\n".'Attempts #'.$_SESSION['_client']['strikes'];
            $ost->logWarning('Failed login attempt (user)', $alert);
        }

        return false;
    }
}
?>
