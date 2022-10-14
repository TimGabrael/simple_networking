#[macro_use]
extern crate glium;

use glium::glutin::event::{ KeyboardInput, ElementState, VirtualKeyCode};
#[allow(unused_imports)]
use glium::{glutin, Surface};
use std::thread;
use std::sync::{mpsc, mpsc::Receiver};
use std::time::{SystemTime};

use imgui::*;

extern crate common;
use common::*;

use std::net::{TcpStream};
use std::io::{prelude::*};


struct ClientData
{
    stream: TcpStream,
    receiver: Receiver<NetworkMessages>,
    all_players: Vec<Player>,
    last_input: PlayerInput,
    last_time: SystemTime,
    local_player_id: u32,
    timer: f32,
    predict_movement: bool,
}





impl Updater for ClientData {
    fn update(&mut self, ui: &Ui, screen_sz: &[f32; 2])
    {
        let cur = SystemTime::now();
        let dt = cur.duration_since(self.last_time).unwrap();
        let in_ms = dt.as_secs_f32();
        self.timer += in_ms;
        self.last_time = cur;

        for msg in &self.receiver.try_recv() {
            match msg {
                NetworkMessages::ClientInputChange(input) => {
                    for p in &mut self.all_players
                    {
                        if p.id == input.id {
                            break;
                        }
                    }
                }
                NetworkMessages::AddLocal(player) => {
                    self.all_players.push(Player{ id: player.id, cur_sequence_id: player.cur_sequence_id, pos: player.pos, col: player.col });
                    self.local_player_id = player.id;
                }
                NetworkMessages::AddPlayer(player) => {
                    self.all_players.push(Player{ id: player.id, cur_sequence_id: player.cur_sequence_id, pos: player.pos, col: player.col });
                }
                NetworkMessages::RemovePlayer{id} => {
                    for i in 0..self.all_players.len()
                    {
                        if self.all_players.get(i).unwrap().id == *id {
                            self.all_players.remove(i);
                            break;
                        }
                    }
                }
                NetworkMessages::Position(player) => {
                    for p in &mut self.all_players
                    {
                        if p.id == player.id {
                            p.pos = player.pos;
                            break;
                        }
                    }
                }
                _ => {
                }
            };
        }

        while self.timer > TICK_RATE {
            let mut left_right = 0.0f32;
            let mut up_down = 0.0f32;
            if ui.is_key_down(Key::UpArrow) { up_down -= 1.0f32;}
            if ui.is_key_down(Key::DownArrow) { up_down += 1.0f32;}
            if ui.is_key_down(Key::LeftArrow) { left_right -= 1.0f32;}
            if ui.is_key_down(Key::RightArrow) { left_right += 1.0f32;}
            if left_right != self.last_input.left_right || up_down != self.last_input.up_down {
                for p in &mut self.all_players {
                    if p.id == self.local_player_id {

                        p.cur_sequence_id += 1;
                        let p_inputs = PlayerInput{
                            id: p.id,
                            cur_sequence_id: p.cur_sequence_id,
                            left_right: left_right,
                            up_down: up_down,
                        };
                        self.last_input = p_inputs.clone();

                        let msg: NetworkMessages = NetworkMessages::ClientInputChange(p_inputs);
                        let serialized_msg = bincode::serialize(&msg).unwrap();
                        self.stream.write(&serialized_msg.as_slice()).unwrap();

                        break;
                    }
                }
            }

            if self.predict_movement {
                for p in &mut self.all_players {
                    if p.id == self.local_player_id {
                        p.update(&self.last_input, TICK_RATE);
                    }
                }
            }

            self.timer -= TICK_RATE;
        }

        Window::new("Test Window")
           .size([300.0, 100.0], Condition::FirstUseEver)
           .build(&ui, || {
               ui.text("Test!");
               ui.separator();
               let mouse_pos = ui.io().mouse_pos;
               ui.text(format!(
                   "Mouse Position: ({:.1},{:.1})",
                   mouse_pos[0], mouse_pos[1]
               ));
           });
  
        let player_size = world_to_screen(&screen_sz, &[PLAYER_SIZE, PLAYER_SIZE]);
        for p in &self.all_players {
            let draw_list = ui.get_background_draw_list();
            let pw = world_to_screen(&screen_sz, &p.pos);
            draw_list.add_rect(pw, [pw[0] + player_size[0], pw[1] + player_size[1]], p.col).filled(true).build();
        }
    }
    
}


fn main() {
    let (sender, receiver) = mpsc::channel::<NetworkMessages>();
    let mut read_stream = TcpStream::connect("127.0.0.1:7878").unwrap();
    let mut client_data = ClientData{
        stream: read_stream.try_clone().unwrap(),
        receiver: receiver,
        last_input: { PlayerInput { id: u32::MAX, cur_sequence_id: 0, up_down: 0.0f32, left_right: 0.0f32 }},
        last_time: SystemTime::now(),
        timer: 0.0f32,
        local_player_id: u32::MAX,
        all_players: Vec::new(),
        predict_movement: true,
    };

    thread::spawn(move || {
        loop {
                let mut data = [0 as u8; 500];
                match read_stream.read(&mut data) {
                Ok(size) => {
                    if size > 0 {
                        let mut cursor = &data[..];
                        while cursor.len() > 0 {
                            let msg: Result<NetworkMessages, _> = bincode::deserialize_from(&mut cursor);
                            if msg.is_ok() {
                                let real_msg = msg.unwrap();
                                if !matches!(real_msg, NetworkMessages::InvalidMessage) {
                                    sender.send(real_msg).unwrap();
                                }
                            }
                        }
                    }
                    else {
                        println!("An error occurred, terminating connection with {}", read_stream.peer_addr().unwrap());
                        break;
                    }
                },
                _ => {
                    println!("An error occurred, terminating connection with {}", read_stream.peer_addr().unwrap());
                    break;
                }
            } { 
    
             }
        }
    });



    let r = MyRenderer::new("Client");
    r.run(move |run, ui, sz| {
        client_data.update(&ui, sz);
    });

}