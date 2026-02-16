use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

// State to hold the shared HTTP client
struct AppState {
    client: Client,
}

// Input format (Piston-like, from local frontend)
#[derive(Deserialize)]
struct PistonRequest {
    language: String,
    files: Vec<PistonFile>,
    stdin: Option<String>,
}

#[derive(Deserialize)]
struct PistonFile {
    content: String,
}

// Output format (Piston-like, for local frontend)
#[derive(Serialize)]
struct PistonResponse {
    run: PistonRunResult,
}

#[derive(Serialize)]
struct PistonRunResult {
    stdout: String,
    stderr: String,
    code: i32,
    signal: Option<String>,
}

// Wandbox Request
#[derive(Serialize)]
struct WandboxRequest {
    compiler: String,
    code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    stdin: Option<String>,
    #[serde(rename = "compiler-option-raw", skip_serializing_if = "Option::is_none")]
    compiler_option_raw: Option<String>,
}

// Wandbox Response
#[derive(Deserialize)]
struct WandboxResponse {
    status: String, // "0" for success, others for error
    program_message: Option<String>, // stdout
    program_error: Option<String>,   // stderr
    compiler_message: Option<String>, // compiler output
    compiler_error: Option<String>,   // compiler errors
}

async fn execute_code(
    state: web::Data<AppState>,
    body: web::Json<PistonRequest>,
) -> impl Responder {
    log::info!("Received request: language={}, files={}", body.language, body.files.len());

    // 1. Map Language to Wandbox Compiler
    let (compiler, options) = match body.language.as_str() {
        "python" => ("cpython-3.10.15", None), 
        "c" => ("gcc-13.2.0-c", None), 
        "java" => ("openjdk-jdk-21+35", None),
        _ => return HttpResponse::BadRequest().body("Unsupported language"),
    };

    // 2. Extract code
    let source_code = match body.files.first() {
        Some(f) => f.content.clone(),
        None => return HttpResponse::BadRequest().body("No source code provided"),
    };

    // 3. Construct Wandbox Request
    let wandbox_req = WandboxRequest {
        compiler: compiler.to_string(),
        code: source_code,
        stdin: body.stdin.clone(),
        compiler_option_raw: options.map(|s: &str| s.to_string()),
    };

    // 4. Send to Wandbox
    log::info!("Sending to Wandbox: compiler={}", compiler);
    let response = state
        .client
        .post("https://wandbox.org/api/compile.json")
        .json(&wandbox_req)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let status = resp.status();
            log::info!("Wandbox HTTP Status: {}", status);

            if !status.is_success() {
                let error_text = resp.text().await.unwrap_or_default();
                log::error!("Wandbox API Error Body: {}", error_text);
                return HttpResponse::InternalServerError().body(format!("Wandbox API Error: {}", error_text));
            }

            // Read text first to debug JSON parsing issues
            let resp_text = resp.text().await.unwrap_or_default();
            log::info!("Wandbox Response Body: {}", resp_text);

            match serde_json::from_str::<WandboxResponse>(&resp_text) {
                Ok(w_resp) => {
                    // 5. Transform Wandbox Response -> Piston Response
                    let stdout = w_resp.program_message.unwrap_or_default();
                    let mut stderr = w_resp.program_error.unwrap_or_default();
                    
                    if let Some(msg) = w_resp.compiler_message {
                        if !msg.is_empty() {
                            stderr.push_str("\n--- Compiler Message ---\n");
                            stderr.push_str(&msg);
                        }
                    }
                    if let Some(err) = w_resp.compiler_error {
                        if !err.is_empty() {
                             stderr.push_str("\n--- Compiler Error ---\n");
                             stderr.push_str(&err);
                        }
                    }

                    let exit_code = w_resp.status.parse::<i32>().unwrap_or(1);

                    let piston_resp = PistonResponse {
                        run: PistonRunResult {
                            stdout,
                            stderr,
                            code: exit_code,
                            signal: None,
                        },
                    };
                    
                    HttpResponse::Ok().json(piston_resp)
                }
                Err(err) => {
                    log::error!("JSON Parse Error: {}", err);
                    HttpResponse::InternalServerError().body(format!("Failed to parse Wandbox response: {}", err))
                }
            }
        }
        Err(err) => {
            log::error!("Reqwest Error: {}", err);
            HttpResponse::InternalServerError().body(format!("Failed to call Wandbox API: {}", err))
        }
    }
}

async fn health() -> impl Responder {
    HttpResponse::Ok().body("OpenLexer Backend (Wandbox Proxy) Live")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let port = env::var("PORT").unwrap_or_else(|_| "8000".to_string());
    let address = format!("0.0.0.0:{}", port);
    
    log::info!("Starting Wandbox proxy on {}", address);

    let client = Client::builder()
        .user_agent("OpenLexer-Backend/0.1")
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .build()
        .expect("Failed to create HTTP client");

    let state = web::Data::new(AppState {
        client,
    });

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header(); 

        App::new()
            .wrap(cors)
            .app_data(state.clone())
            .route("/execute", web::post().to(execute_code))
            .route("/health", web::get().to(health))
    })
    .bind(address)?
    .run()
    .await
}
