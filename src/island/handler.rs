// #![allow(unused)]


use rocket::response::status::NotFound;
use tokio::sync::oneshot;

use crate::{GameMessage};

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

    pub fn get_game(&mut self, username : String) -> Result<GameMessage,String>{
        

        let msg ;

        match &mut self.open_game{
            Some(session)=>{

                if session.is_open(){
                    msg = session.add_player(username);

                }else {
                    
                    self.open_game_id +=1;
                    let mut new_game = Game::new(self.open_game_id);
                    msg = new_game.add_player(username);

                    let old_game = std::mem::replace(&mut self.open_game,Some(new_game));

                    if let Some(game) = old_game{
                        self.running_games.insert(game.id,game);
                    }

                }
            }
            None=>{
                let mut open_game = Game::new(self.open_game_id);
                msg = open_game.add_player(username);
                self.open_game = Some(open_game);
                self.open_game_id += 1;
            }
        }

        msg


    }

    pub fn start_game(&mut self, _game_id: i32, token: u32)->String{

        

        let game = self.open_game.as_mut().expect("expects open game to exist in order to start game");

        
        game.start(token)

    }   

    pub fn make_move(&mut self, _game_id: i32, token:u32 ,start:i32, end:i32,spawn:i32)->bool{


        match &mut self.open_game{
            Some( game)=>{

                game.make_move(token, start, end, spawn)
            }
            _=>{
                false
            }
        }
    }

    pub fn get_board_state(&mut self,_game_id: i32, player_token:u32)-> Result<Result<Option<GameMessage>, oneshot::Receiver<GameMessage>>,NotFound<String>>{
        match &mut self.open_game{
            Some(game)=>{

                // game.get_update(player_id)

                let req = game.request_update(player_token);

                Ok(req)

            }
            None=>{
                println!("no game found.");
                Err(NotFound("not a game.".to_string()))
            }
        }
    }
}
