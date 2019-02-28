#![deny(warnings)]

use actix_web::{
    http, middleware, server, App, AsyncResponder, Error, HttpMessage, HttpRequest, HttpResponse,
};

use pathfinding::prelude::astar;

use env_logger;
use futures::Future;
use rand::Rng;
use serde_derive::{Deserialize, Serialize};

#[macro_use]
use lazy_static::lazy_static;

//TODO: use clap or something to make a nicer interface for this
static IP: &str = "127.0.0.1";
static PORT: &str = "8008";
//static SNAKE_COLOR: &str = "#54A4E5";
//static SNAKE_HEAD: SnakeHead = SnakeHead::Beluga;
//static SNAKE_TAIL: SnakeTail = SnakeTail::Bolt;

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

lazy_static! {
    static ref PERMUTATIONS: Vec<Vec<Vec<Moves>>> = vec![
        permutations(1, None),
        permutations(2, None),
        permutations(3, None),
        permutations(4, None),
        permutations(5, None),
        permutations(6, None),
        permutations(7, None),
        permutations(8, None)
    ];
}

fn permutations(num_snakes: i32, possible_actions: Option<Vec<Vec<Moves>>>) -> Vec<Vec<Moves>> {
    let all_moves = vec![Moves::Up, Moves::Down, Moves::Left, Moves::Right];
    let mut next_actions: Vec<Vec<Moves>> = vec![];
    if let Some(actions) = possible_actions {
        for action in actions {
            for next_move in &all_moves {
                let mut new_action = action.clone();
                new_action.push(next_move.clone());
                next_actions.push(new_action);
            }
        }
    } else {
        next_actions.extend(
            vec![
                vec![Moves::Up],
                vec![Moves::Down],
                vec![Moves::Left],
                vec![Moves::Right],
            ]
            .iter()
            .cloned(),
        );
    }
    if num_snakes == 1 {
        next_actions
    } else {
        permutations(num_snakes - 1, Some(next_actions.clone()))
    }
}

fn step_move_head(snakes_moves: Vec<Moves>, game_state: &mut GameState) -> bool {
    let mut flag: bool = false;
    for (snake, snake_move) in game_state.board.snakes.iter_mut().zip(snakes_moves.into_iter()) {
        let last_move = snake.last_move();
        if last_move == Moves::Down && snake_move == Moves::Up
            || last_move == Moves::Up && snake_move == Moves::Down
            || last_move == Moves::Left && snake_move == Moves::Right
            || last_move == Moves::Right && snake_move == Moves::Left {
                flag = true;
            }
        match snake_move {
            Moves::Up => snake.body.insert(
                0,
                Coord {
                    x: snake.body[0].x,
                    y: snake.body[0].y - 1,
                },
            ),
            Moves::Down => snake.body.insert(
                0,
                Coord {
                    x: snake.body[0].x,
                    y: snake.body[0].y + 1,
                },
            ),
            Moves::Left => snake.body.insert(
                0,
                Coord {
                    x: snake.body[0].x - 1,
                    y: snake.body[0].y,
                },
            ),
            Moves::Right => snake.body.insert(
                0,
                Coord {
                    x: snake.body[0].x + 1,
                    y: snake.body[0].y,
                },
            ),
        }
    }
    flag
}

fn step_reduce_health(game_state: &mut GameState) {
    for snake in game_state.board.snakes.iter_mut() {
        snake.health -= 1;
    }
}

fn step_check_ate_food(game_state: &mut GameState) {
    let mut delete_map: Vec<usize> = vec![];
    for snake in game_state.board.snakes.iter_mut() {
        for (i, food) in game_state.board.food.iter().enumerate() {
            if snake.body[0] == *food {
                snake.health = 100;
                delete_map.push(i);
            }
        }
    }

    for i in delete_map.into_iter().rev() {
        game_state.board.food.remove(i);
    }
}

fn step_remove_tail(game_state: &mut GameState) {
    for snake in game_state.board.snakes.iter_mut() {
        snake.body.pop();
    }
}

fn step_add_body(game_state: &mut GameState) {
    for snake in game_state.board.snakes.iter_mut() {
        if game_state.turn > 1 && snake.health == 100 {
            snake.body.push(snake.body.last().cloned().unwrap())
        }
    }
}

fn step_check_for_death(game_state: &mut GameState) {
    let mut delete_map: Vec<usize> = vec![];
    let snake_list = game_state.board.snakes.clone();
    for (i, snake) in game_state.board.snakes.iter_mut().enumerate() {
        let head = &snake.body[0];
        let size = &snake.len();
        let collided = snake_list.clone().into_iter().any(|other_snake| {
            head.eq(&other_snake.body[0]) && other_snake.len() > *size
                || other_snake
                    .body
                    .into_iter()
                    .skip(1)
                    .any(|segment| head.eq(&segment))
        });
        if head.x >= game_state.board.width
            || head.x < 0
            || head.y >= game_state.board.height
            || head.y < 0
            || snake.health == 0
            || collided
        {
            delete_map.push(i);
        }
    }

    for i in delete_map.into_iter().rev() {
        game_state.board.snakes.remove(i);
    }
}

fn apply_moves(snakes_moves: Vec<Moves>, game_state: GameState) -> Option<GameState> {
    let mut new_state: GameState = game_state;
    if step_move_head(snakes_moves, &mut new_state) {
        return None;
    }
    step_reduce_health(&mut new_state);
    step_check_ate_food(&mut new_state);
    step_remove_tail(&mut new_state);
    step_add_body(&mut new_state);
    step_check_for_death(&mut new_state);
    new_state.fix_board_to_self();
    Some(new_state)
}

fn move_cost(_state1: GameState, state2: &Option<GameState>) -> Option<(GameState, u32)> {
    if let Some(state2) = state2 {
        if state2.board.snakes.iter().any(|s| state2.you.id == s.id) {
            return Some((state2.clone(), 1));
        }
    }
    None
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct Coord {
    x: i32,
    y: i32,
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
    fn len(self) -> usize {
        self.body.len()
    }
    fn last_move(self) -> Moves {
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
    fn successors(&self) -> Vec<(GameState, u32)> {
        let num_snakes = self.board.snakes.len();
        let move_list = &PERMUTATIONS[num_snakes]; //permutations(num_snakes, None);
        move_list
            .into_iter()
            .filter_map(|snakes_moves| move_cost(self.clone(), &apply_moves(snakes_moves, self.clone())))
            .collect()
    }
    #[allow(dead_code)]
    fn success(&self) -> bool {
        self.board.snakes.len() == 1
            && self.board.snakes[0] == self.you
            && self.you.len() as i32 == self.board.width * self.board.height
    }
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

fn random_color() -> String {
    let mut rng = rand::thread_rng();
    format!("#{:X}", rng.gen_range(0,16_581_375))
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
        .and_then(|inital_state: GameState| {
            println!("Game Start: {:?}", inital_state);

            Ok(HttpResponse::Ok().json(StartResponse {
                color: random_color(),
                head_type: random_head(),
                tail_type: random_tail(),
            }))
        })
        .responder()
}

fn handle_move(req: &HttpRequest) -> Box<Future<Item = HttpResponse, Error = Error>> {
    req.json()
        .from_err()
        .and_then(|state: GameState| {
            let mut turns_evaluated = 0;

            let path_to_success = astar(
                &state,
                |p| p.successors(),
                |_| 1,
                |p| {
                    turns_evaluated += 1;
                    turns_evaluated > 2188 || p.success()
                },
            );
            if let Some((path, _)) = path_to_success {
                println!("{:?}", which_move(state.clone(), &path[1]));
                return Ok(HttpResponse::Ok().json(MoveResponse {
                    Move: which_move(state, &path[1]),
                }));
            }
            println!("None path, something went wrong");

            Ok(HttpResponse::Ok().json(MoveResponse { Move: Moves::Right }))
        })
        .responder()
}

fn main() {
    ::std::env::set_var("RUST_LOG", "battlesnake=info");
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
