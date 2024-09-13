use std::time::Instant;

use ed25519_dalek::{verify_batch, Signer, SigningKey};
use rand::distributions::Alphanumeric;
use rand::{rngs::OsRng, Rng};

fn main() {
    let mut csprng = OsRng;
    let verify_size = 1<<10;
    let signing_keys: Vec<_> = (0..verify_size).map(|_| SigningKey::generate(&mut csprng)).collect();
    let smsg = "They're good dogs Brant".to_owned();
    let messages: Vec<_> = (0..verify_size)
        .map(|i| {
            let random_string: String = (0..5).map(|_| csprng.sample(Alphanumeric) as char).collect();
            format!("{}{}{}", smsg, random_string, i)
        })
        .collect();
    let message_refs: Vec<&[u8]> = messages.iter().map(|m| m.as_bytes()).collect();

    let mut i = 0;
    let signatures: Vec<_> = signing_keys
        .iter()
        .map(|key| {
            i = i + 1;
            key.sign(&message_refs[i - 1])
        })
        .collect();

    let start_time = Instant::now();
    let verifying_keys: Vec<_> = signing_keys.iter().map(|key| key.verifying_key()).collect();
    let result = verify_batch(&message_refs, &signatures, &verifying_keys);
    println!("cost: {:?}", start_time.elapsed());
    assert!(result.is_ok());
    match result {
        Ok(_) => println!("Passed: All signatures are valid!"),
        Err(err) => println!("Some signatures are invalid.{}", err),
    }
}
