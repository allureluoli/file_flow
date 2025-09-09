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

const HOST: Absolute<'static> = uri!("http://ddns.curesky.site:7878");
const PASSWORD: &str = "7RCVygHdGTyfeA1KLDed"; // 使用前记得修改密码哦

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
#[post("/save?<password>&<filename>", data = "<paste>")]
async fn save(_auth: AuthGuard, filename: &str, paste: Data<'_>, password: &str) -> std::io::Result<String> {
    let _ = password; // 消除警告
    
    let id = PasteId::new(10);
    let name_file_path = format!("{}.name", id.file_path().display());
    std::fs::write(&name_file_path, filename)?;


    paste.open(200.gibibytes()).into_file(id.file_path()).await?;
    Ok(uri!(HOST, retrieve(&id, PASSWORD)).to_string())
}

#[get("/list?<password>")]
async fn list_files(_auth: AuthGuard, password: &str) -> Result<Json<Vec<(String, u64, String)>>, Status> {
    let _ = password; // 消除警告
    
    let upload_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/", "upload");
    let path = Path::new(upload_dir);
    
    if !path.exists() {
        return Ok(Json(Vec::new()));
    }
    
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().unwrap_or_default() != "name" {
                if let Ok(metadata) = std::fs::metadata(&path) {
                    let file_id = path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    
                    // 检查对应的 .name 文件
                    let name_file_path = path.with_extension("name");
                    let display_name = if name_file_path.exists() {
                        // 从 .name 文件读取原始文件名
                        std::fs::read_to_string(&name_file_path)
                            .unwrap_or_else(|_| file_id.clone())
                    } else {
                        // 不存在 .name 文件，直接返回 ID
                        file_id.clone()
                    };
                    
                    let file_size = metadata.len();
                    files.push((file_id, file_size, display_name));
                }
            }
        }
    }
    
    Ok(Json(files))
}



#[launch]
fn rocket() -> _ {
    // 配置服务器监听地址 - 监听所有网络接口
    let config = rocket::Config {
        address: std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
        port: 7878,
        // 增加请求体大小限制（适合文件上传）
        limits: rocket::data::Limits::new()
            .limit("file", 200.gibibytes())
            .limit("data-form", 200.gibibytes()),
        ..rocket::Config::default()
    };

    // 配置CORS选项，允许所有来源
    let cors = CorsOptions {
        allowed_origins: AllowedOrigins::all(),
        allowed_methods: vec![
            "GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH", "HEAD"
        ].into_iter().map(|s| s.parse().unwrap()).collect(),
        allowed_headers: AllowedHeaders::all(),
        allow_credentials: true,
        expose_headers: {
            let mut set = HashSet::new();
            set.insert("Content-Type".to_string());
            set.insert("Content-Length".to_string());
            set.insert("Content-Disposition".to_string());
            set
        },
        max_age: Some(3600),
        send_wildcard: false,
        fairing_route_base: "/".to_string(),
        fairing_route_rank: 0,
    }
    .to_cors()
    .expect("Failed to create CORS fairing");

    // 确保上传目录存在
    let upload_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/", "upload");
    if !std::path::Path::new(upload_dir).exists() {
        if let Err(e) = std::fs::create_dir_all(upload_dir) {
            eprintln!("警告: 无法创建上传目录 {}: {}", upload_dir, e);
        } else {
            println!("已创建上传目录: {}", upload_dir);
        }
    }

    // 添加日志初始化
    if std::env::var("ROCKET_LOG_LEVEL").is_err() {
        std::env::set_var("ROCKET_LOG_LEVEL", "normal");
    }

    println!("🚀 服务器启动在: http://0.0.0.0:7878");
    println!("📁 上传目录: {}", upload_dir);
    println!("🌐 CORS 已启用，允许所有来源");

    rocket::custom(config)
        .mount("/", routes![index, save, retrieve, list_files])
        .attach(cors)
        // 添加自定义错误处理
      
}
