// #![allow(unused)]


use rocket::response::status::NotFound;
use tokio::sync::oneshot;

use crate::GameState;

use super::game::Game;
use std::{collections::HashMap};


pub struct MatchMaker{

    running_games: HashMap<i32,Game>,
    open_game_id : i32 ,
    open_game : Option< Game>,

}

impl MatchMaker {
    pub fn new()->Self{
        MatchMaker { 
            running_games:HashMap::new(),
            open_game_id:0,
            open_game:None,
        }
    }

    
    pub fn get_game(&mut self, player_id : i32) -> (i32,i32){
        

        let num ;

        match &mut self.open_game{
            Some(session)=>{

                if session.is_open(){
                    num = session.add_player(player_id);

                }else {

                    let mut new_game = Game::new();
                    num = new_game.add_player(player_id);

                    let old_game = std::mem::replace(&mut self.open_game,Some(new_game));

                    if let Some(game) = old_game{
                        self.running_games.insert(self.open_game_id,game);
                    }
                    self.open_game_id +=1;

                }
            }
            None=>{
                let mut open_game = Game::new();
                num = open_game.add_player(player_id);
                self.open_game = Some(open_game);
                self.open_game_id += 1;
            }
        }
        
        (self.open_game_id,num)

    }

    

    pub fn make_move(&mut self, _game_id: i32, player_id:i32 ,start:i32, end:i32,spawn:i32)->bool{


        match &mut self.open_game{
            Some( game)=>{

                game.make_move(player_id, start, end, spawn)
            }
            _=>{
                false
            }
        }
    }

    pub fn get_board_state(&mut self,_game_id: i32, player_id:i32)-> Result<Result<Option<GameState>, oneshot::Receiver<GameState>>,NotFound<String>>{
        match &mut self.open_game{
            Some(game)=>{

                // game.get_update(player_id)

                let req = game.request_update(player_id);

                Ok(req)

            }
            None=>{
                println!("no game found.");
                Err(NotFound("not a game.".to_string()))
            }
        }
    }
}
