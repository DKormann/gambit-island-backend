// #![allow(unused)]

use std::{collections::HashMap};
use std::time;
use tokio::sync::oneshot;
use rand::Rng;
use crate::{GameMessage, MoveResult, database};

const BOARD_SIZE : i32 = 15;
const TREASURE_TILE : i32 = BOARD_SIZE*7+7;
const LAST_TILE : i32 = BOARD_SIZE * BOARD_SIZE;
const VIEW_SIZE : i32 = 9 ;
const VIEW_RADIUS: i32 =  VIEW_SIZE/2;
const ENERGY_REGEN:f32 = 0.2;
const DIAGONALS :[(i32, i32);4] = [(1,1),(-1,1),(1,-1),(-1,-1)];
const STRAIGHTS : [(i32,i32);4] = [(1,0),(-1,0),(0,1),(0,-1)];
const KNIGHTHOPS : [(i32,i32);8] = [(1,2),(2,1),(1,-2),(2,-1),(-1,-2),(-2,-1),(-1,2),(-2,1)];
const MINPLAYERCOUNT : i32 = 2;
const MAXPLAYERCOUNT : i32 = 6;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pos{
    x:i32,
    y:i32,
    n:i32,
}

struct PosError;

impl Pos{


    fn from_ints(x:i32, y:i32)-> Pos{
        Pos{
            x,
            y,
            n: (x + y * BOARD_SIZE) as i32,
        }
    }
    fn step(&self,x:i32,y:i32)->Result<Pos,PosError> {
        let new_x = self.x as i32 + x;
        let new_y = self.y as i32 + y;
        if new_x< 0 || new_x  >= BOARD_SIZE{
            Err(PosError)
        }else if new_y<0 || new_y  >= BOARD_SIZE{
            Err(PosError)
        }else{
            Ok(
                Pos::from_ints(new_x as i32, new_y as i32)
            )
        }
    }
}

#[derive(Eq, PartialEq,Clone, Debug)]
struct PlayerName (String);

#[derive(Clone, Copy, Debug, PartialEq)]
struct PlayerNumber(i32);

// #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
type PlayerToken = u32;


// #[derive(Clone, Copy)]
pub struct Player{
    id: PlayerName,
    number: PlayerNumber,
    token: PlayerToken,
    score: i32,
    king_pos:Pos,
    energy: f32,
    got_treasure : bool,
    last_move_time: time::Instant,
    update_sender : Option<oneshot::Sender<GameMessage>>,
    view_changed : bool,
}

impl Player{
    
    fn relative_to_absolute(&self,num:i32)->Pos{

        let relative_point = ((num)/9,(num)%9);

        let king_pos = self.king_pos;

        Pos::from_ints(
            relative_point.0+king_pos.x - VIEW_RADIUS , 
            relative_point.1+king_pos.y - VIEW_RADIUS ,
        )
    }

    fn can_see(&self,pos:Pos)->bool{

        let dx = (self.king_pos.x - pos.x).abs();
        let dy = (self.king_pos.y - pos.y).abs();

        let res = dx <= VIEW_RADIUS && dy <= VIEW_RADIUS;
        if ! res{
            println!("cant see that far")
        }
        res
    }

    fn send_update(&mut self,msg:GameMessage){
        match std::mem::replace(&mut self.update_sender, None){
            Some( sender)=>{

                _ = sender.send(msg);
            }
            None=>{
                self.view_changed = true;
            }
        }
    }

}

#[derive(Clone, Copy, Debug,PartialEq)]
enum Piece{
    King,
    Rook,
    Knight,
    Bishop,
    Queen,
    Pawn,
}

impl Piece{
    fn from_num(num:i32)->Piece{
        match num{
            1=>Piece::King,
            2=>Piece::Queen,
            3=>Piece::Bishop,
            4=>Piece::Knight,
            5=>Piece::Rook,
            6=>Piece::Pawn,
            _=>{panic!{"cant convert {} to piece",num}},
        }
    }
    fn to_num(&self)->usize{
        match self{
            Piece::King=>1,
            Piece::Queen=>2,
            Piece::Bishop=>3,
            Piece::Knight=>4,
            Piece::Rook=>5,
            Piece::Pawn=>6,
        }
    }
    fn get_cost(&self)->u32{
        match self{
            Piece::King=>{panic!("cant take cost of king")},
            Piece::Queen=>9,
            Piece::Bishop=>3,
            Piece::Knight=>3,
            Piece::Rook=>5,
            Piece::Pawn=>1,
        }
    }
}

#[derive(Clone, Copy,Debug,PartialEq)]
enum Tile{
    Empty,
    Taken( PlayerNumber,Piece),
    Treasure,
}

impl Tile{
}

pub struct Game{

    pub id : i32,
    players : HashMap<PlayerToken, Player>,
    board : [Tile; (BOARD_SIZE * BOARD_SIZE) as usize],
    running : bool,
    value : i32,
    conclusion : Option<GameMessage>,
    treasure_holder: Option<PlayerNumber>,

}

impl Game{

    pub fn new (id:i32)->Game{
        let mut res = Game {
            id,
            players: HashMap::new(),
            board: [Tile::Empty;(BOARD_SIZE * BOARD_SIZE)as usize],
            running: false,
            value: 0,
            conclusion:None,
            treasure_holder:None,

        };
        res.board[TREASURE_TILE as usize] = Tile::Treasure;
        res
    }

    pub fn add_player(&mut self,player_id:String,score:i32)->Result<GameMessage,String>{

        if self.running{
            panic!("cannot add player to running game")
        }

        let id = PlayerName(player_id);

        let mut rng = rand::thread_rng();

        let num = self.players.len() as i32;
        let token = rng.gen::<u32>();

        let start_position = Pos::from_ints( 2,(num*2+2)%BOARD_SIZE);


        self.board[start_position.n as usize] = Tile::Taken(PlayerNumber(num), Piece::King);

        println!("adding king on field {}",start_position.n);


        let new_player = Player { 
            id: id.clone(),
            number: PlayerNumber(self.players.len() as i32),
            token,
            score,
            king_pos:start_position,
            energy: 3.,
            got_treasure: false,
            last_move_time: time::Instant::now(),
            update_sender:None,
            view_changed:true,
        };

        self.players.insert(token,new_player);
        
        // let t  = time::Instant::now();
        let lobby = self.get_lobby_info();
        self.broadcast(lobby);

        Ok(GameMessage::Join { game_id: self.id, number: num, token:token })

    }

    pub fn start(&mut self, token: u32)->Result<(),String>{

        if ! self.players.contains_key(&token){

            return Err("cant find player token to start game".to_string())
        }

        if (self.players.len() as i32) < MINPLAYERCOUNT{
            println!("not enough players to start");
            return Err("not enough players to start game".to_string())
        }

        if self.running{
            return Ok(())
        }

        self.running = true;

        let mut tokens : Vec<_> = vec![];

        let start_time = time::Instant::now();

        for player in self.players.values_mut(){
            tokens.push(player.token);
            self.value += player.score/10;
            player.last_move_time = start_time;
            
        }
        for tok in tokens{
            self.update_player_view(&tok);
        }
        Ok(())
    }

    pub async fn end(&mut self, secret:&String, winner: i32){

        for (_,p) in self.players.iter_mut(){


            let fee = p.score/10;
            let target_value;

            if p.number.0 == winner{
                target_value = p.score + self.value - fee;
            }else{
                target_value = p.score - fee;
            }


            let fut = database::set_player_score(&p.id.0,target_value,secret);
            if let Err(e) = fut.await{
                println!("error setting scores {}",e);
            }
        }
    }

    pub fn get_lobby_info(&self)-> GameMessage{

        let mut player_list = vec![];

        for player in self.players.values(){
            player_list.push((player.id.0.clone(),player.number.0))
        }

        GameMessage::Lobby { players: player_list }
        
    }

    pub fn is_open(&self)->bool{
        // self.players.len() < 10
        (! self.running ) && self.players.len() as i32 <= MAXPLAYERCOUNT
    }

    pub fn make_move(&mut self,token: u32,start:i32,end:i32,spawn:i32)->MoveResult{   

        println!("trying to make move {} {} {} ",start,end,spawn);

        if start == end || start <0 || end < 0{ 
            return MoveResult::Fail
        }

        let mut energy;
        
        let token = token;
        let player_num:PlayerNumber;

        let start_pos:Pos;
        let end_pos:Pos;

        if let Some(player) = self.players.get_mut(&token){

            start_pos = player.relative_to_absolute(start);
            end_pos = player.relative_to_absolute(end);

            if start_pos.n < 0 || end_pos.n < 0 || start_pos.n >= LAST_TILE || end_pos.n >= LAST_TILE{
                return MoveResult::Fail
            }

            player.view_changed = true;
            player_num = player.number;
            energy = player.energy;

            let time_diff = player.last_move_time.elapsed().as_secs_f32();
            player.last_move_time = time::Instant::now();


            energy += time_diff * ENERGY_REGEN;
            energy = f32::min(energy, 10.);


            println!("real move {:?} {:?} ", start_pos, end_pos);

        }else{
            println!("failed to get player for {:?}",&token);
            return MoveResult::Fail
        }

        if energy < 1. {
            println!("no energy");
            self.players.entry(token).and_modify(|p|{
                p.energy = energy;
            });

            return MoveResult::Fail
        }
        
        
        let origin = self.board[start_pos.n as usize];
        let succ = match origin{

            Tile::Taken(num,piece)=>{

                if num != player_num{
                    println!("move not allowed {:?} {:?}",num, player_num);
                    false
                }else{

                    match piece{

                        Piece::King=>{
                            //king action

                            match spawn{
                                0=>{
                                    //try move the king
                                    println!("try tp move king");

                                    if let Tile::Taken(num, _) =  self.board[end_pos.n as usize] {
                                        if num == player_num {
                                            return MoveResult::Fail
                                        }else{
                                            println!("{:?} not playernum {:?}",num,player_num)
                                        }
                                    }else{
                                        println!("end empty")
                                    }

                                    energy -= 1.;

                                    let mut succ = false;

                                    for dir in [STRAIGHTS,DIAGONALS].concat(){

                                        if let Ok(target) = start_pos.step(dir.0, dir.1){
                                            if target.n == end_pos.n{
                                                //move the king


                                                if self.board[end_pos.n as usize] == Tile::Treasure{
                                                    self.take_treasure(&token)
                                                }

                                                if let Tile::Taken(pn, Piece::King) = self.board[end_pos.n as usize]{

                                                    if let Some(holder) = self.treasure_holder {
                                                        if holder == pn{
                                                            self.take_treasure(&token)
                                                        }
                                                    }
                                                };

                                                self.players.entry(token.clone()).and_modify(|player| {
                                                    player.king_pos = target;
                                                });

                                                if end_pos.x == 0{
                                                    if self.players.get(&token).expect("lost player").got_treasure{
                                                        let end_message = GameMessage::End { winning_number: num.0, value: self.value};
                                                        self.broadcast(end_message.clone());
                                                        self.conclusion = Some(end_message);
                                                        return MoveResult::End{winner:num.0};
                                                    }
                                                }

                                                self.board[end_pos.n as usize] = self.board[start_pos.n as usize];
                                                self.board[start_pos.n as usize] = Tile::Empty;


                                                succ = true;
                                                break
                                            }
                                        }
                                    }
                                    succ
                                }
                                _=> self.spawn_piece(&mut energy, spawn, end_pos, start_pos, token.clone(), player_num)
                            }
                        }

                        _=>{

                            
                            if self.piece_move(start_pos, end_pos, &token, piece){
                                energy -= 1.;
                                
                                true
                            }else{
                                false
                            }
                        }
                    }
                }

            }

            Tile::Empty | Tile::Treasure=>{
                println!{"error empty origin"}
                false
            }

        };
        self.players.entry(token).and_modify(|p|{
            p.energy = energy;
        });

        if succ{
            self.update_views(start_pos, end_pos);
            MoveResult::Success
        }else{
            MoveResult::Fail
        }
    
    }

    fn spawn_piece(&mut self, energy: &mut f32, spawn: i32, end_pos: Pos, start_pos: Pos, token: PlayerToken, player_num: PlayerNumber) -> bool {
        let piece = Piece::from_num(spawn);
        println!("spaning {:?}",piece);
        
        let cost = piece.get_cost() as f32;

        if *energy >= cost{
            *energy -= cost
        }else{
            println!("not enough energy");
            return false
        };

        

        if self.board[end_pos.n as usize] == Tile::Empty && self.move_is_possible(start_pos, end_pos, &token, piece){
    
            self.board[end_pos.n as usize] = Tile::Taken(player_num, piece);
            true
        }else{
            false
        }
    }

    fn piece_move(&mut self, start: Pos, end: Pos, token: &PlayerToken, piece: Piece) -> bool {
        if self.move_is_possible(start, end, token, piece){

            if self.board[end.n as usize] == Tile::Treasure 
            {
                self.take_treasure(token);
            }else if let Tile::Taken(pn,Piece::King) = self.board[end.n as usize]{
                if let Some(holder) = self.treasure_holder{
                    if holder == pn{
                        self.take_treasure(token)
                    }
                }
            }

            self.board[end.n as usize] = self.board[start.n as usize];
            self.board[start.n as usize] = Tile::Empty;

            return true
        }
        return false
    }

    fn take_treasure(&mut self, token: &u32) {
        self.players.entry(token.clone()).and_modify(|p|{
            p.got_treasure = true;
            self.treasure_holder = Some(p.number);
        });
    }

    pub fn request_update(&mut self, player_token:u32) -> Result<Option<GameMessage>,oneshot::Receiver<GameMessage>>{

        let token = player_token;

        match self.players.get_mut(&token){
            Some(mut player)=>{


                if player.view_changed{
                    player.view_changed = false;

                    if !self.running{
                        let mut playerlist  = vec![];
                        for (_,val) in  self.players.iter(){
                            playerlist.push((val.id.0.clone(), val.number.0));
                        } 
                        Ok(Some(
                            
                            GameMessage::Lobby{
                                players:playerlist
                            }
                        ))
                    }else{

                        let gs = self.get_current_view(&token);

                        Ok(gs)
                    }

                }else{
                    let (s,r) = oneshot::channel();

                    let otp = self.players.get_mut(&token);
                    if let Some(p) = otp{
                        p.update_sender = Some(s)
                    }

                    Err(r)
                }

            }
            None=>{
                //player not part of this game
                Ok(None)
            }
        }

    }

    fn broadcast(&mut self, msg: GameMessage){

        for player in self.players.values_mut(){
            player.send_update(msg.clone())
        }

    }

    fn update_views(&mut self,start:Pos, end: Pos){


        let mut tokens :Vec<PlayerToken> = vec![];
        
        for player in self.players.values(){
            if  ((player.king_pos.x - end.x).abs() <= VIEW_RADIUS &&
                (player.king_pos.y - end.y).abs() <= VIEW_RADIUS) ||
                ((player.king_pos.x - start.x).abs() <= VIEW_RADIUS &&
                (player.king_pos.y - start.y).abs() <= VIEW_RADIUS)
            {
                tokens.push(player.token.clone());
            }
        }
        for id in tokens{
            self.update_player_view(&id);
        }


    }

    fn update_player_view(&mut self, token: &PlayerToken ){

        // let owned_sender : Option<oneshot::Sender<GameState>>;

        if let Some(view) = self.get_current_view(token){
            self.players.entry(token.clone()).and_modify(|player|{

                player.send_update(view);

            });

        }
    }

    fn get_current_view(&mut self, token: &PlayerToken)->Option<GameMessage>{
        // let player = self.players.get_mut(&PlayerID(player));
        // let player = player.unwrap();

        if let Some(player) = self.players.get_mut(token){

            player.view_changed = false;

            let mut res = [(0,0);81];

            let margin = 4;

            let mut index = 0;
            for i in 0..9{
                for j in 0..9{

                    let x = i + player.king_pos.x as i32 -margin;
                    let y = j + player.king_pos.y as i32 -margin;

                    let nums;


                    if x <0 || y <0 || x >= BOARD_SIZE || y >= BOARD_SIZE{
                        nums = (-1,-1);
                    }else{


                        let n = x + y * BOARD_SIZE as i32;

                        let tile = self.board[n as usize];



                        nums = match tile{
                            Tile::Empty=>{(0,0)}
                            Tile::Treasure => {
                                (-2,-2)
                            }
                            Tile::Taken(player, piece)=>{
                                (
                                    player.0,
                                    piece.to_num() as i32

                                )
                            }
                        };
                    }

                    res[index] = nums;
                    index+=1;

                }
            }

            let energy = player.energy + player.last_move_time.elapsed().as_secs_f32() * ENERGY_REGEN;


            let holder = match self.treasure_holder{
                None=>{
                    -1
                }
                Some(num)=>{
                    num.0
                }
            };
            Some(GameMessage::State { 
                data: Vec::from(res), 
                offset: (player.king_pos.x,player.king_pos.y), 
                energy: energy, 
                got_treasure : player.got_treasure,
                treasure_holder : holder,
            })
        } else{
            None
        }  

    }

    fn move_is_possible(&self, start:Pos,end:Pos,token:&PlayerToken,piece:Piece)->bool{

        if let Some(player) = self.players.get(token){
            println!("move possible?");
            match piece{
                Piece::Knight=>{
                    for hop in KNIGHTHOPS.iter(){
                        let target = Pos::from_ints(start.x + hop.0, start.y + hop.1);
                        if pos_is_on_board(target) && player.can_see(target) && target == end{

                            match self.board[target.n as usize]{
                                Tile::Taken(pn,_)=>{
                                    if pn != player.number{
                                        return true
                                    }else{

                                        println!("field blocked");
                                        return false
                                    }
                                }
                                Tile::Empty | Tile::Treasure=>{
                                    return true
                                }
                            }
                        }
                    }
                    println!("no match ing knight mvoe");
                    false
                }
                Piece::Pawn=>{
                    for hop in STRAIGHTS.iter(){
                        let target = Pos::from_ints(start.x + hop.0, start.y + hop.1);
                        if pos_is_on_board(target) && player.can_see(target) && target == end{

                            if let Tile::Taken( _,_ ) = self.board[target.n as usize]{
                                return false
                            }else{
                                return true
                            }
                        }
                    }
                    for hop in DIAGONALS.iter(){
                        let target = Pos::from_ints(start.x + hop.0, start.y + hop.1);
                        if pos_is_on_board(target) && player.can_see(target) && target == end{

                            if let Tile::Taken( other,_ ) = self.board[target.n as usize]{
                                return other != player.number
                            }else{
                                return false
                            }
                        }
                    }
                    false

                }
                Piece::Bishop=>{

                    self.check_lines(start, end, player, DIAGONALS)
                }
                Piece::Queen=>{
                    self.check_lines(start, end ,player, DIAGONALS)||
                    self.check_lines(start, end, player, STRAIGHTS)
                }
                Piece::Rook=>{
                    self.check_lines(start, end, player, STRAIGHTS)
                }
                _=>{panic!()}   
            }

        }else {
            false
        }
    }

    fn check_lines(&self,start:Pos,end:Pos,player:&Player,dirs: [(i32,i32);4])-> bool{

        println!("looking for {:?}",end);

        for dir in dirs.iter(){

            let mut target = start;
            loop{
                target = Pos::from_ints(target.x + dir.0, target.y + dir.1);

                println!("{:?}",target);

                if pos_is_on_board(target) && player.can_see(target){

                    if target == end{
                        if let Tile::Taken(pn,_) = self.board[target.n as usize]{
                            println!("not possible cant hit own unit");
                            return pn != player.number
                        }else {
                            println!("move is possible");
                            return true
                        }
                    }
                    if let Tile::Taken(_,_) = self.board[target.n as usize]{

                        break
                    }
                }else{
                    println!("{:?}",target);
                    break
                }
            }
        }
        false

    }

}

fn pos_is_on_board(pos:Pos)->bool{
    let res = !(pos.x < 0 || pos.y < 0 || pos.x >= BOARD_SIZE || pos.y >= BOARD_SIZE);
    if ! res {
        println!("not on board")
    }
    res
}

