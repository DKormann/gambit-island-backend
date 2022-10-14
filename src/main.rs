

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

use island::handler:: MatchMaker;

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


    

    let bank:Bank = Arc::new( Mutex::new(MatchMaker::new()));

    let db:DB = Arc::new(database::get_api());

    let mm = MatchMaker::new();

    let _rocket = rocket::build()
    .mount("/", routes![

        make_move,
        register,
        login,
        join_game,
        get_update,
        start_game,
        ])
    .manage(bank)
    .manage(mm)
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
    
    let mut matchmaker = bank.lock().or_else(|e|{Err(format!("error getting matchmaker {}",e.to_string()))})?;
    let res = matchmaker.get_game(username);

    res.and_then(|gmsg|{Ok(Json(gmsg))})

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
        data: Vec<(i32,i32)>,
        offset: (i32,i32),
        energy: f32,
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

            println!("deliver update {:?}",msg);

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
    mm.start_game(game_id, player_token)
}

#[derive(Serialize)]
pub struct GameState{
    data : Vec<(i32,i32)>,
    offset :(i32,i32),
    energy : f32,
}


#[post("/api/make_move/<game_id>/<player_token>/<start>/<end>/<spawn>")]
async fn make_move(
    game_id:i32,
    player_token:u32,
    start:i32,
    end:i32,
    spawn:i32,
    bank:&State<Bank>)->Json<bool>{


    let res = match bank.lock(){
        Ok(mut mm)=>{

            mm.make_move(game_id, player_token, start, end, spawn);
            true
        }
        _=>{false}
    };
    Json(res)
}

