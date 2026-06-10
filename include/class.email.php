<?php
/*********************************************************************
    class.email.php

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

include_once(INCLUDE_DIR.'class.dept.php');
include_once(INCLUDE_DIR.'class.mailfetch.php');

// @implements FS-040.3: Email Account Save Validation & Provisioning — email account model + persistence
// @implements FS-040.12: Outbound Mail Composition & From-Address Selection
class Email {
    var $id;
    var $address;

    var $dept;
    var $ht;

    // @implements FS-040.2: Email Account Create / Edit Form — construct account by id
    function Email($id) {
        $this->id=0;
        $this->load($id);
    }

    // @implements FS-040.1: Email Account Listing — load account row + compose "Name <email>" address
    function load($id=0) {

        if(!$id && !($id=$this->getId()))
            return false;

        $sql='SELECT * FROM '.EMAIL_TABLE.' WHERE email_id='.db_input($id);
        if(!($res=db_query($sql)) || !db_num_rows($res))
            return false;


        $this->ht=db_fetch_array($res);
        $this->id=$this->ht['email_id'];
        $this->address=$this->ht['name']?($this->ht['name'].'<'.$this->ht['email'].'>'):$this->ht['email'];

        $this->dept = null;

        return true;
    }

    // @implements FS-040.3: Email Account Save Validation & Provisioning — reload after save
    function reload() {
        return $this->load();
    }

    // @implements FS-040.1: Email Account Listing — id accessor
    function getId() {
        return $this->id;
    }

    // @implements FS-040.1: Email Account Listing — bare email address accessor
    function getEmail() {
        return $this->ht['email'];
    }

    // @implements FS-040.12: Outbound Mail Composition & From-Address Selection — "Name <email>" From address
    function getAddress() {
        return $this->address;
    }

    // @implements FS-040.12: Outbound Mail Composition & From-Address Selection — display name accessor
    function getName() {
        return $this->ht['name'];
    }

    // @implements FS-040.2: Email Account Create / Edit Form — default priority id accessor
    function getPriorityId() {
        return $this->ht['priority_id'];
    }

    // @implements FS-040.2: Email Account Create / Edit Form — routing department id accessor
    function getDeptId() {
        return $this->ht['dept_id'];
    }

    // @implements FS-040.2: Email Account Create / Edit Form — resolve routing department
    function getDept() {

        if(!$this->dept && $this->getDeptId())
            $this->dept=Dept::lookup($this->getDeptId());

        return $this->dept;
    }

    // @implements FS-040.2: Email Account Create / Edit Form — autoresponder-enabled flag
    function autoRespond() {
        return (!$this->ht['noautoresp']);
    }

    // @implements FS-040.3: Email Account Save Validation & Provisioning — decrypt stored mailbox/SMTP password
    // @implements BS-040.24: Blank Password on Edit Preserves the Stored Secret
    function getPasswd() {
        return $this->ht['userpass']?Crypto::decrypt($this->ht['userpass'], SECRET_SALT, $this->ht['userid']):'';
    }

    // @implements FS-040.2: Email Account Create / Edit Form — raw record accessor (form prefill)
    function getHashtable() {
        return $this->ht;
    }

    // @implements FS-040.2: Email Account Create / Edit Form — info alias (form prefill)
    function getInfo() {
        return $this->getHashtable();
    }

    // @implements FS-040.3: Email Account Save Validation & Provisioning — assemble fetch-account descriptor (consumed by FS-041 MailFetcher)
    function getMailAccountInfo() {

        /*NOTE: Do not change any of the tags - otherwise mail fetching will fail */
        $info = array(
                //Mail server info
                'host'  => $this->ht['mail_host'],
                'port'  => $this->ht['mail_port'],
                'protocol'  => $this->ht['mail_protocol'],
                'encryption' => $this->ht['mail_encryption'],
                'username'  => $this->ht['userid'],
                'password' => Crypto::decrypt($this->ht['userpass'], SECRET_SALT, $this->ht['userid']),
                //osTicket specific
                'email_id'  => $this->getId(), //Required for email routing to work.
                'max_fetch' => $this->ht['mail_fetchmax'],
                'delete_mail' => $this->ht['mail_delete'],
                'archive_folder' => $this->ht['mail_archivefolder']
                );

        return $info;
    }

    // @implements FS-040.12: Outbound Mail Composition & From-Address Selection — SMTP-transport-available predicate
    function isSMTPEnabled() {

        return (
                $this->ht['smtp_active']
                    && ($info=$this->getSMTPInfo())
                    && (!$info['auth'] || $info['password'])
                );
    }

    // @implements FS-040.12: Outbound Mail Composition & From-Address Selection — header-spoofing-allowed flag
    function allowSpoofing() {
        return ($this->ht['smtp_spoofing']);
    }

    // @implements FS-040.12: Outbound Mail Composition & From-Address Selection — assemble SMTP transport descriptor
    function getSMTPInfo() {

        $info = array (
                'host' => $this->ht['smtp_host'],
                'port' => $this->ht['smtp_port'],
                'auth' => (bool) $this->ht['smtp_auth'],
                'username' => $this->ht['userid'],
                'password' => Crypto::decrypt($this->ht['userpass'], SECRET_SALT, $this->ht['userid'])
                );

        return $info;
    }

    // @implements FS-040.12: Outbound Mail Composition & From-Address Selection — compose + dispatch via Mailer
    function send($to, $subject, $message, $attachments=null, $options=null) {

        $mailer = new Mailer($this);
        if($attachments)
            $mailer->addAttachments($attachments);

        return $mailer->send($to, $subject, $message, $options);
    }

    // @implements FS-040.13: Specialized Send Wrappers — auto-reply send (Precedence: auto_reply headers)
    function sendAutoReply($to, $subject, $message, $attachments=null, $options=array()) {
        $options+= array('autoreply' => true);
        return $this->send($to, $subject, $message, $attachments, $options);
    }

    // @implements FS-040.13: Specialized Send Wrappers — notice/alert send (auto-generated suppress headers)
    function sendAlert($to, $subject, $message, $attachments=null, $options=array()) {
        $options+= array('notice' => true);
        return $this->send($to, $subject, $message, $attachments, $options);
    }

    // @implements FS-040.3: Email Account Save Validation & Provisioning — update account (carry current password forward)
    // @implements BS-040.24: Blank Password on Edit Preserves the Stored Secret
    function update($vars,&$errors) {
        $vars=$vars;
        $vars['cpasswd']=$this->getPasswd(); //Current decrypted password.

        if(!$this->save($this->getId(), $vars, $errors))
            return false;

        $this->reload();

        return true;
    }


   // @implements FS-040.4: Email Account Bulk Delete — guarded delete (not default/alert account) + re-home dept refs
   function delete() {
        global $cfg;
        //Make sure we are not trying to delete default emails.
        if(!$cfg || $this->getId()==$cfg->getDefaultEmailId() || $this->getId()==$cfg->getAlertEmailId()) //double...double check.
            return 0;

        $sql='DELETE FROM '.EMAIL_TABLE.' WHERE email_id='.db_input($this->getId()).' LIMIT 1';
        if(db_query($sql) && ($num=db_affected_rows())) {
            $sql='UPDATE '.DEPT_TABLE.' SET autoresp_email_id=0 '.
                 ',email_id='.db_input($cfg->getDefaultEmailId()).
                 ' WHERE email_id='.db_input($this->getId());
            db_query($sql);
        }

        return $num;
    }


    /******* Static functions ************/

   // @implements BS-040.1: Email Address Uniqueness — address→id lookup (uniqueness check)
   function getIdByEmail($email) {

        $sql='SELECT email_id FROM '.EMAIL_TABLE.' WHERE email='.db_input($email);
        if(($res=db_query($sql)) && db_num_rows($res))
            list($id)=db_fetch_row($res);

        return $id;
    }

    // @implements FS-040.1: Email Account Listing — load account by id or address
    function lookup($var) {
        $id=is_numeric($var)?$var:Email::getIdByEmail($var);
        return ($id && is_numeric($id) && ($email=new Email($id)) && $email->getId())?$email:null;
    }

    // @implements FS-040.3: Email Account Save Validation & Provisioning — create new account
    function create($vars,&$errors) {
        return Email::save(0,$vars,$errors);
    }


    // @implements FS-040.3: Email Account Save Validation & Provisioning — validate, live-check IMAP/SMTP, persist account
    // @implements BS-040.1: Email Address Uniqueness
    // @implements BS-040.2: Mailbox Host + Username Uniqueness
    // @implements BS-040.25: Hidden Record-Id Integrity Guard
    // @implements BS-040.26: Save Normalizes Operational Fields and Defaults
    function save($id,$vars,&$errors) {
        global $cfg;
        //very basic checks

        $vars['name']=Format::striptags(trim($vars['name']));
        $vars['email']=trim($vars['email']);

        if($id && $id!=$vars['id'])
            $errors['err']='Internal error. Get technical help.';

        if(!$vars['email'] || !Validator::is_email($vars['email'])) {
            $errors['email']='Valid email required';
        }elseif(($eid=Email::getIdByEmail($vars['email'])) && $eid!=$id) {
            $errors['email']='Email already exists';
        }elseif($cfg && !strcasecmp($cfg->getAdminEmail(), $vars['email'])) {
            $errors['email']='Email already used as admin email!';
        }elseif(Staff::getIdByEmail($vars['email'])) { //make sure the email doesn't belong to any of the staff
            $errors['email']='Email in use by a staff member';
        }

        if(!$vars['name'])
            $errors['name']='Email name required';

        if($vars['mail_active'] || ($vars['smtp_active'] && $vars['smtp_auth'])) {
            if(!$vars['userid'])
                $errors['userid']='Username missing';

            if(!$id && !$vars['passwd'])
                $errors['passwd']='Password required';
            elseif($vars['passwd']
                    && $vars['userid']
                    && !Crypto::encrypt($vars['passwd'], SECRET_SALT, $vars['userid'])
                    )
                $errors['passwd'] = 'Unable to encrypt password - get technical support';
        }

        if($vars['mail_active']) {
            //Check pop/imapinfo only when enabled.
            if(!function_exists('imap_open'))
                $errors['mail_active']= 'IMAP doesn\'t exist. PHP must be compiled with IMAP enabled.';
            if(!$vars['mail_host'])
                $errors['mail_host']='Host name required';
            if(!$vars['mail_port'])
                $errors['mail_port']='Port required';
            if(!$vars['mail_protocol'])
                $errors['mail_protocol']='Select protocol';
            if(!$vars['mail_fetchfreq'] || !is_numeric($vars['mail_fetchfreq']))
                $errors['mail_fetchfreq']='Fetch interval required';
            if(!$vars['mail_fetchmax'] || !is_numeric($vars['mail_fetchmax']))
                $errors['mail_fetchmax']='Maximum emails required';
            if(!$vars['dept_id'] || !is_numeric($vars['dept_id']))
                $errors['dept_id']='You must select a Dept.';
            if(!$vars['priority_id'])
                $errors['priority_id']='You must select a priority';

            if(!isset($vars['postfetch']))
                $errors['postfetch']='Indicate what to do with fetched emails';
            elseif(!strcasecmp($vars['postfetch'],'archive') && !$vars['mail_archivefolder'] )
                $errors['postfetch']='Valid folder required';
        }

        if($vars['smtp_active']) {
            if(!$vars['smtp_host'])
                $errors['smtp_host']='Host name required';
            if(!$vars['smtp_port'])
                $errors['smtp_port']='Port required';
        }

        //abort on errors
        if($errors) return false;

        if(!$errors && ($vars['mail_host'] && $vars['userid'])) {
            $sql='SELECT email_id FROM '.EMAIL_TABLE
                .' WHERE mail_host='.db_input($vars['mail_host']).' AND userid='.db_input($vars['userid']);
            if($id)
                $sql.=' AND email_id!='.db_input($id);

            if(db_num_rows(db_query($sql)))
                $errors['userid']=$errors['host']='Host/userid combination already in use.';
        }

        $passwd=$vars['passwd']?$vars['passwd']:$vars['cpasswd'];
        if(!$errors && $vars['mail_active']) {
            //note: password is unencrypted at this point...MailFetcher expect plain text.
            $fetcher = new MailFetcher(
                    array(
                        'host'  => $vars['mail_host'],
                        'port'  => $vars['mail_port'],
                        'username'  => $vars['userid'],
                        'password'  => $passwd,
                        'protocol'  => $vars['mail_protocol'],
                        'encryption' => $vars['mail_encryption'])
                    );
            if(!$fetcher->connect()) {
                $errors['err']='Invalid login. Check '.Format::htmlchars($vars['mail_protocol']).' settings';
                $errors['mail']='<br>'.$fetcher->getLastError();
            }elseif($vars['mail_archivefolder'] && !$fetcher->checkMailbox($vars['mail_archivefolder'],true)) {
                 $errors['postfetch']='Invalid or unknown mail folder! >> '.$fetcher->getLastError().'';
                 if(!$errors['mail'])
                     $errors['mail']='Invalid or unknown archive folder!';
            }
        }

        if(!$errors && $vars['smtp_active']) { //Check SMTP login only.
            require_once 'Mail.php'; // PEAR Mail package
            $smtp = mail::factory('smtp',
                    array ('host' => $vars['smtp_host'],
                           'port' => $vars['smtp_port'],
                           'auth' => (bool) $vars['smtp_auth'],
                           'username' =>$vars['userid'],
                           'password' =>$passwd,
                           'timeout'  =>20,
                           'debug' => false,
                           ));
            $mail = $smtp->connect();
            if(PEAR::isError($mail)) {
                $errors['err']='Unable to log in. Check SMTP settings.';
                $errors['smtp']='<br>'.$mail->getMessage();
            }else{
                $smtp->disconnect(); //Thank you, sir!
            }
        }

        if($errors) return false;

        //Default to default priority and dept..
        if(!$vars['priority_id'] && $cfg)
            $vars['priority_id']=$cfg->getDefaultPriorityId();
        if(!$vars['dept_id'] && $cfg)
            $vars['dept_id']=$cfg->getDefaultDeptId();

        $sql='updated=NOW(),mail_errors=0, mail_lastfetch=NULL'.
             ',email='.db_input($vars['email']).
             ',name='.db_input(Format::striptags($vars['name'])).
             ',dept_id='.db_input($vars['dept_id']).
             ',priority_id='.db_input($vars['priority_id']).
             ',noautoresp='.db_input(isset($vars['noautoresp'])?1:0).
             ',userid='.db_input($vars['userid']).
             ',mail_active='.db_input($vars['mail_active']).
             ',mail_host='.db_input($vars['mail_host']).
             ',mail_protocol='.db_input($vars['mail_protocol']?$vars['mail_protocol']:'POP').
             ',mail_encryption='.db_input($vars['mail_encryption']).
             ',mail_port='.db_input($vars['mail_port']?$vars['mail_port']:0).
             ',mail_fetchfreq='.db_input($vars['mail_fetchfreq']?$vars['mail_fetchfreq']:0).
             ',mail_fetchmax='.db_input($vars['mail_fetchmax']?$vars['mail_fetchmax']:0).
             ',smtp_active='.db_input($vars['smtp_active']).
             ',smtp_host='.db_input($vars['smtp_host']).
             ',smtp_port='.db_input($vars['smtp_port']?$vars['smtp_port']:0).
             ',smtp_auth='.db_input($vars['smtp_auth']).
             ',smtp_spoofing='.db_input(isset($vars['smtp_spoofing'])?1:0).
             ',notes='.db_input($vars['notes']);

        //Post fetch email handling...
        if($vars['postfetch'] && !strcasecmp($vars['postfetch'],'delete'))
            $sql.=',mail_delete=1,mail_archivefolder=NULL';
        elseif($vars['postfetch'] && !strcasecmp($vars['postfetch'],'archive') && $vars['mail_archivefolder'])
            $sql.=',mail_delete=0,mail_archivefolder='.db_input($vars['mail_archivefolder']);
        else
            $sql.=',mail_delete=0,mail_archivefolder=NULL';

        if($vars['passwd']) //New password - encrypt.
            $sql.=',userpass='.db_input(Crypto::encrypt($vars['passwd'],SECRET_SALT, $vars['userid']));

        if($id) { //update
            $sql='UPDATE '.EMAIL_TABLE.' SET '.$sql.' WHERE email_id='.db_input($id);
            if(db_query($sql) && db_affected_rows())
                return true;

            $errors['err']='Unable to update email. Internal error occurred';
        }else {
            $sql='INSERT INTO '.EMAIL_TABLE.' SET '.$sql.',created=NOW()';
            if(db_query($sql) && ($id=db_insert_id()))
                return $id;

            $errors['err']='Unable to add email. Internal error';
        }

        return false;
    }
}
?>
