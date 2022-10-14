use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{prelude::*};
use std::sync::{Arc, Mutex, mpsc, mpsc::Sender, mpsc::Receiver};
use std::time::{SystemTime};
use std::thread;

use imgui::*;
extern crate common;
use common::*;
use glium::glutin::event::KeyboardInput;

struct ServerPlayerInfo
{
    player: Player,
    input: PlayerInput,
    changed: bool,
}
struct ServerStreamData
{
    id: u32,
    stream: TcpStream,
}


struct ServerData
{
    all_players: Arc<Mutex<Vec<ServerPlayerInfo>>>,
    all_write_streams: Arc<Mutex<Vec<ServerStreamData>>>,
    receiver: Receiver<NetworkMessages>,
    last_time: SystemTime,
    timer: f32,
    update_width_tick: bool,
    has_invalid_stream: bool,
}

impl ServerData {
    fn update_every_positions(&mut self, dt: f32)
    {
        let mut p_list = self.all_players.lock().unwrap();
        for p in p_list.as_mut_slice()
        {
            if p.input.left_right != 0.0f32 || p.input.up_down != 0.0f32 {
                p.changed = true;
                p.player.cur_sequence_id = p.input.cur_sequence_id;
            }
            p.player.update(&p.input, dt);
        }
    }
    fn send_all_changed_positions(&mut self)
    {
        let mut p_list = self.all_players.lock().unwrap();
        let mut all_positions: Vec<u8> = Vec::new();
        for p in p_list.as_mut_slice()
        {
            if p.changed {
                let msg = NetworkMessages::Position(Player { id: p.player.id, cur_sequence_id: p.player.cur_sequence_id, pos: p.player.pos, col: p.player.col });
                let mut v = bincode::serialize(&msg).unwrap();
                all_positions.append(&mut v);
                p.changed = false;
            }
        }
        if all_positions.len() > 0 {
            let mut all_stream = self.all_write_streams.lock().unwrap();
            for i in 0..all_stream.len() {
                let stream_data = all_stream.get_mut(i).unwrap();
                if stream_data.stream.write(&all_positions.as_slice()).is_err() {
                    self.has_invalid_stream = true;
                }
            }
        }
    }
    fn remove_invalid_streams(&mut self)
    {
        let mut all_stream = self.all_write_streams.lock().unwrap();
        while self.has_invalid_stream {
            self.has_invalid_stream = false;
            for i in 0..all_stream.len() {
                let mut peek_data: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
                if all_stream.get(i).unwrap().stream.peek(&mut peek_data).is_err() {
                    all_stream.remove(i);
                    self.has_invalid_stream = true;
                    break;
                }
            }
        }
    }
}




impl Updater for ServerData {
    fn update(&mut self, ui: &Ui, screen_sz: &[f32; 2])
    {
        let cur = SystemTime::now();
        let dt = cur.duration_since(self.last_time).unwrap();
        let in_ms = dt.as_secs_f32();
        self.timer += in_ms;
        self.last_time = cur;



        while self.timer > TICK_RATE {

            for msg in &self.receiver.try_recv() {
                match msg {
                    NetworkMessages::ClientInputChange(input) => {
                        let mut p_list = self.all_players.lock().unwrap();
                        for p in p_list.as_mut_slice()
                        {
                            if p.player.id == input.id {
                                p.input.left_right = f32::min(f32::max(input.left_right, -1.0f32), 1.0f32);
                                p.input.up_down = f32::min(f32::max(input.up_down, -1.0f32), 1.0f32);
                                p.input.cur_sequence_id = input.cur_sequence_id;
                                break;
                            }
                        }
                    }
                    NetworkMessages::RemovePlayer{id} => {
                        let mut p_list = self.all_players.lock().unwrap();
                        for i in 0..p_list.len()
                        {
                            if p_list.get(i).unwrap().player.id == *id {
                                p_list.remove(i);
                                break;
                            }
                        }
                        let mut all_streams = self.all_write_streams.lock().unwrap();

                        for i in 0..all_streams.len() {
                            if all_streams.get(i).unwrap().id == *id {
                                all_streams.remove(i);
                                break;
                            }
                        }
                        let ser_msg = bincode::serialize(msg).unwrap();
                        for stream in all_streams.as_mut_slice() {
                            if stream.stream.write(&ser_msg).is_err() {
                                self.has_invalid_stream = true;
                            }
                        }
                    }
                    NetworkMessages::AddLocal(player) => {
                        println!("[WARNING] GOT ADD LOCAL");
                    }
                    NetworkMessages::Position(player) => {
                        println!("[WARNING] GOT POSITION");
                    }
                    NetworkMessages::AddPlayer(player) => {
                        println!("[WARNING] GOT ADD PLAYER");
                    }
                    _ => {
                        println!("[WARNING] GOT INVALID?");
                    }
                };
    
            }
            
            if self.update_width_tick {
                self.update_every_positions(TICK_RATE);
            }

            self.send_all_changed_positions();
            
            
            self.timer -= TICK_RATE;
        }
        self.remove_invalid_streams();
        
        let p_list = self.all_players.lock().unwrap();
        let player_size = world_to_screen(&screen_sz, &[PLAYER_SIZE, PLAYER_SIZE]);
        for p in p_list.as_slice() {
            let draw_list = ui.get_background_draw_list();
            let pw = world_to_screen(&screen_sz, &p.player.pos);
            draw_list.add_rect(pw, [pw[0] + player_size[0], pw[1] + player_size[1]], p.player.col).filled(true).build();
            
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

    }
   
}


fn handle_client(mut stream_data: ServerStreamData, sender: Sender<NetworkMessages>) {
    
    loop {
            let mut data = [0 as u8; 500];
            match stream_data.stream.read(&mut data) {
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
                    println!("An error occurred, terminating connection with {}", stream_data.stream.peer_addr().unwrap());
                    stream_data.stream.shutdown(Shutdown::Both).unwrap();
                    break;
                }
            },
            _ => {
                println!("An error occurred, terminating connection with {}", stream_data.stream.peer_addr().unwrap());
                stream_data.stream.shutdown(Shutdown::Both).unwrap();
                break;
            }
        } { 

         }
    }
    let msg = NetworkMessages::RemovePlayer { id: stream_data.id };
    sender.send(msg).unwrap();

}



fn main() {

    let (sender, receiver) = mpsc::channel::<NetworkMessages>();

    let mut data = ServerData{
        receiver: receiver,
        timer: 0.0f32,
        update_width_tick: true,
        last_time: SystemTime::now(),
        all_players: Arc::new(Mutex::new(Vec::new())),
        all_write_streams: Arc::new(Mutex::new(Vec::new())),
        has_invalid_stream: false,
    };

    let write_stream_copy = data.all_write_streams.clone();
    let player_list_copy = data.all_players.clone();
    thread::spawn(move ||{
        let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    println!("New connection: {}", stream.peer_addr().unwrap());
                    
                    
                    let mut write_list = write_stream_copy.lock().unwrap();
                    let mut p_list = player_list_copy.lock().unwrap();

                    let mut new_id = 0;
                    loop {
                        let mut found = false;
                        for s_data in write_list.as_slice() {
                            let data = s_data;
                            if data.id == new_id  { found = true; break;}
                        }
                        if found {
                            new_id += 1;
                        }
                        else
                        {
                            break;
                        }
                    }

                    let new_player = create_random_player(new_id);
                    let msg: NetworkMessages = NetworkMessages::AddLocal(new_player);
                    let serialized_msg = bincode::serialize(&msg).unwrap();
                    stream.write(serialized_msg.as_slice()).unwrap();
                    for p in p_list.as_slice() 
                    {
                        let add_player_msg: NetworkMessages = NetworkMessages::AddPlayer(p.player);
                        let ser_msg = bincode::serialize(&add_player_msg).unwrap();
                        stream.write(ser_msg.as_slice()).unwrap();
                    }

                    p_list.push(ServerPlayerInfo { player: Player{ id: new_player.id, cur_sequence_id: 0, pos: new_player.pos, col: new_player.col }, input: PlayerInput { id: new_player.id, cur_sequence_id: 0, up_down: 0.0f32, left_right: 0.0f32 }, changed: false });
                    let player_add_msg = NetworkMessages::AddPlayer(p_list.last().unwrap().player);
                    let ser_msg = bincode::serialize(&player_add_msg).unwrap();
                    for stream in write_list.as_mut_slice()
                    {
                        if stream.id == new_player.id { continue; }
                        if stream.stream.write(ser_msg.as_slice()).is_err() {

                        }
                    }


                    let stream_data = ServerStreamData{
                        id: new_id,
                        stream: stream,
                    };
                    write_list.push(ServerStreamData{
                        id: stream_data.id,
                        stream: stream_data.stream.try_clone().unwrap(),
                    });

                    let sender_copy = sender.clone();
                    thread::spawn(move|| {
                        handle_client(stream_data, sender_copy);
                    });
                }
                Err(e) => {
                    println!("Error: {}", e);
                    panic!();
                }
            }
        }
    });

    let r = MyRenderer::new("Server");
    r.run(move |run, ui, sz| {
            data.update(&ui, sz);
        }
    );
} 