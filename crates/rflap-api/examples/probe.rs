//! Sends one request to the API and prints the response.
//!
//! ```text
//! cargo run -p rflap-api --example probe -- '{"op": "capabilities"}'
//! echo '{"op": "capabilities"}' | cargo run -p rflap-api --example probe
//! ```

use std::io::Read;

fn main() {
    let request = match std::env::args().nth(1) {
        Some(arg) => arg,
        None => {
            let mut text = String::new();
            std::io::stdin()
                .read_to_string(&mut text)
                .expect("could not read the request from stdin");
            text
        }
    };
    println!("{}", rflap_api::call(&request));
}
