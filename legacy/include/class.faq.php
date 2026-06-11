<?php
/*********************************************************************
    class.faq.php

    Backend support for article creates, edits, deletes, and attachments.

    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
require_once('class.file.php');
require_once('class.category.php');

// @implements FS-050.7: Public Single-Article View — FAQ article domain entity
// @implements FS-050.17: FAQ Validation Rules — article persistence + validation
// @implements BS-050.1: Public Visibility Requires Published Article AND Public Category
class FAQ {

    var $id;
    var $ht;

    var $category;
    var $attachments;

    // @implements FS-050.7: Public Single-Article View — construct/load an article by id
    function FAQ($id) {
        $this->id=0;
        $this->ht = array();
        $this->load($id);
    }

    // @implements FS-050.7: Public Single-Article View — hydrate article + joined category ispublic + attachment count
    function load($id) {

        $sql='SELECT faq.*,cat.ispublic, count(attach.file_id) as attachments '
            .' FROM '.FAQ_TABLE.' faq '
            .' LEFT JOIN '.FAQ_CATEGORY_TABLE.' cat ON(cat.category_id=faq.category_id) '
            .' LEFT JOIN '.FAQ_ATTACHMENT_TABLE.' attach ON(attach.faq_id=faq.faq_id) '
            .' WHERE faq.faq_id='.db_input($id)
            .' GROUP BY faq.faq_id';

        if (!($res=db_query($sql)) || !db_num_rows($res))
            return false;

        $this->ht = db_fetch_array($res);
        $this->ht['id'] = $this->id = $this->ht['faq_id'];
        $this->category = null;
        $this->attachments = array();

        return true;
    }

    function reload() {
        return $this->load($this->getId());
    }

    /* ------------------> Getter methods <--------------------- */
    function getId() { return $this->id; }
    function getHashtable() { return $this->ht; }
    function getKeywords() { return $this->ht['keywords']; }
    function getQuestion() { return $this->ht['question']; }
    function getAnswer() { return $this->ht['answer']; }
    function getNotes() { return $this->ht['notes']; }
    function getNumAttachments() { return $this->ht['attachments']; }

    // @implements BS-050.1: Public Visibility Requires Published Article AND Public Category — ispublished AND category ispublic
    function isPublished() { return (!!$this->ht['ispublished'] && !!$this->ht['ispublic']); }

    function getCreateDate() { return $this->ht['created']; }
    function getUpdateDate() { return $this->ht['updated']; }

    function getCategoryId() { return $this->ht['category_id']; }
    function getCategory() {
        if(!$this->category && $this->getCategoryId())
            $this->category = Category::lookup($this->getCategoryId());

        return $this->category;
    }

    // @implements FS-050.18: Help-Topic Association Management — resolve associated topic ids
    function getHelpTopicsIds() {

        if (!isset($this->ht['topics']) && ($topics=$this->getHelpTopics())) {
            $this->ht['topics'] = array_keys($topics);
        }

        return $this->ht['topics'];
    }

    // @implements FS-050.18: Help-Topic Association Management — resolve associated topics (parent / child path)
    function getHelpTopics() {
        //XXX: change it to obj (when needed)!

        if (!isset($this->topics)) {
            $this->topics = array();
            $sql='SELECT t.topic_id, CONCAT_WS(" / ", pt.topic, t.topic) as name  FROM '.TOPIC_TABLE.' t '
                .' INNER JOIN '.FAQ_TOPIC_TABLE.' ft ON(ft.topic_id=t.topic_id AND ft.faq_id='.db_input($this->id).') '
                .' LEFT JOIN '.TOPIC_TABLE.' pt ON(pt.topic_id=t.topic_pid) '
                .' ORDER BY t.topic';
            if (($res=db_query($sql)) && db_num_rows($res)) {
                while(list($id,$name) = db_fetch_row($res))
                    $this->topics[$id]=$name;
            }
        }

        return $this->topics;
    }

    /* ------------------> Setter methods <--------------------- */
    function setPublished($val) { $this->ht['ispublished'] = !!$val; }
    function setQuestion($question) { $this->ht['question'] = Format::striptags(trim($question)); }
    function setAnswer($text) { $this->ht['answer'] = $text; }
    function setKeywords($words) { $this->ht['keywords'] = $words; }
    function setNotes($text) { $this->ht['notes'] = $text; }

    /* For ->attach() and ->detach(), use $this->attachments() */
    // @implements FS-050.9: FAQ Article Attachments — attach/detach a file to the article
    function attach($file) { return $this->_attachments->add($file); }
    function detach($file) { return $this->_attachments->remove($file); }

    // @implements FS-050.12: Staff FAQ Mutation Actions — publish the article
    function publish() {
        $this->setPublished(1);

        return $this->apply();
    }

    // @implements FS-050.12: Staff FAQ Mutation Actions — unpublish the article
    function unpublish() {
        $this->setPublished(0);

        return $this->apply();
    }

    /* Same as update - but mainly called after one or more setters are changed. */
    // @implements FS-050.12: Staff FAQ Mutation Actions — persist setter-driven changes (publish/unpublish)
    function apply() {
        //XXX: set errors and add ->getErrors() & ->getError()
        return $this->update($this->ht, $errors);               # nolint
    }

    // @implements FS-050.18: Help-Topic Association Management — sync faq_topic (add new, remove unchecked)
    function updateTopics($ids){

        if($ids) {
            $topics = $this->getHelpTopicsIds();
            foreach($ids as $id) {
                if($topics && in_array($id,$topics)) continue;
                $sql='INSERT IGNORE INTO '.FAQ_TOPIC_TABLE
                    .' SET faq_id='.db_input($this->getId())
                    .', topic_id='.db_input($id);
                db_query($sql);
            }
        }

        $sql='DELETE FROM '.FAQ_TOPIC_TABLE.' WHERE faq_id='.db_input($this->getId());
        if($ids)
            $sql.=' AND topic_id NOT IN('.implode(',', db_input($ids)).')';

        db_query($sql);

        return true;
    }

    // @implements FS-050.12: Staff FAQ Mutation Actions — update article + re-sync topics + attachment keep/delete
    // @implements FS-050.9: FAQ Article Attachments — keep-list detach + new uploads on update
    function update($vars, &$errors) {

        if(!$this->save($this->getId(), $vars, $errors))
            return false;

        $this->updateTopics($vars['topics']);

        //Delete removed attachments.
        $keepers = $vars['files']?$vars['files']:array();
        if(($attachments = $this->getAttachments())) {
            foreach($attachments as $file) {
                if($file['id'] && !in_array($file['id'], $keepers))
                    $this->deleteAttachment($file['id']);
            }
        }

        //Upload new attachments IF any.
        if($_FILES['attachments'] && ($files=AttachmentFile::format($_FILES['attachments'])))
            $this->uploadAttachments($files);

        $this->reload();

        return true;
    }


    // @implements FS-050.9: FAQ Article Attachments — list joined attachment files (storage owned by FS-022)
    function getAttachments() {

        if(!$this->attachments && $this->getNumAttachments()) {

            $sql='SELECT f.id, f.size, f.hash, f.name '
                .' FROM '.FILE_TABLE.' f '
                .' INNER JOIN '.FAQ_ATTACHMENT_TABLE.' a ON(f.id=a.file_id) '
                .' WHERE a.faq_id='.db_input($this->getId());

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

    // @implements FS-050.9: FAQ Article Attachments — render file.php?h=<hash> download links (hash per FS-022)
    function getAttachmentsLinks($separator=' ',$target='') {

        $str='';
        if(($attachments=$this->getAttachments())) {
            foreach($attachments as $attachment ) {
            /* The h key must match validation in file.php */
            $hash=$attachment['hash'].md5($attachment['id'].session_id().$attachment['hash']);
            if($attachment['size'])
                $size=sprintf('&nbsp;<small>(<i>%s</i>)</small>',Format::file_size($attachment['size']));

            $str.=sprintf('<a class="Icon file" href="file.php?h=%s" target="%s">%s</a>%s&nbsp;%s',
                    $hash, $target, Format::htmlchars($attachment['name']), $size, $separator);

            }
        }
        return $str;
    }

    // @implements FS-050.9: FAQ Article Attachments — upload/link files (numeric id reused, else AttachmentFile::upload)
    function uploadAttachments($files) {

        $i=0;
        foreach($files as $file) {
            if(($fileId=is_numeric($file)?$file:AttachmentFile::upload($file)) && is_numeric($fileId)) {
                $sql ='INSERT INTO '.FAQ_ATTACHMENT_TABLE
                     .' SET faq_id='.db_input($this->getId()).', file_id='.db_input($fileId);
                if(db_query($sql)) $i++;
            }
        }

        if($i) $this->reload();

        return $i;
    }

    // @implements FS-050.9: FAQ Article Attachments — detach one attachment + reclaim orphaned file (FS-022)
    function deleteAttachment($file_id) {
        $deleted = 0;
        $sql='DELETE FROM '.FAQ_ATTACHMENT_TABLE
            .' WHERE faq_id='.db_input($this->getId())
            .'   AND file_id='.db_input($file_id);
        if(db_query($sql) && db_affected_rows()) {
            $deleted = AttachmentFile::deleteOrphans();
        }
        return ($deleted > 0);
    }

    // @implements FS-050.9: FAQ Article Attachments — detach all attachments + reclaim orphaned files (FS-022)
    function deleteAttachments(){

        $deleted=0;
        $sql='DELETE FROM '.FAQ_ATTACHMENT_TABLE
            .' WHERE faq_id='.db_input($this->getId());
        if(db_query($sql) && db_affected_rows()) {
            $deleted = AttachmentFile::deleteOrphans();
        }

        return $deleted;
    }


    // @implements FS-050.18: Help-Topic Association Management — delete article cascades faq_topic + attachments
    // @implements FS-050.9: FAQ Article Attachments — reclaim attachments on article delete
    function delete() {

        $sql='DELETE FROM '.FAQ_TABLE
            .' WHERE faq_id='.db_input($this->getId())
            .' LIMIT 1';
        if(!db_query($sql) || !db_affected_rows())
            return false;

        //Cleanup help topics.
        db_query('DELETE FROM '.FAQ_TOPIC_TABLE.' WHERE faq_id='.db_input($this->id));
        //Cleanup attachments.
        $this->deleteAttachments();

        return true;
    }

    /* ------------------> Static methods <--------------------- */

    // @implements FS-050.12: Staff FAQ Mutation Actions — create article then link topics/attachments
    function add($vars, &$errors) {
        if(!($id=self::create($vars, $errors)))
            return false;

        if(($faq=self::lookup($id))) {
            $faq->updateTopics($vars['topics']);

            if($_FILES['attachments'] && ($files=AttachmentFile::format($_FILES['attachments'])))
                $faq->uploadAttachments($files);

            $faq->reload();
        }

        return $faq;
    }

    // @implements FS-050.17: FAQ Validation Rules — create entry point (id=0)
    function create($vars, &$errors) {
        return self::save(0, $vars, $errors);
    }

    // @implements FS-050.3: Public Article / Category Routing — strict lookup (validates getId()==id)
    function lookup($id) {
        return ($id && is_numeric($id) && ($obj= new FAQ($id)) && $obj->getId()==$id)? $obj : null;
    }

    // @implements BS-050.2: Public KB Reachability Requires Toggle AND At Least One Published Public FAQ
    function countPublishedFAQs() {
        $sql='SELECT count(faq.faq_id) '
            .' FROM '.FAQ_TABLE.' faq '
            .' INNER JOIN '.FAQ_CATEGORY_TABLE.' cat ON(cat.category_id=faq.category_id AND cat.ispublic=1) '
            .' WHERE faq.ispublished=1';

        return db_result(db_query($sql));
    }

    // @implements BS-050.4: Article Question Uniqueness — resolve article id by exact question
    function findIdByQuestion($question) {
        $sql='SELECT faq_id FROM '.FAQ_TABLE
            .' WHERE question='.db_input($question);

        list($id) =db_fetch_row(db_query($sql));

        return $id;
    }

    // @implements BS-050.4: Article Question Uniqueness — resolve article by exact question
    function findByQuestion($question) {

        if(($id=self::findIdByQuestion($question)))
            return self::lookup($id);

        return false;
    }

    // @implements FS-050.17: FAQ Validation Rules — validate (question/category/answer) + persist
    // @implements BS-050.4: Article Question Uniqueness — "Question already exists" guard
    // @implements EC-050.12: Mismatched hidden id on edit submit — "Internal error. Try again"
    function save($id, $vars, &$errors, $validation=false) {

        //Cleanup.
        $vars['question']=Format::striptags(trim($vars['question']));

        //validate
        if($id && $id!=$vars['id'])
            $errors['err'] = 'Internal error. Try again';

        if(!$vars['question'])
            $errors['question'] = 'Question required';
        elseif(($qid=self::findIdByQuestion($vars['question'])) && $qid!=$id)
            $errors['question'] = 'Question already exists';

        if(!$vars['category_id'] || !($category=Category::lookup($vars['category_id'])))
            $errors['category_id'] = 'Category is required';

        if(!$vars['answer'])
            $errors['answer'] = 'FAQ answer is required';

        if($errors || $validation) return (!$errors);

        //save
        $sql=' updated=NOW() '
            .', question='.db_input($vars['question'])
            .', answer='.db_input(Format::safe_html($vars['answer']))
            .', category_id='.db_input($vars['category_id'])
            .', ispublished='.db_input(isset($vars['ispublished'])?$vars['ispublished']:0)
            .', notes='.db_input($vars['notes']);

        if($id) {
            $sql='UPDATE '.FAQ_TABLE.' SET '.$sql.' WHERE faq_id='.db_input($id);
            if(db_query($sql))
                return true;

            $errors['err']='Unable to update FAQ.';

        } else {
            $sql='INSERT INTO '.FAQ_TABLE.' SET '.$sql.',created=NOW()';
            if(db_query($sql) && ($id=db_insert_id()))
                return $id;

            $errors['err']='Unable to create FAQ. Internal error';
        }

        return false;
    }
}
?>
