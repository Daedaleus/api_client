//! `api-client-cli`: see the `ac_cli` crate.

fn main() -> std::process::ExitCode {
    ac_cli::main(std::env::args_os())
}
