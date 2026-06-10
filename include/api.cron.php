<?php

include_once INCLUDE_DIR.'class.cron.php';

// @implements FS-043.9: Remote HTTP Cron Execution — HTTP cron API controller invoked via POST /tasks/cron
class CronApiController extends ApiController {

    // @implements FS-043.9: Remote HTTP Cron Execution — requires a cron-authorized API key, then runs the cron cycle
    // @implements FS-043.6: API Key Authentication & IP Binding — requireApiKey + canExecuteCron permission gate, 401 otherwise
    function execute() {

        if(!($key=$this->requireApiKey()) || !$key->canExecuteCron())
            return $this->exerr(401, 'API key not authorized');

        $this->run();
    }

    /* private */
    // @implements FS-043.7: Cron Job Inventory — runs the full ordered cron cycle (Cron::run) and logs/acknowledges completion
    // @implements BS-433: Full Cron Cycle Is Upgrade-Gated and Ordered — delegates to Cron::run for the ordered job sequence
    function run() {
        global $ost;

        Cron::run();
       
        $ost->logDebug('Cron Job','Cron job executed ['.$_SERVER['REMOTE_ADDR'].']');
        $this->response(200,'Completed');
    }
}

// @implements FS-043.10: Local (Command-Line) Cron Execution — CLI cron controller subclass reporting via process exit codes
class LocalCronApiController extends CronApiController {

    // @implements FS-043.10: Local (Command-Line) Cron Execution — maps cron outcome to exit codes (0 silent success, 1 on error) for the shell
    function response($code, $resp) {

        if($code == 200) //Success - exit silently.
            exit(0);
        
        //On error echo the response (error)
        echo $resp;
        exit(1);
    }
        
    // @implements FS-043.10: Local (Command-Line) Cron Execution — static entry invoked by api/cron.php to run a local cron cycle
    function call() {
        $cron = new LocalCronApiController();
        $cron->run();
    }
}
?>
