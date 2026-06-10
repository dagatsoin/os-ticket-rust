<?php
/*********************************************************************
    mysql.php

    Collection of MySQL helper interface functions.

    Mostly wrappers with error/resource checking.

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

    // @implements FS-001.5: System & Configuration Singleton Startup — legacy mysql connection establishment used by the bootstrap
    // @implements FS-003.29: Connection lifecycle & MySQL session-variable read/write — sets UTF-8 charset and forces empty sql_mode on connect
    // @implements BS-028: Connect-time sql_mode reset — db_set_variable('sql_mode','') disables strict modes for the session
    function db_connect($host, $user, $passwd, $options = array()) {

        //Assert
        if(!strlen($user) || !strlen($passwd) || !strlen($host))
      	    return NULL;

        //Connect
        $start = (double) microtime() * 1000000;
        if(!($dblink =@mysql_connect($host, $user, $passwd)))
            return NULL;

        //Select the database, if any.
        if($options['db']) db_select_database($options['db']);

        //set desired encoding just in case mysql charset is not UTF-8 - Thanks to FreshMedia
        @mysql_query('SET NAMES "utf8"');
        @mysql_query('SET CHARACTER SET "utf8"');
        @mysql_query('SET COLLATION_CONNECTION=utf8_general_ci');

        @db_set_variable('sql_mode', '');

        // Use connection timing to seed the random number generator
        Misc::__rand_seed(((double) microtime() * 1000000) - $start);

        return $dblink;
    }

    // @implements FS-003.29: Connection lifecycle & MySQL session-variable read/write — closes the active connection handle
    function db_close() {
        global $dblink;
        return @mysql_close($dblink);
    }

    // @implements FS-060.6: Database connection, version & prefix-collision check — reads the MySQL server version for the installer's compatibility gate
    function db_version() {

        $version=0;
        if(preg_match('/(\d{1,2}\.\d{1,2}\.\d{1,2})/',
                mysql_result(db_query('SELECT VERSION()'),0,0),
                $matches))                                      # nolint
            $version=$matches[1];                               # nolint

        return $version;
    }

    // @implements FS-003.29: Connection lifecycle & MySQL session-variable read/write — convenience reader for @@session.time_zone
    function db_timezone() {
        return db_get_variable('time_zone');
    }

    // @implements FS-003.29: Connection lifecycle & MySQL session-variable read/write — reads a MySQL server session/global variable
    function db_get_variable($variable, $type='session') {
        $sql =sprintf('SELECT @@%s.%s', $type, $variable);
        return db_result(db_query($sql));
    }

    // @implements FS-003.29: Connection lifecycle & MySQL session-variable read/write — writes a MySQL server variable (value escaped via db_input)
    function db_set_variable($variable, $value, $type='session') {
        $sql =sprintf('SET %s %s=%s',strtoupper($type), $variable, db_input($value));
        return db_query($sql);
    }


    // @implements FS-003.29: Connection lifecycle & MySQL session-variable read/write — selects the active database on the connection
    function db_select_database($database) {
        return ($database && @mysql_select_db($database));
    }

    // @implements FS-060.7: Schema load, default seeding, admin & config provisioning — creates the helpdesk database during installation
    function db_create_database($database, $charset='utf8', $collate='utf8_general_ci') {
        return @mysql_query(sprintf('CREATE DATABASE %s DEFAULT CHARACTER SET %s COLLATE %s', $database, $charset, $collate));
    }

	// @implements FS-003.27: Query execution & "smart" parameterized query — runs a query and logs DB errors via $ost->logDBError when enabled
	// @implements FS-001.5: System & Configuration Singleton Startup — the bootstrap-owned query primitive every config/log read flows through
	// execute sql query
	function db_query($query, $logError=true) {
        global $ost;

        $res = mysql_query($query);

        if(!$res && $logError && $ost) { //error reporting
            $msg='['.$query.']'."\n\n".db_error();
            $ost->logDBError('DB Error #'.db_errno(), $msg);
            //echo $msg; #uncomment during debuging or dev.
        }

        return $res;
	}

	// @implements FS-003.27: Query execution & "smart" parameterized query — replaces ? placeholders with escaped args via sprintf before db_query
	function db_squery($query) { //smart db query...utilizing args and sprintf

		$args  = func_get_args();
  		$query = array_shift($args);
  		$query = str_replace("?", "%s", $query);
  		$args  = array_map('db_real_escape', $args);
  		array_unshift($args, $query);
  		$query = call_user_func_array('sprintf', $args);
		return db_query($query);
	}

	// @implements FS-003.27: Query execution & "smart" parameterized query — convenience scalar/count wrapper over db_result(db_query())
	function db_count($query) {
        return db_result(db_query($query));
	}

    // @implements FS-003.28: Result-set accessors & magic-quotes output reversal — single scalar at a row, routed through db_output
    function db_result($res, $row=0) {
        return ($res)?mysql_result($res, $row):NULL;
    }

	// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — next row as associative/numeric array via db_output
	function db_fetch_array($res, $mode=false) {
   	    return ($res)?db_output(mysql_fetch_array($res, ($mode)?$mode:MYSQL_ASSOC)):NULL;
  	}

    // @implements FS-003.28: Result-set accessors & magic-quotes output reversal — next row as a numeric-indexed array via db_output
    function db_fetch_row($res) {
        return ($res)?db_output(mysql_fetch_row($res)):NULL;
    }

    // @implements FS-003.28: Result-set accessors & magic-quotes output reversal — next field-metadata object (not output-filtered)
    function db_fetch_field($res) {
        return ($res)?mysql_fetch_field($res):NULL;
    }

    // @implements FS-003.28: Result-set accessors & magic-quotes output reversal — accumulates all remaining rows into an array
    function db_assoc_array($res, $mode=false) {
	    if($res && db_num_rows($res)) {
      	    while ($row=db_fetch_array($res, $mode))
         	    $result[]=$row;
        }
        return $result;
    }

    // @implements FS-003.28: Result-set accessors & magic-quotes output reversal — row count of a result handle (0 on falsy)
    function db_num_rows($res) {
   	    return ($res)?mysql_num_rows($res):0;
    }

	// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — rows touched by the last write
	function db_affected_rows() {
      return mysql_affected_rows();
    }

  	// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — moves the row cursor to n
  	function db_data_seek($res, $row_number) {
   	    return mysql_data_seek($res, $row_number);
  	}

  	// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — resets the row cursor to row 0
  	function db_data_reset($res) {
   	    return mysql_data_seek($res,0);
  	}

  	// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — auto-increment id of the last insert
  	function db_insert_id() {
   	    return mysql_insert_id();
  	}

	// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — releases the result handle
	function db_free_result($res) {
   	    return mysql_free_result($res);
  	}

	// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — magic-quotes output reversal; no-op when get_magic_quotes_runtime is OFF
	// @implements BS-027: Result output reverses magic quotes only when the runtime added them — stripslashes non-numeric scalars only when the directive is ON
	function db_output($var) {

        if(!function_exists('get_magic_quotes_runtime') || !get_magic_quotes_runtime()) //Sucker is NOT on - thanks.
            return $var;

        if (is_array($var))
            return array_map('db_output', $var);

        return (!is_numeric($var))?stripslashes($var):$var;

    }

    // @implements FS-003.26: SQL input escaping (db_input / db_real_escape) — driver-level escape (+optional quoting); must not be called directly
    // @implements BS-026: All SQL values pass through db_input; numeric values fast-path — db_real_escape is the underlying escape primitive
    //Do not call this function directly...use db_input
    function db_real_escape($val, $quote=false) {

        //Magic quotes crap is taken care of in main.inc.php
        $val=mysql_real_escape_string($val);

        return ($quote)?"'$val'":$val;
    }

    // @implements FS-003.26: SQL input escaping (db_input / db_real_escape) — public escaping funnel; numeric fast-path (^\d+(\.\d+)?$), else escape+quote
    // @implements BS-026: All SQL values pass through db_input; numeric values fast-path — system-wide SQL-injection defense with legacy numeric pattern
    function db_input($var, $quote=true) {

        if(is_array($var))
            return array_map('db_input', $var, array_fill(0, count($var), $quote));
        elseif($var && preg_match("/^\d+(\.\d+)?$/", $var))
            return $var;

        return db_real_escape($var, $quote);
    }

	// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — last driver error text (feeds db_query error logging)
	function db_error() {
   	    return mysql_error();
	}

    // @implements FS-003.28: Result-set accessors & magic-quotes output reversal — connection error text
    function db_connect_error() {
        return db_error();
    }

    // @implements FS-003.28: Result-set accessors & magic-quotes output reversal — last driver error number
    function db_errno() {
        return mysql_errno();
    }

    // @implements FS-003.28: Result-set accessors & magic-quotes output reversal — a column's declared type / field metadata
    function db_field_type($res, $col=0) {
        return mysql_field_type($res, $col);
    }
?>
