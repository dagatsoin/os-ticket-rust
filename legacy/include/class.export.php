<?php
/*************************************************************************
    class.export.php

    Exports stuff (details to follow)

    Jared Hancock <jared@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

// @implements FS-090.20: Result-Set Export — Header-Mapped Column Projection — export entry points
class Export {

    // @implements FS-090.20: Result-Set Export — Header-Mapped Column Projection — pick exporter by format + dump
    // @implements FS-090.31: Export Format Selection Has No Fallback
    /* static */ function dumpQuery($sql, $headers, $how='csv', $filter=false) {
        $exporters = array(
            'csv' => CsvResultsExporter,
            'json' => JsonResultsExporter
        );
        $exp = new $exporters[$how]($sql, $headers, $filter);
        return $exp->dump();
    }

    # XXX: Think about facilitated exporting. For instance, we might have a
    #      TicketExporter, which will know how to formulate or lookup a
    #      format query (SQL), and cooperate with the output process to add
    #      extra (recursive) information. In this funciton, the top-level
    #      SQL is exported, but for something like tickets, we will need to
    #      export attached messages, reponses, and notes, as well as
    #      attachments associated with each, ...
    // @implements FS-090.25: Ticket Export Column Set — ticket-queue header map + dump (consumed by FS-020)
    /* static */ function dumpTickets($sql, $how='csv') {
        return self::dumpQuery($sql,
            array(
                'ticketID' =>       'Ticket Id',
                'created' =>        'Date',
                'subject' =>        'Subject',
                'name' =>           'From',
                'priority_desc' =>  'Priority',
                'dept_name' =>      'Department',
                'helptopic' =>      'Help Topic',
                'source' =>         'Source',
                'status' =>         'Current Status',
                'effective_date' => 'Last Updated',
                'duedate' =>        'Due Date',
                'isoverdue' =>      'Overdue',
                'isanswered' =>     'Answered',
                'assigned' =>       'Assigned To',
                'staff' =>          'Staff Assigned',
                'team' =>           'Team Assigned',
                'thread_count' =>   'Thread Count',
                'attachments' =>    'Attachment Count',
            ),
            $how);
    }

    // @implements FS-090.23: Ticket-Queue CSV Export Flow (Token-Backed) — buffer ticket dump → file download
    // @implements FS-090.24: File-Download Response Helper
    /* static */ function saveTickets($sql, $filename, $how='csv') {
        ob_start();
        self::dumpTickets($sql, $how);
        $stuff = ob_get_contents();
        ob_end_clean();
        if ($stuff)
            Http::download($filename, "text/$how", $stuff);

        return false;
    }
}

// @implements FS-090.20: Result-Set Export — Header-Mapped Column Projection — base streaming exporter
class ResultSetExporter {
    // @implements FS-090.20: Result-Set Export — Header-Mapped Column Projection — strip LIMIT, intersect headers with query columns
    // @implements BS-090.9: Export Strips the Page LIMIT — Full Result Set
    // @implements BS-090.10: Export Columns Are the Intersection of the Header Map and the Query
    // @implements BS-090.15: Column Projection Requires a Non-Empty Result Set
    function ResultSetExporter($sql, $headers, $filter=false) {
        $this->headers = array_values($headers);
        if ($s = strpos(strtoupper($sql), ' LIMIT '))
            $sql = substr($sql, 0, $s);
        # TODO: If $filter, add different LIMIT clause to query
        $this->_res = db_query($sql);
        if ($row = db_fetch_array($this->_res)) {
            $query_fields = array_keys($row);
            $this->headers = array();
            $this->keys = array();
            $this->lookups = array();
            foreach ($headers as $field=>$name) {
                if (array_key_exists($field, $row)) {
                    $this->headers[] = $name;
                    $this->keys[] = $field;
                    # Remember the location of this header in the query results
                    # (column-wise) so we don't have to do hashtable lookups for every
                    # column of every row.
                    $this->lookups[] = array_search($field, $query_fields);
                }
            }
            db_data_reset($this->_res);
        }
    }

    // @implements FS-090.30: Result-Set Exporter — Empty Result Set Header Behavior — resolved header labels
    function getHeaders() {
        return $this->headers;
    }

    // @implements FS-090.20: Result-Set Export — Header-Mapped Column Projection — next projected row (positional lookups)
    function next() {
        if (!($row = db_fetch_row($this->_res)))
            return false;

        $record = array();
        foreach ($this->lookups as $idx)
            $record[] = $row[$idx];
        return $record;
    }

    // @implements FS-090.22: JSON Exporter Format — next projected row keyed by field name
    function nextArray() {
        if (!($row = $this->next()))
            return false;
        return array_combine($this->keys, $row);
    }

    // @implements FS-090.20: Result-Set Export — Header-Mapped Column Projection — base dump (debug; overridden per format)
    function dump() {
        # Useful for debug output
        while ($row=$this->nextArray()) {
            var_dump($row);
        }
    }
}

// @implements FS-090.21: CSV Exporter Format Rules — CSV result exporter
class CsvResultsExporter extends ResultSetExporter {
    // @implements FS-090.21: CSV Exporter Format Rules — quote every value, double embedded quotes
    // @implements BS-090.11: CSV Values Are Always Quoted; Embedded Quotes Are Doubled
    function dump() {
        echo '"' . implode('","', $this->getHeaders()) . "\"\n";
        while ($row=$this->next()) {
            foreach ($row as &$val)
                # Escape enclosed double-quotes
                $val = str_replace('"','""',$val);
            echo '"' . implode('","', $row) . "\"\n";
        }
    }
}

// @implements FS-090.22: JSON Exporter Format — JSON result exporter
class JsonResultsExporter extends ResultSetExporter {
    // @implements FS-090.22: JSON Exporter Format — encode all rows as a JSON array of field-keyed objects
    function dump() {
        require_once(INCLUDE_DIR.'class.json.php');
        $exp = new JsonDataEncoder();
        $rows = array();
        while ($row=$this->nextArray()) {
            $rows[] = $row;
        }
        echo $exp->encode($rows);
    }
}

require_once INCLUDE_DIR . 'class.json.php';
require_once INCLUDE_DIR . 'class.migrater.php';
require_once INCLUDE_DIR . 'class.signal.php';

// @implements FS-090.27: Full-Database Backup Exporter — record-separated self-describing DB dump
// @implements BS-090.14: Backup Dump Is Record-Separated and Self-Describing
class DatabaseExporter {

    var $stream;
    var $tables = array(CONFIG_TABLE, SYSLOG_TABLE, FILE_TABLE,
        FILE_CHUNK_TABLE, STAFF_TABLE, DEPT_TABLE, TOPIC_TABLE, GROUP_TABLE,
        GROUP_DEPT_TABLE, TEAM_TABLE, TEAM_MEMBER_TABLE, FAQ_TABLE,
        FAQ_ATTACHMENT_TABLE, FAQ_TOPIC_TABLE, FAQ_CATEGORY_TABLE,
        CANNED_TABLE, CANNED_ATTACHMENT_TABLE, TICKET_TABLE,
        TICKET_THREAD_TABLE, TICKET_ATTACHMENT_TABLE, TICKET_PRIORITY_TABLE,
        TICKET_LOCK_TABLE, TICKET_EVENT_TABLE, TICKET_EMAIL_INFO_TABLE,
        EMAIL_TABLE, EMAIL_TEMPLATE_TABLE, EMAIL_TEMPLATE_GRP_TABLE,
        FILTER_TABLE, FILTER_RULE_TABLE, SLA_TABLE, API_KEY_TABLE,
        TIMEZONE_TABLE, SESSION_TABLE, PAGE_TABLE);

    // @implements FS-090.27: Full-Database Backup Exporter — bind output stream
    function DatabaseExporter($stream) {
        $this->stream = $stream;
    }

    // @implements FS-090.27: Full-Database Backup Exporter — write JSON block + 0x1E record separator
    // @implements BS-090.14: Backup Dump Is Record-Separated and Self-Describing
    function write_block($what) {
        fwrite($this->stream, JsonDataEncoder::encode($what));
        fwrite($this->stream, "\x1e");
    }

    // @implements FS-090.27: Full-Database Backup Exporter — emit signed header + per-table schema/index/row blocks
    function dump($error_stream) {
        // Allow plugins to change the tables exported
        Signal::send('export.tables', $this, $this->tables);

        $header = array(
            array(OSTICKET_BACKUP_SIGNATURE, OSTICKET_BACKUP_VERSION),
            array(
                'version'=>THIS_VERSION,
                'table_prefix'=>TABLE_PREFIX,
                'salt'=>SECRET_SALT,
                'dbtype'=>DBTYPE,
                'streams'=>DatabaseMigrater::getUpgradeStreams(
                    UPGRADE_DIR . 'streams/'),
            ),
        );
        $this->write_block($header);

        foreach ($this->tables as $t) {
            if ($error_stream) $error_stream->write("$t\n");
            // Inspect schema
            $table = $indexes = array();
            $res = db_query("show columns from $t");
            while ($field = db_fetch_array($res))
                $table[] = $field;

            $res = db_query("show indexes from $t");
            while ($col = db_fetch_array($res))
                $indexes[] = $col;

            $res = db_query("select * from $t");
            $types = array();

            if (!$table) {
                if ($error_stream) $error_stream->write(
                    $t.': Cannot export table with no fields'."\n");
                die();
            }
            $this->write_block(
                array('table', substr($t, strlen(TABLE_PREFIX)), $table,
                    $indexes));

            // Dump row data
            while ($row = db_fetch_row($res))
                $this->write_block($row);

            $this->write_block(array('end-table'));
        }
    }
}
