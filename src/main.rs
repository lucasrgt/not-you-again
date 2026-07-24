#[rustfmt::skip]
fn main() { std::process::exit(nya::run_cli_env().unwrap_or_else(|error| { eprintln!("nya: {error:#}"); 2 })) }
