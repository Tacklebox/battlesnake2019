#![deny(warnings)]

use actix_web::{
    App,
    AsyncResponder,
    Error,
    HttpMessage,
    HttpRequest,
    HttpResponse,
    http,
    middleware,
    server,
};

use env_logger;
use futures::Future;
use serde_derive::{ Serialize, Deserialize };

//TODO: use clap or something to make a nicer interface for this
static IP: &str = "127.0.0.1";
static PORT: &str = "8008";
static SNAKE_COLOR: &str = "#54A4E5";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SnakeHead {
    Beluga,
    Bendr,
    Dead,
    Evil,
    Fang,
    Pixel,
    Regular,
    Safe,
    SandWorm,
    Shades,
    Silly,
    Smile,
    Tongue,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum SnakeTail {
    BlockBum,
    Bolt,
    Curled,
    FatRattle,
    Freckled,
    Hook,
    Pixel,
    Regular,
    RoundBum,
    Sharp,
    Skinny,
    SmallRattle,
}

#[derive(Debug, Serialize, Deserialize)]
struct Game {
    id: String
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Moves {
    Up,
    Down,
    Left,
    Right
}

#[derive(Debug, Serialize, Deserialize)]
struct Coord {
    x: i32,
    y: i32
}

#[derive(Debug, Serialize, Deserialize)]
struct Snake {
    id: String,
    name: String,
    health: i32,
    body: Vec<Coord>
}

#[derive(Debug, Serialize, Deserialize)]
struct Board {
    height: i32,
    width: i32,
    food: Vec<Coord>,
    snakes: Vec<Snake>
}

#[derive(Debug, Serialize, Deserialize)]
struct StartMove {
    game: Game,
    turn: i32,
    board: Board,
    you: Snake
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartResponse {
    color: String,
    head_type: SnakeHead,
    tail_type: SnakeTail
}


#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveResponse {
    Move: Moves
}

fn handle_start(req: &HttpRequest) -> Box<Future<Item = HttpResponse, Error = Error>> {
    req.json()
        .from_err()
        .and_then(|inital_state: StartMove| {
            println!("Game Start: {:?}", inital_state);
            Ok(HttpResponse::Ok().json(StartResponse { color: String::from(SNAKE_COLOR), head_type: SnakeHead::Safe, tail_type: SnakeTail::Hook}))
        })
        .responder()
}

fn handle_move(req: &HttpRequest) -> Box<Future<Item = HttpResponse, Error = Error>> {
    req.json()
        .from_err()
        .and_then(|state: StartMove| {
            println!("model: {:?}", state);
            Ok(HttpResponse::Ok().json(MoveResponse { Move: Moves::Right }))
        })
        .responder()
}

fn main() {
    ::std::env::set_var("RUST_LOG", "bs-log=info");
    env_logger::init();
    let sys = actix::System::new("BattleSnake 2019");

    server::new(|| {
        App::new()
            .middleware(middleware::Logger::default())
            .route("/ping", http::Method::POST, |_: HttpRequest| HttpResponse::Ok())
            .route("/end", http::Method::POST, |_: HttpRequest| HttpResponse::Ok())
            .resource("/start", |r| r.method(http::Method::POST).f(handle_start))
            .resource("/move", |r| r.method(http::Method::POST).f(handle_move))
    }).bind(format!("{}:{}", IP, PORT))
        .unwrap()
        .shutdown_timeout(1)
        .start();

    println!("Started http server: {}:{}", IP, PORT);
    let _ = sys.run();

}
