use std::thread;
use std::sync::Mutex;
use rand::distributions::{Distribution, Uniform};
extern crate bincode;
use serde::{Serialize, Deserialize};


pub const GAME_AREA_WIDTH: f32 = 1000.0f32;
pub const GAME_AREA_HEIGHT: f32 = 1000.0f32;
pub const PLAYER_SIZE: f32 = 20.0f32;
pub const TICK_RATE: f32 = 1.0f32 / 30.0f32;
const SPAWN_PADDING: f32 = 10.0f32;

#[derive(Default,Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Player {
    pub id: u32,
    pub cur_sequence_id: u32,
    pub pos: [f32; 2],
    pub col: [f32; 4],
}

#[derive(Default,Serialize, Deserialize, Debug, Clone, Copy)]
pub struct PlayerInput {
    pub id: u32,
    pub cur_sequence_id: u32,
    pub up_down: f32,
    pub left_right: f32,
}


#[derive(Serialize, Deserialize, Debug)]
pub enum NetworkMessages
{
    InvalidMessage,
    AddLocal(Player),
    AddPlayer(Player),
    RemovePlayer{id: u32},
    ClientInputChange(PlayerInput),
    Position(Player),
}



impl Player 
{
    pub fn update(&mut self, input: &PlayerInput, dt: f32)
    {
        self.pos[0] += input.left_right * dt * 300.0f32;
        self.pos[1] += input.up_down * dt * 300.0f32;
        self.pos[0] = f32::max(f32::min(self.pos[0], GAME_AREA_WIDTH-10.0f32), 0.0f32);
        self.pos[1] = f32::max(f32::min(self.pos[1], GAME_AREA_HEIGHT-10.0f32), 0.0f32);
    }
}


pub fn create_random_player(id: u32) -> Player
{
    let range_x = Uniform::new(0.0f32 + SPAWN_PADDING, GAME_AREA_WIDTH - SPAWN_PADDING);
    let range_y = Uniform::new(0.0f32 + SPAWN_PADDING, GAME_AREA_HEIGHT - SPAWN_PADDING);
    
    let range_col = Uniform::new(0.0f32, 1.0f32);
    
    
    let mut rng = rand::thread_rng();
    let rand_x = range_x.sample(&mut rng);
    let rand_y = range_y.sample(&mut rng);
    
    let rand_r = range_col.sample(&mut rng);
    let rand_g = range_col.sample(&mut rng);
    let rand_b = range_col.sample(&mut rng);
    
    Player {
        id: id,
        cur_sequence_id: 0,
        pos: [rand_x, rand_y],
        col: [rand_r, rand_g, rand_b, 1.0f32],
    }
}


pub fn world_to_screen(screen_sz: &[f32; 2], pos: &[f32; 2]) -> [f32; 2]
{
    let res = [ pos[0] * (screen_sz[0] / GAME_AREA_WIDTH), pos[1] * (screen_sz[1] / GAME_AREA_WIDTH) ];
    res
}