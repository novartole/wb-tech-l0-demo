use std::net::Ipv4Addr;

use clap::builder::NonEmptyStringValueParser;

#[derive(clap::Parser)]
pub struct Cli {
    /// Listening IP
    #[clap(
        short, 
        long, 
        default_value_t = Ipv4Addr::LOCALHOST,
        env = "WBTECH_L0_DEMO_IP",
    )]
    pub ip: Ipv4Addr,

    /// Listening port
    #[clap(
        short,
        long, 
        value_parser = clap::value_parser!(u16).range(1..),
        default_value_t = 3000,
        env = "WBTECH_L0_DEMO_PORT"
    )]
    pub port: u16,

    /// Current thread scheduler
    #[clap(long, default_value_t = false, group = "runtime")]
    pub current_thread: bool,

    /// Multi thread scheduler
    #[clap(long, default_value_t = true, group = "runtime")]
    pub multi_thread: bool,

    /// Sets the number of worker threads
    #[clap(long, conflicts_with = "current_thread")]
    pub workers: Option<usize>,

    /// Database configuration string
    #[clap(
        long, 
        value_parser = NonEmptyStringValueParser::new(),
        env = "WBTECH_L0_DEMO_DB_PARAMS",
    )]
    pub db_params: String,

    /// Cache configuration string (optional).
    /// It won't be configured if this option isn't used.
    #[clap(
        long, 
        value_parser = NonEmptyStringValueParser::new(),
        env = "WBTECH_L0_DEMO_CACHE_PARAMS",
    )]
    pub cache_params: Option<String>,
}
