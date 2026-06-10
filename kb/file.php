<?php
/*********************************************************************
    file.php
    
    Simply downloads the file...on hash validation as follows;
    
    * Hash must be 64 chars long.
    * First 32 chars is the perm. file hash
    * Next 32 chars  is md5(file_id.session_id().file_hash)

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/
require('kb.inc.php');
require_once(INCLUDE_DIR.'class.file.php');
// @implements FS-050.9: FAQ Article Attachments — downloads a KB article's attached file after hash validation
// @implements FS-022.10: Authorized Attachment Download & Inline Display — serves the stored file via the content-addressed file store
// @implements BS-022.8: Download Access Requires a Fresh Session-Bound Hash — verifies the 64-char hash (file hash + md5(id+session+hash))
$h=trim($_GET['h']);
//basic checks
if(!$h  || strlen($h)!=64  //32*2
        || !($file=AttachmentFile::lookup(substr($h,0,32))) //first 32 is the file hash.
        || strcasecmp(substr($h,-32),md5($file->getId().session_id().$file->getHash()))) //next 32 is file id + session hash.
    die('Unknown or invalid file. #'.Format::htmlchars($_GET['h']));

$file->download();
?>
