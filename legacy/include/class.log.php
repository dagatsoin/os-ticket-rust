<?php
/*********************************************************************
    class.log.php

    Log

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

// @implements FS-033.8: Single Log Record Detail — Log read model over the syslog store
// @implements FS-033.3: Log Results Table — backing entity for log record fields
class Log {

    var $id;
    var $info;

    // @implements FS-033.8: Single Log Record Detail — construct + load a log record by id
    function Log($id){
        $this->id=0;
        return $this->load($id);
    }

    // @implements FS-033.8: Single Log Record Detail — fetch one syslog row into the read model
    function load($id){

        $sql='SELECT * FROM '.SYSLOG_TABLE.' WHERE log_id='.db_input($id);
        if(!($res=db_query($sql)) || !db_num_rows($res))
            return false;

        $this->info=db_fetch_array($res);
        $this->id=$this->info['log_id'];
        
        return $this->id;
    }

    function reload(){
        return $this->load($this->getId());
    }

    function getId(){
        return $this->id;
    }
        
    function getType(){
        return $this->info['log_type'];    
    }

    function getTitle(){
        return $this->info['title'];
    }

    function getText(){
        return $this->info['log'];
    }

    function getIp(){
        return $this->info['ip_address'];
    }

    function getCreateDate(){
        return $this->info['created'];
    }

    function getInfo(){
        return $this->info;
    }

    /*** static function ***/
    // @implements FS-033.8: Single Log Record Detail — static lookup of a log record by id
    function lookup($id){
        return ($id && is_numeric($id) && ($l= new Log($id)) && $l->getId()==$id)?$l:null;
    }
}
?>
