mod games;
mod lobby;
mod handlers;

use actix_files::Files;
use actix_web::{web, App, HttpServer, middleware};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let state = lobby::new_shared_state();

    log::info!("Starting Board Games Portal on http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .wrap(middleware::Logger::default())
            // API routes
            .route("/api/rooms", web::post().to(handlers::create_room))
            .route("/api/rooms", web::get().to(handlers::list_rooms))
            .route("/api/rooms/{room_id}", web::get().to(handlers::get_room))
            // WebSocket route
            .route("/ws/{room_id}", web::get().to(handlers::ws_handler))
            // Static files (serve index.html for all unmatched routes for SPA)
            .service(Files::new("/", "./static").index_file("index.html"))
    })
    // .bind("127.0.0.1:8080")?
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
