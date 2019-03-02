#![deny(warnings)]

use actix_web::{
    http, middleware, server, App, AsyncResponder, Error, HttpMessage, HttpRequest, HttpResponse,
};

use pathfinding::prelude::astar;

use env_logger;
use futures::Future;
use rand::Rng;
use serde_derive::{Deserialize, Serialize};

static moves: &[Moves] = &[Moves::Up, Moves::Down, Moves::Left, Moves::Right];
//TODO: use clap or something to make a nicer interface for this
static IP: &str = "0.0.0.0";
static PORT: &str = "8008";
//static SNAKE_COLOR: &str = "#51EBB0";
//static SNAKE_HEAD: SnakeHead = SnakeHead::Evil;
//static SNAKE_TAIL: SnakeTail = SnakeTail::SmallRattle;

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

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct Game {
    id: String,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
enum Moves {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct Coord {
    x: i32,
    y: i32,
}

impl Coord {
    fn neighboors(&self) -> Vec<Coord> {
        vec![
            Coord {
                x: self.x + 1,
                y: self.y,
            },
            Coord {
                x: self.x - 1,
                y: self.y,
            },
            Coord {
                x: self.x,
                y: self.y + 1,
            },
            Coord {
                x: self.x,
                y: self.y - 1,
            },
        ]
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct Snake {
    id: String,
    name: String,
    health: i32,
    body: Vec<Coord>,
}

#[allow(dead_code)]
impl Snake {
    fn len(&self) -> usize {
        self.body.len()
    }
    fn last_move(&self) -> Moves {
        if self.body[1].x < self.body[0].x {
            Moves::Right
        } else if self.body[1].x > self.body[0].x {
            Moves::Left
        } else if self.body[1].y < self.body[0].y {
            Moves::Down
        } else if self.body[1].y > self.body[0].y {
            Moves::Up
        } else if self.body[1].y == self.body[0].y && self.body[1].x == self.body[0].x {
            panic!();
        } else {
            unreachable!();
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct Board {
    height: i32,
    width: i32,
    food: Vec<Coord>,
    snakes: Vec<Snake>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct GameState {
    game: Game,
    turn: i32,
    board: Board,
    you: Snake,
}

impl GameState {
    fn fix_board_to_self(&mut self) {
        if let Some(snake_in_board) = self.board.snakes.iter().find(|s| s.id == self.you.id) {
            self.you = snake_in_board.clone()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartResponse {
    color: String,
    head_type: SnakeHead,
    tail_type: SnakeTail,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MoveResponse {
    Move: Moves,
}

fn random_color() -> String {
    let mut rng = rand::thread_rng();
    format!("#{:X}", rng.gen_range(0, 16_581_375))
}

fn random_head() -> SnakeHead {
    let mut rng = rand::thread_rng();
    match rng.gen_range(0, 12) {
        0 => SnakeHead::Beluga,
        1 => SnakeHead::Bendr,
        2 => SnakeHead::Dead,
        3 => SnakeHead::Evil,
        4 => SnakeHead::Fang,
        5 => SnakeHead::Pixel,
        6 => SnakeHead::Regular,
        7 => SnakeHead::Safe,
        8 => SnakeHead::SandWorm,
        9 => SnakeHead::Shades,
        10 => SnakeHead::Silly,
        11 => SnakeHead::Smile,
        _ => SnakeHead::Tongue,
    }
}

fn random_tail() -> SnakeTail {
    let mut rng = rand::thread_rng();
    match rng.gen_range(0, 11) {
        0 => SnakeTail::BlockBum,
        1 => SnakeTail::Bolt,
        2 => SnakeTail::Curled,
        3 => SnakeTail::FatRattle,
        4 => SnakeTail::Freckled,
        5 => SnakeTail::Hook,
        6 => SnakeTail::Pixel,
        7 => SnakeTail::Regular,
        8 => SnakeTail::RoundBum,
        9 => SnakeTail::Sharp,
        10 => SnakeTail::Skinny,
        _ => SnakeTail::SmallRattle,
    }
}

fn handle_start(req: &HttpRequest) -> Box<Future<Item = HttpResponse, Error = Error>> {
    req.json()
        .from_err()
        .and_then(|_inital_state: GameState| {
            Ok(HttpResponse::Ok().json(StartResponse {
                color: random_color(),    // SNAKE_COLOUR,
                head_type: random_head(), // SNAKE_HEAD,
                tail_type: random_tail(), // SNAKE_TAIL,
            }))
        })
        .responder()
}

fn handle_move(req: &HttpRequest) -> Box<Future<Item = HttpResponse, Error = Error>> {
    req.json()
        .from_err()
        .and_then(|state: GameState| {
            let should_eat = !state
                .board
                .snakes
                .iter()
                .any(|snake| snake.id != state.you.id && snake.len() >= state.you.len() - 1);
            if should_eat {
                let best_food_path = state.board.food.iter().filter_map(|food| {
                    astar(
                        food,
                        |coord| {
                            coord
                                .neighboors()
                                .iter()
                                .filter_map(|coord| {
                                    if coord.x < 0
                                        || coord.y < 0
                                        || coord.x >= state.board.width
                                        || coord.y >= state.board.height
                                    {
                                        None
                                    } else {
                                        for snake in state.board.snakes.iter() {
                                            for snake_body_piece in snake.body.iter() {
                                                if coord == snake_body_piece
                                                    && *coord != state.you.body[0]
                                                {
                                                    return None;
                                                }
                                            }
                                        }
                                        return Some((*coord, 1));
                                    }
                                })
                                .collect::<Vec<_>>()
                        },
                        |coord| {
                            (coord.x - state.you.body[0].x).abs()
                                + (coord.y - state.you.body[0].y).abs()
                        },
                        |coord| *coord == state.you.body[0],
                    )
                }).min_by_key(|path| path.1);

                if let Some((path, _)) = best_food_path {
                    let next_coord = &path[path.len() - 2];
                    let next_move: Moves = if next_coord.x > state.you.body[0].x {
                        Moves::Right
                    } else if next_coord.x < state.you.body[0].x {
                        Moves::Left
                    } else if next_coord.y > state.you.body[0].y {
                        Moves::Up
                    } else {
                        Moves::Down
                    };
                    return Ok(HttpResponse::Ok().json(MoveResponse {
                        Move: next_move
                    }));
                }
            }

            // let path_to_success = astar(
            //     &game_root,
            //     |p| match &p.children {
            //         Some(children) => children.iter().map(|child| (child.clone(), child.cost)).collect(),
            //         None => vec![]
            //     },
            //     |_| 1,
            //     |p| if num_snakes == 1 {
            //         p.game_state.turn == turn + desired_depth -1
            //         // p.game_state.you.len() == board_size as usize
            //     } else {
            //         p.game_state.board.snakes.len() == 1 && p.game_state.board.snakes[0].id == p.game_state.you.id
            //     }
            //     );


            Ok(HttpResponse::Ok().json(MoveResponse { Move: Moves::Right }))
        })
        .responder()
}

fn main() {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();
    let sys = actix::System::new("battlesnake");

    server::new(|| {
        App::new()
            .middleware(middleware::Logger::default())
            .route("/ping", http::Method::POST, |_: HttpRequest| {
                HttpResponse::Ok()
            })
            .route("/end", http::Method::POST, |_: HttpRequest| {
                HttpResponse::Ok()
            })
            .resource("/start", |r| r.method(http::Method::POST).f(handle_start))
            .resource("/move", |r| r.method(http::Method::POST).f(handle_move))
    })
    .bind(format!("{}:{}", IP, PORT))
    .unwrap()
    .shutdown_timeout(1)
    .start();

    println!("Started http server: {}:{}", IP, PORT);
    let _ = sys.run();
}
