// SPDX-License-Identifier: Apache-2.0
fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match lockstep_headless::run(&arguments) {
        Ok(line) => println!("{line}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
