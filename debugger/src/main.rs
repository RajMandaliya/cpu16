    /// cpu16 debugger — Axum HTTP server
///
/// Endpoints:
///   POST /api/load      { source: string }  → assemble + load program, reset CPU
///   POST /api/step                          → execute one instruction
///   POST /api/run       { cycles: u64 }     → run up to N cycles
///   POST /api/reset                         → reset CPU, keep program
///   GET  /api/state                         → full CPU state as JSON
///   GET  /              → serve index.html

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use cpu16::{
    assembler::Assembler,
    cpu::{Cpu, CpuState, PROG_BASE},
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tower_http::{cors::CorsLayer, services::ServeDir};

// ── Shared state ──────────────────────────────────────────────────────────────

struct AppState {
    cpu: Mutex<Cpu>,
    program_bytes: Mutex<Vec<u8>>,
}

// ── API types ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct LoadRequest {
    source: String,
}

#[derive(Deserialize)]
struct RunRequest {
    cycles: Option<u64>,
}

#[derive(Serialize)]
struct ApiError {
    error: String,
}

#[derive(Serialize)]
struct CpuStateResponse {
    regs: [u16; 4],
    pc: u16,
    sp: u16,
    flags: FlagsResponse,
    cycles: u64,
    state: String,
    cache: CacheStatsResponse,
    memory: Vec<MemoryRow>,
    pipeline: Option<PipelineResponse>,
}

#[derive(Serialize)]
struct FlagsResponse {
    zero: bool,
    carry: bool,
    negative: bool,
    overflow: bool,
    int_enable: bool,
    raw: u8,
}

#[derive(Serialize)]
struct CacheStatsResponse {
    reads: u64,
    writes: u64,
    hits: u64,
    misses: u64,
    cold_misses: u64,
    conflict_misses: u64,
    hit_rate: f64,
    lines: Vec<CacheLineResponse>,
}

#[derive(Serialize)]
struct CacheLineResponse {
    index: usize,
    valid: bool,
    tag: u16,
    data: u16,
}

#[derive(Serialize)]
struct MemoryRow {
    addr: u16,
    bytes: Vec<u8>,
}

#[derive(Serialize)]
struct PipelineResponse {
    if_id: StageResponse,
    id_ex: StageResponse,
    ex_mem: StageResponse,
    mem_wb: StageResponse,
}

#[derive(Serialize)]
struct StageResponse {
    valid: bool,
    label: String,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn handle_load(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoadRequest>,
) -> impl IntoResponse {
    let asm = Assembler::new(PROG_BASE);
    match asm.assemble(&req.source) {
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        ),
        Ok(output) => {
            let bytes: Vec<u8> = output.words.iter().flat_map(|w| w.to_le_bytes()).collect();
            let mut cpu = state.cpu.lock().unwrap();
            *cpu = Cpu::new();
            cpu.load_program(&bytes);
            *state.program_bytes.lock().unwrap() = bytes;
            (StatusCode::OK, Json(serde_json::json!({ "ok": true, "words": output.words.len() })))
        }
    }
}

async fn handle_step(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut cpu = state.cpu.lock().unwrap();
    match cpu.step() {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "ok": true }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

async fn handle_run(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RunRequest>,
) -> impl IntoResponse {
    let cycles = req.cycles.unwrap_or(100_000);
    let mut cpu = state.cpu.lock().unwrap();
    match cpu.run(cycles) {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({ "ok": true }))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

async fn handle_reset(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let bytes = state.program_bytes.lock().unwrap().clone();
    let mut cpu = state.cpu.lock().unwrap();
    *cpu = Cpu::new();
    if !bytes.is_empty() {
        cpu.load_program(&bytes);
    }
    (StatusCode::OK, Json(serde_json::json!({ "ok": true })))
}

async fn handle_state(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let cpu = state.cpu.lock().unwrap();

    let cpu_state_str = match cpu.state {
        CpuState::Running => "running",
        CpuState::Halted => "halted",
        CpuState::WaitingForInterrupt => "waiting",
    };

    // Build cache lines
    let cache_lines: Vec<CacheLineResponse> = (0..cpu16::cache::NUM_LINES)
        .map(|i| {
            let line = cpu.cache.get_line(i);
            CacheLineResponse {
                index: i,
                valid: line.valid,
                tag: line.tag,
                data: line.data,
            }
        })
        .collect();

    let cache = CacheStatsResponse {
        reads: cpu.cache.stats.reads,
        writes: cpu.cache.stats.writes,
        hits: cpu.cache.stats.hits,
        misses: cpu.cache.stats.misses,
        cold_misses: cpu.cache.stats.cold_misses,
        conflict_misses: cpu.cache.stats.conflict_misses,
        hit_rate: cpu.cache.stats.hit_rate(),
        lines: cache_lines,
    };

    // Memory dump: show region around PC and stack
    let mut memory: Vec<MemoryRow> = Vec::new();
    // Program region: PROG_BASE to PROG_BASE + 256 bytes
    let prog_start = PROG_BASE;
    for row in (0..16u16).map(|r| prog_start.wrapping_add(r * 16)) {
        let bytes: Vec<u8> = (0..16u16)
            .map(|b| cpu.mem.read_byte(row.wrapping_add(b)))
            .collect();
        memory.push(MemoryRow { addr: row, bytes });
    }

    let response = CpuStateResponse {
        regs: cpu.regs,
        pc: cpu.pc,
        sp: cpu.sp,
        flags: FlagsResponse {
            zero: cpu.flags.zero(),
            carry: cpu.flags.carry(),
            negative: cpu.flags.negative(),
            overflow: cpu.flags.overflow(),
            int_enable: cpu.flags.int_enable(),
            raw: cpu.flags.0,
        },
        cycles: cpu.cycles,
        state: cpu_state_str.to_string(),
        cache,
        memory,
        pipeline: None,
    };

    Json(response)
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        cpu: Mutex::new(Cpu::new()),
        program_bytes: Mutex::new(Vec::new()),
    });

    let app = Router::new()
        .route("/api/load", post(handle_load))
        .route("/api/step", post(handle_step))
        .route("/api/run", post(handle_run))
        .route("/api/reset", post(handle_reset))
        .route("/api/state", get(handle_state))
        .nest_service("/", ServeDir::new("debugger/static"))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = "127.0.0.1:3000";
    println!("cpu16 debugger running at http://{}", addr);
    println!("Open your browser and navigate to http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}