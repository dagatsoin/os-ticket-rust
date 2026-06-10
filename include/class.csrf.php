<?php
/*********************************************************************
    class.csrf.php

    Provides mechanisms to protect against cross-site request forgery
    attacks. This is accomplished by using a token that is not stored in a
    session, but required to make changes to the system.

    This can be accomplished by emitting a hidden field in a form, or
    sending a separate header (X-CSRFToken) when forms are submitted (e.g Ajax).

    This technique is based on the protection mechanism in the Django
    project, detailed at and thanks to
    https://docs.djangoproject.com/en/dev/ref/contrib/csrf/.

    * TIMEOUT
    Token can be expired after X seconds of inactivity (timeout) independent of the session.


    Jared Hancock
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

// @implements FS-001.11: Cross-Site Request Forgery Protection — per-session CSRF token holder (consumed by FS-002.7)
Class CSRF {

    var $name;
    var $timeout;

    var $csrf;

    // @implements FS-001.11: Cross-Site Request Forgery Protection — construct holder with fixed token name + optional timeout
    function CSRF($name='__CSRFToken__', $timeout=0) {

        $this->name = $name;
        $this->timeout = $timeout;
        $this->csrf = &$_SESSION['csrf'];
    }

    // @implements FS-001.11: Cross-Site Request Forgery Protection — clear stored token state
    function reset() {
        $this->csrf = array();
    }

    // @implements FS-001.11: Cross-Site Request Forgery Protection — inactivity-timeout expiry check
    function isExpired() {
       return ($this->timeout && (time()-$this->csrf['time'])>$this->timeout);
    }

    // @implements FS-001.11: Cross-Site Request Forgery Protection — token field name accessor
    function getTokenName() {
        return $this->name;
    }

    // @implements FS-001.11: Cross-Site Request Forgery Protection — mint token = sha1(session_id + random + secret salt)
    function rotate() {
        $this->csrf['token'] = sha1(session_id().Crypto::random(16).SECRET_SALT);
        $this->csrf['time'] = time();
    }

    // @implements FS-001.11: Cross-Site Request Forgery Protection — lazy-mint/rotate on read, reset activity timer
    function getToken() {

        if (!$this->csrf['token'] || $this->isExpired()) {
            $this->rotate();
        } else {
            //Reset the timer
            $this->csrf['time'] = time();
        }

        return $this->csrf['token'];
    }

    // @implements FS-001.11: Cross-Site Request Forgery Protection — validate presented token against current, non-expired
    // @implements FS-002.7: CSRF protection on state-changing requests — server-side validation consumed by staff gate
    function validateToken($token) {
        return ($token && trim($token)==$this->getToken() && !$this->isExpired());
    }

    // @implements FS-001.11: Cross-Site Request Forgery Protection — hidden form input emitter
    function getFormInput($name='') {
        if(!$name) $name = $this->name;

        return sprintf('<input type="hidden" name="%s" value="%s" />', $name, $this->getToken());
    }
}

/* global function to add hidden token input with to forms */
// @implements FS-001.11: Cross-Site Request Forgery Protection — global hidden-token form helper
function csrf_token() {
    global $ost;

    if($ost && $ost->getCSRF())
        echo $ost->getCSRFFormInput();
}
?>
