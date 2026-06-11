<?php
/*********************************************************************
    class.dept.php

    Department class

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
// @implements FS-030.5: Department creation & update — department model + persistence
// @implements FS-030.6: Department deletion & re-homing
class Dept {
    var $id;

    var $email;
    var $sla;
    var $manager;
    var $members;
    var $groups;

    var $ht;

    // @implements FS-030.4: Department add/edit form — construct department by id
    function Dept($id) {
        $this->id=0;
        $this->load($id);
    }

    // @implements FS-030.3: Department list view — load department row + staff count
    function load($id=0) {
        global $cfg;

        if(!$id && !($id=$this->getId()))
            return false;

        $sql='SELECT dept.*,dept.dept_id as id,dept.dept_name as name, dept.dept_signature as signature, count(staff.staff_id) as users '
            .' FROM '.DEPT_TABLE.' dept '
            .' LEFT JOIN '.STAFF_TABLE.' staff ON (dept.dept_id=staff.dept_id) '
            .' WHERE dept.dept_id='.db_input($id)
            .' GROUP BY dept.dept_id';

        if(!($res=db_query($sql)) || !db_num_rows($res))
            return false;



        $this->ht=db_fetch_array($res);
        $this->id=$this->ht['dept_id'];
        $this->email=$this->sla=$this->manager=null;
        $this->getEmail(); //Auto load email struct.
        $this->members=$this->groups=array();

        return true;
    }

    // @implements FS-030.5: Department creation & update — reload after save
    function reload() {
        return $this->load();
    }

    // @implements FS-030.3: Department list view — template-variable name
    function asVar() {
        return $this->getName();
    }

    // @implements FS-030.3: Department list view — id accessor
    function getId() {
        return $this->id;
    }

    // @implements FS-030.3: Department list view — name accessor
    function getName() {
        return $this->ht['name'];
    }


    // @implements FS-030.4: Department add/edit form — outbound email id accessor
    // @implements BS-030-02 — Email identity required
    function getEmailId() {
        return $this->ht['email_id'];
    }

    // @implements FS-030.4: Department add/edit form — resolve outbound email account
    function getEmail() {

        if(!$this->email && $this->getEmailId())
            $this->email=Email::lookup($this->getEmailId());

        return $this->email;
    }

    // @implements FS-030.6: Department deletion & re-homing — home-staff count (delete guard)
    // @implements BS-030-06 — Department deletion requires zero home staff
    function getNumStaff() {
        return $this->ht['users'];
    }


    // @implements FS-030.6: Department deletion & re-homing — users-count alias (delete guard)
    function getNumUsers() {
        return $this->getNumStaff();
    }

    // @implements FS-030.3: Department list view — effective member count
    function getNumMembers() {
        return count($this->getMembers());
    }

    // @implements FS-030.5: Department creation & update — resolve effective members (home + manager + group access)
    // @implements BS-030-08 — Allowed-group access is a full-replace sync
    function getMembers() {

        if(!$this->members) {
            $this->members = array();
            $sql='SELECT DISTINCT s.staff_id FROM '.STAFF_TABLE.' s '
                .' LEFT JOIN '.GROUP_DEPT_TABLE.' g ON(s.group_id=g.group_id) '
                .' INNER JOIN '.DEPT_TABLE.' d
                       ON(d.dept_id=s.dept_id
                            OR d.manager_id=s.staff_id
                            OR (d.dept_id=g.dept_id AND d.group_membership=1)
                        ) '
                .' WHERE d.dept_id='.db_input($this->getId())
                .' ORDER BY s.lastname, s.firstname';

            if(($res=db_query($sql)) && db_num_rows($res)) {
                while(list($id)=db_fetch_row($res))
                    $this->members[] = Staff::lookup($id);
            }
        }

        return $this->members;
    }


    // @implements FS-030.4: Department add/edit form — SLA plan id accessor
    function getSLAId() {
        return $this->ht['sla_id'];
    }

    // @implements FS-030.4: Department add/edit form — resolve SLA plan
    function getSLA() {

        if(!$this->sla && $this->getSLAId())
            $this->sla=SLA::lookup($this->getSLAId());

        return $this->sla;
    }

    // @implements FS-030.4: Department add/edit form — template group id accessor
    // @implements BS-030-03 — Template required
    function getTemplateId() {
         return $this->ht['tpl_id'];
    }

    // @implements FS-030.4: Department add/edit form — resolve template group
    function getTemplate() {

        if(!$this->template && $this->getTemplateId())
            $this->template = EmailTemplateGroup::lookup($this->getTemplateId());

        return $this->template;
    }

    // @implements FS-030.4: Department add/edit form — auto-response email (falls back to dept email)
    function getAutoRespEmail() {

        if(!$this->autorespEmail && $this->ht['autoresp_email_id'] && ($email=Email::lookup($this->ht['autoresp_email_id'])))
            $this->autorespEmail=$email;
        else // Defualt to dept email if autoresp is not specified or deleted.
            $this->autorespEmail=$this->getEmail();

        return $this->autorespEmail;
    }

    // @implements FS-030.4: Department add/edit form — outbound email address accessor
    function getEmailAddress() {
        if(($email=$this->getEmail()))
            return $email->getAddress();
    }

    // @implements FS-030.4: Department add/edit form — signature accessor
    // @implements BS-030-10 — Signature is public-only and optional
    function getSignature() {
        return $this->ht['signature'];
    }

    // @implements BS-030-10 — Signature is public-only and optional — offer signature only when public + present
    function canAppendSignature() {
        return ($this->getSignature() && $this->isPublic());
    }

    // @implements FS-030.4: Department add/edit form — manager staff id accessor
    function getManagerId() {
        return $this->ht['manager_id'];
    }

    // @implements FS-030.4: Department add/edit form — resolve manager staff
    function getManager() {

        if(!$this->manager && $this->getManagerId())
            $this->manager=Staff::lookup($this->getManagerId());

        return $this->manager;
    }

    // @implements FS-030.5: Department creation & update — is-manager check (retains access regardless of group list)
    function isManager($staff) {

        if(is_object($staff)) $staff=$staff->getId();

        return ($this->getManagerId() && $this->getManagerId()==$staff);
    }


    // @implements FS-030.4: Department add/edit form — public/private flag accessor
    // @implements BS-030-04 — Default department cannot be private
    function isPublic() {
         return ($this->ht['ispublic']);
    }

    // @implements FS-030.4: Department add/edit form — new-ticket autoresponse flag
    function autoRespONNewTicket() {
        return ($this->ht['ticket_auto_response']);
    }

    // @implements FS-030.4: Department add/edit form — new-message autoresponse flag
    function autoRespONNewMessage() {
        return ($this->ht['message_auto_response']);
    }

    // @implements FS-030.4: Department add/edit form — no-reply autoresponse flag
    function noreplyAutoResp() {
         return ($this->ht['noreply_autoresp']);
    }


    // @implements FS-030.4: Department add/edit form — group-membership-access flag
    // @implements BS-030-08 — Allowed-group access is a full-replace sync
    function isGroupMembershipEnabled() {
        return ($this->ht['group_membership']);
    }

    // @implements FS-030.4: Department add/edit form — raw record accessor (form prefill)
    function getHashtable() {
        return $this->ht;
    }

    // @implements FS-030.4: Department add/edit form — info alias (form prefill)
    function getInfo() {
        return $this->getHashtable();
    }



    // @implements FS-030.5: Department creation & update — read allowed-group access set
    // @implements BS-030-08 — Allowed-group access is a full-replace sync
    function getAllowedGroups() {

        if($this->groups) return $this->groups;

        $sql='SELECT group_id FROM '.GROUP_DEPT_TABLE
            .' WHERE dept_id='.db_input($this->getId());

        if(($res=db_query($sql)) && db_num_rows($res)) {
            while(list($id)=db_fetch_row($res))
                $this->groups[] = $id;
        }

        return $this->groups;
    }

    // @implements FS-030.5: Department creation & update — full-replace sync of allowed-group access
    // @implements BS-030-08 — Allowed-group access is a full-replace sync
    function updateAllowedGroups($groups) {

        if($groups && is_array($groups)) {
            foreach($groups as $k=>$id) {
                $sql='INSERT IGNORE INTO '.GROUP_DEPT_TABLE
                    .' SET dept_id='.db_input($this->getId()).', group_id='.db_input($id);
                db_query($sql);
            }
        }


        $sql='DELETE FROM '.GROUP_DEPT_TABLE.' WHERE dept_id='.db_input($this->getId());
        if($groups && is_array($groups))
            $sql.=' AND group_id NOT IN('.implode(',', db_input($groups)).')';

        db_query($sql);

        return true;

    }

    // @implements FS-030.5: Department creation & update — update existing department + sync groups + reload
    function update($vars, &$errors) {

        if(!$this->save($this->getId(), $vars, $errors))
            return false;

        $this->updateAllowedGroups($vars['groups']);
        $this->reload();

        return true;
    }

    // @implements FS-030.6: Department deletion & re-homing — guarded delete + re-home tickets/staff/topics, drop group rows
    // @implements BS-030-05 — Default department cannot be deleted or disabled
    // @implements BS-030-06 — Department deletion requires zero home staff
    // @implements BS-030-07 — Deletion re-homes dependents to the default department
    function delete() {
        global $cfg;

        if(!$cfg || $this->getId()==$cfg->getDefaultDeptId() || $this->getNumUsers())
            return 0;

        $id=$this->getId();
        $sql='DELETE FROM '.DEPT_TABLE.' WHERE dept_id='.db_input($id).' LIMIT 1';
        if(db_query($sql) && ($num=db_affected_rows())) {
            // DO SOME HOUSE CLEANING
            //Move tickets to default Dept. TODO: Move one ticket at a time and send alerts + log notes.
            db_query('UPDATE '.TICKET_TABLE.' SET dept_id='.db_input($cfg->getDefaultDeptId()).' WHERE dept_id='.db_input($id));
            //Move Dept members: This should never happen..since delete should be issued only to empty Depts...but check it anyways
            db_query('UPDATE '.STAFF_TABLE.' SET dept_id='.db_input($cfg->getDefaultDeptId()).' WHERE dept_id='.db_input($id));
            //make help topic using the dept default to default-dept.
            db_query('UPDATE '.TOPIC_TABLE.' SET dept_id='.db_input($cfg->getDefaultDeptId()).' WHERE dept_id='.db_input($id));
            //Delete group access
            db_query('DELETE FROM '.GROUP_DEPT_TABLE.' WHERE dept_id='.db_input($id));
        }

        return $num;
    }

    /*----Static functions-------*/
	// @implements BS-030-01 — Department name required, minimum length, unique — name→id lookup (uniqueness check)
	function getIdByName($name) {
        $id=0;
        $sql ='SELECT dept_id FROM '.DEPT_TABLE.' WHERE dept_name='.db_input($name);
        if(($res=db_query($sql)) && db_num_rows($res))
            list($id)=db_fetch_row($res);

        return $id;
    }

    // @implements FS-030.3: Department list view — load department by id
    function lookup($id) {
        return ($id && is_numeric($id) && ($dept = new Dept($id)) && $dept->getId()==$id)?$dept:null;
    }

    // @implements FS-030.3: Department list view — resolve department name by id
    function getNameById($id) {

        if($id && ($dept=Dept::lookup($id)))
            $name= $dept->getName();

        return $name;
    }

    // @implements FS-030.6: Department deletion & re-homing — default department name (re-home target)
    function getDefaultDeptName() {
        global $cfg;
        return ($cfg && $cfg->getDefaultDeptId() && ($name=Dept::getNameById($cfg->getDefaultDeptId())))?$name:null;
    }

    // @implements FS-030.3: Department list view — list departments (optional public-only / manager filter)
    function getDepartments( $criteria=null) {

        $depts=array();
        $sql='SELECT dept_id, dept_name FROM '.DEPT_TABLE.' WHERE 1';
        if($criteria['publiconly'])
            $sql.=' AND  ispublic=1';

        if(($manager=$criteria['manager']))
            $sql.=' AND manager_id='.db_input(is_object($manager)?$manager->getId():$manager);

        if(($res=db_query($sql)) && db_num_rows($res)) {
            while(list($id, $name)=db_fetch_row($res))
                $depts[$id] = $name;
        }

        return $depts;
    }

    // @implements FS-030.3: Department list view — public departments only
    function getPublicDepartments() {
        return self::getDepartments(array('publiconly'=>true));
    }

    // @implements FS-030.5: Department creation & update — create + seed allowed-group access
    function create($vars, &$errors) {
        if(($id=self::save(0, $vars, $errors)) && ($dept=self::lookup($id)))
            $dept->updateAllowedGroups($vars['groups']);

        return $id;
    }

    // @implements FS-030.5: Department creation & update — validate + insert/update department row (HTML-strip name/signature)
    // @implements BS-030-01 — Department name required, minimum length, unique
    // @implements BS-030-02 — Email identity required
    // @implements BS-030-03 — Template required
    // @implements BS-030-04 — Default department cannot be private
    // @implements BS-030-11 — Name and signature are HTML-stripped on save
    function save($id, $vars, &$errors) {
        global $cfg;

        if($id && $id!=$vars['id'])
            $errors['err']='Missing or invalid Dept ID (internal error).';

        if(!$vars['email_id'] || !is_numeric($vars['email_id']))
            $errors['email_id']='Email selection required';

        if(!is_numeric($vars['tpl_id']))
            $errors['tpl_id']='Template selection required';

        if(!$vars['name']) {
            $errors['name']='Name required';
        } elseif(strlen($vars['name'])<4) {
            $errors['name']='Name is too short.';
        } elseif(($did=Dept::getIdByName($vars['name'])) && $did!=$id) {
            $errors['name']='Department already exists';
        }

        if(!$vars['ispublic'] && ($vars['id']==$cfg->getDefaultDeptId()))
            $errors['ispublic']='System default department cannot be private';

        if($errors) return false;


        $sql='SET updated=NOW() '
            .' ,ispublic='.db_input($vars['ispublic'])
            .' ,email_id='.db_input($vars['email_id'])
            .' ,tpl_id='.db_input($vars['tpl_id'])
            .' ,sla_id='.db_input($vars['sla_id'])
            .' ,autoresp_email_id='.db_input($vars['autoresp_email_id'])
            .' ,manager_id='.db_input($vars['manager_id']?$vars['manager_id']:0)
            .' ,dept_name='.db_input(Format::striptags($vars['name']))
            .' ,dept_signature='.db_input(Format::striptags($vars['signature']))
            .' ,group_membership='.db_input(isset($vars['group_membership'])?1:0)
            .' ,ticket_auto_response='.db_input(isset($vars['ticket_auto_response'])?$vars['ticket_auto_response']:1)
            .' ,message_auto_response='.db_input(isset($vars['message_auto_response'])?$vars['message_auto_response']:1);


        if($id) {
            $sql='UPDATE '.DEPT_TABLE.' '.$sql.' WHERE dept_id='.db_input($id);
            if(db_query($sql) && db_affected_rows())
                return true;

            $errors['err']='Unable to update '.Format::htmlchars($vars['name']).' Dept. Error occurred';

        } else {
            $sql='INSERT INTO '.DEPT_TABLE.' '.$sql.',created=NOW()';
            if(db_query($sql) && ($id=db_insert_id()))
                return $id;


            $errors['err']='Unable to create department. Internal error';

        }


        return false;
    }

}
?>
