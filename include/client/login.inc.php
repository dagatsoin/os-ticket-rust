<?php /* @implements FS-010.2: Check-Ticket-Status Login Form — Check-Ticket-Status login form: OSTCLIENTINC guard, lemail/lticket prefill from POST else GET, CSRF token, login-error banner, View Status submit */ ?>
<?php
if(!defined('OSTCLIENTINC')) die('Access Denied');

$email=Format::input($_POST['lemail']?$_POST['lemail']:$_GET['e']);
$ticketid=Format::input($_POST['lticket']?$_POST['lticket']:$_GET['t']);
?>
<h1>Check Ticket Status</h1>
<p>To view the status of a ticket, provide us with the login details below.</p>
<form action="login.php" method="post" id="clientLogin">
    <?php csrf_token(); ?>
    <strong><?php echo Format::htmlchars($errors['login']); ?></strong>
    <br>
    <div>
        <label for="email">E-Mail Address:</label>
        <input id="email" type="text" name="lemail" size="30" value="<?php echo $email; ?>">
    </div>
    <div>
        <label for="ticketno">Ticket ID:</label>
        <input id="ticketno" type="text" name="lticket" size="16" value="<?php echo $ticketid; ?>"></td>
    </div>
    <p>
        <input class="btn" type="submit" value="View Status">
    </p>
</form>
<br>
<p>
If this is your first time contacting us or you've lost the ticket ID, please <a href="open.php">open a new ticket</a>.    
</p>
