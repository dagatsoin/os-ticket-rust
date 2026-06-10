#!/usr/bin/env php
<?php

require_once "modules/class.module.php";

if (!function_exists('noop')) { function noop() {} }
session_set_save_handler('noop','noop','noop','noop','noop','noop');

// @implements FS-092.1: CLI front controller & action dispatch — Manager dispatches an action arg to a self-registered module
class Manager extends Module {
    var $prologue =
        "Manage one or more osTicket installations";

    var $arguments = array(
        'action' => "Action to be managed"
    );

    var $usage = '$script action [options] [arguments]';

    var $autohelp = false;

    // @implements FS-092.3: Aggregated and per-module help rendering — load every module, print one prologue line per registered action
    function showHelp() {
        foreach (glob(dirname(__file__).'/modules/*.php') as $script)
            include_once $script;

        global $registered_modules;
        $this->epilog =
            "Currently available modules follow. Use 'manage.php <module>
            --help' for usage regarding each respective module:";

        parent::showHelp();

        echo "\n";
        foreach ($registered_modules as $name=>$mod)
            echo str_pad($name, 20) . $mod->prologue . "\n";
    }

    // @implements FS-092.1: CLI front controller & action dispatch — run: resolve + delegate, else "Unknown action given"
    // @implements BS-092-02: Action token is consumed before module parsing — strip the action token from argv
    function run($args, $options) {
        if ($options['help'] && !$args['action'])
            $this->showHelp();

        else {
            $action = $args['action'];

            global $argv;
            foreach ($argv as $idx=>$val)
                if ($val == $action)
                    unset($argv[$idx]);

            foreach (glob(dirname(__file__).'/modules/*.php') as $script)
                include_once $script;
            if (($module = Module::getInstance($action)))
                return $module->_run($args['action']);

            $this->stderr->write("Unknown action given\n");
            $this->showHelp();
        }
    }
}

// @implements FS-092.1: CLI front controller & action dispatch — CLI-only invocation gate
// @implements BS-092-01: CLI-only execution — die when not run from the command line
// @implements EC-092-01: Non-CLI invocation — terminates "Management only supported from command-line"
if (php_sapi_name() != "cli")
    die("Management only supported from command-line\n");

$manager = new Manager();
$manager->parseOptions();
$manager->_run(basename(__file__));

?>
