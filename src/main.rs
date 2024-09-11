mod cache;
mod cli;
mod dto;
mod error;
mod handler;
mod model;
mod repo;
mod state;

use std::{env, net::SocketAddr};

use anyhow::Ok;
use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use cli::Cli;
use state::{CacheOrder, StoreOrder};
use tokio::{net::TcpListener, runtime as tokio_runtime};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

use crate::{cache::RedisCache, repo::PostgresRepo, state::AppState};

fn main() {
    // parse arguments
    let cli = Cli::parse();

    setup_tracing();

    // main task of server
    let run = async {
        let listener = {
            let addr = SocketAddr::from((cli.ip, cli.port));
            TcpListener::bind(addr).await?
        };

        let app = {
            info!(database = "postgres", cache = "redis");

            // setup and get connection to db
            let postgres = PostgresRepo::try_new(&cli.db_params).await?;
            // setup and get connection to cache service (optional)
            let maybe_redis = {
                let mut service = None;
                if let Some(params) = cli.cache_params.as_ref() {
                    service.replace(RedisCache::try_new(params).await?);
                }
                service
            };
            // grab all services into one state
            let state = AppState::new(postgres, maybe_redis);
            // extract it into separate fn for easy testing and cleaner code
            app_with_state(state)
        };

        info!("start listening on {:?}:{}", cli.ip, cli.port);
        Ok(axum::serve(listener, app).await?)
    };

    // setup runtime
    if cli.current_thread {
        debug!(name = "tokio", "build with 'current_thread' flavor");
        tokio_runtime::Builder::new_current_thread()
    } else if cli.multi_thread {
        debug!(name = "tokio", "build with 'multi_thread' flavor");
        let mut builder = tokio_runtime::Builder::new_multi_thread();
        if let Some(n) = cli.workers {
            debug!(name = "tokio", "build with {} workers", n);
            builder.worker_threads(n);
        }
        builder
    } else {
        panic!("neither 'current thread' nor 'multi thread' flavor was selected");
    }
    .enable_all()
    .build()
    .expect("failed to build tokio runtime")
    .block_on(async { run.await.expect("failed to serve API") })
}

fn app_with_state(
    state: AppState<
        impl StoreOrder + Clone + Send + Sync + 'static,
        impl CacheOrder + Clone + Send + Sync + 'static,
    >,
) -> Router {
    Router::new()
        .route("/order", post(handler::create_order))
        .route("/orders/:order_id", get(handler::get_order))
        .route("/orders/:order_id/delivery", get(handler::get_delivery))
        .route("/orders/:order_id/items", get(handler::get_items))
        .route("/orders/:order_id/payment", get(handler::get_payment))
        .with_state(state)
}

fn setup_tracing() {
    // use "info" level dy default
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    // Turn on error backtrace by default.
    // FYI:
    // - if you want panics and errors to both have backtraces, set RUST_BACKTRACE=1,
    // - If you want only errors to have backtraces, set RUST_LIB_BACKTRACE=1,
    // - if you want only panics to have backtraces, set RUST_BACKTRACE=1 and RUST_LIB_BACKTRACE=0.
    if env::var("RUST_LIB_BACKTRACE").is_err() {
        env::set_var("RUST_LIB_BACKTRACE", "1");
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        // setup globally
        .init();
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use crate::{
        error::Error,
        model::{Delivery, Item, Order, Payment},
        state::{AppState, CacheOrder, StoreOrder},
    };

    use super::app_with_state;

    #[rustfmt::skip]
    impl CacheOrder for () {
        async fn get_order(&self, _: &str) -> Result<Option<Order>, Error> { unreachable!() }
        async fn insert_order(&self, _: &Order) -> Result<(), Error> { unreachable!() }
    }

    #[tokio::test]
    async fn get_items_of_no_order() {
        #[derive(Clone)]
        struct MockRepo;

        #[rustfmt::skip]
        impl StoreOrder for MockRepo {
            async fn create_order(&self, _: Order) -> Result<(), Error> { unreachable!() }
            async fn get_order(&self, _: &str) -> Result<Option<Order>, Error> { unreachable!() }
            async fn get_delivery(&self, _: &str) -> Result<Option<Delivery>, Error> { unreachable!() }
            async fn get_payment(&self, _: &str) -> Result<Option<Payment>, Error> { unreachable!() }

            // the only implementation we need
            async fn get_items(&self, _: &str) -> Result<Option<Vec<Item>>, Error> {
                Ok(None)
            }
        }

        let state = AppState::new(MockRepo, Option::<()>::None);
        let response = app_with_state(state)
            .oneshot(
                Request::builder()
                    .uri("/orders/defenetly_does_not_exist_order_id/items")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn get_items_of_order_without_items() {
        #[derive(Clone)]
        struct MockRepo;

        #[rustfmt::skip]
        impl StoreOrder for MockRepo {
            async fn create_order(&self, _: Order) -> Result<(), Error> { unreachable!() }
            async fn get_order(&self, _: &str) -> Result<Option<Order>, Error> { unreachable!() }
            async fn get_delivery(&self, _: &str) -> Result<Option<Delivery>, Error> { unreachable!() }
            async fn get_payment(&self, _: &str) -> Result<Option<Payment>, Error> { unreachable!() }

            // the only implementation we need
            async fn get_items(&self, _: &str) -> Result<Option<Vec<Item>>, Error> {
                Ok(Some(vec![]))
            }
        }

        let state = AppState::new(MockRepo, Option::<()>::None);
        let response = app_with_state(state)
            .oneshot(
                Request::builder()
                    .uri("/orders/b563feb7b2b84b6test/items")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
