use std::{convert::Infallible, env, net::SocketAddr, time::Duration};
use tracing::{field::Empty, Level, Span};
use tracing_subscriber::{layer::SubscriberExt, Registry};
use warp::{http::StatusCode, Filter};

const HTML: &str = r#"<ul>
    <li><a href="/">home</a></li>
    <li><a href="/sleep/100">sleep 100ms</a></li>
    <li><a href="/sleep/300">sleep 300ms</a></li>
    <li><a href="/sleep/500">sleep 500ms</a></li>
    <li><a href="/404">404</a></li>
</ul>"#;

fn home() -> impl warp::Reply {
    Span::current().record("name", &"GET /");

    warp::reply::html(HTML)
}

async fn sleep(ms: u64) -> Result<impl warp::Reply, Infallible> {
    Span::current().record("name", &"GET /sleep/:ms");

    tracing::info!(ms, "sleep {}ms", ms);

    tokio::time::sleep(Duration::from_millis(ms)).await;

    Ok(warp::reply::html(HTML))
}

fn not_found() -> impl warp::Reply {
    Span::current()
        .record("name", &"not found")
        .record("otel.status_code", &"ERROR")
        .record("otel.status_description", &"not found");

    warp::reply::with_status(warp::reply::html(HTML), StatusCode::NOT_FOUND)
}

#[tokio::main]
async fn main() {
    let newrelic = tracing_newrelic::layer(env::var("API_KEY").expect("API_KEY not found"));

    let fmt = tracing_subscriber::fmt::layer();

    let target = tracing_subscriber::filter::Targets::new().with_target("warp", Level::INFO);

    let subscriber = Registry::default().with(newrelic).with(fmt).with(target);

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to initilize tracing subscriber");

    let home = warp::path::end().and(warp::get()).map(home);

    let sleep = warp::path!("sleep" / u64).and(warp::get()).and_then(sleep);

    let not_found = warp::any().map(not_found);

    let routes = home.or(sleep).or(not_found).with(warp::trace(|_| {
        tracing::info_span!(
            "request",
            span.kind = "server",
            service.name = "tracing-newrelic-demo",
            name = Empty,
            otel.status_code = Empty,
            otel.status_description = Empty,
        )
    }));

    let addr: SocketAddr = "127.0.0.1:5555".parse().unwrap();

    println!("API server running at {}", addr);

    warp::serve(routes).run(addr).await;
}
