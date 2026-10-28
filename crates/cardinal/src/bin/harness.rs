//! `harness`: the name of `cardinal` before the rename, installed alongside it so the hooks of a project synced
//! before it keep running (spec `specs/tool-binaries.md`, BIN-18).

#[path = "../main.rs"]
mod cardinal_main;

fn main() -> std::process::ExitCode {
    cardinal_main::main()
}
