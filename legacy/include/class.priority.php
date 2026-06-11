<?php
/*********************************************************************
    class.priority.php

    Priority handle

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

// @implements FS-091.5: Priority Reference Set & Urgency Ordering — Priority catalog entity
// @implements FS-032.8: Ticket Priority Reference Set — admin priority reference
// @implements BS-091.3: Lower Urgency Number Means Higher Priority
class Priority {
    
    var $id;
    var $ht;

    // @implements FS-091.5: Priority Reference Set & Urgency Ordering — construct + load priority by id
    function Priority($id){
        
        $this->id =0;
        $this->load($id);
    }

    // @implements FS-091.5: Priority Reference Set & Urgency Ordering — load priority row (tag/desc/color/urgency/ispublic)
    function load($id) {
        if(!$id && !($id=$this->getId()))
            return false;


        $sql='SELECT *  FROM '.PRIORITY_TABLE
            .' WHERE priority_id='.db_input($id);
        if(!($res=db_query($sql)) || !db_num_rows($res))
            return false;

        $this->ht= db_fetch_array($res);
        $this->id= $this->ht['priority_id'];

        return true;;
    }

    function getId() {
        return $this->id;
    }

    function getTag() {
        return $this->ht['priority'];
    }

    function getDesc() {
        return $this->ht['priority_desc'];
    }

    function getColor() {
        return $this->ht['priority_color'];
    }

    function getUrgency() {
        return $this->ht['priority_urgency'];
    }

    function isPublic() {
        return ($this->ht['ispublic']);
    }

    /* ------------- Static ---------------*/
    // @implements FS-091.5: Priority Reference Set & Urgency Ordering — id-validated priority lookup
    function lookup($id) {
        return ($id && is_numeric($id) && ($p=new Priority($id)) && $p->getId()==$id)?$p:null;
    }

    // @implements FS-091.5: Priority Reference Set & Urgency Ordering — list priorities (optionally public-only)
    // @implements FS-032.8: Ticket Priority Reference Set — priority option list for admin pickers
    function getPriorities( $publicOnly=false) {

        $priorities=array();
        $sql ='SELECT priority_id, priority_desc FROM '.PRIORITY_TABLE;
        if($publicOnly)
            $sql.=' WHERE ispublic=1';

        if(($res=db_query($sql)) && db_num_rows($res)) {
            while(list($id, $name)=db_fetch_row($res))
                $priorities[$id] = $name;
        }

        return $priorities;
    }

    // @implements FS-091.5: Priority Reference Set & Urgency Ordering — public-visible priorities only (ispublic=1)
    // @implements FS-011: Public Ticket Submission Web Form — priority picker for the client open form
    function getPublicPriorities() {
        return self::getPriorities(true);
    }
}
?>
