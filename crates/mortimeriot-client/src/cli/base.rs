use crate::{Error, Result, api::ApiClient, config::ConnectionArgs};
use clap::{Args, Parser, Subcommand, ValueEnum};
use mortimeriot_core::{WeatherHistoryQuery, WeatherMeasurement};
use tracing::{Level, info};

#[derive(Debug, Parser)]
#[command(name = "mortimeriot-client")]
#[command(about = "Client for mortimeriot server")]
pub struct Cli {
    #[arg(long, value_enum, default_value_t = LogLevel::Info)]
    pub log_level: LogLevel,
    #[command(flatten)]
    pub connection: ConnectionArgs,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Health,
    Version,
    Weather {
        #[command(subcommand)]
        command: WeatherCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum WeatherCommand {
    Send(WeatherSendArgs),
    List(WeatherListArgs),
    Latest,
}

#[derive(Debug, Clone, Args)]
pub struct WeatherSendArgs {
    #[arg(long)]
    pub temperature: f64,
    #[arg(long)]
    pub humidity: f64,
}

#[derive(Debug, Clone, Args)]
pub struct WeatherListArgs {
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub to: Option<String>,
    #[arg(long)]
    pub limit: Option<u32>,
    #[arg(long)]
    pub cursor: Option<i64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn to_tracing_level(self) -> Level {
        match self {
            Self::Error => Level::ERROR,
            Self::Warn => Level::WARN,
            Self::Info => Level::INFO,
            Self::Debug => Level::DEBUG,
            Self::Trace => Level::TRACE,
        }
    }
}

pub async fn run() -> Result {
    let cli = Cli::parse();
    init_tracing(cli.log_level);
    info!(level = ?cli.log_level, "starting client command execution");

    let api = ApiClient::new(cli.connection.base_url, cli.connection.auth_key);

    match cli.command {
        Command::Health => {
            let response = api.healthcheck().await?;
            println!("{}", serde_json::to_string_pretty(&response)?);
        }
        Command::Version => {
            let version = api.version().await?;
            println!("{}", serde_json::to_string_pretty(&version)?);
        }
        Command::Weather { command } => match command {
            WeatherCommand::Send(args) => {
                let payload = WeatherMeasurement {
                    temperature: args.temperature,
                    humidity: args.humidity,
                };
                payload
                    .validate()
                    .map_err(|err| Error::InvalidInput(err.to_owned()))?;

                let reading = api.post_weather(&payload).await?;
                println!("{}", serde_json::to_string_pretty(&reading)?);
            }
            WeatherCommand::List(args) => {
                let query = WeatherHistoryQuery {
                    from: args.from,
                    to: args.to,
                    limit: args.limit,
                    cursor: args.cursor,
                };
                let response = api.list_weather_data(&query).await?;
                println!("{}", serde_json::to_string_pretty(&response)?);
            }
            WeatherCommand::Latest => {
                let response = api.latest_weather_data().await?;
                println!("{}", serde_json::to_string_pretty(&response)?);
            }
        },
    }

    Ok(())
}

fn init_tracing(level: LogLevel) {
    let _ = tracing_subscriber::fmt()
        .with_max_level(level.to_tracing_level())
        .with_target(false)
        .compact()
        .try_init();
}
