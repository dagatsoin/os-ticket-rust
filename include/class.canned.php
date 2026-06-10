<?php
/*********************************************************************
    class.canned.php

    Canned Responses AKA Premade replies

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
include_once(INCLUDE_DIR.'class.file.php');

// @implements FS-022.3: Edit / Update Canned Response — canned response model + persistence
// @implements FS-022.2: Create Canned Response
class Canned {
    var $id;
    var $ht;

    var $attachments;
    
    // @implements FS-022.3: Edit / Update Canned Response — construct response by id
    function Canned($id){
        $this->id=0;
        $this->load($id);
    }

    // @implements FS-022.1: Canned Response Library (List View) — load row + attachment/filter counts
    function load($id=0) {

        if(!$id && !($id=$this->getId()))
            return false;

        $sql='SELECT canned.*, count(attach.file_id) as attachments, '
            .' count(filter.id) as filters '
            .' FROM '.CANNED_TABLE.' canned '
            .' LEFT JOIN '.CANNED_ATTACHMENT_TABLE.' attach ON (attach.canned_id=canned.canned_id) ' 
            .' LEFT JOIN '.FILTER_TABLE.' filter ON (canned.canned_id = filter.canned_response_id) '
            .' WHERE canned.canned_id='.db_input($id)
            .' GROUP BY canned.canned_id';

        if(!($res=db_query($sql)) ||  !db_num_rows($res))
            return false;

        
        $this->ht = db_fetch_array($res);
        $this->id = $this->ht['canned_id'];
        $this->attachments = array();
    
        return true;
    }
  
    // @implements FS-022.3: Edit / Update Canned Response — reload after save
    function reload() {
        return $this->load();
    }
    
    // @implements FS-022.1: Canned Response Library (List View) — id accessor
    function getId(){
        return $this->id;
    }

    // @implements BS-022.2: Only Enabled Responses Are Offered for Use — enabled flag accessor
    function isEnabled() {
         return ($this->ht['isenabled']);
    }

    // @implements BS-022.2: Only Enabled Responses Are Offered for Use — active alias
    function isActive(){
        return $this->isEnabled();
    }

    // @implements BS-022.6: A Filter-Referenced Canned Response Cannot Be Deleted — referencing-filter count
    function getNumFilters() {
        return $this->ht['filters'];
    }
    
    // @implements FS-022.1: Canned Response Library (List View) — title accessor
    function getTitle() {
        return $this->ht['title'];
    }

    // @implements FS-022.14: Canned Response Consumption (Reference) — response body accessor
    function getResponse() {
        return $this->ht['response'];
    }

    // @implements FS-022.14: Canned Response Consumption (Reference) — reply-body alias
    function getReply() {
        return $this->getResponse();
    }

    // @implements FS-022.3: Edit / Update Canned Response — internal notes accessor
    function getNotes() {
        return $this->ht['notes'];
    }
    
    // @implements BS-022.1: Department Scope Determines Availability — owning department id accessor
    function getDeptId(){
        return $this->ht['dept_id'];
    }

    // @implements FS-022.3: Edit / Update Canned Response — raw record accessor (form prefill)
    function getHashtable() {
        return $this->ht;
    }

    // @implements FS-022.3: Edit / Update Canned Response — info alias (form prefill)
    function getInfo() {
        return $this->getHashtable();
    }

    // @implements BS-022.6: A Filter-Referenced Canned Response Cannot Be Deleted — list referencing filter names
    function getFilters() {
        if (!$this->_filters) {
            $this->_filters = array();
            $res = db_query(
                  'SELECT name FROM '.FILTER_TABLE
                .' WHERE canned_response_id = '.db_input($this->getId())
                .' ORDER BY name');
            while ($row = db_fetch_row($res))
                $this->_filters[] = $row[0];
        }
        return $this->_filters;
    }

    // @implements FS-022.3: Edit / Update Canned Response — update existing response + reload
    function update($vars, &$errors) {

        if(!$this->save($this->getId(),$vars,$errors))
            return false;
        
        $this->reload();

        return true;
    }
   
    // @implements FS-022.8: Canned Response Attachment Count Cap — current attachment count
    // @implements BS-022.4: Ten-Attachment Cap per Canned Response
    function getNumAttachments() {
        return $this->ht['attachments'];
    }
   
    // @implements FS-022.4: Bind Attachments to a Canned Response — list bound files with session-bound download key
    // @implements BS-022.8: Download Access Requires a Fresh Session-Bound Hash
    function getAttachments() {

        if(!$this->attachments && $this->getNumAttachments()) {
            
            $sql='SELECT f.id, f.size, f.hash, f.name '
                .' FROM '.FILE_TABLE.' f '
                .' INNER JOIN '.CANNED_ATTACHMENT_TABLE.' a ON(f.id=a.file_id) '
                .' WHERE a.canned_id='.db_input($this->getId());

            $this->attachments = array();
            if(($res=db_query($sql)) && db_num_rows($res)) {
                while($rec=db_fetch_array($res)) {
                    $rec['key'] =md5($rec['id'].session_id().$rec['hash']);
                    $this->attachments[] = $rec;
                }
            }
        }
        
        return $this->attachments;
    }
    /*
    @files is an array - hash table of multiple attachments.
    */
    // @implements FS-022.4: Bind Attachments to a Canned Response — upload/bind files (content-addressed dedupe)
    function uploadAttachments($files) {

        $i=0;
        foreach($files as $file) {
            if(($fileId=is_numeric($file)?$file:AttachmentFile::upload($file)) && is_numeric($fileId)) {
                $sql ='INSERT INTO '.CANNED_ATTACHMENT_TABLE
                     .' SET canned_id='.db_input($this->getId()).', file_id='.db_input($fileId);
                if(db_query($sql)) $i++;
            }
        }

        if($i) $this->reload();

        return $i;
    }

    // @implements FS-022.5: Remove Attachment(s) from a Canned Response — unbind one file + reclaim orphan bytes
    // @implements BS-022.10: Shared Files Are Reference-Counted; Bytes Purged Only When Orphaned
    function deleteAttachment($file_id) {
        $deleted = 0;
        $sql='DELETE FROM '.CANNED_ATTACHMENT_TABLE
            .' WHERE canned_id='.db_input($this->getId())
            .'   AND file_id='.db_input($file_id);
        if(db_query($sql) && db_affected_rows()) {
            $deleted = AttachmentFile::deleteOrphans();
        }
        return ($deleted > 0);
    }

    // @implements FS-022.5: Remove Attachment(s) from a Canned Response — unbind all files + reclaim orphan bytes
    // @implements BS-022.5: Deletion Cascades to Attachment Bindings
    function deleteAttachments(){

        $deleted=0;
        $sql='DELETE FROM '.CANNED_ATTACHMENT_TABLE
            .' WHERE canned_id='.db_input($this->getId());
        if(db_query($sql) && db_affected_rows()) {
            $deleted = AttachmentFile::deleteOrphans();
        }

        return $deleted;
    }

    // @implements FS-022.6: Mass-Process Canned Responses (Enable / Disable / Delete) — delete (refused when filter-referenced)
    // @implements BS-022.5: Deletion Cascades to Attachment Bindings
    // @implements BS-022.6: A Filter-Referenced Canned Response Cannot Be Deleted
    function delete(){
        if ($this->getNumFilters() > 0) return false;

        $sql='DELETE FROM '.CANNED_TABLE.' WHERE canned_id='.db_input($this->getId()).' LIMIT 1';
        if(db_query($sql) && ($num=db_affected_rows())) {
            $this->deleteAttachments();
        }

        return $num;
    }

    /*** Static functions ***/
    // @implements FS-022.1: Canned Response Library (List View) — load response by id
    function lookup($id){
        return ($id && is_numeric($id) && ($c= new Canned($id)) && $c->getId()==$id)?$c:null;
    }

    // @implements FS-022.2: Create Canned Response — create new response
    function create($vars,&$errors) { 
        return self::save(0,$vars,$errors);
    }

    // @implements BS-022.3: Title Uniqueness and Minimum Length — title→id lookup (uniqueness check)
    function getIdByTitle($title) {
        $sql='SELECT canned_id FROM '.CANNED_TABLE.' WHERE title='.db_input($title);
        if(($res=db_query($sql)) && db_num_rows($res))
            list($id)=db_fetch_row($res);

        return $id;
    }

    // @implements BS-022.1: Department Scope Determines Availability — enabled responses for a department (+ global)
    // @implements BS-022.2: Only Enabled Responses Are Offered for Use
    function getCannedResponses($deptId=0, $explicit=false) {

        $sql='SELECT canned_id, title FROM '.CANNED_TABLE
           .' WHERE isenabled';
        if($deptId){
            $sql.=' AND (dept_id='.db_input($deptId);
            if(!$explicit)
                $sql.=' OR dept_id=0';
            $sql.=')';
        }
        $sql.=' ORDER BY title';

        $responses = array();
        if(($res=db_query($sql)) && db_num_rows($res)) {
            while(list($id,$title)=db_fetch_row($res))
                $responses[$id]=$title;
        }

        return $responses;
    }

    // @implements BS-022.1: Department Scope Determines Availability — responses-by-department alias
    function responsesByDeptId($deptId, $explicit=false) {
        return self::getCannedResponses($deptId, $explicit);
    }

    // @implements FS-022.7: Canned Response Field Validation — validate + insert/update response (tags stripped)
    // @implements BS-022.3: Title Uniqueness and Minimum Length
    // @implements BS-022.7: Canned Content Is Stored Plain-Text (Tags Stripped)
    function save($id,$vars,&$errors) {

        //We're stripping html tags - until support is added to tickets.
        $vars['title']=Format::striptags(trim($vars['title']));
        $vars['response']=Format::striptags(trim($vars['response']));
        $vars['notes']=Format::striptags(trim($vars['notes']));

        if($id && $id!=$vars['id'])
            $errors['err']='Internal error. Try again';

        if(!$vars['title'])
            $errors['title']='Title required';
        elseif(strlen($vars['title'])<3)
            $errors['title']='Title is too short. 3 chars minimum';
        elseif(($cid=self::getIdByTitle($vars['title'])) && $cid!=$id)
            $errors['title']='Title already exists';

        if(!$vars['response'])
            $errors['response']='Response text required';
            
        if($errors) return false;

        $sql=' updated=NOW() '.
             ',dept_id='.db_input($vars['dept_id']?$vars['dept_id']:0).
             ',isenabled='.db_input($vars['isenabled']).
             ',title='.db_input($vars['title']).
             ',response='.db_input($vars['response']).
             ',notes='.db_input($vars['notes']);

        if($id) {
            $sql='UPDATE '.CANNED_TABLE.' SET '.$sql.' WHERE canned_id='.db_input($id);
            if(db_query($sql))
                return true;

            $errors['err']='Unable to update canned response.';

        } else {
            $sql='INSERT INTO '.CANNED_TABLE.' SET '.$sql.',created=NOW()';
            if(db_query($sql) && ($id=db_insert_id()))
                return $id;

            $errors['err']='Unable to create the canned response. Internal error';
        }

        return false;
    }
}
?>
