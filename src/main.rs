

#[macro_use] extern crate rocket;

use rocket::response::status::NotFound;
use rocket::{State, serde::json::Json};
use serde::{Serialize};

// use tokio::sync::oneshot;


use std::sync::{Arc, Mutex};
// use std::env;

use rocket::http::Header;
use rocket::{Request, Response};
use rocket::fairing::{Fairing, Info, Kind};

use island::handler::MatchMaker;
use island::game::Tile;

mod island;

mod database;


pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}


type Bank = Arc<Mutex<MatchMaker>>;
type DB = Arc<database::DBApi>;


#[rocket::main]
async fn main()->Result<(), rocket::Error>{


    let api = database::get_api();

    let bank:Bank = Arc::new( Mutex::new(MatchMaker::new()));

    let db:DB = Arc::new(api);



    let _rocket = rocket::build()
    .mount("/", routes![
        make_move,
        register,
        login,
        join_game,
        get_update,
        start_game,
        leave_game,
        leave_lobby,
        ])
    .manage(bank)
    .manage(db)
    .attach(CORS)
    

    .ignite().await?
    .launch().await;

    Ok(())
}


#[get("/api/register/<username>/<email>/<passhash>")]
async fn register (username: &str, email:&str, passhash: &str, db : &State<DB>) ->  Result<String,String> {

    println!("register new user {}",username);

    let s = &db.secret;

    let res = database::create_user(username, email, passhash, &s).await;

    match res{
        Ok(content)=>{
            println!("got db response {}",content);
            Ok(content)
        }
        Err(code)=>{
            println!("db responded with error {}",code);
            Err(code)
        }
    }
}

#[get("/api/login/<username>/<passhash>")]
async fn login (username: &str, passhash: &str, db:&State<DB>) -> Result<String,String>{
    println!("logging in {}",username);
    let s = &db.secret;

    let check = database::check_user_credentials(username, passhash, s).await;

    check.and_then(|succ|{Ok(succ.to_string())})
}


#[derive(Serialize)]
pub struct GameJoinMessage{
    game_id: i32,
    token: u32,
}

#[get ("/api/join_game/<username>/<passhash>")]
async fn join_game(username: String, passhash: String, api: &State<DB>, bank: &State<Bank>)-> Result<Json<GameMessage>,String>{

    println!("join game request from {} {}",username,passhash);

    //check user creds
    if ! database::check_user_credentials(&username, &passhash, &api.secret)
    .await.or_else(|e| {Err(e)})?{
        return Err("authentication failed.".to_string())
    }

    let score = database::get_player_score(&username, &api.secret).await.or_else(|e|{Err(format!("cant get score {:?}",e))})?;
    
    let mut matchmaker = bank.lock().or_else(|e|{Err(format!("error getting matchmaker {}",e.to_string()))})?;
    let res = matchmaker.get_game(username,score);

    res.and_then(|gmsg|{Ok(Json(gmsg))})

}

#[get ("/api/leave_lobby/<game_id>/<player_token>")]
async fn leave_lobby(game_id: i32, player_token:u32, bank: &State<Bank>) -> Result<(),NotFound<String>>{

    let mut mm = bank.lock().or_else(|_|{
        return Err(NotFound("cant get lock".to_string()))
    })?;

    mm.leave_lobby(game_id, player_token)?;


    Ok(())
}

#[get ("/api/leave_game/<game_id>/<player_token>")]
async fn leave_game(game_id:i32, player_token:u32,bank : &State<Bank>,api: &State<DB>)-> Result<Json<GameMessage>,NotFound<String>>{
    let res;
    {
        let mut mm = bank.lock().or_else(|_|{
            return Err(NotFound("cant get lock".to_string()))
        })?;

        res = mm.leave_ongoing_game(game_id, player_token)?;
    }

    match &res{
        GameMessage::Leave { name, value }=>{
            database::set_player_score(&name, *value, &api.secret).await?;
        }
        GameMessage::End { winning_number,value:_} =>{
            //end game
            end_game(bank, api, game_id, *winning_number).await;
            
        }
        msg=>{
            panic!("leave can only produce end or leave message not {:?}",msg);
        }
    }

    Ok(Json(res))
}

async fn end_game(bank: &State<Bank>,api:&State<DB>, game_id:i32,winning_number: i32){
    let g = match bank.lock(){
        Ok(mut mm)=>{
            mm.take_game(game_id)
        }
        _=>{None}
    };

    if let Some(mut game) = g{
        game.end(&api.secret,winning_number).await
    }
}

#[derive(Serialize,Clone,Debug)]
pub enum GameMessage{
    Join{
        game_id: i32,
        number: i32,
        token: u32,
    },
    Lobby{
        players:Vec<(String, i32)>,
    },
    State{
        data: Vec<Option<Tile>>,
        offset: (i32,i32),
        energy: f32,
        got_treasure : bool,
        treasure_holder : i32,
    },
    Leave{
        name: String,
        value: i32,
    },
    End{
        winning_number: i32,
        value: i32,
    }
}

#[get("/api/get_update/<game_id>/<player_token>")]
async fn get_update ( game_id:i32, player_token:u32,bank :&State<Bank>) -> Result<Json<GameMessage>, NotFound<String>>{

    let res ;

    {
        let mut mm = bank.lock().or_else(|_|{
            return Err(NotFound("cant get lock".to_string()))
        })?;

        res = mm.get_board_state(game_id, player_token).or_else(|_|{
            return Err (NotFound("cant get game update".to_string()))
        })?;
    }

    let listener = match res{
        Ok(Some(gmsg))=>{

            return Ok(Json(gmsg))
        },
        Ok(None)=>{
            return Err(NotFound("cant find player".to_string()))
        },
        Err(rec) =>{
            rec
        }
    };

    match listener.await{

        Ok(msg)=>{

            // println!("deliver update {:?}",msg);
            Ok(Json(msg))

        }
        Err(_)=>{
            Err(NotFound("cant find update".into()))
        }

    }

}

#[get("/api/start_game/<game_id>/<player_token>")]
async fn start_game(game_id:i32, player_token:u32,bank : &State<Bank>) -> String {
    println!("starting game {}",player_token);
    let mut mm = bank.lock().expect("cannot get bank lock");
    if let Err(msg) = mm.start_game(game_id, player_token){
        msg
    }else{
        "Ok".to_string()
    }
}

#[derive(Serialize)]
pub enum MoveResult{
    Fail,
    Success,
    End{
        winner: i32
    },
}

#[post("/api/make_move/<game_id>/<player_token>/<start>/<end>/<spawn>")]
async fn make_move(
    game_id:i32,
    player_token:u32,
    start:i32,
    end:i32,
    spawn:i32,
    api: &State<DB>,
    bank:&State<Bank>)->Json<MoveResult>{

    let res = match bank.lock(){
        Ok(mut mm)=>{

            mm.make_move(game_id, player_token, start, end, spawn)

        }
        _=>{
            MoveResult::Fail
        }
    };

    match res{
        MoveResult::End{winner: winning_number}=>{

            end_game(bank, api, game_id, winning_number).await;
        }
        _=>{}
    }

    Json(res)
}

