#[tokio::main]
async fn main() {
    if let Err(err) = mortimeriot::cli::run().await {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
