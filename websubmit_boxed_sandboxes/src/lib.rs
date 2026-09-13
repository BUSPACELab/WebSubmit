use sha2::{Sha256, Digest};

use sesame_derive::SesameSandbox;

// Sandbox functions.
#[SesameSandbox()]
pub fn hash(inputs: (String, String)) -> String {
    let mut hasher = Sha256::new();
    hasher.update(&inputs.0);
    hasher.update(&inputs.1);
    format!("{:x}", hasher.finalize())
}
