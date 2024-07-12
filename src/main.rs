use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use serde::Deserialize;
use serde::Serialize;
use sqlx::mysql::MySqlPool;
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use std::time::{Duration, Instant};

#[derive(Deserialize)]
struct SearchQuery {
    query: String,
}

#[derive(Serialize)]
struct SearchResult {
    nicknames: Vec<String>,
}

struct AppState {
    db_pool: MySqlPool,
    cache: Arc<RwLock<HashMap<String, (Vec<String>, Instant)>>>,
    refresh_lock: Arc<Mutex<()>>,
}

async fn search(
    query: web::Query<SearchQuery>,
    data: web::Data<AppState>
) -> impl Responder {
    let search_query = query.query.to_lowercase();
    let mut cache = data.cache.write().await;

    if let Some((cached_nicknames, timestamp)) = cache.get(&search_query) {
        if timestamp.elapsed() < Duration::from_secs(30) {
            return HttpResponse::Ok().json(SearchResult { nicknames: cached_nicknames.clone() });
        }
    }

    let _refresh_lock = data.refresh_lock.lock().await;

    // Double-check
    if let Some((cached_nicknames, timestamp)) = cache.get(&search_query) {
        if timestamp.elapsed() < Duration::from_secs(30) {
            return HttpResponse::Ok().json(SearchResult { nicknames: cached_nicknames.clone() });
        }
    }

    let search_pattern = format!("%{}%", search_query);
    let rows = sqlx::query("SELECT nickname FROM economy WHERE LOWER(nickname) LIKE ?")
        .bind(search_pattern)
        .fetch_all(&data.db_pool)
        .await
        .expect("Failed to fetch stats");

    let nicknames: Vec<String> = rows.into_iter().map(|row| row.get::<String, _>("nickname")).collect();

    cache.insert(search_query.clone(), (nicknames.clone(), Instant::now()));

    HttpResponse::Ok().json(SearchResult { nicknames })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let database_url = "mysql://USER:PSWRD@localhost/DB_NAME";
    let db_pool = MySqlPool::connect(database_url).await.expect("Failed to create pool.");
    let cache: Arc<RwLock<HashMap<String, (Vec<String>, Instant)>>> = Arc::new(RwLock::new(HashMap::new()));
    let refresh_lock: Arc<Mutex<()>> = Arc::new(Mutex::new(()));

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                db_pool: db_pool.clone(),
                cache: cache.clone(),
                refresh_lock: refresh_lock.clone(),
            }))
            .route("/search", web::get().to(search))
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
