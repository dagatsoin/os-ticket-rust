<?php
require_once INCLUDE_DIR.'class.migrater.php';

// @implements FS-061.5: Resumable procedural migration tasks — single-shot mail-account password re-encryption
// @implements FS-003.1: Two-key reversible encryption — re-encrypts stored userpass via Crypto::encrypt under the new tagged scheme
class CryptoMigrater extends MigrationTask {
    var $description = "Migrating encrypted password";
    var $status ='Making the world a better place!';

    // @implements FS-003.1: Two-key reversible encryption — decrypts legacy ciphertext then Crypto::encrypt re-wraps it per email account
    function run() {

        $sql='SELECT email_id, userpass, userid FROM '.EMAIL_TABLE
            ." WHERE userpass <> ''";
        if(($res=db_query($sql)) && db_num_rows($res)) {
            while(list($id, $passwd, $username) = db_fetch_row($res)) {
                if(!$passwd) continue;
                $ciphertext = Crypto::encrypt(self::_decrypt($passwd, SECRET_SALT), SECRET_SALT, $username);
                $sql='UPDATE '.EMAIL_TABLE
                    .' SET userpass='.db_input($ciphertext)
                    .' WHERE email_id='.db_input($id);
                db_query($sql);
            }
        }
    }

    /*
      XXX: This is not a  good way of decrypting data - use to descrypt old
      data.
     */
    // @implements FS-003.1: Two-key reversible encryption — legacy mcrypt Rijndael-256 ECB decryption of pre-1.7 ciphertext (one-way migration helper)
    function _decrypt($text, $salt) {

        if(!function_exists('mcrypt_encrypt') || !function_exists('mcrypt_decrypt'))
            return $text;

        return trim(mcrypt_decrypt(MCRYPT_RIJNDAEL_256, $salt, base64_decode($text), MCRYPT_MODE_ECB,
                        mcrypt_create_iv(mcrypt_get_iv_size(MCRYPT_RIJNDAEL_256, MCRYPT_MODE_ECB), MCRYPT_RAND)));
    }
}
return 'CryptoMigrater';
?>
