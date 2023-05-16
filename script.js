function hashPassword() {
    var passwordInput = document.getElementById("passwordInput");
    var hashedPassword = CryptoJS.SHA256(passwordInput.value).toString();

    // Create a hidden input field
    var hiddenField = document.getElementById("hashedPassword");
    hiddenField.setAttribute("value", hashedPassword);
}