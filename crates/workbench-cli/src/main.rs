fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--version") | Some("-V") => {
            println!("gww {}", env!("CARGO_PKG_VERSION"));
        }
        Some(other) => {
            eprintln!("gww: command `{other}` is not implemented yet (Phase 1 stub)");
            std::process::exit(2);
        }
        None => {
            eprintln!("gww: usage: gww --version");
            std::process::exit(2);
        }
    }
}
