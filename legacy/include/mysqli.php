<?php
/*********************************************************************
    mysqli.php

    Collection of MySQL helper interface functions.

    Mostly wrappers with error/resource checking.

    Peter Rotich <peter@osticket.com>
    Jared Hancock <jared@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
$__db = null;

// @implements FS-001.5: System & Configuration Singleton Startup — mysqli connection establishment (with optional SSL + host:port parsing) used by the bootstrap
// @implements FS-003.29: Connection lifecycle & MySQL session-variable read/write — sets UTF-8 charset and forces empty sql_mode on connect
// @implements BS-028: Connect-time sql_mode reset — db_set_variable('sql_mode','') disables strict modes for the session
function db_connect($host, $user, $passwd, $options = array()) {
    global $__db;

    //Assert
    if(!strlen($user) || !strlen($host))
        return NULL;

    if (!($__db = mysqli_init()))
        return NULL;

    // Setup SSL if enabled
    if (isset($options['ssl']))
        $__db->ssl_set( # nolint
                $options['ssl']['key'],
                $options['ssl']['cert'],
                $options['ssl']['ca'],
                null, null);
    elseif(!$passwd)
        return NULL;

    $port = ini_get("mysqli.default_port");
    $socket = ini_get("mysqli.default_socket");
    if (strpos($host, ':') !== false) {
        list($host, $portspec) = explode(':', $host);
        // PHP may not honor the port number if connecting to 'localhost'
        if ($portspec && is_numeric($portspec)) {
            if (!strcasecmp($host, 'localhost'))
                // XXX: Looks like PHP gethostbyname() is IPv4 only
                $host = gethostbyname($host);
            $port = (int) $portspec;
        }
        elseif ($portspec) {
            $socket = $portspec;
        }
    }

    // Connect
    $start = microtime(true);
    if (!@$__db->real_connect($host, $user, $passwd, null, $port, $socket)) # nolint
        return NULL;

    //Select the database, if any.
    if(isset($options['db'])) $__db->select_db($options['db']); # nolint

    //set desired encoding just in case mysql charset is not UTF-8 - Thanks to FreshMedia
    @$__db->query('SET NAMES "utf8"');                          # nolint
    @$__db->query('SET CHARACTER SET "utf8"');                  # nolint
    @$__db->query('SET COLLATION_CONNECTION=utf8_general_ci');  # nolint

    @db_set_variable('sql_mode', '');

    // Use connection timing to seed the random number generator
    Misc::__rand_seed((microtime(true) - $start) * 1000000);

    return $__db;
}

// @implements FS-003.29: Connection lifecycle & MySQL session-variable read/write — closes the active mysqli connection
function db_close() {
    global $__db;
    return @$__db->close();
}

// @implements FS-060.6: Database connection, version & prefix-collision check — reads the MySQL server version for the installer's compatibility gate
function db_version() {

    $version=0;
    if(preg_match('/(\d{1,2}\.\d{1,2}\.\d{1,2})/',
            db_result(db_query('SELECT VERSION()')),
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
    global $__db;
    return ($database && @$__db->select_db($database)); # nolint
}

// @implements FS-060.7: Schema load, default seeding, admin & config provisioning — creates the helpdesk database during installation
function db_create_database($database, $charset='utf8',
        $collate='utf8_general_ci') {
    global $__db;
    return @$__db->query( # nolint
        sprintf('CREATE DATABASE %s DEFAULT CHARACTER SET %s COLLATE %s',
            $database, $charset, $collate));
}

// @implements FS-003.27: Query execution & "smart" parameterized query — runs a query, retries up to 3x on deadlock #1213, logs DB errors via $ost->logDBError
// @implements FS-001.5: System & Configuration Singleton Startup — the bootstrap-owned query primitive every config/log read flows through
// execute sql query
function db_query($query, $logError=true) {
    global $ost, $__db;

    $tries = 3;
    do {
        $res = $__db->query($query);
        // Retry the query due to deadlock error (#1213)
        // TODO: Consider retry on #1205 (lock wait timeout exceeded)
        // TODO: Log warning
    } while (!$res && --$tries && $__db->errno == 1213);

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

// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — single scalar at a row (data_seek + first col), via db_output
function db_result($res, $row=0) {
    if (!$res)
        return NULL;

    $res->data_seek($row); # nolint
    list($value) = db_output($res->fetch_row());
    return $value;
}

// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — next row as associative/numeric array via db_output
function db_fetch_array($res, $mode=MYSQLI_ASSOC) {
    return ($res) ? db_output($res->fetch_array($mode)) : NULL; # nolint
}

// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — next row as a numeric-indexed array via db_output
function db_fetch_row($res) {
    return ($res) ? db_output($res->fetch_row()) : NULL; # nolint
}

// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — next field-metadata object (not output-filtered)
function db_fetch_field($res) {
    return ($res) ? $res->fetch_field() : NULL; # nolint
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
    return ($res) ? $res->num_rows : 0; # nolint
}

// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — rows touched by the last write
function db_affected_rows() {
    global $__db;
    return $__db->affected_rows;
}

// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — moves the row cursor to n
function db_data_seek($res, $row_number) {
    return ($res && $res->data_seek($row_number)); # nolint
}

// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — resets the row cursor to row 0
function db_data_reset($res) {
    return db_data_seek($res, 0);
}

// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — auto-increment id of the last insert
function db_insert_id() {
    global $__db;
    return $__db->insert_id;
}

// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — releases the result handle
function db_free_result($res) {
    return ($res && $res->free()); # nolint
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
    global $__db;

    //Magic quotes crap is taken care of in main.inc.php
    $val=$__db->real_escape_string($val);

    return ($quote)?"'$val'":$val;
}

// @implements FS-003.26: SQL input escaping (db_input / db_real_escape) — public escaping funnel; mysqli numeric fast-path (no leading-zero ints), else escape+quote
// @implements BS-026: All SQL values pass through db_input; numeric values fast-path — leading-zero integers are escaped/quoted under the mysqli pattern
function db_input($var, $quote=true) {

    if(is_array($var))
        return array_map('db_input', $var, array_fill(0, count($var), $quote));
    elseif($var && preg_match("/^(?:\d+\.\d+|[1-9]\d*)$/S", $var))
        return $var;

    return db_real_escape($var, $quote);
}

// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — a column's declared type / field metadata
function db_field_type($res, $col=0) {
    global $__db;
    return $res->fetch_field_direct($col); # nolint
}

// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — connection error text
function db_connect_error() {
    global $__db;
    return $__db->connect_error;
}

// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — last driver error text (feeds db_query error logging)
function db_error() {
    global $__db;
    return $__db->error;
}

// @implements FS-003.28: Result-set accessors & magic-quotes output reversal — last driver error number
function db_errno() {
    global $__db;
    return $__db->errno;
}
?>
