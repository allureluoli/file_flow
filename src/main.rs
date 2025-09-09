#[macro_use] extern crate rocket;
mod paste_id;
use paste_id::PasteId;

use rocket::data::{Data, ToByteUnit};
use rocket::http::uri::Absolute;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use std::path::Path;
use rocket::tokio::fs::File;
use rocket::serde::json::Json;

use rocket::{get, post, routes};
use rocket_cors::{CorsOptions, AllowedOrigins, AllowedHeaders};
use std::collections::HashSet;

const HOST: Absolute<'static> = uri!("http://localhost:8000");
const PASSWORD: &str = "password"; // 使用前记得修改密码哦

// 密码验证结构体
struct AuthGuard;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthGuard {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // 从查询参数获取密码
        if let Some(password) = request.query_value::<String>("password") {
            if let Ok(provided_password) = password {
                if provided_password == PASSWORD {
                    return Outcome::Success(AuthGuard);
                }
            }
        }

        Outcome::Error((Status::Unauthorized, ()))
    }
}

#[get("/")]
fn index() -> &'static str {
    "File upload service"
}

// 文件读取（需要密码验证）
#[get("/<id>?<password>")]
async fn retrieve(_auth: AuthGuard, id: PasteId<'_>, password: &str) -> Option<File> {
    let _ = password; // 消除警告
    
    let upload_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/", "upload");
    let filename = Path::new(upload_dir).join(id.as_str());
    File::open(&filename).await.ok()
}

// 文件上传（需要密码验证）
#[post("/save?<password>", data = "<paste>")]
async fn save(_auth: AuthGuard, paste: Data<'_>, password: &str) -> std::io::Result<String> {
    let _ = password; // 消除警告
    
    let id = PasteId::new(10);
    paste.open(200.gibibytes()).into_file(id.file_path()).await?;
    Ok(uri!(HOST, retrieve(&id, PASSWORD)).to_string())
}

#[get("/list?<password>")]
async fn list_files(_auth: AuthGuard, password: &str) -> Result<Json<Vec<String>>, Status> {
    let _ = password; // 消除警告
    
    let upload_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/", "upload");
    let path = Path::new(upload_dir);
    
    if !path.exists() {
        return Ok(Json(Vec::new()));
    }
    
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(file_name) = entry.file_name().into_string() {
                files.push(file_name);
            }
        }
    }
    
    Ok(Json(files))
}

#[launch]
fn rocket() -> _ {
    // 配置CORS选项，允许所有来源
    let cors = CorsOptions {
        allowed_origins: AllowedOrigins::all(),
        allowed_methods: vec![
            "GET", "POST", "PUT", "DELETE", "OPTIONS"
        ].into_iter().map(|s| s.parse().unwrap()).collect(),
        allowed_headers: AllowedHeaders::all(),
        allow_credentials: true,
        expose_headers: HashSet::new(),
        max_age: Some(3600),
        send_wildcard: false,
        fairing_route_base: "/".to_string(),
        fairing_route_rank: 0,
    }
    .to_cors()
    .expect("Failed to create CORS fairing");

    rocket::build()
        .mount("/", routes![index, save, retrieve, list_files])
        .attach(cors)
}
