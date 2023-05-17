function hashPassword(password) {
    return CryptoJS.SHA256(password).toString();
}