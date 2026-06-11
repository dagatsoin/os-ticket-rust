<?php
/*********************************************************************
    class.category.php

    Backend support for article categories.

    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

// @implements FS-032.14: FAQ Category Create / Edit Form — FAQ-category model + persistence
// @implements FS-050.15: Staff Category Detail (`faq-category.inc.php`) — category read model
class Category {
    var $id;
    var $ht;

    // @implements FS-032.14: FAQ Category Create / Edit Form — construct category by id
    function Category($id) {
        $this->id=0;
        $this->load($id);
    }

    // @implements FS-050.15: Staff Category Detail (`faq-category.inc.php`) — load category row + article count
    function load($id) {

        $sql=' SELECT cat.*,count(faq.faq_id) as faqs '
            .' FROM '.FAQ_CATEGORY_TABLE.' cat '
            .' LEFT JOIN '.FAQ_TABLE.' faq ON(faq.category_id=cat.category_id) '
            .' WHERE cat.category_id='.db_input($id)
            .' GROUP BY cat.category_id';

        if (!($res=db_query($sql)) || !db_num_rows($res)) 
            return false;

        $this->ht = db_fetch_array($res);
        $this->id = $this->ht['category_id'];

        return true;
    }

    // @implements FS-032.14: FAQ Category Create / Edit Form — reload after save
    function reload() {
        return $this->load($this->getId());
    }

    /* ------------------> Getter methods <--------------------- */
    // @implements FS-050.15: Staff Category Detail (`faq-category.inc.php`) — id accessor
    function getId() { return $this->id; }
    // @implements FS-050.15: Staff Category Detail (`faq-category.inc.php`) — name accessor
    function getName() { return $this->ht['name']; }
    // @implements FS-050.15: Staff Category Detail (`faq-category.inc.php`) — article count accessor
    function getNumFAQs() { return  $this->ht['faqs']; }
    // @implements FS-050.15: Staff Category Detail (`faq-category.inc.php`) — safe-HTML description accessor
    function getDescription() { return $this->ht['description']; }
    // @implements FS-032.14: FAQ Category Create / Edit Form — internal notes accessor
    function getNotes() { return $this->ht['notes']; }
    // @implements FS-050.15: Staff Category Detail (`faq-category.inc.php`) — created-date accessor
    function getCreateDate() { return $this->ht['created']; }
    // @implements FS-050.15: Staff Category Detail (`faq-category.inc.php`) — last-updated accessor
    function getUpdateDate() { return $this->ht['updated']; }

    // @implements BS-050.1: Public Visibility Requires Published Article AND Public Category — public flag accessor
    function isPublic() { return ($this->ht['ispublic']); }
    // @implements FS-032.14: FAQ Category Create / Edit Form — raw record accessor (form prefill)
    function getHashtable() { return $this->ht; }
    
    /* ------------------> Setter methods <--------------------- */
    // @implements FS-032.14: FAQ Category Create / Edit Form — name setter
    function setName($name) { $this->ht['name']=$name; }
    // @implements FS-032.14: FAQ Category Create / Edit Form — notes setter
    function setNotes($notes) { $this->ht['notes']=$notes; }
    // @implements FS-032.14: FAQ Category Create / Edit Form — description setter
    function setDescription($desc) { $this->ht['description']=$desc; }

    /* --------------> Database access methods <---------------- */
    // @implements FS-032.14: FAQ Category Create / Edit Form — update existing category + reload
    function update($vars, &$errors) { 

        if(!$this->save($this->getId(), $vars, $errors))
            return false;

        //TODO: move FAQs if requested.

        $this->reload();

        return true;
    }

    // @implements FS-032.16: FAQ Category Deletion Cascade — delete category + cascade-delete its FAQs
    // @implements BS-032.16: Deleting a Category Deletes Its FAQs
    // @implements BS-050.5: An Article Belongs to Exactly One Category; Deleting a Category Deletes Its Articles
    function delete() {

        $sql='DELETE FROM '.FAQ_CATEGORY_TABLE
            .' WHERE category_id='.db_input($this->getId())
            .' LIMIT 1';
        if(db_query($sql) && ($num=db_affected_rows())) {
            db_query('DELETE FROM '.FAQ_TABLE
                    .' WHERE category_id='.db_input($this->getId()));
    
        }

        return $num;
    }

    /* ------------------> Static methods <--------------------- */

    // @implements FS-050.15: Staff Category Detail (`faq-category.inc.php`) — load category by id
    function lookup($id) {
        return ($id && is_numeric($id) && ($c = new Category($id)))?$c:null;
    }

    // @implements BS-032.14: FAQ Category Name Is Trimmed, ≥3 Chars, and Unique — name→id lookup (uniqueness check)
    function findIdByName($name) {
        $sql='SELECT category_id FROM '.FAQ_CATEGORY_TABLE.' WHERE name='.db_input($name);
        list($id) = db_fetch_row(db_query($sql));

        return $id;
    }

    // @implements FS-050.15: Staff Category Detail (`faq-category.inc.php`) — load category by name
    function findByName($name) {
        if(($id=self::findIdByName($name)))
            return new Category($id);

        return false;
    }

    // @implements FS-032.14: FAQ Category Create / Edit Form — validation-only save pass
    function validate($vars, &$errors) {
         return self::save(0, $vars, $errors,true);
    }

    // @implements FS-032.14: FAQ Category Create / Edit Form — create new category
    function create($vars, &$errors) {
        return self::save(0, $vars, $errors);
    }

    // @implements FS-032.14: FAQ Category Create / Edit Form — validate + insert/update category (name HTML-stripped, desc safe-HTML)
    // @implements BS-032.14: FAQ Category Name Is Trimmed, ≥3 Chars, and Unique
    function save($id, $vars, &$errors, $validation=false) {

        //Cleanup.
        $vars['name']=Format::striptags(trim($vars['name']));
      
        //validate
        if($id && $id!=$vars['id'])
            $errors['err']='Internal error. Try again';
      
        if(!$vars['name'])
            $errors['name']='Category name is required';
        elseif(strlen($vars['name'])<3)
            $errors['name']='Name is too short. 3 chars minimum';
        elseif(($cid=self::findIdByName($vars['name'])) && $cid!=$id)
            $errors['name']='Category already exists';

        if(!$vars['description'])
            $errors['description']='Category description is required';

        if($errors) return false;

        /* validation only */
        if($validation) return true;

        //save
        $sql=' updated=NOW() '.
             ',ispublic='.db_input(isset($vars['ispublic'])?$vars['ispublic']:0).
             ',name='.db_input($vars['name']).
             ',description='.db_input(Format::safe_html($vars['description'])).
             ',notes='.db_input($vars['notes']);

        if($id) {
            $sql='UPDATE '.FAQ_CATEGORY_TABLE.' SET '.$sql.' WHERE category_id='.db_input($id);
            if(db_query($sql))
                return true;

            $errors['err']='Unable to update FAQ category.';

        } else {
            $sql='INSERT INTO '.FAQ_CATEGORY_TABLE.' SET '.$sql.',created=NOW()';
            if(db_query($sql) && ($id=db_insert_id()))
                return $id;

            $errors['err']='Unable to create FAQ category. Internal error';
        }

        return false;
    }
}
?>
