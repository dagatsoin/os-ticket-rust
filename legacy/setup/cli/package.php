#!/usr/bin/env php
<?php

// @implements FS-092.10: Release packager — pre-flight, staging, version stamping & archiving — CLI-only invocation gate
// @implements BS-092-01: CLI-only execution — die when not run from the command line
// @implements EC-092-01: Non-CLI invocation — terminates "Only command-line packaging is supported"
if (php_sapi_name() != 'cli')
    die("Only command-line packaging is supported");

$stage_folder = "stage";
$stage_path = dirname(__file__) . '/' . $stage_folder;

// @implements FS-092.10: Release packager — pre-flight, staging, version stamping & archiving — locate repo root by walking up to main.inc.php
function get_osticket_root_path() {
    # Hop up to the root folder
    $start = dirname(__file__);
    for (;;) {
        if (file_exists($start . '/main.inc.php')) break;
        $start .= '/..';
    }
    return realpath($start);
}

function run_tests($root) {
    return (require "$root/setup/test/run-tests.php");
}

# Check PHP syntax across all php files
// @implements FS-092.10: Release packager — pre-flight, staging, version stamping & archiving — recursive glob helper (used to wipe the stage/ tree)
function glob_recursive($pattern, $flags = 0) {
    $files = glob($pattern, $flags);
    foreach (glob(dirname($pattern).'/*', GLOB_ONLYDIR|GLOB_NOSORT) as $dir) {
        $files = array_merge($files,
            glob_recursive($dir.'/'.basename($pattern), $flags));
    }
    return $files;
}

$root = get_osticket_root_path();

// @implements FS-092.11: Release packager — staged layout (`stage/upload` tree) — fnmatch-based exclusion helper for the staging copy
function exclude($pattern, $match) {
    if (is_array($pattern)) {
        foreach ($pattern as $p)
            if (fnmatch($p, $match))
                return true;
    }
    else
        return fnmatch($pattern, $match);
    return false;
}

// @implements FS-092.11: Release packager — staged layout (`stage/upload` tree) — copy matched files into stage/<destination> tree (recursive, exclusion-aware)
function package($pattern, $destination, $recurse=false, $exclude=false) {
    global $root, $stage_path;
    $search = $root . '/' . $pattern;
    echo "Packaging " . $search . "\n";
    foreach (glob($search, GLOB_BRACE|GLOB_NOSORT) as $file) {
        if (is_file($file)) {
            if ($exclude && exclude($exclude, $file))
                continue;
            if (!is_dir("$stage_path/$destination"))
                mkdir("$stage_path/$destination", 0777, true);
            copy($file, $stage_path . '/' . $destination . '/' . basename($file));
        }
    }
    if ($recurse) {
        foreach (glob(dirname($search).'/*', GLOB_ONLYDIR|GLOB_NOSORT) as $dir) {
            if ($exclude && exclude($exclude, $dir))
                continue;
            package(dirname($pattern).'/'.basename($dir).'/'.basename($pattern),
                $destination.'/'.basename($dir),
                $recurse-1, $exclude);
        }
    }
}

# Run tests before continuing
// @implements FS-092.10: Release packager — pre-flight, staging, version stamping & archiving — regression test pre-flight gate
// @implements BS-092-07: Tests gate the release — any failure aborts packaging
// @implements EC-092-08: Tests fail during packaging — abort "Regression tests failed. Cowardly refusing to package"
if (run_tests($root) > 0)
    die("Regression tests failed. Cowardly refusing to package\n");

# Create the stage folder for the install files
// @implements FS-092.10: Release packager — pre-flight, staging, version stamping & archiving — (re)create a clean stage/ tree: wipe files, remove dirs deepest-first
if (!is_dir($stage_path))
    mkdir($stage_path);
else {
    $dirs = array();
    foreach (glob_recursive($stage_path . '/*') as $file)
        if (is_dir($file))
            $dirs[] = $file;
        else
            unlink($file);
    sort($dirs);
    foreach (array_reverse($dirs) as $dir)
        rmdir($dir);
}

# Source code goes into 'upload'
// @implements FS-092.11: Release packager — staged layout (`stage/upload` tree) — app source under stage/upload, scripts/ standalone, license/docs at root
mkdir($stage_path . '/upload');

# Load the root directory files
package("*.php", 'upload/');
package("web.config", 'upload/');

# Load the client interface
foreach (array('assets','css','images','js') as $dir)
    package("$dir/*", "upload/$dir", -1, "*less");

# Load API and pages
package('api/{,.}*', 'upload/api');
package('pages/{,.}*', 'upload/pages');

# Load the knowledgebase
package("kb/*.php", "upload/kb");

# Load the staff interface
package("scp/*.php", "upload/scp/", -1);
foreach (array('css','images','js') as $dir)
    package("scp/$dir/*", "upload/scp/$dir", -1);

# Load in the scripts
mkdir("$stage_path/scripts/");
package("setup/scripts/*", "scripts/", -1, "*stage");

# Load the heart of the system
package("include/{,.}*", "upload/include", -1, array('*ost-config.php', '*.sw[a-z]'));

# Include the installer
package("setup/*.{php,txt,html}", "upload/setup", -1, array("*scripts","*test","*stage"));
foreach (array('css','images','js') as $dir)
    package("setup/$dir/*", "upload/setup/$dir", -1);
package("setup/inc/streams/*.sql", "upload/setup/inc/streams", -1);

# Load the license and documentation
package("*.{txt,md}", "");

#Rename markdown as text TODO: Do html version before rename.
if(($mds = glob("$stage_path/*.md"))) {
    foreach($mds as $md)
        rename($md, preg_replace('/\.md$/', '.txt', $md));
}

# Make an archive of the stage folder
// @implements FS-092.10: Release packager — pre-flight, staging, version stamping & archiving — release version label from git describe (stamp + archive names)
$version = exec('git describe');

$pwd = getcwd();
chdir($stage_path);

// Replace THIS_VERSION in the stage/ folder

// @implements FS-092.12: Release packager — version stamp & production error-display hardening — stamp THIS_VERSION + force display_errors/display_startup_errors=0
// @implements BS-092-08: Shipped builds suppress error display (inverse of install-time) — packaged build forces error display off
shell_exec("find . -name '*.inc.php' -print0 | xargs -0 sed -ri -e \"
    s/( *)define\('THIS_VERSION'.*/\\1define('THIS_VERSION', '$version');/
    s/( *)ini_set\( *'display_errors'[^)]+\);/\\1ini_set('display_errors', 0);/
    s/( *)ini_set\( *'display_startup_errors'[^)]+\);/\\1ini_set('display_startup_errors', 0);/
    \"");

// @implements FS-092.10: Release packager — pre-flight, staging, version stamping & archiving — produce .tar.bz2 + .zip release archives named for the version
shell_exec("tar cjf '$pwd/osTicket-$version.tar.bz2' *");
shell_exec("zip -r '$pwd/osTicket-$version.zip' *");

chdir($pwd);
?>
