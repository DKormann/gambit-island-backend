#[allow (unused_imports)]

#[macro_use] extern crate rocket;

use rocket::response::status::NotFound;
use rocket::{State, serde::json::Json};
use serde::{Serialize};
use tokio::sync::oneshot;
// use serde::{Deserialize };
// use rocket::serde::json::Json;
use std::sync::{Arc, Mutex};


use rocket::http::Header;
use rocket::{Request, Response};
use rocket::fairing::{Fairing, Info, Kind};

use island::handler:: MatchMaker;

mod island;

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


#[rocket::main]
async fn main()->Result<(), rocket::Error>{

    let bank:Bank = Arc::new( Mutex::new(MatchMaker::new()));

    let mm = MatchMaker::new();

    let _rocket = rocket::build()
    .mount("/", routes![
        get_game,
        get_state,
        make_move,
        ])
    .manage(bank)
    .manage(mm)
    .attach(CORS)
    

    .ignite().await?
    .launch().await;

    Ok(())
}


#[get("/api/new_game/<player>")]
async fn get_game (player:i32, bank : &State<Bank>) -> Result<Json<(i32,i32)>,NotFound<String>>{

    match bank.lock(){

        Ok(mut mm)=>{
            let res=  mm.get_game(player);
            Ok(Json(res))
        }
        Err(_)=>{
            Err(NotFound("mutex error".to_string()))
        }
    }
}


#[derive(Serialize)]
pub struct GameState{
    data : Vec<(i32,i32)>,
    offset :(i32,i32),
    energy : f32,
}


// use rocket::tokio::time::{sleep,Duration};

#[get("/api/get_state/<game_id>/<player_id>")]
async fn get_state (game_id:i32, player_id:i32,bank: &State<Bank>) -> Result<Json<GameState>,NotFound<String>>{

    let listener : oneshot::Receiver<GameState>;
    match bank.lock(){
        Ok(mut mm)=>{
            
            let future = mm.get_board_state(game_id, player_id);
            match  future{
                Ok(req)=>{

                    match req{
                        Ok(Some(gs))=>{
                            return Ok(Json(gs))

                        },
                        Ok(None)=>{
                            return Err(NotFound("cant find player for this game".into()))
                        }
                        Err(o)=>{
                            listener = o;
                        }
                    }

                }
                Err(_)=>{
                    return Err(NotFound("cant get game update".to_string()));
                }
            }
        }
        Err(_)=>{
            println!("failed to get match maker");
            return  Err(NotFound("not found".to_string()));
        }
    }

    // sleep(Duration::from_secs(1)).await;

    match listener.await{
        Ok(gs)=>{
            Ok(Json(gs))
        }
        Err(_)=>{
            Err(NotFound("game transmission faied".into()))
        }
    }

}

#[post("/api/make_move/<game_id>/<player_id>/<start>/<end>/<spawn>")]
async fn make_move(game_id:i32,player_id:i32,start:i32,end:i32,spawn:i32,bank:&State<Bank>)->Json<bool>{
    let res = match bank.lock(){
        Ok(mut mm)=>{

            mm.make_move(game_id, player_id, start, end, spawn);
            true
        }
        _=>{false}
    };
    Json(res)

}
