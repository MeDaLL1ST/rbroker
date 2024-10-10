use prometrics_sb::axpromlib::{get_str_var, load_env, run_metrics};

use selfmqr::handlers::*;

#[tokio::main]
async fn main() {
    load_env();
    let pool = prometrics_sb::create_pool!(get_str_var("MAX_THREADS").parse().unwrap());
    let app = axum::Router::new()
        .route("/add", axum::routing::get(add_handler))
        .with_state(pool.clone())
        .route("/list", axum::routing::get(list_handler))
        .with_state(pool.clone())
        .route("/info", axum::routing::get(info_handler))
        .with_state(pool.clone())
        .route("/subscribe", axum::routing::get(subs_handler))
        .route("/metrics", axum::routing::get(run_metrics));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:".to_string() + &get_str_var("PORT"))
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
