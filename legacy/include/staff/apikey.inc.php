<?php
// @implements FS-043.2: API Key Administration Screens — add/edit view second-layer admin gate (OSTADMININC + $thisstaff->isAdmin else die 'Access Denied')
if(!defined('OSTADMININC') || !$thisstaff || !$thisstaff->isAdmin()) die('Access Denied');
// @implements FS-043.2: API Key Administration Screens — add-vs-edit mode select
// @implements FS-043.1: API Key Entity & Auto-Generated Secret — edit shows read-only IP/key (immutable secret)
$info=array();
$qstr='';
if($api && $_REQUEST['a']!='add'){
    $title='Update API Key';
    $action='update';
    $submit_text='Save Changes';
    $info=$api->getHashtable();
    $qstr.='&id='.$api->getId();
}else {
    $title='Add New API Key';
    $action='add';
    $submit_text='Add Key';
    $info['isactive']=isset($info['isactive'])?$info['isactive']:1;
    $qstr.='&a='.urlencode($_REQUEST['a']);
}
$info=Format::htmlchars(($errors && $_POST)?$_POST:$info);
?>
<?php /* @implements FS-043.2: API Key Administration Screens — add/edit form (Status radio, IP required on add, service checkboxes, notes; CSRF-protected) */ ?>
<form action="apikeys.php?<?php echo $qstr; ?>" method="post" id="save">
 <?php csrf_token(); ?>
 <input type="hidden" name="do" value="<?php echo $action; ?>">
 <input type="hidden" name="a" value="<?php echo Format::htmlchars($_REQUEST['a']); ?>">
 <input type="hidden" name="id" value="<?php echo $info['id']; ?>">
 <h2>API Key</h2>
 <table class="form_table" width="940" border="0" cellspacing="0" cellpadding="2">
    <thead>
        <tr>
            <th colspan="2">
                <h4><?php echo $title; ?></h4>
                <em>API Key is auto-generated. Delete and re-add to change the key.</em>
            </th>
        </tr>
    </thead>
    <tbody>
        <tr>
            <td width="150" class="required">
                Status:
            </td>
            <td>
                <input type="radio" name="isactive" value="1" <?php echo $info['isactive']?'checked="checked"':''; ?>><strong>Active</strong>
                <input type="radio" name="isactive" value="0" <?php echo !$info['isactive']?'checked="checked"':''; ?>>Disabled
                &nbsp;<span class="error">*&nbsp;</span>
            </td>
        </tr>
        <?php /* @implements FS-043.1: API Key Entity & Auto-Generated Secret — edit mode shows IP Address + API Key read-only (immutable secret) */ ?>
        <?php /* @implements BS-431: API Key Secret Is System-Generated and Immutable — no edit path for key/IP */ ?>
        <?php if($api){ ?>
        <tr>
            <td width="150">
                IP Address:
            </td>
            <td>
                <?php echo $api->getIPAddr(); ?>
            </td>
        </tr>
        <tr>
            <td width="150">
                API Key:
            </td>
            <td><?php echo $api->getKey(); ?> &nbsp;</td>
        </tr>
        <?php }else{ ?>
        <tr>
            <td width="150" class="required">
               IP Address:
            </td>
            <td>
                <input type="text" size="30" name="ipaddr" value="<?php echo $info['ipaddr']; ?>">
                &nbsp;<span class="error">*&nbsp;<?php echo $errors['ipaddr']; ?></span>
            </td>
        </tr>
        <?php } ?>
        <?php /* @implements FS-043.1: API Key Entity & Auto-Generated Secret — per-key permission flags (can_create_tickets + can_exec_cron) */ ?>
        <?php /* @implements BS-430: API Keys Are IP-Bound, Single-Permission-Gated Secrets — endpoint-specific permission flag */ ?>
        <tr>
            <th colspan="2">
                <em><strong>Services:</strong>: Check applicable API services enabled for the key.</em>
            </th>
        </tr>
        <tr>
            <td colspan=2 style="padding-left:5px">
                <label>
                    <input type="checkbox" name="can_create_tickets" value="1" <?php echo $info['can_create_tickets']?'checked="checked"':''; ?> >
                    Can Create Tickets <em>(XML/JSON/EMAIL)</em>
                </label>
            </td>
        </tr>
        <tr>
            <td colspan=2 style="padding-left:5px">
                <label>
                    <input type="checkbox" name="can_exec_cron" value="1" <?php echo $info['can_exec_cron']?'checked="checked"':''; ?> >
                    Can Execute Cron
                </label>
            </td>
        </tr>
        <tr>
            <th colspan="2">
                <em><strong>Admin Notes</strong>: Internal notes.&nbsp;</em>
            </th>
        </tr>
        <tr>
            <td colspan=2>
                <textarea name="notes" cols="21" rows="8" style="width: 80%;"><?php echo $info['notes']; ?></textarea>
            </td>
        </tr>
    </tbody>
</table>
<p style="padding-left:225px;">
    <input type="submit" name="submit" value="<?php echo $submit_text; ?>">
    <input type="reset"  name="reset"  value="Reset">
    <input type="button" name="cancel" value="Cancel" onclick='window.location.href="apikeys.php"'>
</p>
</form>
