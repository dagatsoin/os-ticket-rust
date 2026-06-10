<?php
/*********************************************************************
    class.attachment.php

    Attachment Handler - mainly used for lookup...doesn't save!

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
require_once(INCLUDE_DIR.'class.ticket.php');
require_once(INCLUDE_DIR.'class.file.php');

// @implements FS-022.10: Authorized Attachment Download & Inline Display — ticket↔file attachment binding read model
class Attachment {
    var $id;
    var $file_id;
    var $ticket_id;

    var $info;
    
    // @implements FS-022.10: Authorized Attachment Download & Inline Display — load attachment row (optionally ticket-scoped)
    function Attachment($id,$tid=0) {

        $sql='SELECT * FROM '.TICKET_ATTACHMENT_TABLE.' WHERE attach_id='.db_input($id);
        if($tid)
            $sql.=' AND ticket_id='.db_input($tid);

        if(!($res=db_query($sql)) || !db_num_rows($res))
            return false;
        
        $this->ht=db_fetch_array($res);
        
        $this->id=$this->ht['attach_id'];
        $this->file_id=$this->ht['file_id'];
        $this->ticket_id=$this->ht['ticket_id'];
        
        $this->file=null;
        $this->ticket=null;
        
        return true;
    }
    
    // @implements FS-022.10: Authorized Attachment Download & Inline Display — attachment id accessor
    function getId() {
        return $this->id;
    }

    // @implements FS-022.10: Authorized Attachment Download & Inline Display — owning ticket id (access scope)
    function getTicketId() {
        return $this->ticket_id;
    }

    // @implements FS-022.10: Authorized Attachment Download & Inline Display — resolve owning ticket
    function getTicket() {
        if(!$this->ticket && $this->getTicketId())
            $this->ticket = Ticket::lookup($this->getTicketId());

        return $this->ticket;
    }
    
    // @implements FS-022.12: Content-Addressed Chunked File Storage — backing file id accessor
    function getFileId() {
        return $this->file_id;
    }

    // @implements FS-022.12: Content-Addressed Chunked File Storage — resolve backing stored file
    function getFile() {
        if(!$this->file && $this->getFileId())
            $this->file = AttachmentFile::lookup($this->getFileId());

        return $this->file;
    }

    // @implements FS-022.10: Authorized Attachment Download & Inline Display — attachment creation date accessor
    function getCreateDate() {
        return $this->ht['created'];
    }
    
    // @implements FS-022.10: Authorized Attachment Download & Inline Display — raw record accessor
    function getHashtable() {
        return $this->ht;
    }

    // @implements FS-022.10: Authorized Attachment Download & Inline Display — info alias
    function getInfo() {
        return $this->getHashtable();
    }

    /* Static functions */
    // @implements FS-022.12: Content-Addressed Chunked File Storage — resolve attachment by content hash
    // @implements BS-022.9: Files Are Addressed by Content Hash
    function getIdByFileHash($hash, $tid=0) {
        $sql='SELECT attach_id FROM '.TICKET_ATTACHMENT_TABLE.' a '
            .' INNER JOIN '.FILE_TABLE.' f ON(f.id=a.file_id) '
            .' WHERE f.hash='.db_input($hash);
        if($tid)
            $sql.=' AND a.ticket_id='.db_input($tid);

        return db_result(db_query($sql));
    }

    // @implements FS-022.10: Authorized Attachment Download & Inline Display — load attachment by id or content hash
    function lookup($var,$tid=0) {
        $id=is_numeric($var)?$var:self::getIdByFileHash($var,$tid);

        return ($id && is_numeric($id)
            && ($attach = new Attachment($id,$tid))
            && $attach->getId()==$id)?$attach:null;
    }

}
?>
