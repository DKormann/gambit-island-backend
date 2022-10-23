// #![allow(unused)]


use rocket::response::status::NotFound;
use tokio::sync::oneshot;

use crate::{GameMessage, MoveResult};

use super::game::Game;
use std::{collections::HashMap};


pub struct MatchMaker{

    running_games: HashMap<i32,Game>,
    open_game_id : i32 ,
}

impl MatchMaker {

    pub fn new()->Self{
        let mut res = MatchMaker { 
            running_games:HashMap::new(),
            open_game_id:-1,
        };
        res.create_new_game();
        res
    }

    pub fn create_new_game(&mut self) -> &mut Game {
        self.open_game_id += 1;
        let game = Game::new(self.open_game_id);
        self.running_games.insert(game.id, game);

        self.running_games.get_mut(&self.open_game_id).expect("cant find game that was just added (prob impossible)")
    }

    pub fn get_game(&mut self, username : String, score: i32) -> Result<GameMessage,String>{

        let g = self.running_games.get_mut(&self.open_game_id);

        if let Some(game) = g{
            if game.is_open(){
                return game.add_player(username,score)
            }
        }

        self.create_new_game();

        self.running_games.get_mut(&self.open_game_id).expect("cant get created game").add_player(username,score)

    }

    pub fn leave_lobby(&mut self, game_id: i32, token: u32) -> Result<(),NotFound<String>>{

        match self.running_games.get_mut(&game_id){
            Some (game)=>{
                if game.is_open(){
                    return game.remove_player_from_lobby(&token)
                }else{
                    return Err(NotFound("cant leave lobby, game allready started".to_string()))
                }
            }
            None=>{
                return Err(NotFound("cant find this game".to_string()))
            }
        }
    }

    pub fn leave_ongoing_game(&mut self, game_id: i32, token: u32)->Result<GameMessage,NotFound<String>>{
        match self.running_games.get_mut(&game_id){
            Some(game)=>{
                return game.remove_player_from_game(&token)
            }
            None=>{
                return Err(NotFound("cant find game".to_string()))
            }
        }
    }

    pub fn start_game(&mut self, _game_id: i32, token: u32) -> Result<(),String>{

        match self.running_games.get_mut(&_game_id){
            Some(game)=>{
                return game.start(token)
            }
            None=>{
                return Err("cant get game".to_owned())
            }
        }
        
    }   

    pub fn make_move(&mut self, game_id: i32, token:u32 ,start:i32, end:i32,spawn:i32)->MoveResult{

        let game = self.running_games.get_mut(&game_id);

        match game{
            Some( game)=>{
                game.make_move(token, start, end, spawn)
            }
            _=>{
                MoveResult::Fail
            }
        }
    }

    pub fn take_game(&mut self, game_id:i32)->Option<Game>{
        let g = self.running_games.remove(&game_id);
        g
    }

    pub fn get_board_state(&mut self,_game_id: i32, player_token:u32)-> Result<Result<Option<GameMessage>, oneshot::Receiver<GameMessage>>,NotFound<String>>{

        match self.running_games.get_mut(&_game_id){
            Some(game)=>{
                Ok(game.request_update(player_token))
            }
            None=>{
                Err(NotFound("cant get game".to_owned()))
            }
        }
    }
}
