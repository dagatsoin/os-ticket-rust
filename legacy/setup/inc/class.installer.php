<?php
/*********************************************************************
    class.installer.php

    osTicket Intaller - installs the latest version.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
require_once INCLUDE_DIR.'class.migrater.php';
require_once INCLUDE_DIR.'class.setup.php';

// @implements FS-060.3: Configuration-file check (step `config`) — config-file existence/writability checks
// @implements FS-060.5: Install-form field validation — field + cross-field validation
// @implements FS-060.6: Database connection, version & prefix-collision check — DB connect/version/collision
// @implements FS-060.7: Schema load, default seeding, admin & config provisioning — full install routine
class Installer extends SetupWizard {

    var $config;

    // @implements FS-060.3: Configuration-file check (step `config`) — constructor binds config-file path
    function Installer($configfile) {
        $this->config =$configfile;
        $this->errors=array();
    }

    function getConfigFile() {
        return $this->config;
    }

    // @implements FS-060.3: Configuration-file check (step `config`) — config file existence check
    function config_exists() {
        return ($this->getConfigFile() && file_exists($this->getConfigFile()));
    }

    // @implements FS-060.3: Configuration-file check (step `config`) — config file writability check
    function config_writable() {
        return ($this->getConfigFile() && is_writable($this->getConfigFile()));
    }

    // @implements FS-060.3: Configuration-file check (step `config`) — combined exists+writable config gate
    function check_config() {
        return ($this->config_exists() && $this->config_writable());
    }

    // @implements FS-060.5: Install-form field validation — master install routine begins with field validation
    // @implements FS-060.6: Database connection, version & prefix-collision check — connect/create DB phase
    // @implements FS-060.7: Schema load, default seeding, admin & config provisioning — load schema → seed → admin/email/config → rewrite config → warm system
    //XXX: Latest version insall logic...no carry over.
    function install($vars) {

        $this->errors=$f=array();

        // @implements FS-060.5: Install-form field validation — field definitions for Validator::process (required + type per field)
        $f['name']          = array('type'=>'string',   'required'=>1, 'error'=>'Name required');
        $f['email']         = array('type'=>'email',    'required'=>1, 'error'=>'Valid email required');
        $f['fname']         = array('type'=>'string',   'required'=>1, 'error'=>'First name required');
        $f['lname']         = array('type'=>'string',   'required'=>1, 'error'=>'Last name required');
        $f['admin_email']   = array('type'=>'email',    'required'=>1, 'error'=>'Valid email required');
        $f['username']      = array('type'=>'username', 'required'=>1, 'error'=>'Username required');
        $f['passwd']        = array('type'=>'password', 'required'=>1, 'error'=>'Password required');
        $f['passwd2']       = array('type'=>'string',   'required'=>1, 'error'=>'Confirm password');
        $f['prefix']        = array('type'=>'string',   'required'=>1, 'error'=>'Table prefix required');
        $f['dbhost']        = array('type'=>'string',   'required'=>1, 'error'=>'Hostname required');
        $f['dbname']        = array('type'=>'string',   'required'=>1, 'error'=>'Database name required');
        $f['dbuser']        = array('type'=>'string',   'required'=>1, 'error'=>'Username required');
        $f['dbpass']        = array('type'=>'string',   'required'=>1, 'error'=>'password required');


        // @implements FS-060.5: Install-form field validation — generic field validation + inner fallback error
        if(!Validator::process($f,$vars,$this->errors) && !$this->errors['err'])
            $this->errors['err']='Missing or invalid data - correct the errors and try again.';


        // @implements BS-060-04: Admin email must differ from system default email — case-insensitive admin_email != email
        //Staff's email can't be same as system emails.
        if($vars['admin_email'] && $vars['email'] && !strcasecmp($vars['admin_email'],$vars['email']))
            $this->errors['admin_email']='Conflicts with system email above';
        // @implements BS-060-05: Password confirmation must match — passwd == passwd2
        //Admin's pass confirmation.
        if(!$this->errors && strcasecmp($vars['passwd'],$vars['passwd2']))
            $this->errors['passwd2']='passwords to not match!';
        // @implements BS-060-06: Table prefix must end with underscore — non-empty prefix must end in _
        //Check table prefix underscore required at the end!
        if($vars['prefix'] && substr($vars['prefix'], -1)!='_')
            $this->errors['prefix']='Bad prefix. Must have underscore (_) at the end. e.g \'ost_\'';

        // @implements BS-060-07: Admin username must not be predictable — block admin/admins/username/osticket
        // @implements EC-060-10: Reserved-username bypass via casing/whitespace — lowercases but does not trim
        //Make sure admin username is not very predictable. XXX: feels dirty but necessary
        if(!$this->errors['username'] && in_array(strtolower($vars['username']),array('admin','admins','username','osticket')))
            $this->errors['username']='Bad username';

        // @implements BS-060-08: Database port range validation — numeric :port must be in 1..65535
        // Support port number specified in the hostname with a colon (:)
        list($host, $port) = explode(':', $vars['dbhost']);
        if ($port && is_numeric($port) && ($port < 1 || $port > 65535))
            $this->errors['db'] = 'Invalid database port number';

        // @implements FS-060.6: Database connection, version & prefix-collision check — connect, MySQL>=4.4, select/create DB, best-effort utf8
        // @implements BS-060-09: Prefix-collision aborts install — SELECT FROM <prefix>config guard
        // @implements KL-060-09: MySQL version comparison is by array, not semantic — explode('.') array compare against 4.4
        //MYSQL: Connect to the DB and check the version & database (create database if it doesn't exist!)
        if(!$this->errors) {
            if(!db_connect($vars['dbhost'],$vars['dbuser'],$vars['dbpass']))
                $this->errors['db']='Unable to connect to MySQL server. '.db_connect_error();
            elseif(explode('.', db_version()) < explode('.', $this->getMySQLVersion()))
                $this->errors['db']=sprintf('osTicket requires MySQL %s or better!',$this->getMySQLVersion());
            elseif(!db_select_database($vars['dbname']) && !db_create_database($vars['dbname'])) {
                $this->errors['dbname']='Database doesn\'t exist';
                $this->errors['db']='Unable to create the database.';
            } elseif(!db_select_database($vars['dbname'])) {
                $this->errors['dbname']='Unable to select the database';
            } else {
                //Abort if we have another installation (or table) with same prefix.
                $sql = 'SELECT * FROM `'.$vars['prefix'].'config` LIMIT 1';
                if(db_query($sql, false)) {
                    $this->errors['err'] = 'We have a problem - another installation with same table prefix exists!';
                    $this->errors['prefix'] = 'Prefix already in-use';
                } else {
                    //Try changing charset and collation of the DB - no bigie if we fail.
                    db_query('ALTER DATABASE '.$vars['dbname'].' DEFAULT CHARACTER SET utf8 COLLATE utf8_general_ci', false);
                }
            }
        }

        //bailout on errors.
        if($this->errors) return false;

        // @implements FS-060.7: Schema load, default seeding, admin & config provisioning — ADMIN_EMAIL/PREFIX defines for in-install SQL error reporting
        // @implements KL-060-04: Hard-coded debug echo of SQL errors — $debug hard-coded true
        /*************** We're ready to install ************************/
        define('ADMIN_EMAIL',$vars['admin_email']); //Needed to report SQL errors during install.
        define('PREFIX',$vars['prefix']); //Table prefix

        $debug = true; // Change it to false to squelch SQL errors.

        // @implements FS-060.7: Schema load, default seeding, admin & config provisioning — re-verify config readable + openable for writing (errors #2/#3)
        //Last minute checks.
        if(!file_exists($this->getConfigFile()) || !($configFile=file_get_contents($this->getConfigFile())))
            $this->errors['err']='Unable to read config file. Permission denied! (#2)';
        elseif(!($fp = @fopen($this->getConfigFile(),'r+')))
            $this->errors['err']='Unable to open config file for writing. Permission denied! (#3)';

        // @implements FS-060.7: Schema load, default seeding, admin & config provisioning — per-stream schema load via load_sql_file
        // @implements BS-060-10: Schema streams are signature-verified before load — md5 content vs trimmed .sig
        // @implements EC-060-05: Schema signature mismatch / corrupted download — abort per-stream on hash mismatch
        else {
            $streams = DatabaseMigrater::getUpgradeStreams(INCLUDE_DIR.'upgrader/streams/');
            foreach ($streams as $stream=>$signature) {
                $schemaFile = INC_DIR."streams/$stream/install-mysql.sql";
                if (!file_exists($schemaFile) || !($fp2 = fopen($schemaFile, 'rb')))
                    $this->errors['err'] = $stream
                        . ': Internal Error - please make sure your download is the latest (#1)';
                elseif (
                        // TODO: Make the hash algo configurable in the streams
                        //       configuration ( core : md5 )
                        !($hash = md5(fread($fp2, filesize($schemaFile))))
                        || strcasecmp($signature, $hash))
                    $this->errors['err'] = $stream
                        .': Unknown or invalid schema signature ('
                        .$signature.' .. '.$hash.')';
                elseif (!$this->load_sql_file($schemaFile, $vars['prefix'], true, $debug))
                    $this->errors['err'] = $stream
                        .': Error parsing SQL schema! Get help from developers (#4)';
            }
        }

        // @implements FS-060.7: Schema load, default seeding, admin & config provisioning — read back seeded default IDs
        // @implements BS-060-11: Default-record IDs are resolved by deterministic lookup — first SLA/dept/template/group; Eastern timezone offset=-5.0
        $sql='SELECT `id` FROM '.PREFIX.'sla ORDER BY `id` LIMIT 1';
        $sla_id_1 = db_result(db_query($sql, false), 0);

        $sql='SELECT `dept_id` FROM '.PREFIX.'department ORDER BY `dept_id` LIMIT 1';
        $dept_id_1 = db_result(db_query($sql, false), 0);

        $sql='SELECT `tpl_id` FROM '.PREFIX.'email_template_group ORDER BY `tpl_id` LIMIT 1';
        $template_id_1 = db_result(db_query($sql, false), 0);

        $sql='SELECT `group_id` FROM '.PREFIX.'groups ORDER BY `group_id` LIMIT 1';
        $group_id_1 = db_result(db_query($sql, false), 0);

        $sql='SELECT `id` FROM '.PREFIX.'timezone WHERE offset=-5.0 LIMIT 1';
        $eastern_timezone = db_result(db_query($sql, false), 0);

        // @implements FS-060.7: Schema load, default seeding, admin & config provisioning — create admin staff row (error #6)
        // @implements BS-060-12: A single all-powerful admin is created — isadmin=1, isactive=1, Admins group, Eastern tz, max_page_size=25
        if(!$this->errors) {
            //Create admin user.
            $sql='INSERT INTO '.PREFIX.'staff SET created=NOW() '
                .", isactive=1, isadmin=1, group_id=$group_id_1, dept_id=$dept_id_1"
                .", timezone_id=$eastern_timezone, max_page_size=25"
                .', email='.db_input($vars['admin_email'])
                .', firstname='.db_input($vars['fname'])
                .', lastname='.db_input($vars['lname'])
                .', username='.db_input($vars['username'])
                .', passwd='.db_input(Passwd::hash($vars['passwd']));
            if(!db_query($sql, false) || !($uid=db_insert_id()))
                $this->errors['err']='Unable to create admin user (#6)';
        }

        // @implements FS-060.7: Schema load, default seeding, admin & config provisioning — create system email rows
        // @implements BS-060-13: Three system email addresses are derived from the default email — Support/alerts/noreply from default email domain
        if(!$this->errors) {
            //Create default emails!
            $email = $vars['email'];
            list(,$domain)=explode('@',$vars['email']);
            $sql='INSERT INTO '.PREFIX.'email (`name`,`email`,`created`,`updated`) VALUES '
                    ." ('Support','$email',NOW(),NOW())"
                    .",('osTicket Alerts','alerts@$domain',NOW(),NOW())"
                    .",('','noreply@$domain',NOW(),NOW())";
            $support_email_id = db_query($sql, false) ? db_insert_id() : 0;


            $sql='SELECT `email_id` FROM '.PREFIX."email WHERE `email`='alerts@$domain' LIMIT 1";
            $alert_email_id = db_result(db_query($sql, false), 0);

            //Create config settings---default settings!
            //XXX: rename ostversion  helpdesk_* ??
            // XXX: Some of this can go to the core install file
			// @implements FS-060.7: Schema load, default seeding, admin & config provisioning — UPDATE core config rows (error #7)
			// @implements BS-060-14: Runtime config rows are populated from the install form — default IDs, schema_signature, helpdesk_url/title
			// @implements KL-060-05: Helpdesk starts offline — isonline forced to 0
			$defaults = array('isonline'=>'0', 'default_email_id'=>$support_email_id,
				'alert_email_id'=>$alert_email_id, 'default_dept_id'=>$dept_id_1, 'default_sla_id'=>$sla_id_1,
				'default_timezone_id'=>$eastern_timezone, 'default_template_id'=>$template_id_1,
				'admin_email'=>db_input($vars['admin_email']),
				'schema_signature'=>db_input($streams['core']),
				'helpdesk_url'=>db_input(URL),
				'helpdesk_title'=>db_input($vars['name']));
			foreach ($defaults as $key=>$value) {
				$sql='UPDATE '.PREFIX.'config SET updated=NOW(), value='.$value
					.' WHERE namespace="core" AND `key`='.db_input($key);
	            if(!db_query($sql, false))
	                $this->errors['err']='Unable to create config settings (#7)';
			}
			
			// @implements FS-060.7: Schema load, default seeding, admin & config provisioning — INSERT per-namespace schema_signature for non-core streams
			// @implements BS-060-14: Runtime config rows are populated from the install form — per-namespace schema_signature config row
			foreach($streams as $stream=>$signature){
				if($stream!='core'){
				    $sql='INSERT INTO '.PREFIX.'config (`namespace`, `key`, `value`, `updated`) '
				    .'VALUES ('.db_input($stream).', '.db_input('schema_signature')
				    .', '.db_input($signature).', NOW())';		    
				    if(!db_query($sql, false))
	                		$this->errors['err']='Unable to create config settings (#7)';
				}
			}
        }

        if($this->errors) return false; //Abort on internal errors.


        // @implements FS-060.7: Schema load, default seeding, admin & config provisioning — rewrite config file LAST (error #5)
        // @implements BS-060-15: Config file is rewritten last for recoverability — flip OSTINSTALLED after all DB work
        // @implements BS-060-17: A secret salt is generated per install — fill 32-char SECRET_SALT via Misc::randCode(32)
        // @implements EC-060-07: Config-file write fails after DB provisioning — write/truncate failure leaves OSTINSTALLED FALSE
        //Rewrite the config file - MUST be done last to allow for installer recovery.
        $configFile= str_replace("define('OSTINSTALLED',FALSE);","define('OSTINSTALLED',TRUE);",$configFile);
        $configFile= str_replace('%ADMIN-EMAIL',$vars['admin_email'],$configFile);
        $configFile= str_replace('%CONFIG-DBHOST',$vars['dbhost'],$configFile);
        $configFile= str_replace('%CONFIG-DBNAME',$vars['dbname'],$configFile);
        $configFile= str_replace('%CONFIG-DBUSER',$vars['dbuser'],$configFile);
        $configFile= str_replace('%CONFIG-DBPASS',$vars['dbpass'],$configFile);
        $configFile= str_replace('%CONFIG-PREFIX',$vars['prefix'],$configFile);
        $configFile= str_replace('%CONFIG-SIRI',Misc::randCode(32),$configFile);
        if(!$fp || !ftruncate($fp,0) || !fwrite($fp,$configFile)) {
            $this->errors['err']='Unable to write to config file. Permission denied! (#5)';
            return false;
        }
        @fclose($fp);

        // @implements FS-060.7: Schema load, default seeding, admin & config provisioning — post-config best-effort wiring
        // @implements BS-060-18: Welcome ticket & install syslog are best-effort — email dept_id, dept email_id/autoresp_email_id
        /************* Make the system happy ***********************/

        $sql='UPDATE '.PREFIX."email SET dept_id=$dept_id_1";
        db_query($sql, false);
        $sql='UPDATE '.PREFIX."department SET email_id=$support_email_id"
            .", autoresp_email_id=$support_email_id";
        db_query($sql, false);

        // @implements FS-060.7: Schema load, default seeding, admin & config provisioning — welcome ticket + thread
        // @implements BS-060-18: Welcome ticket & install syslog are best-effort — "osTicket Installed!" open ticket + initial thread
        // @implements EC-060-14: Welcome ticket uses fixed sentinel values — priority_id=0, topic_id=0, support@osticket.com requester
        // @implements KL-060-11: Welcome ticket bypasses the normal ticket-creation pipeline — direct SQL INSERTs
        //Create a ticket to make the system warm and happy.
        $sql='INSERT INTO '.PREFIX.'ticket SET created=NOW(), status="open", source="Web" '
            ." ,priority_id=0, dept_id=$dept_id_1, topic_id=0 "
            .' ,ticketID='.db_input(Misc::randNumber(6))
            .' ,email="support@osticket.com" '
            .' ,name="osTicket Support" '
            .' ,subject="osTicket Installed!"';
        if(db_query($sql, false) && ($tid=db_insert_id())) {
            if(!($msg=file_get_contents(INC_DIR.'msg/installed.txt')))
                $msg='Congratulations and Thank you for choosing osTicket!';

            $sql='INSERT INTO '.PREFIX.'ticket_thread SET created=NOW()'
                .', source="Web" '
                .', thread_type="M" '
                .', ticket_id='.db_input($tid)
                .', title='.db_input('osTicket Installed')
                .', body='.db_input($msg);
            db_query($sql, false);
        }
        //TODO: create another personalized ticket and assign to admin??

        // @implements FS-060.7: Schema load, default seeding, admin & config provisioning — install-complete syslog row
        // @implements BS-060-18: Welcome ticket & install syslog are best-effort — Debug syslog records operator IP
        //Log a message.
        $msg="Congratulations osTicket basic installation completed!\n\nThank you for choosing osTicket!";
        $sql='INSERT INTO '.PREFIX.'syslog SET created=NOW(), updated=NOW(), log_type="Debug" '
            .', title="osTicket installed!"'
            .', log='.db_input($msg)
            .', ip_address='.db_input($_SERVER['REMOTE_ADDR']);
        db_query($sql, false);

        return true;
    }
}
?>
