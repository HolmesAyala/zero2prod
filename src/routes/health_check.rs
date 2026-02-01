use actix_web::HttpResponse;

pub async fn health_check_controller() -> HttpResponse {
    HttpResponse::Ok().finish()
}
