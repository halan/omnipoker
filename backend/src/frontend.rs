use actix_web::{get, web, HttpResponse, Responder};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../frontend/dist"]
struct Assets;

#[get("/{filename:.*}")]
pub async fn assets(filename: web::Path<String>) -> impl Responder {
    let filename = if filename == web::Path::from("".to_owned()) {
        "index.html"
    } else {
        &*filename
    };
    if let Some(content) = Assets::get(&filename) {
        let body = content.data;
        let mime_type = mime_guess::from_path(&*filename).first_or_text_plain();
        HttpResponse::Ok()
            .content_type(mime_type.as_ref())
            .body(body)
    } else {
        HttpResponse::NotFound().body("404 - Not Found")
    }
}
