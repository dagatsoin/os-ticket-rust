<?php
/*********************************************************************
    cli/import.php

    osTicket data importer, used for migration and backup recovery

    Jared Hancock <jared@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
require_once dirname(__file__) . "/class.module.php";

// @implements FS-092.13: Backup importer — stream input, header verification & restore loop — Importer module consumes the FS-090.27 dump
// @implements FS-092.16: Backup importer — DDL reconstruction & batched INSERT emission — reconstructs CREATE/INDEX/batched INSERT SQL
class Importer extends Module {
    var $prologue =
        "Imports data from a previous backup (using the exporter)";

    var $options = array(
        'stream' => array('-i', '--input', 'default'=>'php://stdin',
            'metavar'=>'FILE', 'help'=>
            "File or stream from which to read the export. As a default,
            data is received from standard in."),
        'compress' => array('-z', '--compress', 'action'=>'store_true',
            'help'=>'Read zlib compressed data (use -z with the export
                command)'),
        'tables' => array('-t', '--table', 'action'=>'append',
            'metavar'=>'TABLE', 'help'=>
            "Table to be restored from the backup. Default is to restore all
            tables. This option can be specified more than once"),
        'drop' => array('-D', '--drop', 'action'=>'store_true', 'help'=>
            'Issue DROP TABLE statements before the create statemente'),
    );

    var $epilog =
        "The SQL of the import is written to standard outout";

    var $stream;
    var $header;
    var $source_ost_info;

    // @implements FS-092.14: Backup importer — dump-format consumption (block framing) — verify header: signature match + mysql-only dbtype guard
    // @implements BS-092-09: Only MySQL backups are importable — dbtype must equal mysql
    // @implements FS-090.27: Full-Database Backup Exporter — consumes the documented dump framing
    function verify_header() {
        list($header, $info) = $this->read_block();
        if (!$header || $header[0] != OSTICKET_BACKUP_SIGNATURE) {
            $this->stderr->write('Header mismatch -- not an osTicket backup');
            return false;
        }
        else
            $this->header = $header;

        if (!$info || $info['dbtype'] != 'mysql') {
            $this->stderr->write('Only mysql imports are supported currently');
            return false;
        }
        $this->source_ost_info = $info;
        return true;
    }

    // @implements FS-092.14: Backup importer — dump-format consumption (block framing) — read one \x1e-terminated JSON block (format owned by FS-090.27)
    // @implements EC-092-11: Unreadable / undecodable block — non-empty undecodable block dies "Unable to read block from input"
    function read_block() {
        $block = '';
        while (!feof($this->stream) && (($c = fgetc($this->stream)) != "\x1e"))
            $block .= $c;

        if ($json = JsonDataParser::decode($block))
            return $json;

        if (strlen($block)) {
            $this->stderr->write("Unable to read block from input");
            die();
        }
    }

    // @implements FS-092.16: Backup importer — DDL reconstruction & batched INSERT emission — statement sink
    // @implements BS-092-13: Default import sink is SQL to stdout, not live execution — live in prime-time mode, else write SQL + ";" to stdout
    function send_statement($stmt) {
        if ($this->getOption('prime-time'))
            db_query($stmt);
        else {
            $this->stdout->write($stmt);
            $this->stdout->write(";\n");
        }
    }

    // @implements FS-092.15: Backup importer — per-table reconstruction loop — read table block, emit DDL, stream rows until end-table
    // @implements EC-092-12: Table block out of order — non-table block writes "Unable to read table header" and ends the loop
    // @implements EC-092-13: Truncated table stream — end-of-stream without end-table returns failure for that table
    function import_table() {
        if (!($header = $this->read_block()))
            return false;

        else if ($header[0] != 'table') {
            $this->stderr->write('Unable to read table header');
            return false;
        }

        // TODO: Consider included tables and excluded tables

        $this->stderr->write("Importing table: {$header[1]}\n");
        $this->create_table($header);
        $this->create_indexes($header);

        while (($row=$this->read_block())) {
            if (isset($row[0]) && ($row[0] == 'end-table')) {
                $this->load_row(null, null, true);
                return true;
            }
            $this->load_row($header, $row);
        }
        return false;
    }

    // @implements FS-092.16: Backup importer — DDL reconstruction & batched INSERT emission — reconstruct CREATE TABLE from dumped column metadata (PK assembly, utf8)
    // @implements BS-092-10: Table prefix is re-applied locally on import — local TABLE_PREFIX concatenated with dumped table name
    function create_table($info) {
        if ($this->getOption('drop'))
            $this->send_statement('DROP TABLE IF EXISTS `'.TABLE_PREFIX.'`');
        $sql = 'CREATE TABLE `'.TABLE_PREFIX.$info[1].'` (';
        $pk = array();
        $fields = array();
        foreach ($info[2] as $col) {
            $field = "`{$col['Field']}` {$col['Type']}";
            if ($col['Null'] == 'NO')
                $field .= ' NOT NULL ';
            if ($col['Default'] == 'CURRENT_TIMESTAMP')
                $field .= ' DEFAULT CURRENT_TIMESTAMP';
            elseif ($col['Default'] !== null)
                $field .= ' DEFAULT '.db_input($col['Default']);
            $field .= ' '.$col['Extra'];
            $fields[] = $field;
        }
        // Generate PRIMARY KEY
        foreach ($info[3] as $idx) {
            if ($idx['Key_name'] == 'PRIMARY') {
                $col = '`'.$idx['Column_name'].'`';
                if ($idx['Collation'] != 'A')
                    $col .= ' DESC';
                $pk[(int)$idx['Seq_in_index']] = $col;
            }
        }
        $sql .= implode(", ", $fields);
        if ($pk)
            $sql .= ', PRIMARY KEY ('.implode(',',$pk).')';
        $sql .= ') DEFAULT CHARSET=utf8';
        $queries[] = $sql;
        $this->send_statement($sql);
    }

    // @implements FS-092.16: Backup importer — DDL reconstruction & batched INSERT emission — reconstruct CREATE INDEX
    // @implements BS-092-10: Table prefix is re-applied locally on import — strip source prefix, re-apply local prefix
    // @implements KL-092-05: Single-prefix index reconstruction — index table re-prefixed by stripping the source prefix
    function create_indexes($header) {
        $indexes = array();
        foreach ($header[3] as $idx) {
            if ($idx['Key_name'] == 'PRIMARY')
                continue;
            if (!isset($indexes[$idx['Key_name']]))
                $indexes[$idx['Key_name']] = array(
                    'cols'=>array(),
                    // XXX: Drop table-prefix
                    'table'=>substr($idx['Table'],
                        strlen($this->source_ost_info['table_prefix'])),
                    'type'=>$idx['Index_type'],
                    'unique'=>!$idx['Non_unique']);
            $index = &$indexes[$idx['Key_name']];
            $col = '`'.$idx['Column_name'].'`';
            if ($idx['Collation'] != 'A')
                $col .= ' DESC';
            $index[(int)$idx['Seq_in_index']] = $col;
            $index['cols'][] = $col;
        }
        foreach ($indexes as $name=>$info) {
            $cols = array();
            $this->send_statement('CREATE '
                .(($info['unique']) ? 'UNIQUE ' : '')
                .'INDEX `'.$name
                .'` USING '.$info['type']
                .' ON `'.TABLE_PREFIX.$info['table'].'` ('
                .implode(',', $info['cols'])
                .')');
        }
    }

    // @implements FS-092.16: Backup importer — DDL reconstruction & batched INSERT emission — restore-over-existing helper: TRUNCATE TABLE + DROP INDEX IF EXISTS per non-primary index
    function truncate_table($info) {
        $this->send_statement('TRUNCATE TABLE '.TABLE_PREFIX.$info[1]);
        $indexes = array();
        foreach ($info[3] as $idx) {
            if ($idx['Key_name'] == 'PRIMARY')
                continue;
            $indexes[$idx['Key_name']] =
                '`'.TABLE_PREFIX.$info[1].'`.`'.$idx['Key_name'].'`';
        }
        foreach ($indexes as $T=>$fqn)
            $this->send_statement('DROP INDEX IF EXISTS '.$fqn);
    }

    // @implements FS-092.16: Backup importer — DDL reconstruction & batched INSERT emission — buffer rows into batched multi-row INSERT
    // @implements BS-092-11: Binary-safe row values via hex literals — non-numeric values emitted as 0x hex binary literals
    // @implements BS-092-12: Batched inserts bounded by accumulated value length — flush when accumulated values exceed ~16KB
    function load_row($info, $row, $flush=false) {
        static $header = null;
        static $rows = array();
        static $length = 0;

        if ($info && $header === null) {
            $header = "INSERT INTO `".TABLE_PREFIX.$info[1].'` (';
            $cols = array();
            foreach ($info[2] as $col)
                $cols[] = "`{$col['Field']}`";
            $header .= implode(', ', $cols);
            $header .= ") VALUES ";
        }
        if ($row) {
            $values = array();
            foreach ($info[2] as $i=>$col)
                $values[] = (is_numeric($row[$i]))
                    ? $row[$i]
                    : ($row[$i] ? '0x'.bin2hex($row[$i]) : "''");
            $values = "(" . implode(', ', $values) . ")";
            $length += strlen($values);
            $rows[] = &$values;
        }
        if (($flush || $length > 16000) && $header) {
            $this->send_statement($header . implode(',', $rows));
            $header = null;
            $rows = array();
            $length = 0;
        }
    }

    // @implements FS-092.13: Backup importer — stream input, header verification & restore loop — run: boot app, open (optionally zlib) stream, verify header, loop import_table
    // @implements EC-092-09: Backup header mismatch — failed verification dies "Unable to verify backup header"
    function run($args, $options) {
        require_once dirname(__file__) . '/../../../main.inc.php';
        require_once INCLUDE_DIR . 'class.json.php';

        $stream = $options['stream'];
        if ($options['compress']) $stream = "compress.zlib://$stream";
        if (!($this->stream = fopen($stream, 'rb'))) {
            $this->stderr->write('Unable to open input stream');
            die();
        }

        if (!$this->verify_header())
            die('Unable to verify backup header');

        while ($this->import_table());
        @fclose($this->stream);
    }
}

Module::register('import', 'Importer');
?>
