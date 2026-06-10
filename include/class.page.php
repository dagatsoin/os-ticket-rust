<?php
/*********************************************************************
    class.page.php

    Page class

    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

// @implements FS-033.16: Single-Page Enable/Disable/Delete Guards (Entity Rules) — Site Page entity
// @implements FS-033.13: Create & Edit a Site Page
// @implements BS-033.9: Default (Bound) Pages Are In-Use and Protected
class Page {

    var $id;
    var $ht;

    // @implements FS-033.13: Create & Edit a Site Page — construct + load page by id
    function Page($id) {
        $this->id=0;
        $this->ht = array();
        $this->load($id);
    }

    // @implements FS-033.13: Create & Edit a Site Page — load page row + linked-topic count
    function load($id=0) {

        if(!$id && !($id=$this->getId()))
            return false;

        $sql='SELECT page.*, count(topic.page_id) as topics '
            .' FROM '.PAGE_TABLE.' page '
            .' LEFT JOIN '.TOPIC_TABLE. ' topic ON(topic.page_id=page.id) '
            .' WHERE page.id='.db_input($id)
            .' GROUP By page.id';

        if (!($res=db_query($sql)) || !db_num_rows($res))
            return false;

        $this->ht = db_fetch_array($res);
        $this->id = $this->ht['id'];

        return true;
    }

    function reload() {
        return $this->load();
    }

    function getId() {
        return $this->id;
    }

    function getHashtable() {
        return $this->ht;
    }

    function getType() {
        return $this->ht['type'];
    }

    function getName() {
        return $this->ht['name'];
    }

    function getBody() {
        return $this->ht['body'];
    }

    function getNotes() {
        return $this->ht['notes'];
    }

    function isActive() {
        return ($this->ht['isactive']);
    }

    // @implements FS-033.16: Single-Page Enable/Disable/Delete Guards — in-use = linked topics OR default page
    // @implements BS-033.9: Default (Bound) Pages Are In-Use and Protected
    function isInUse() {
        global $cfg;

        return  ($this->getNumTopics()
                    || in_array($this->getId(), $cfg->getDefaultPages()));
    }


    function getCreateDate() {
        return $this->ht['created'];
    }

    function getUpdateDate() {
        return $this->ht['updated'];
    }

    function getNumTopics() {
        return $this->ht['topics'];
    }

    // @implements FS-033.16: Single-Page Enable/Disable/Delete Guards — reject disabling an in-use page
    function update($vars, &$errors) {

        if(!$vars['isactive'] && $this->isInUse()) {
            $errors['err'] = 'A page currently in-use CANNOT be disabled!';
            $errors['isactive'] = 'Page is in-use!';
        }

        if($errors || !$this->save($this->getId(), $vars, $errors))
            return false;

        $this->reload();

        return true;
    }

    // @implements FS-033.16: Single-Page Enable/Disable/Delete Guards — disable no-op when inactive, fail when in-use
    function disable() {

        if(!$this->isActive())
            return true;

        if($this->isInUse())
            return false;


        $sql=' UPDATE '.PAGE_TABLE.' SET isactive=0 '
            .' WHERE id='.db_input($this->getId());

        if(!db_query($sql) || !db_affected_rows())
            return false;

        $this->reload();

        return true;
    }

    // @implements FS-033.16: Single-Page Enable/Disable/Delete Guards — delete fails when in-use, unlinks topics
    function delete() {

        if($this->isInUse())
            return false;

        $sql='DELETE FROM '.PAGE_TABLE
            .' WHERE id='.db_input($this->getId())
            .' LIMIT 1';

        if(!db_query($sql) || !db_affected_rows())
            return false;

        db_query('UPDATE '.TOPIC_TABLE.' SET page_id=0 WHERE page_id='.db_input($this->getId()));

        return true;
    }

    /* ------------------> Static methods <--------------------- */

    // @implements FS-033.13: Create & Edit a Site Page — add = create then lookup
    function add($vars, &$errors) {
        if(!($id=self::create($vars, $errors)))
            return false;

        return self::lookup($id);
    }

    function create($vars, &$errors) {
        return self::save(0, $vars, $errors);
    }

    // @implements FS-033.12: Site Pages Results Table, Sorting & Pagination — list pages by criteria, name-ordered
    function getPages($criteria=array()) {

        $sql = ' SELECT id FROM '.PAGE_TABLE.' WHERE 1';
        if(isset($criteria['active']))
            $sql.=' AND  isactive='.db_input($criteria['active']?1:0);
        if(isset($criteria['type']))
            $sql.=' AND `type`='.db_input($criteria['type']);

        $sql.=' ORDER BY name';

        $pages = array();
        if(($res=db_query($sql)) && db_num_rows($res))
            while(list($id) = db_fetch_row($res))
                $pages[] = Page::lookup($id);

        return array_filter($pages);
    }

    // @implements FS-033.12: Site Pages Results Table, Sorting & Pagination — active-only page listing
    function getActivePages($criteria=array()) {

        $criteria = array_merge($criteria, array('active'=>true));

        return self::getPages($criteria);
    }

    // @implements FS-011: Public Ticket Submission Web Form — active thank-you pages for post-submit landing
    function getActiveThankYouPages() {
        return self::getActivePages(array('type' => 'thank-you'));
    }

    // @implements FS-033.14: Site Page Field Validation & Uniqueness — name lookup for uniqueness check
    function getIdByName($name) {

        $id = 0;
        $sql = ' SELECT id FROM '.PAGE_TABLE.' WHERE name='.db_input($name);
        if(($res=db_query($sql)) && db_num_rows($res))
            list($id) = db_fetch_row($res);

        return $id;
    }

    // @implements FS-033.13: Create & Edit a Site Page — id-validated page lookup
    function lookup($id) {
        return ($id
                && is_numeric($id)
                && ($p= new Page($id))
                && $p->getId()==$id)
            ? $p : null;
    }

    // @implements FS-033.14: Site Page Field Validation & Uniqueness — validate type/name/body + persist
    // @implements BS-033.8: A Site Page Has Exactly One of Four Types
    function save($id, $vars, &$errors) {

        //Cleanup.
        $vars['name']=Format::striptags(trim($vars['name']));

        //validate
        if($id && $id!=$vars['id'])
            $errors['err'] = 'Internal error. Try again';

        if(!$vars['type'])
            $errors['type'] = 'Type required';
        elseif(!in_array($vars['type'], array('landing', 'offline', 'thank-you', 'other')))
            $errors['type'] = 'Invalid selection';

        if(!$vars['name'])
            $errors['name'] = 'Name required';
        elseif(($pid=self::getIdByName($vars['name'])) && $pid!=$id)
            $errors['name'] = 'Name already exists';

        if(!$vars['body'])
            $errors['body'] = 'Page body is required';

        if($errors) return false;

        //save
        $sql=' updated=NOW() '
            .', `type`='.db_input($vars['type'])
            .', name='.db_input($vars['name'])
            .', body='.db_input(Format::safe_html($vars['body']))
            .', isactive='.db_input($vars['isactive'] ? 1 : 0)
            .', notes='.db_input($vars['notes']);

        if($id) {
            $sql='UPDATE '.PAGE_TABLE.' SET '.$sql.' WHERE id='.db_input($id);
            if(db_query($sql))
                return true;

            $errors['err']='Unable to update page.';

        } else {
            $sql='INSERT INTO '.PAGE_TABLE.' SET '.$sql.', created=NOW()';
            if(db_query($sql) && ($id=db_insert_id()))
                return $id;

            $errors['err']='Unable to create page. Internal error';
        }

        return false;
    }
}
?>
