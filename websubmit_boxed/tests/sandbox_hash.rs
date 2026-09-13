use sesame::pcon::PCon;
use sesame::policy::{AnyPolicy, NoPolicy};
use sesame::sandbox::execute_sandbox;

use websubmit_boxed_sandboxes::hash;

#[test]
fn sandbox_hash() {
    let email = PCon::new(String::from("email"), NoPolicy {});
    let secret = PCon::new(String::from("secret"), NoPolicy {});
    let result: PCon<String, AnyPolicy> =
        execute_sandbox::<hash, _, _, _>((email, secret));
    let result = result.specialize_policy::<NoPolicy>().unwrap();
    assert_eq!(result.discard_box(), String::from("7843776296c8ae944dae58c7c49067263dd56b4f279b250af7fd13c278292c33"));
}
