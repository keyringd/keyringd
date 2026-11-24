pub const CANCEL: &str = "Cancel";
pub const CLOSE: &str = "Close";
pub const CONTINUE: &str = "Continue";
pub const BACK: &str = "Back";
pub const DONE: &str = "Done";
pub const SAVE: &str = "Save";
pub const OPEN_WINDOW_FAILED: &str =
    "Could not open the requested window. Try again. If the problem continues, restart the app.";
pub const COPY_FAILED: &str = "Could not copy to the clipboard. Try again.";

pub const OPEN_PUBLIC_KEYS_FAILED: &str = "Could not open Public Keys. Try again.";

pub const TRUST_ENTRIES_WINDOW_TITLE: &str = "SSH Public Keys";
pub const TRUST_ENTRIES_DESCRIPTION: &str = "Copy public keys to register on your SSH server.";
pub const TRUST_ENTRIES_REGISTRATION_UPDATE: &str = "Update your server registration";
pub const TRUST_ENTRIES_CERTIFICATE_SERVERS: &str = "For servers that support SSH certificates";
pub const TRUST_ENTRIES_CERTIFICATE_DESCRIPTION: &str =
    "Recommended. Your server registration stays the same when you add or replace a computer.";
pub const TRUST_ENTRIES_DIRECT_SERVERS: &str = "For servers that do not support SSH certificates";
pub const TRUST_ENTRIES_DIRECT_DESCRIPTION: &str = "For example, GitHub. Registering both public keys keeps access available after replacing your computer.";
pub const TRUST_ENTRIES_CA_DESCRIPTION: &str = "Register this public key as a certificate authority to trust certificates issued with your passkey.";
pub const TRUST_ENTRIES_PASSKEY_DESCRIPTION: &str =
    "Portable fallback that can be used from another computer.";
pub const TRUST_ENTRIES_TPM_KEY_DESCRIPTION: &str =
    "TPM-backed key for this PC. Its entry changes when the Device Key is replaced.";
pub const TRUST_ENTRIES_SECURE_ENCLAVE_KEY_DESCRIPTION: &str =
    "Secure Enclave key for this Mac. Its entry changes when the Device Key is replaced.";
pub const TRUST_ENTRIES_MENU_ACTION: &str = "Public Keys…";
pub const TOAST_DISMISS: &str = "Dismiss notification";
pub const SSH_CONNECTION_TEST_WINDOW_TITLE: &str = "Test SSH Connection";
pub const SSH_CONNECTION_TEST_DESCRIPTION: &str =
    "Enter a hostname, user@host, or a Host alias from your SSH config.";
pub const SSH_CONNECTION_TEST_TESTING: &str =
    "Testing the OpenSSH configuration and public-key authentication…";
pub const SSH_CONNECTION_TEST_HOST_LABEL: &str = "SSH destination";
pub const SSH_CONNECTION_TEST_HOST_PLACEHOLDER: &str = "user@example.com";
pub const SSH_CONNECTION_TEST_IDENTITY_LABEL: &str = "Test with";
pub const SSH_CONNECTION_TEST_TEST: &str = "Test";
pub const SSH_CONNECTION_TEST_TESTING_LABEL: &str = "Testing…";
pub const SSH_CONNECTION_TEST_HOST_REQUIRED: &str = "Enter an SSH destination.";
pub const SSH_CONNECTION_TEST_RESULT_WINDOW_TITLE: &str = "SSH Connection Result";
pub const SSH_CONNECTION_TEST_AUTHORIZED_KEYS_HEADING: &str = "1. Check authorized_keys";
pub const SSH_CONNECTION_TEST_AUTHORIZED_KEYS_EXPLANATION: &str =
    "The target account’s authorized_keys file must contain this exact entry:";
pub const SSH_CONNECTION_TEST_SERVER_HEADING: &str = "2. Check the SSH server";
pub const SSH_CONNECTION_TEST_COPY_ENTRY: &str = "Copy Entry";
pub const SSH_CONNECTION_TEST_COPIED: &str = "Public key copied";
pub const SSH_CONNECTION_TEST_REJECTION_GUIDANCE: &str = "SSH rejection guidance";
pub const SSH_CONNECTION_TEST_TPM_KEY_DESCRIPTION: &str = "TPM-backed key for this PC";
pub const SSH_CONNECTION_TEST_SECURE_ENCLAVE_KEY_DESCRIPTION: &str =
    "Secure Enclave-backed key for this Mac";
pub const PROFILE_LIST_ITEM_DETAILS: &str = "Details…";
pub const PROFILE_LIST_ITEM_TEST_CONNECTION: &str = "Test SSH connection…";
pub const PROFILE_LIST_ITEM_ENABLED: &str = "Enabled";
pub const PROFILE_LIST_ITEM_ACTIONS: &str = "Profile actions";
pub const PROFILE_LIST_ITEM_RENAME: &str = "Rename profile";
pub const PROFILE_LIST_ITEM_DRAG: &str = "Drag to reorder";
pub const ABOUT_TITLE: &str = "About Keyringd";
pub const ABOUT_LICENSE_NAME: &str = "License: Apache License 2.0";
pub const ABOUT_WEBSITE: &str = "Website";
pub const ABOUT_REPORT_ISSUE: &str = "Report an issue";
pub const ABOUT_LICENSE: &str = "License…";
pub const ABOUT_THIRD_PARTY_LICENSES: &str = "Third-party Licenses…";
pub const ABOUT_LICENSE_ACCESSIBILITY: &str = "License text";
pub const ABOUT_MAINTAINERS: &str = "Keyringd maintainers";
pub const ABOUT_WEBSITE_URL: &str = "https://github.com/keyringd/keyringd";
pub const ABOUT_ISSUES_URL: &str = "https://github.com/keyringd/keyringd/issues";
pub const PROFILE_SETUP_REGISTRATION_REQUIRED: &str =
    "Add its Public Keys to the SSH server before connecting.";
pub const PROFILE_SETUP_NOT_FOUND: &str = "No Keyringd passkey was found. Choose how to continue.";
pub const PROFILE_SETUP_RECOVERY_WARNING: &str =
    "Creating a new passkey will not restore access to servers you used with the previous passkey.";
pub const PROFILE_SETUP_WINDOW_TITLE: &str = "Add SSH profile";
pub const PROFILE_SETUP_TITLE: &str = "Create SSH Profile";
pub const PROFILE_SETUP_COMPLETED_TITLE: &str = "SSH Profile Created";
pub const PROFILE_SETUP_FORM_DESCRIPTION: &str = "Name the profile and choose a passkey.";
pub const PROFILE_SETUP_NAME: &str = "Profile name";
pub const PROFILE_SETUP_NAME_PLACEHOLDER: &str = "Work, Personal, …";
pub const PROFILE_SETUP_PASSKEY: &str = "Passkey";
pub const PROFILE_SETUP_CREATE_PASSKEY: &str = "Create a new passkey";
pub const PROFILE_SETUP_CREATE_PASSKEY_DESCRIPTION: &str = "Set up Keyringd for the first time";
pub const PROFILE_SETUP_EXISTING_PASSKEY: &str = "Use an existing passkey";
pub const PROFILE_SETUP_EXISTING_PASSKEY_DESCRIPTION: &str =
    "Use an existing passkey to set up SSH on this computer";
pub const PROFILE_SETUP_CREATE_CHOICE: &str =
    "Create a new passkey\nSet up Keyringd for the first time";
pub const PROFILE_SETUP_EXISTING_CHOICE: &str =
    "Use an existing passkey\nUse an existing passkey to set up SSH on this computer";
pub const PROFILE_SETUP_ADVANCED: &str = "Advanced settings";
pub const PROFILE_SETUP_ADVANCED_COLLAPSED: &str = "▸ Advanced settings";
pub const PROFILE_SETUP_ADVANCED_EXPANDED: &str = "▾ Advanced settings";
pub const PROFILE_SETUP_SIGNATURE_FORMAT: &str = "SSH authentication signature format";
pub const PROFILE_SETUP_SIGNATURE_DESCRIPTION: &str =
    "Used when authenticating with a Device Certificate or the Device Key’s public key.";
pub const PROFILE_SETUP_ECDSA_FORMAT: &str = "ecdsa-sha2-nistp256";
pub const PROFILE_SETUP_SECURITY_KEY_FORMAT: &str = "sk-ecdsa-sha2-nistp256@openssh.com";
pub const PROFILE_SETUP_ECDSA_DESCRIPTION: &str = "Default. Broad server compatibility.";
pub const PROFILE_SETUP_SECURITY_KEY_DESCRIPTION: &str = "Includes whether local authentication was performed. Requires a server that supports SSH security keys.";
pub const PROFILE_SETUP_ECDSA_CHOICE: &str =
    "ecdsa-sha2-nistp256\nDefault. Broad server compatibility.";
pub const PROFILE_SETUP_SECURITY_KEY_CHOICE: &str = "sk-ecdsa-sha2-nistp256@openssh.com\nIncludes whether local authentication was performed. Requires a server that supports SSH security keys.";
pub const PROFILE_SETUP_FORM_HINT: &str =
    "You will choose a security key or a nearby phone or tablet after continuing.";
pub const PROFILE_SETUP_CREATE: &str = "Create";
pub const PROFILE_SETUP_RETRY: &str = "Try again";
pub const PROFILE_SETUP_PUBLIC_KEYS_LABEL: &str = "Public Keys";
pub const PROFILE_SETUP_CHOOSE_PASSKEY: &str = "Choose Passkey";
pub const PROFILE_SETUP_CHOOSE_PASSKEY_DESCRIPTION: &str = "Multiple Passkeys were found on this device. Choose the one for the profile you want to recover.";
pub const PROFILE_SETUP_NOT_FOUND_TITLE: &str = "Passkey Not Found";
pub const PROFILE_SETUP_TRY_ANOTHER_DEVICE: &str = "Try another device";
pub const PROFILE_SETUP_TRY_ANOTHER_DESCRIPTION: &str =
    "Look for the existing Passkey on another device";
pub const PROFILE_SETUP_CREATE_INSTEAD: &str = "Create a new passkey instead";
pub const PROFILE_SETUP_CREATE_INSTEAD_DESCRIPTION: &str =
    "Won’t restore access to servers trusting the old key";
pub const PROFILE_SETUP_CANCELLED: &str = "Setup cancelled. You can try again.";
pub const PROFILE_SETUP_CANCELLING: &str = "Cancelling profile setup…";
pub const PROFILE_SETUP_CANCELLING_DESCRIPTION: &str =
    "Wait for the operation and key cleanup to finish.";
pub const PROFILE_SETUP_PREPARING: &str = "Preparing…";
pub const PROFILE_SETUP_PREPARING_NEXT: &str = "Preparing the next setup step…";
pub const PROFILE_SETUP_FINISHING_STEP: &str = "Finishing this step…";
pub const PROFILE_SETUP_APPROVE_REQUEST: &str =
    "Approve the request on your phone, tablet, or security key.";
pub const PROFILE_SETUP_CREATE_DESCRIPTION: &str =
    "This creates the portable Passkey used to authorize your computers.";
pub const PROFILE_SETUP_CREATE_SCAN: &str =
    "Scan with the phone or tablet where you want to create the Passkey.";
pub const PROFILE_SETUP_FIND_DESCRIPTION: &str =
    "Use the passkey you previously used with Keyringd.";
pub const PROFILE_SETUP_FIND_SCAN: &str =
    "Scan with the phone or tablet that contains your existing passkey.";
pub const PROFILE_SETUP_RECOVER_PUBLIC_KEY_DESCRIPTION: &str =
    "This confirms the passkey public key before it is used to authorize this computer.";
pub const PROFILE_SETUP_AUTHORIZE_NEW_DESCRIPTION: &str =
    "Your new Passkey will sign this computer’s Device Key. This is the final step.";
pub const PROFILE_SETUP_AUTHORIZE_DESCRIPTION: &str =
    "Your Passkey will sign this computer’s Device Key. This is the final step.";
pub const PROFILE_SETUP_PIN: &str = "PIN";
pub const PROFILE_SETUP_CONFIRM_PIN: &str = "Confirm PIN";
pub const PROFILE_SETUP_PIN_TITLE: &str = "Security Key PIN";
pub const PROFILE_SETUP_CONFIRM_OPERATION: &str = "Confirm the passkey operation";
pub const PROFILE_SETUP_CHOICES_ACCESSIBILITY: &str = "Authenticator transport choices";
pub const PROFILE_SETUP_PASSKEY_CHOICES_ACCESSIBILITY: &str = "Passkey choices";
pub const PROFILE_SETUP_PASSKEYS: &str = "Passkeys";
pub const PROFILE_SETUP_COMPLETED_ACCESSIBILITY: &str = "Profile creation completed";
pub const PROFILE_SETUP_FORM_ACCESSIBILITY: &str = "Profile setup";
pub const PROFILE_SETUP_AUTHORIZE_NEW_SCAN: &str =
    "Scan one more time with the same phone or tablet using this new QR code.";
pub const PROFILE_SETUP_AUTHORIZE_EXISTING_SCAN: &str =
    "Scan one last time with the same phone or tablet using this final QR code.";
pub const PROFILE_SETUP_RECOVER_PUBLIC_KEY_SCAN: &str =
    "Scan this new QR code again with the same phone or tablet.";
pub const PROFILE_SETUP_UNEXPECTED_STOP: &str = "Profile setup stopped unexpectedly.";
pub const REMOVE_PROFILE_MENU_ACTION: &str = "Remove profile…";
pub const REMOVE_PROFILE_WINDOW_TITLE: &str = "Remove SSH profile";
pub const REMOVE_PROFILE_DESCRIPTION: &str =
    "Its Device Key and local profile data will be removed from this computer.";
pub const REMOVE_PROFILE_REMOVING: &str = "Removing the Device Key and local profile data… You can close this window; the result will appear in Profiles.";
pub const REMOVE_PROFILE_REMOVE: &str = "Remove";
pub const REMOVE_PROFILE_RETRY: &str = "Retry removal";
pub const REMOVE_PROFILE_ERROR_TITLE: &str = "Could not remove profile";
pub const QR_PROMPT_BLUETOOTH_REQUIRED: &str = "Bluetooth must be enabled on both devices.";
pub const QR_PROMPT_UNAVAILABLE_TITLE: &str = "Can't connect to your phone or tablet";
pub const QR_PROMPT_SECURITY_KEY_HEADING: &str = "Use a security key instead?";
pub const QR_PROMPT_SECURITY_KEY_DESCRIPTION: &str =
    "You can also use a connected USB security key.";
pub const QR_PROMPT_RENDER_FAILED: &str =
    "The QR code could not be displayed. Cancel and try again, or use a security key.";
pub const QR_PROMPT_USE_SECURITY_KEY: &str = "Use security key…";
pub const APP_MENU_SSH_CLIENT_CONFIGURATION: &str = "Configure SSH Clients…";
pub const APP_MENU_PROFILES: &str = "Profiles…";
pub const APP_MENU_ADD_PROFILE: &str = "Add Profile…";
pub const APP_MENU_ABOUT: &str = "About Keyringd";
pub const APP_MENU_EXIT: &str = "Exit";
pub const APP_MENU_QUIT: &str = "Quit Keyringd";
pub const APP_MENU_ACCESSIBILITY: &str = "Application menu";
pub const APP_MENU_TOOLTIP: &str = "Main Menu";
pub const SSH_CLIENT_CONFIGURATION_DESCRIPTION: &str = "Point SSH clients at the Keyringd agent. Server Public Keys can be copied from each profile’s action menu.";
pub const SSH_CLIENT_CONFIGURATION_AGENT_DESCRIPTION: &str = "Use Keyringd as your SSH agent.";
pub const SSH_CLIENT_CONFIGURATION_CONFIGURE: &str = "Configure";
pub const SSH_CLIENT_CONFIGURATION_UNCONFIGURE: &str = "Unconfigure";
pub const SSH_CLIENT_CONFIGURATION_APPLYING: &str = "Applying…";
#[cfg(not(target_os = "macos"))]
pub const SSH_CLIENT_CONFIGURATION_APPLIED: &str = "Sign out and back in to apply the change.";
#[cfg(target_os = "macos")]
pub const SSH_CLIENT_CONFIGURATION_APPLIED: &str =
    "Reopen your terminal or SSH client to use the new setting.";
pub const SSH_CLIENT_CONFIGURATION_COPYING: &str = "Copying…";
pub const SSH_CLIENT_CONFIGURATION_COPIED: &str = "SSH setting copied.";
pub const SSH_CLIENT_CONFIGURATION_WINDOW_TITLE: &str = "Configure SSH Clients to Use Keyringd";
pub const SSH_CLIENT_CONFIGURATION_HEADING: &str = "Configure SSH Clients to Use Keyringd";
pub const SSH_CLIENT_CONFIGURATION_CONFIGURE_QUESTION: &str = "Configure SSH_AUTH_SOCK?";
pub const SSH_CLIENT_CONFIGURATION_UNCONFIGURE_QUESTION: &str =
    "Remove SSH_AUTH_SOCK configuration?";
pub const SSH_CLIENT_CONFIGURATION_FAILED: &str =
    "Could not change SSH_AUTH_SOCK configuration. Try again.";
pub const PROFILE_LIST_NAME_REQUIRED: &str = "Enter a profile name";
pub const PROFILE_LIST_RENAME: &str = "Rename";
pub const PROFILE_LIST_MOVE_UP: &str = "Move up";
pub const PROFILE_LIST_MOVE_DOWN: &str = "Move down";
pub const PROFILE_LIST_TITLE: &str = "SSH Profiles";
pub const PROFILE_LIST_DESCRIPTION: &str = "Keys available through the Keyringd agent";
pub const PROFILE_LIST_EMPTY_TITLE: &str = "No SSH Profiles";
pub const PROFILE_LIST_EMPTY_DESCRIPTION: &str =
    "Create a profile to make a Device Key available through the Keyringd agent.";
pub const PROFILE_LIST_ADD: &str = "Create Profile";
pub const PROFILE_LIST_ADD_TOOLTIP: &str = "Create SSH Profile";
pub const PROFILE_LIST_KEY_ACCESSIBILITY: &str = "SSH key";
pub const DEVICE_KEY_PROTECTION_ENABLE: &str = "Enable local authentication…";
pub const DEVICE_KEY_PROTECTION_RECONFIGURE: &str = "Reconfigure local authentication…";
pub const DEVICE_KEY_PROTECTION_CHANGE_PASSWORD: &str = "Change Device Key Password…";
pub const DEVICE_KEY_PROTECTION_DISABLE: &str = "Disable local authentication…";
pub const DEVICE_KEY_PROTECTION_RETRY: &str = "Try again";
pub const DEVICE_KEY_PROTECTION_ERROR_TITLE: &str = "Could not update local authentication";
pub const REPLACE_DEVICE_KEY_MENU_ACTION: &str = "Replace Device Key…";
pub const REPLACE_DEVICE_KEY_WINDOW_TITLE: &str = "Replace Device Key";
pub const REPLACE_DEVICE_KEY_CONFIRM_TITLE: &str = "Replace Device Key?";
pub const REPLACE_DEVICE_KEY_COMPLETED_TITLE: &str = "Device Key Replaced";
pub const REPLACE_DEVICE_KEY_ERROR_TITLE: &str = "Device Key Replacement Failed";
pub const REPLACE_DEVICE_KEY_DESCRIPTION: &str = "Existing Passkey registrations remain valid. Update the Device Key public key on servers that do not support SSH certificates.";
pub const REPLACE_DEVICE_KEY_AUTHORIZING: &str =
    "Creating and authorizing the replacement Device Key…";
pub const REPLACE_DEVICE_KEY_COMPLETED: &str = "Replace the old Device Key public key wherever you registered it. Open Public Keys… to copy the new key.\n\nExisting Passkey and CA registrations remain valid.";
pub const REPLACE_DEVICE_KEY_CANCELLED: &str =
    "Replacement canceled. The current Device Key is unchanged.";
pub const REPLACE_DEVICE_KEY_REPLACE: &str = "Replace";
pub const REPLACE_DEVICE_KEY_RETRY: &str = "Try again";
pub const PIN_PROMPT_PIN_LABEL: &str = "PIN";
pub const PIN_PROMPT_CONFIRM_PIN_LABEL: &str = "Confirm PIN";
pub const PIN_PROMPT_PIN_REASON: &str = "Enter the PIN for this security key.";
pub const PROFILE_DETAILS_WINDOW_TITLE: &str = "Profile Details";
pub const PROFILE_DETAILS_STATUS: &str = "Status";
pub const PROFILE_DETAILS_SIGNATURE_FORMAT: &str = "SSH authentication signature format";
pub const PROFILE_DETAILS_LOCAL_AUTHENTICATION: &str = "Local authentication";
pub const PROFILE_DETAILS_ACCESS: &str = "SSH Access";
pub const PROFILE_DETAILS_ADVANCED: &str = "Advanced";
pub const PROFILE_DETAILS_REPLACE: &str = "Replace…";
pub const PROFILE_DETAILS_REGISTRATIONS: &str = "Server authorization entries";

pub const PROFILE_DISABLED: &str = "Disabled";

pub const PIN_PROMPT_CREATE_HEADING: &str = "Set your security key PIN";

pub const PIN_PROMPT_ENTER_HEADING: &str = "Enter your security key PIN";

pub const DISABLE: &str = "Disable";

pub const ENABLE: &str = "Enable";

pub const QR_PROMPT_PROTECTION_HEADING: &str = "Authorize the Device Key";

pub const QR_PROMPT_REPLACEMENT_HEADING: &str = "Authorize the New Device Key";

pub const QR_PROMPT_AGENT_HEADING: &str = "Use a passkey on your phone or tablet";

pub const QR_PROMPT_DEFAULT_HEADING: &str = "Approve the passkey operation";

pub const QR_PROMPT_AGENT_DESCRIPTION: &str =
    "Scan this QR code with the phone or tablet that contains your passkey.";

pub const QR_PROMPT_DEFAULT_DESCRIPTION: &str = "Scan this QR code to approve this setup step.";

pub const QR_PROMPT_NO_BLUETOOTH: &str = "No Bluetooth adapter was found. Connect or enable one to use a passkey on your phone or tablet.";

pub const QR_PROMPT_BLUETOOTH_DISABLED: &str =
    "Bluetooth is off. Turn it on to use a passkey on your phone or tablet.";

pub const SECURITY_KEY_SELECTION_HEADING: &str = "Choose your security key";

pub const SECURITY_KEY_VERIFICATION_HEADING: &str = "Verify your identity on your security key";

pub const SECURITY_KEY_RETRY_HEADING: &str = "Approve on your security key again";

pub const SECURITY_KEY_APPROVAL_HEADING: &str = "Approve on your security key";

pub const SECURITY_KEY_SELECTION_DESCRIPTION: &str = "Touch the security key you want to use. After choosing a key, you will confirm the requested operation on that key.";

pub const SECURITY_KEY_SETUP_ACTION: &str = "approve this setup step";

pub const SECURITY_KEY_SSH_ACTION: &str = "approve this SSH operation";

pub const SECURITY_KEY_CREATE_PIN: &str = "Enter your security key PIN to create the Passkey.";

pub const SECURITY_KEY_AUTHORIZE_NEW_PIN: &str =
    "Enter your security key PIN again to authorize this computer.";

pub const SECURITY_KEY_FIND_PIN: &str = "Enter your security key PIN to find the existing Passkey.";

pub const SECURITY_KEY_VERIFY_PIN: &str =
    "Enter your security key PIN again to verify the Passkey.";

pub const SECURITY_KEY_AUTHORIZE_EXISTING_PIN: &str =
    "Enter your security key PIN one last time to authorize this computer.";

pub const SECURITY_KEY_DEFAULT_HEADING: &str = "Confirm on your security key";

pub const SECURITY_KEY_DEFAULT_DESCRIPTION: &str = "Follow the instructions on your security key.";

pub const AUTHENTICATOR_PHONE_HEADING: &str = "Confirm on your phone or tablet";

pub const AUTHENTICATOR_PHONE_DESCRIPTION: &str =
    "Complete the passkey confirmation shown on your phone or tablet.";

pub const AUTHENTICATOR_DEFAULT_DESCRIPTION: &str =
    "Complete the verification requested by your authenticator.";

pub const SECURITY_KEY_CREATE_HEADING: &str = "Approve Passkey creation";

pub const SECURITY_KEY_AUTHORIZE_HEADING: &str = "Authorize this computer";

pub const SECURITY_KEY_FIND_HEADING: &str = "Find the Passkey";

pub const SECURITY_KEY_VERIFY_HEADING: &str = "Verify the Passkey";

pub const SECURITY_KEY_CREATE_TOUCH: &str = "Touch your security key to create the Passkey.";

pub const SECURITY_KEY_AUTHORIZE_NEW_TOUCH: &str =
    "Touch your security key again to authorize this computer.";

pub const SECURITY_KEY_FIND_TOUCH: &str = "Touch your security key to find the existing passkey.";

pub const SECURITY_KEY_VERIFY_TOUCH: &str = "Touch your security key again to verify the Passkey.";

pub const SECURITY_KEY_AUTHORIZE_EXISTING_TOUCH: &str =
    "Touch your security key one last time to authorize this computer.";

pub const SECURITY_KEY_DEFAULT_TOUCH: &str =
    "Touch the blinking security key to approve this operation.";

pub const SECURITY_KEY_CREATE_VERIFICATION: &str =
    "Verify your identity on your security key to create the Passkey.";

pub const SECURITY_KEY_AUTHORIZE_NEW_VERIFICATION: &str =
    "Verify your identity on your security key again to authorize this computer.";

pub const SECURITY_KEY_FIND_VERIFICATION: &str =
    "Verify your identity on your security key to find the existing Passkey.";

pub const SECURITY_KEY_VERIFY_VERIFICATION: &str =
    "Verify your identity on your security key again to confirm the Passkey.";

pub const SECURITY_KEY_AUTHORIZE_EXISTING_VERIFICATION: &str =
    "Verify your identity on your security key one last time to authorize this computer.";

pub const SECURITY_KEY_DEFAULT_VERIFICATION: &str =
    "Verify your identity on your security key to approve this operation.";

pub const DEVICE_KEY_PROTECTION_ENABLE_OPERATION: &str = "Enable local authentication";

pub const DEVICE_KEY_PROTECTION_CHANGE_OPERATION: &str = "Reconfigure local authentication";

pub const DEVICE_KEY_PROTECTION_DISABLE_OPERATION: &str = "Disable local authentication";

pub const DEVICE_KEY_PROTECTION_ENABLED_NOTICE: &str = "Device Key local authentication enabled";

pub const DEVICE_KEY_PROTECTION_UPDATED_NOTICE: &str = "Device Key local authentication updated";

pub const DEVICE_KEY_PROTECTION_DISABLED_NOTICE: &str = "Device Key local authentication disabled";

pub const LOCAL_AUTHENTICATION_REQUIRED: &str = "Local authentication required";

pub const LOCAL_AUTHENTICATION_NOT_REQUIRED: &str = "No local authentication";

pub const SSH_CONNECTION_TEST_CERTIFICATE_CHOICE: &str = "Device Certificate (Recommended)";

pub const SSH_CONNECTION_TEST_DEVICE_KEY_CHOICE: &str = "Device Key (Legacy-compatible)";

pub const SSH_CONNECTION_TEST_PASSKEY_CHOICE: &str = "Passkey (Legacy-compatible)";

pub const SSH_CONNECTION_TEST_CERTIFICATE_DESCRIPTION: &str = "For SSH servers with CA support";

pub const SSH_CONNECTION_TEST_PASSKEY_DESCRIPTION: &str = "Portable fallback key";

pub const PROFILE_SETUP_CREATE_HEADING: &str = "Step 1/2 · Create Passkey";

pub const PROFILE_SETUP_FIND_HEADING: &str = "Step 1/3 · Find Passkey";

pub const PROFILE_SETUP_RECOVER_PUBLIC_KEY_HEADING: &str = "Step 2/3 · Verify Passkey";

pub const PROFILE_SETUP_AUTHORIZE_NEW_HEADING: &str = "Step 2/2 · Authorize this computer";

pub const PROFILE_SETUP_AUTHORIZE_EXISTING_HEADING: &str = "Step 3/3 · Authorize this computer";

pub const PROFILE_SETUP_MISSING_NAME: &str = "Enter a profile name.";

pub const PROFILE_SETUP_PASSKEY_NOT_FOUND: &str = "No Keyringd passkey was found.";

pub const PROFILE_SETUP_STARTING: &str = "Preparing the SSH profile…";

pub const PROFILE_SETUP_CREATING_PASSKEY: &str = "Creating the passkey…";

pub const PROFILE_SETUP_DISCOVERING_PASSKEY: &str = "Recovering the existing passkey…";

pub const PROFILE_SETUP_RECOVERING_PASSKEY_PUBLIC_KEY: &str = "Verifying the existing passkey…";

pub const PROFILE_SETUP_PROVISIONING_DEVICE_KEY: &str =
    "Creating and authorizing this computer’s Device Key…";

pub const PROFILE_SETUP_COMMITTING_PROFILE: &str = "Saving the SSH profile…";
