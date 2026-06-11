<?php
/*********************************************************************
    class.timezone.php

    Time zone get utils.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

// @implements FS-003.25: Timezone reference read model — read-model accessor for ost_timezone entries used in time conversion
// @implements FS-091.15: Installed Table Inventory — reads the ost_timezone reference table (schema owned by FS-091)
class Timezone {

    var $id;
    var $ht;

    function Timezone($id){
        $this->id=0;
        return $this->load($id);
    }

    // @implements FS-003.25: Timezone reference read model — loads a timezone row by id into the object's data slot ($ht)
    function load($id=0) {

        if(!$id && !($id=$this->getId()))
            return false;

        $sql='SELECT * FROM '.TIMEZONE_TABLE.' WHERE id='.db_input($id);
        if(!($res=db_query($sql)) || !db_num_rows($res))
            return false;

        $this->ht=db_fetch_array($res);
        $this->id=$this->ht['id'];
        
        return $this->id;
    }

    // @implements FS-003.25: Timezone reference read model — reloads the current timezone row
    function reload() {
        return $this->load();
    }

    // @implements FS-003.25: Timezone reference read model — returns the loaded timezone id
    function getId() { 
        return $this->id;
    }
        
    // @implements FS-003.25: Timezone reference read model — returns the entry's UTC offset (read correctly from the $ht data slot)
    function getOffset() {
        return $this->ht['offset'];    
    }

    // @implements FS-003.25: Timezone reference read model — name accessor
    // @implements KL-013: Timezone name not reliably retrievable — reads the never-populated $this->info slot, always empty
    function getName() {
        return $this->info['timezone'];
    }

    // @implements FS-003.25: Timezone reference read model — description accessor (delegates to the broken getName, KL-013)
    function getDesc() {
        return $this->getName();
    }

    /* static functions */
    // @implements FS-003.25: Timezone reference read model — static lookup returning a populated Timezone only on a numeric id-match
    function lookup($id) {
        return ($id && is_numeric($id) && ($tz= new Timezone($id)) && $tz->getId()==$id)?$tz:null;
    }

    // @implements FS-003.25: Timezone reference read model — returns the entry's UTC offset by id, or 0 when no entry matches
    function getOffsetById($id) {
        return ($tz=Timezone::lookup($id))?$tz->getOffset():0;
    }
}
?>
