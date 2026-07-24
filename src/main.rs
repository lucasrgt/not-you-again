#[rustfmt::skip]
fn main() { std::process::exit(nya::run_cli_env().unwrap_or_else(|error| { nya::print_error(&error); 2 })) }
