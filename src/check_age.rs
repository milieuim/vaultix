pub fn check() {
    let data = vec![0u8; 100];
    match age::Decryptor::new(&data[..]) {
        Ok(age::Decryptor::Recipients(_)) => println!("Recipients"),
        Ok(age::Decryptor::Passphrase(_)) => println!("Passphrase"),
        Err(_) => println!("Error"),
    }
}
