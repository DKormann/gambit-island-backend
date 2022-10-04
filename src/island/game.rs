#![allow(unused)]

use std::collections::HashMap;
use std::slice::Iter;
use std::time;

use rocket::response::status::NotFound;
use tokio::sync::oneshot;

use crate::GameState;


const BOARD_SIZE : i32 = 20;
const VIEW_SIZE : i32 = 9 ;
const VIEW_RADIUS: i32 =  VIEW_SIZE/2;

const DIAGONALS :[(i32, i32);4] = [(1,1),(-1,1),(1,-1),(-1,-1)];
const STRAIGHTS : [(i32,i32);4] = [(1,0),(-1,0),(0,1),(0,-1)];
const KNIGHTHOPS : [(i32,i32);8] = [(1,2),(2,1),(1,-2),(2,-1),(-1,-2),(-2,-1),(-1,2),(-2,1)];

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pos{
    x:i32,
    y:i32,
    n:i32,
}

struct PosError;

impl Pos{
    fn from_num(n:i32) -> Pos{

        Pos{x:n %BOARD_SIZE , y : n / BOARD_SIZE ,n: n}
    }
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

#[derive(Eq, Hash, PartialEq,Clone, Copy, Debug)]
struct PlayerID (i32);

#[derive(Clone, Copy, Debug, PartialEq)]
struct PlayerNumber(i32);

// #[derive(Clone, Copy)]
pub struct Player{
    id: PlayerID,
    number: PlayerNumber,
    king_pos:Pos,
    energy: f32,
    last_move_time: time::Instant,
    update_sender : Option<oneshot::Sender<GameState>>,
    view_changed : bool,
}

impl Player{
    
    fn relative_to_absolute(&self,num:i32)->Pos{

        let relative_point = ((num-1)/9,(num-1)%9+1);

        let king_pos = self.king_pos;

        Pos::from_ints(
            relative_point.0+king_pos.x-VIEW_RADIUS -1 , 
            relative_point.1+king_pos.y -VIEW_RADIUS -1,
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
}

impl Tile{
}

pub struct Game{

    players : HashMap<PlayerID, Player>,
    board : [Tile; (BOARD_SIZE * BOARD_SIZE) as usize],
    mover : Option<Player>,

}

impl Game{
    pub fn new ()->Game{
        let mut res = Game {
            players: HashMap::new(),
            board: [Tile::Empty;(BOARD_SIZE * BOARD_SIZE)as usize],
            mover: None,
        };
        res
    }

    pub fn add_player(&mut self,player_id:i32)->i32{


        let id = PlayerID(player_id);

        if self.players.contains_key(&id){
            return -1
        }


        let num = self.players.len() as i32;

        let start_position = Pos::from_ints( 2,(num*2+2)%BOARD_SIZE);


        self.board[start_position.n as usize] = Tile::Taken(PlayerNumber(num), Piece::King);
        self.update_views(start_position,start_position);


        println!("adding king on field {}",start_position.n);

        let new_player = Player { 
            id: PlayerID(player_id),
            number: PlayerNumber(self.players.len() as i32),
            king_pos:start_position,
            energy: 5.,
            last_move_time: time::Instant::now(),
            update_sender:None,
            view_changed:true,
        };

        self.players.insert(id,new_player);
        
        let t  = time::Instant::now();
        num
    }

    pub fn is_open(&self)->bool{
        // self.players.len() < 10
        true
    }

    pub fn get_player_by_id(&self,id:i32)->Option<&Player>{
        self.players.get(&PlayerID(id))
    }

    pub fn make_move(&mut self,player_id: i32,start:i32,end:i32,spawn:i32)->bool{   

        println!("trying to make move {} {} {} ",start,end,spawn);

        if start == end{
            return false
        }

        let mut energy=0.0;
        
        let player_id = PlayerID(player_id);
        let mut player_num:PlayerNumber = PlayerNumber(-1);

        let start_pos:Pos;
        let end_pos:Pos;

        if let Some(player) = self.players.get_mut(&player_id){

            player.view_changed = true;
            player_num = player.number;
            energy = player.energy;

            let time_diff = player.last_move_time.elapsed().as_secs_f32();
            player.last_move_time = time::Instant::now();


            energy += (time_diff * 0.2) ;

            energy = f32::min(energy, 10.);


            start_pos = player.relative_to_absolute(start);
            end_pos = player.relative_to_absolute(end);

        }else{
            println!("failed to get player for {:?}",player_id);
            return false
        }

        if energy < 1. {
            println!("no energy");
            self.players.entry(player_id).and_modify(|p|{
                p.energy = energy;
            });

            return false
        }
        
        

        let succ = match self.board[start_pos.n as usize]{

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

                                    energy -= 1.;

                                    let mut succ = false;

                                    for dir in [STRAIGHTS,DIAGONALS].concat(){

                                        if let Ok(target) = start_pos.step(dir.0, dir.1){
                                            if target.n == end_pos.n{
                                                //move the king

                                                self.board[end_pos.n as usize] = self.board[start_pos.n as usize];
                                                self.board[start_pos.n as usize] = Tile::Empty;

                                                self.players.entry(player_id).and_modify(|player| {
                                                    player.king_pos = target;
                                                });

                                                succ = true;
                                                break
                                            }
                                        }
                                    }
                                    succ
                                }
                                _=> self.spawn_piece(&mut energy, spawn, end_pos, start_pos, player_id, player_num)
                            }
                        }

                        _=>{

                            
                            if self.piece_move(start_pos, end_pos, &player_id, piece){
                                energy -= 1.;
                                true
                            }else{
                                false
                            }
                        }
                    }
                }

            }

            Tile::Empty=>{
                println!{"error empty origin"}
                false
            }

        };
        self.players.entry(player_id).and_modify(|p|{
            p.energy = energy;
        });

        if succ{
            self.update_views(start_pos, end_pos);
        }
        succ
    
    }

    fn spawn_piece(&mut self, energy: &mut f32, spawn: i32, end_pos: Pos, start_pos: Pos, player_id: PlayerID, player_num: PlayerNumber) -> bool {
        let piece = Piece::from_num(spawn);
        println!("spaning {:?}",piece);
        
        let cost = piece.get_cost() as f32;

        if *energy >= cost{
            *energy -= cost
        }else{
            println!("not enough energy");
            return false
        };

        

        if self.board[end_pos.n as usize] == Tile::Empty && self.move_is_possible(start_pos, end_pos, &player_id, piece){
    
            self.board[end_pos.n as usize] = Tile::Taken(player_num, piece);
            true
        }else{
            false
        }
    }

    fn piece_move(&mut self, start: Pos, end: Pos, player_id: &PlayerID, piece: Piece) -> bool {
        if self.move_is_possible(start, end, player_id, piece){

            self.board[end.n as usize] = self.board[start.n as usize];
            self.board[start.n as usize] = Tile::Empty;

            return true
        }
        return false
    }

    pub fn request_update(&mut self, player_id:i32) -> Result<Option<GameState>,oneshot::Receiver<GameState>>{

        let player_id = &PlayerID(player_id);

        match self.players.get(player_id){
            Some(player)=>{


                if player.view_changed{

                    let gs = self.get_current_view(player_id);

                    Ok(gs)

                }else{
                    let (s,r) = oneshot::channel();

                    let otp = self.players.get_mut(player_id);
                    if let Some(p) = otp{
                        p.update_sender = Some(s)
                    }

                    Err(r)
                }

            }
            None=>{
                Ok(None)
            }
        }

    }

    fn update_views(&mut self,start:Pos, end: Pos){

        let mut ids = vec![];

        for player in self.players.values(){
            if  ((player.king_pos.x - end.x).abs() <= VIEW_RADIUS &&
                (player.king_pos.y - end.y).abs() <= VIEW_RADIUS) ||
                ((player.king_pos.x - start.x).abs() <= VIEW_RADIUS &&
                (player.king_pos.y - start.y).abs() <= VIEW_RADIUS)
            {
                ids.push(player.id.clone());
            }
        }

        for id in ids{
            self.update_player_view(&id);
        }

    }

    fn update_player_view(&mut self, player_id: &PlayerID ){


        let owned_sender : Option<oneshot::Sender<GameState>>;

        if let Some(view) = self.get_current_view(player_id){


            self.players.entry(player_id.clone()).and_modify(|player|{
                if let Some(sender) = std::mem::replace(&mut player.update_sender, None){
                    sender.send(view);
                }else{
                    player.view_changed = true;
                }
            });

        }


        
    }

    fn get_current_view(&mut self, player_id: &PlayerID)->Option<GameState>{
        // let player = self.players.get_mut(&PlayerID(player));
        // let player = player.unwrap();

        if let Some(player) = self.players.get_mut(player_id){

        

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

        Some(GameState{
            data:Vec::from(res),
            offset:(player.king_pos.x,player.king_pos.y),
            energy : player.energy,
        })
        } else{
            None
        }  

    }

    fn move_is_possible(&self, start:Pos,end:Pos,player_id:&PlayerID,piece:Piece)->bool{

        if let Some(player) = self.players.get(player_id){
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
                                Tile::Empty=>{
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

