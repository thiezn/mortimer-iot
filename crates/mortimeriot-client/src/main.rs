#[tokio::main]
async fn main() {
    if let Err(err) = mortimeriot_client::cli::run().await {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
