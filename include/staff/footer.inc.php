    </div>
    <?php /* @implements FS-090.14: Footer Chrome & Auto-Cron Beacon — staff control-panel footer chrome (copyright bar) */ ?>
    <div id="footer">
        Copyright &copy; 2006-<?php echo date('Y'); ?>&nbsp;osTicket.com. &nbsp;All Rights Reserved.
    </div>
<?php
// @implements FS-043.11: Autocron Web Fallback — autocron web-fallback beacon: 1x1 GIF -> autocron.php (removing it stops autocron)
if(is_object($thisstaff) && $thisstaff->isStaff()) { ?>
    <div>
        <!-- Do not remove <img src="autocron.php" alt="" width="1" height="1" border="0" /> or your auto cron will cease to function -->
        <img src="autocron.php" alt="" width="1" height="1" border="0" />
        <!-- Do not remove <img src="autocron.php" alt="" width="1" height="1" border="0" /> or your auto cron will cease to function -->
    </div>
<?php
} ?>
</div>
<div id="overlay"></div>
<div id="loading">
    <h4>Please Wait!</h4>
    <p>Please wait... it will take a second!</p>
</div>
</body>
</html>
