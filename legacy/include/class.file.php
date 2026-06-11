<?php
/*********************************************************************
    class.file.php

    Peter Rotich <peter@osticket.com>
    Copyright (c)  2006-2013 osTicket
    http://www.osticket.com

    Released under the GNU General Public License WITHOUT ANY WARRANTY.
    See LICENSE.TXT for details.

    vim: expandtab sw=4 ts=4 sts=4:
**********************************************************************/

// @implements FS-022.12: Content-Addressed Chunked File Storage — stored file record entity
// @implements BS-022.9: Files Are Addressed by Content Hash
// @implements BS-022.10: Shared Files Are Reference-Counted; Bytes Purged Only When Orphaned
class AttachmentFile {

    var $id;
    var $ht;

    // @implements FS-022.12: Content-Addressed Chunked File Storage — construct/load a file by id
    function AttachmentFile($id) {
        $this->id =0;
        return ($this->load($id));
    }

    // @implements FS-022.12: Content-Addressed Chunked File Storage — hydrate file + ticket/canned ref counts
    // @implements BS-022.10: Shared Files Are Reference-Counted — counts canned + ticket references
    function load($id=0) {

        if(!$id && !($id=$this->getId()))
            return false;

        $sql='SELECT id, type, size, name, hash, f.created, '
            .' count(DISTINCT c.canned_id) as canned, count(DISTINCT t.ticket_id) as tickets '
            .' FROM '.FILE_TABLE.' f '
            .' LEFT JOIN '.CANNED_ATTACHMENT_TABLE.' c ON(c.file_id=f.id) '
            .' LEFT JOIN '.TICKET_ATTACHMENT_TABLE.' t ON(t.file_id=f.id) '
            .' WHERE f.id='.db_input($id)
            .' GROUP BY f.id';
        if(!($res=db_query($sql)) || !db_num_rows($res))
            return false;

        $this->ht=db_fetch_array($res);
        $this->id =$this->ht['id'];

        return true;
    }

    function reload() {
        return $this->load();
    }

    function getHashtable() {
        return $this->ht;
    }

    function getInfo() {
        return $this->getHashtable();
    }

    function getNumTickets() {
        return $this->ht['tickets'];
    }

    // @implements BS-022.10: Shared Files Are Reference-Counted — file is referenced by a canned response
    function isCanned() {
        return ($this->ht['canned']);
    }

    // @implements BS-022.10: Shared Files Are Reference-Counted — file is still referenced (ticket or canned)
    function isInUse() {
        return ($this->getNumTickets() || $this->isCanned());
    }

    function getId() {
        return $this->id;
    }

    function getType() {
        return $this->ht['type'];
    }

    function getMime() {
        return $this->getType();
    }

    function getSize() {
        return $this->ht['size'];
    }

    function getName() {
        return $this->ht['name'];
    }

    function getHash() {
        return $this->ht['hash'];
    }

    function lastModified() {
        return $this->ht['created'];
    }

    /**
     * Retrieve a hash that can be sent to scp/file.php?h= in order to
     * download this file
     */
    // @implements FS-022.12: Content-Addressed Chunked File Storage — combined download hash (content + session-bound half)
    // @implements BS-022.8: Download Access Requires a Fresh Session-Bound Hash
    function getDownloadHash() {
        return strtolower($this->getHash() . md5($this->getId().session_id().$this->getHash()));
    }

    // @implements FS-022.12: Content-Addressed Chunked File Storage — open chunked reader for this file
    function open() {
        return new AttachmentChunkedData($this->id);
    }

    // @implements FS-022.11: Download vs Display Delivery Semantics — stream bytes chunk-by-chunk
    function sendData() {
        @ini_set('zlib.output_compression', 'Off');
        $file = $this->open();
        while ($chunk = $file->read())
            echo $chunk;
    }

    // @implements FS-022.11: Download vs Display Delivery Semantics — buffer full bytes into memory (discouraged helper)
    function getData() {
        # XXX: This is horrible, and is subject to php's memory
        #      restrictions, etc. Don't use this function!
        ob_start();
        $this->sendData();
        $data = &ob_get_contents();
        ob_end_clean();
        return $data;
    }

    // @implements BS-022.10: Shared Files Are Reference-Counted; Bytes Purged Only When Orphaned — delete file + chunks
    function delete() {

        $sql='DELETE FROM '.FILE_TABLE.' WHERE id='.db_input($this->getId()).' LIMIT 1';
        if(!db_query($sql) || !db_affected_rows())
            return false;

        //Delete file data.
        AttachmentChunkedData::deleteOrphans();

        return true;
    }

    // @implements FS-022.11: Download vs Display Delivery Semantics — cache headers + 304 conditional short-circuit
    // @implements BS-022.12: Conditional Requests Short-Circuit With 304
    function makeCacheable($ttl=3600) {
        // Thanks, http://stackoverflow.com/a/1583753/1025836
        $last_modified = Misc::db2gmtime($this->lastModified());
        header("Last-Modified: ".date('D, d M y H:i:s', $last_modified)." GMT", false);
        header('ETag: "'.$this->getHash().'"');
        header("Cache-Control: private, max-age=$ttl");
        header('Expires: ' . gmdate(DATE_RFC822, time() + $ttl)." GMT");
        header('Pragma: private');
        if (@strtotime($_SERVER['HTTP_IF_MODIFIED_SINCE']) == $last_modified ||
            @trim($_SERVER['HTTP_IF_NONE_MATCH']) == $this->getHash()) {
                header("HTTP/1.1 304 Not Modified");
                exit();
        }
    }

    // @implements FS-022.11: Download vs Display Delivery Semantics — inline display mode (no Content-Disposition)
    function display() {
        $this->makeCacheable();

        header('Content-Type: '.($this->getType()?$this->getType():'application/octet-stream'));
        header('Content-Length: '.$this->getSize());
        $this->sendData();
        exit();
    }

    // @implements FS-022.11: Download vs Display Delivery Semantics — forced download mode + UA-specific filename encoding
    function download() {
        $this->makeCacheable();

        header('Content-Type: '.($this->getType()?$this->getType():'application/octet-stream'));

        $filename=basename($this->getName());
        $user_agent = strtolower ($_SERVER['HTTP_USER_AGENT']);
        if (false !== strpos($user_agent,'msie') && false !== strpos($user_agent,'win'))
            header('Content-Disposition: filename='.rawurlencode($filename).';');
        elseif (false !== strpos($user_agent, 'safari') && false === strpos($user_agent, 'chrome'))
            // Safari and Safari only can handle the filename as is
            header('Content-Disposition: filename='.str_replace(',', '', $filename).';');
        else
            // Use RFC5987
            header("Content-Disposition: filename*=UTF-8''".rawurlencode($filename).';' );

        header('Content-Transfer-Encoding: binary');
        header('Content-Length: '.$this->getSize());
        $this->sendData();
        exit();
    }

    /* Function assumes the files types have been validated */
    // @implements FS-022.12: Content-Addressed Chunked File Storage — upload one file (content hash + time)
    // @implements BS-022.9: Files Are Addressed by Content Hash
    function upload($file, $ft='T') {

        if(!$file['name'] || $file['error'] || !is_uploaded_file($file['tmp_name']))
            return false;

        $info=array('type'=>$file['type'],
                    'filetype'=>$ft,
                    'size'=>$file['size'],
                    'name'=>$file['name'],
                    'hash'=>MD5(MD5_FILE($file['tmp_name']).time()),
                    'data'=>file_get_contents($file['tmp_name'])
                    );

        return AttachmentFile::save($info);
    }

    // @implements FS-022.15: Logo File Upload (Shared File Store) — logo upload (ft='L') + GIF/JPEG/PNG + aspect ratio
    function uploadLogo($file, &$error, $aspect_ratio=3) {
        /* Borrowed in part from
         * http://salman-w.blogspot.com/2009/04/crop-to-fit-image-using-aspphp.html
         */
        if (!extension_loaded('gd'))
            return self::upload($file, 'L');

        $source_path = $file['tmp_name'];

        list($source_width, $source_height, $source_type) = getimagesize($source_path);

        switch ($source_type) {
            case IMAGETYPE_GIF:
            case IMAGETYPE_JPEG:
            case IMAGETYPE_PNG:
                break;
            default:
                // TODO: Return an error
                $error = 'Invalid image file type';
                return false;
        }

        $source_aspect_ratio = $source_width / $source_height;

        if ($source_aspect_ratio >= $aspect_ratio)
            return self::upload($file, 'L');

        $error = 'Image is too square. Upload a wider image';
        return false;
    }

    // @implements FS-022.12: Content-Addressed Chunked File Storage — persist file record + write chunked data
    function save($file) {

        if(!$file['hash'])
            $file['hash']=MD5(MD5($file['data']).time());
        if(!$file['size'])
            $file['size']=strlen($file['data']);

        $sql='INSERT INTO '.FILE_TABLE.' SET created=NOW() '
            .',type='.db_input($file['type'])
            .',size='.db_input($file['size'])
            .',name='.db_input($file['name'])
            .',hash='.db_input($file['hash']);

        # XXX: ft does not exists during the upgrade when attachments are
        #      migrated!
        if(isset($file['filetype']))
            $sql.=',ft='.db_input($file['filetype']);

        if (!(db_query($sql) && ($id=db_insert_id())))
            return false;

        $data = new AttachmentChunkedData($id);
        if (!$data->write($file['data']))
            return false;

        return $id;
    }

    /* Static functions */
    // @implements FS-022.12: Content-Addressed Chunked File Storage — resolve file id by permanent content hash
    function getIdByHash($hash) {

        $sql='SELECT id FROM '.FILE_TABLE.' WHERE hash='.db_input($hash);
        if(($res=db_query($sql)) && db_num_rows($res))
            list($id)=db_fetch_row($res);

        return $id;
    }

    // @implements FS-022.12: Content-Addressed Chunked File Storage — lookup by numeric id or content hash
    function lookup($id) {

        $id = is_numeric($id)?$id:AttachmentFile::getIdByHash($id);

        return ($id && ($file = new AttachmentFile($id)) && $file->getId()==$id)?$file:null;
    }

    /*
      Method formats http based $_FILE uploads - plus basic validation.
      @restrict - make sure file type & size are allowed.
     */
    // @implements FS-022.13: Upload Validation (Type & Size) — normalize $_FILES + type/size validation
    // @implements BS-022.13: Allowed File Types Are Matched by Extension Only
    // @implements BS-022.14: An Empty Allow-List Rejects Everything (Default-Deny)
    function format($files, $restrict=false) {
        global $ost;

        if(!$files || !is_array($files))
            return null;

        //Reformat $_FILE  for the sane.
        $attachments = array();
        foreach($files as $k => $a) {
            if(is_array($a))
                foreach($a as $i => $v)
                    $attachments[$i][$k] = $v;
        }

        //Basic validation.
        foreach($attachments as $i => &$file) {
            //skip no file upload "error" - why PHP calls it an error is beyond me.
            if($file['error'] && $file['error']==UPLOAD_ERR_NO_FILE) {
                unset($attachments[$i]);
                continue;
            }

            if($file['error']) //PHP defined error!
                $file['error'] = 'File upload error #'.$file['error'];
            elseif(!$file['tmp_name'] || !is_uploaded_file($file['tmp_name']))
                $file['error'] = 'Invalid or bad upload POST';
            elseif($restrict) { // make sure file type & size are allowed.
                if(!$ost->isFileTypeAllowed($file))
                    $file['error'] = 'Invalid file type for '.Format::htmlchars($file['name']);
                elseif($ost->getConfig()->getMaxFileSize()
                        && $file['size']>$ost->getConfig()->getMaxFileSize())
                    $file['error'] = sprintf('File %s (%s) is too big. Maximum of %s allowed',
                            Format::htmlchars($file['name']),
                            Format::file_size($file['size']),
                            Format::file_size($ost->getConfig()->getMaxFileSize()));
            }
        }
        unset($file);

        return array_filter($attachments);
    }

    /**
     * Removes files and associated meta-data for files which no ticket,
     * canned-response, or faq point to any more.
     */
    // @implements BS-022.10: Shared Files Are Reference-Counted; Bytes Purged Only When Orphaned — purge unreferenced files
    // @implements BS-022.11: Orphan Reclamation Is Scoped to Ticket-Type Files — only ft='T' files reclaimed
    /* static */ function deleteOrphans() {

        $sql = 'DELETE FROM '.FILE_TABLE.' WHERE id NOT IN ('
                .'SELECT file_id FROM '.TICKET_ATTACHMENT_TABLE
                .' UNION '
                .'SELECT file_id FROM '.CANNED_ATTACHMENT_TABLE
                .' UNION '
                .'SELECT file_id FROM '.FAQ_ATTACHMENT_TABLE
            .") AND `ft` = 'T'";

        db_query($sql);

        //Delete orphaned chuncked data!
        AttachmentChunkedData::deleteOrphans();

        return true;

    }

    // @implements FS-022.15: Logo File Upload (Shared File Store) — enumerate logo-type (ft='L') files
    /* static */
    function allLogos() {
        $sql = 'SELECT id FROM '.FILE_TABLE.' WHERE ft="L"
            ORDER BY created';
        $logos = array();
        $res = db_query($sql);
        while (list($id) = db_fetch_row($res))
            $logos[] = AttachmentFile::lookup($id);
        return $logos;
    }
}

/**
 * Attachments stored in the database are cut into 256kB chunks and stored
 * in the FILE_CHUNK_TABLE to overcome the max_allowed_packet limitation of
 * LOB fields in the MySQL database
 */
define('CHUNK_SIZE', 500*1024); # Beware if you change this...
// @implements FS-022.12: Content-Addressed Chunked File Storage — fixed-size chunked byte store (CHUNK_SIZE)
class AttachmentChunkedData {
    // @implements FS-022.12: Content-Addressed Chunked File Storage — bind reader/writer to a file id
    function AttachmentChunkedData($file) {
        $this->_file = $file;
        $this->_pos = 0;
    }

    // @implements FS-022.12: Content-Addressed Chunked File Storage — total byte length across chunks
    function length() {
        list($length) = db_fetch_row(db_query(
             'SELECT SUM(LENGTH(filedata)) FROM '.FILE_CHUNK_TABLE
            .' WHERE file_id='.db_input($this->_file)));
        return $length;
    }

    // @implements FS-022.12: Content-Addressed Chunked File Storage — read next chunk in (file_id, chunk_id) order
    function read() {
        # Read requested length of data from attachment chunks
        list($buffer) = @db_fetch_row(db_query(
            'SELECT filedata FROM '.FILE_CHUNK_TABLE.' WHERE file_id='
            .db_input($this->_file).' AND chunk_id='.$this->_pos++));
        return $buffer;
    }

    // @implements FS-022.12: Content-Addressed Chunked File Storage — REPLACE INTO upsert per (file_id, chunk_id)
    function write($what, $chunk_size=CHUNK_SIZE) {
        $offset=0;
        for (;;) {
            $block = substr($what, $offset, $chunk_size);
            if (!$block) break;
            if (!db_query('REPLACE INTO '.FILE_CHUNK_TABLE
                    .' SET filedata=0x'.bin2hex($block).', file_id='
                    .db_input($this->_file).', chunk_id='.db_input($this->_pos++)))
                return false;
            $offset += strlen($block);
        }

        return $this->_pos;
    }

    // @implements BS-022.10: Shared Files Are Reference-Counted; Bytes Purged Only When Orphaned — purge chunks with no parent file
    function deleteOrphans() {
        $deleted = 0;
        $sql = 'SELECT c.file_id, c.chunk_id FROM '.FILE_CHUNK_TABLE.' c '
             . ' LEFT JOIN '.FILE_TABLE.' f ON(f.id=c.file_id) '
             . ' WHERE f.id IS NULL';

        $res = db_query($sql);
        while (list($file_id, $chunk_id) = db_fetch_row($res)) {
            db_query('DELETE FROM '.FILE_CHUNK_TABLE
                .' WHERE file_id='.db_input($file_id)
                .' AND chunk_id='.db_input($chunk_id));
            $deleted += db_affected_rows();
        }
        return $deleted;
    }
}
?>
