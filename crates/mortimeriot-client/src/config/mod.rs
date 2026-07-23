use clap::Args;

#[derive(Debug, Clone, Args)]
pub struct ConnectionArgs {
    #[arg(long, default_value = "http://127.0.0.1:2111")]
    pub base_url: String,
    #[arg(long, env = "MORTIMERIOT_API_KEY")]
    pub auth_key: Option<String>,
}
