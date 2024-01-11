use axum::{
    http::HeaderMap,
    response::{Html, IntoResponse},
    routing::{get, post},
    Extension, Form, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};
// use std::fs::read_to_string;
use tower_http::services::ServeDir;

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug)]
struct User {
    id: i32,
    name: String,
}

async fn index(
    Extension(mut templates): Extension<Box<minijinja::Environment<'_>>>,
) -> Html<String> {
    if cfg!(debug_assertions) {
        templates.clear_templates();
    }
    let template = templates.get_template("index.html").unwrap();
    Html(
        template
            .render(minijinja::context!(name => "World"))
            .unwrap(),
    )
}

async fn click() -> Html<&'static str> {
    Html("<h2>You clicked the button</h2>")
}

async fn mypage(
    Extension(dbpool): Extension<PgPool>,
    Extension(mut templates): Extension<Box<minijinja::Environment<'_>>>,
) -> Html<String> {
    let users = sqlx::query_as::<_, User>("SELECT * FROM user_account")
        .fetch_all(&dbpool)
        .await
        .unwrap();

    if cfg!(debug_assertions) {
        templates.clear_templates();
    }
    let template = templates.get_template("mypage.html").unwrap();
    Html(
        template
            .render(minijinja::context!(users => users))
            .unwrap(),
    )
}

#[derive(Deserialize)]
struct Rename {
    name: String,
}

async fn rename(Form(form): Form<Rename>) -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        "HX-Trigger-After-Swap",
        r#"{ "namechanged": { "message": "Name successfully changed!" } }"#
            .parse()
            .unwrap(),
    );
    (headers, Html(form.name))
}

#[tokio::main]
async fn main() {
    let dbpool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&"postgresql://postgres@localhost/test")
        .await
        .expect("failed to connect to database");

    let mut templates = minijinja::Environment::new();
    templates.set_loader(minijinja::path_loader("templates"));

    let env_box = Box::new(templates); // WARN: Maybe a Mutex should be used, because we will clear templates on every `render()`
                                       // which could cause a race condition

    let app = Router::new()
        .route("/", get(index))
        .route("/click", post(click))
        .route("/rename", post(rename))
        .route("/mypage", get(mypage))
        .layer(Extension(dbpool))
        .layer(Extension(env_box))
        .fallback_service(ServeDir::new("static"));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
