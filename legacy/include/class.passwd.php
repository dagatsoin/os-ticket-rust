<?php
/*************************************************************************
    class.passwd.php

    Password Hasher - Interface for phpass bcrypt hasher.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

require_once(INCLUDE_DIR.'PasswordHash.php'); //helper class - will be removed then we move to php5

define('DEFAULT_WORK_FACTOR',8);

// @implements FS-002.13: Password hashing characteristics — bcrypt (phpass) hasher interface with a default work factor
// @implements BS-002-06: Default work factor and range — DEFAULT_WORK_FACTOR 8, clamped to the 4..31 range
class Passwd {

    // @implements FS-002.5: Password verification with legacy fallback and silent rehash — bcrypt password comparison via phpass CheckPassword
    function cmp($passwd,$hash,$work_factor=0){
        
        if($work_factor < 4 || $work_factor > 31)
            $work_factor=DEFAULT_WORK_FACTOR;

        $hasher = new PasswordHash($work_factor,FALSE);

        return ($hasher && $hasher->CheckPassword($passwd,$hash));
    }

    // @implements FS-002.13: Password hashing characteristics — generates a bcrypt hash at the (range-clamped) work factor
    // @implements BS-002-06: Default work factor and range — work factor outside 4..31 falls back to DEFAULT_WORK_FACTOR
    function hash($passwd, $work_factor=0){
       
        if($work_factor < 4 || $work_factor > 31)
            $work_factor=DEFAULT_WORK_FACTOR;

        $hasher = new PasswordHash($work_factor,FALSE);
        
        return ($hasher && ($hash=$hasher->HashPassword($passwd)))?$hash:null;
    }
}
?>
