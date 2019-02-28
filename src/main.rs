#![deny(warnings)]

use actix_web::{
    http, middleware, server, App, AsyncResponder, Error, HttpMessage, HttpRequest, HttpResponse,
};

use pathfinding::prelude::astar;

use env_logger;
use futures::Future;
use serde_derive::{Deserialize, Serialize};

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

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
struct Game {
    id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
enum Moves {
    Up,
    Down,
    Left,
    Right,
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

fn turn_step1(snakes_moves: Vec<Moves>, game_state: &mut GameState) {
    for (i, snake) in game_state.board.snakes.iter_mut().enumerate() {
        match snakes_moves[i] {
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
}

fn turn_step2(game_state: &mut GameState) {
    for snake in game_state.board.snakes.iter_mut() {
        snake.health -= 1;
    }
}

fn turn_step3(game_state: &mut GameState) {
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

fn turn_step4(game_state: &mut GameState) {
    for snake in game_state.board.snakes.iter_mut() {
        snake.body.pop();
    }
}

fn turn_step5(game_state: &mut GameState) {
    for snake in game_state.board.snakes.iter_mut() {
        if game_state.turn > 1 && snake.health == 100 {
            snake.body.push(snake.body.last().cloned().unwrap())
        }
    }
}

fn turn_step6(game_state: &mut GameState) {
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

fn apply_moves(snakes_moves: Vec<Moves>, game_state: GameState) -> GameState {
    let mut new_state: GameState = game_state;
    turn_step1(snakes_moves, &mut new_state);
    turn_step2(&mut new_state);
    turn_step3(&mut new_state);
    turn_step4(&mut new_state);
    turn_step5(&mut new_state);
    turn_step6(&mut new_state);
    new_state.fix_board_to_self();
    new_state
}

fn which_move(state1: GameState, state2: &GameState) -> Moves {
    let head1 = &state1.you.body[0];
    let head2 = &state2.you.body[0];
    if head1.x < head2.x {
        Moves::Right
    } else if head1.x > head2.x {
        Moves::Left
    } else if head1.y < head2.y {
        Moves::Down
    } else {
        Moves::Up
    }
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
    fn len(&self) -> usize {
        self.body.len()
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
        let num_snakes = self.board.snakes.len() as i32;
        let move_list = permutations(num_snakes, None);
        move_list
            .into_iter()
            .map(|snakes_moves| (apply_moves(snakes_moves, self.clone()), 1))
            .collect()
    }
    #[allow(dead_code)]
    fn success(&self) -> bool {
        self.board.snakes.len() == 1 && self.board.snakes[0] == self.you
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

fn handle_start(req: &HttpRequest) -> Box<Future<Item = HttpResponse, Error = Error>> {
    println!("handle_start\n");
    req.json()
        .from_err()
        .and_then(|inital_state: GameState| {
            println!("Game Start: {:?}", inital_state);
            Ok(HttpResponse::Ok().json(StartResponse {
                color: String::from(SNAKE_COLOR),
                head_type: SnakeHead::Safe,
                tail_type: SnakeTail::Hook,
            }))
        })
        .responder()
}

fn handle_move(req: &HttpRequest) -> Box<Future<Item = HttpResponse, Error = Error>> {
    println!("handle_move\n\n");
    req.json()
        .from_err()
        .and_then(|state: GameState| {
            let path_to_success = astar(&state, |p| p.successors(), |_| 1, |p| p.success());
            if let Some((path, _)) = path_to_success {
                println!("{:?}", which_move(state.clone(), &path[0]));
                return Ok(HttpResponse::Ok().json(MoveResponse {
                    Move: which_move(state, &path[0]),
                }));
            }
            println!("None path, something went wrong");

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
