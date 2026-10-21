fn main() {
    let code = app::cli::execute(std::env::args_os().skip(1).collect());
    std::process::exit(code);
}
