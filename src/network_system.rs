use crate::player::Uninitialized;
use crate::player::{PlayerBundle, PlayerConnection};
use bevy_ecs::prelude::*;
use std::net::TcpListener;

#[derive(Resource)]
pub struct NetworkHandle {
    pub listener: TcpListener,
}

pub fn accept_connections(handle: Res<NetworkHandle>, mut commands: Commands) {
    if let Some(Ok(socket)) = handle.listener.incoming().next() {
        println!("Got a connection from {:?}", socket);
        /* socket.set_nonblocking(false).unwrap(); */
        commands
            .spawn(PlayerBundle {
                connection: PlayerConnection {
                    stream: Some(socket),
                },
                ..Default::default()
            })
            .insert(Uninitialized {});
    }
}
