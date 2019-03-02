#![deny(warnings)]

use actix_web::{
    http, middleware, server, App, AsyncResponder, Error, HttpMessage, HttpRequest, HttpResponse,
};

use pathfinding::prelude::astar;

use env_logger;
use futures::Future;
use rand::Rng;
use serde_derive::{Deserialize, Serialize};

// static MOVES: &[Moves] = &[Moves::Up, Moves::Down, Moves::Left, Moves::Right];
//TODO: use clap or something to make a nicer interface for this
static IP: &str = "0.0.0.0";
static PORT: &str = "80";
static SNAKE_COLOUR: &str = "#51EBB0";
static SNAKE_HEAD: SnakeHead = SnakeHead::Evil;
static SNAKE_TAIL: SnakeTail = SnakeTail::SmallRattle;

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    fn dist(&self, coord: &Coord) -> i32 {
        (self.x - coord.x).abs() + (self.y - coord.y).abs()
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
    #[allow(dead_code)]
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

#[allow(dead_code)]
fn random_color() -> String {
    let mut rng = rand::thread_rng();
    format!("#{:X}", rng.gen_range(0, 16_581_375))
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
                color: String::from(SNAKE_COLOUR), // SNAKE_COLOUR,
                head_type: SNAKE_HEAD.clone(),     // SNAKE_HEAD,
                tail_type: SNAKE_TAIL.clone(),     // SNAKE_TAIL,
            }))
        })
        .responder()
}

fn big_snake_cost(board_size: i32, our_size: usize, coord: &Coord, snakes: &[Snake]) -> i32 {
    let too_close = board_size / 3;
    for snake in snakes.iter() {
        if snake.len() >= our_size {
            let dist_to_head = coord.dist(&snake.body[0]);
            if dist_to_head <= too_close {
                return 20 * (too_close - dist_to_head);
            }
        }
    }
    1
}

fn handle_move(req: &HttpRequest) -> Box<Future<Item = HttpResponse, Error = Error>> {
    req.json()
        .from_err()
        .and_then(|state: GameState| {
            if state.board.snakes.len() == 1 && state.board.snakes[0].len() == 1 {
                return Ok(HttpResponse::Ok().json(MoveResponse { Move: Moves::Up }));
            }
            let mut state = state.clone();
            loop {
                let should_eat =
                    state.board.snakes.iter().any(|snake| {
                        snake.id != state.you.id && snake.len() >= state.you.len() - 1
                    }) || state.board.snakes.len() == 1;
                println!("should_eat: {}", should_eat);

                if should_eat {
                    let best_food_path = state
                        .board
                        .food
                        .iter()
                        .filter_map(|food| {
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
                                                Some((
                                                    coord.clone(),
                                                    big_snake_cost(
                                                        state.board.width,
                                                        state.you.len(),
                                                        coord,
                                                        &state.board.snakes,
                                                    ),
                                                ))
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                },
                                |coord| coord.dist(&state.you.body[0]),
                                |coord| *coord == state.you.body[0],
                            )
                        })
                        .min_by_key(|path| path.1);
                    if let Some((path, _)) = best_food_path {
                        let next_coord = &path[path.len() - 2];
                        let next_move: Moves = if next_coord.x > state.you.body[0].x {
                            Moves::Right
                        } else if next_coord.x < state.you.body[0].x {
                            Moves::Left
                        } else if next_coord.y > state.you.body[0].y {
                            Moves::Down
                        } else {
                            Moves::Up
                        };
                        return Ok(HttpResponse::Ok().json(MoveResponse { Move: next_move }));
                    }
                }
                let best_kill_path = state
                    .board
                    .snakes
                    .iter()
                    .filter_map(|enemy_snake| {
                        if enemy_snake.len() < state.you.len() {
                            astar(
                                &enemy_snake.body[0],
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
                                                Some((coord.clone(), 1))
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                },
                                |coord| coord.dist(&state.you.body[0]),
                                |coord| *coord == state.you.body[0],
                            )
                        } else {
                            None
                        }
                    })
                    .min_by_key(|path| path.1);

                if let Some((path, _)) = best_kill_path {
                    let next_coord = &path[path.len() - 2];
                    let next_move: Moves = if next_coord.x > state.you.body[0].x {
                        Moves::Right
                    } else if next_coord.x < state.you.body[0].x {
                        Moves::Left
                    } else if next_coord.y > state.you.body[0].y {
                        Moves::Down
                    } else {
                        Moves::Up
                    };
                    return Ok(HttpResponse::Ok().json(MoveResponse { Move: next_move }));
                }

                let chase_tail_path = state
                    .board
                    .snakes
                    .iter()
                    .filter_map(|snake| {
                        astar(
                            &snake.body[snake.len() - 1],
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
                                            Some((coord.clone(), 1))
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            },
                            |coord| coord.dist(&state.you.body[0]),
                            |coord| *coord == state.you.body[0],
                        )
                    })
                    .min_by_key(|path| path.1);

                if let Some((path, _)) = chase_tail_path {
                    let next_coord = &path[path.len() - 2];
                    let next_move: Moves = if next_coord.x > state.you.body[0].x {
                        Moves::Right
                    } else if next_coord.x < state.you.body[0].x {
                        Moves::Left
                    } else if next_coord.y > state.you.body[0].y {
                        Moves::Down
                    } else {
                        Moves::Up
                    };
                    return Ok(HttpResponse::Ok().json(MoveResponse { Move: next_move }));
                }
                for snake in state.board.snakes.iter_mut() {
                    snake.body.pop();
                }
                state.fix_board_to_self();
            }
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
