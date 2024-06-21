use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use serde::Deserialize;
use serde::Serialize;
use sqlx::mysql::MySqlPool;
use sqlx::Row;

#[derive(Deserialize)]
struct SearchQuery {
    query: String,
}

#[derive(Serialize)]
struct SearchResult {
    nicknames: Vec<String>,
}

async fn search(
    query: web::Query<SearchQuery>,
    db_pool: web::Data<MySqlPool>
) -> impl Responder {
    let search_query = format!("%{}%", query.query.to_lowercase());

    let rows = sqlx::query("SELECT nickname FROM stats WHERE LOWER(nickname) LIKE ?")
        .bind(search_query)
        .fetch_all(db_pool.get_ref())
        .await
        .expect("Failed to fetch stats");

    let nicknames: Vec<String> = rows
        .into_iter()
        .map(|row| row.get::<String, _>("nickname"))
        .collect();

    HttpResponse::Ok().json(SearchResult { nicknames })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let database_url = "mysql://USER:PSWRD@localhost/DB_NAME";
    let db_pool = MySqlPool::connect(database_url).await.expect("Failed to create pool.");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .route("/search", web::get().to(search))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
