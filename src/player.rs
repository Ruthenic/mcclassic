use crate::{
    packets::{
        C2sMessage, C2sPlayerIdentification, FShort, IntoMinecraftString, MinecraftString,
        S2cLevelDataChunk, S2cLevelFinalize, S2cLevelInitialize, S2cMessage, S2cSpawnPlayer,
    },
    PlayerIdCounter,
};
use bevy_ecs::{prelude::*, world::World};
use byteorder::ReadBytesExt;

#[derive(Default, Debug, Component)]
pub struct Player {
    pub id: Option<i8>,
    pub name: Option<MinecraftString>,
}

#[derive(Default, Resource, Component)]
pub struct PlayerConnection {
    pub stream: Option<std::net::TcpStream>,
}

#[derive(Default, Component)]
pub struct Position {
    pub x: FShort,
    pub y: FShort,
    pub z: FShort,
}

#[derive(Default, Component)]
pub struct Uninitialized {}

#[derive(Bundle, Default)]
pub struct PlayerBundle {
    pub connection: PlayerConnection,
    pub player: Player,
    pub position: Position,
}

// NOTE: a lot of this junk is hardcoded and doesnt use the ecs at all. bleh
pub fn initialize_player(
    mut query: Query<
        (&mut Player, &mut PlayerConnection, &mut Position, Entity),
        With<Uninitialized>,
    >,
    mut all_players: Query<(&Player, &mut PlayerConnection), Without<Uninitialized>>,
    world: Res<crate::world::World>,
    mut id_counter: ResMut<PlayerIdCounter>,
    mut commands: Commands,
) {
    for (mut player, mut conn, mut position, entity) in query.iter_mut() {
        let ident =
            C2sPlayerIdentification::parse_from_socket(conn.stream.as_mut().unwrap()).unwrap();

        println!("Initializing player {:?}", ident.username);

        player.name = Some(ident.username);
        player.id = Some(id_counter.next_id);

        let handshake = crate::packets::S2cHandshake::new(
            0x07,
            "Test Server".to_string().to_mcstring(),
            "Test MOTD".to_string().to_mcstring(),
            0x00,
        );

        handshake.send(conn.stream.as_mut().unwrap()).unwrap();

        S2cLevelInitialize::send(conn.stream.as_mut().unwrap()).unwrap();

        let chunks = world.into_byte_array();
        for (chunk_data, chunk_length) in chunks {
            S2cLevelDataChunk::new(chunk_length as i16, chunk_data, 100)
                .send(conn.stream.as_mut().unwrap())
                .unwrap();
        }

        S2cLevelFinalize::new(16, 16, 16)
            .send(conn.stream.as_mut().unwrap())
            .unwrap();

        position.x = FShort::from_num(0);
        position.y = FShort::from_num(0);
        position.z = FShort::from_num(0);

        id_counter.next_id += 1;

        for (sending_player, mut sending_conn) in all_players.iter_mut() {
            let packet = S2cSpawnPlayer::new(
                player.id.unwrap(),
                sending_player.name.clone().unwrap(),
                position.x,
                position.y,
                position.z,
                0,
                0,
            );
            packet.send(sending_conn.stream.as_mut().unwrap()).unwrap();
        }

        commands.entity(entity).remove::<Uninitialized>();
    }
}

pub fn get_player_packet(
    mut query: Query<(&Player, &mut PlayerConnection), Without<Uninitialized>>,
    mut all_players: Query<Entity, Without<Uninitialized>>,
    mut commands: Commands,
) {
    for (player, mut conn) in query.iter_mut() {
        if let Ok(packet_id) = conn.stream.as_mut().unwrap().read_u8() {
            match packet_id {
                0x0d => {
                    let packet =
                        C2sMessage::parse_from_socket(conn.stream.as_mut().unwrap()).unwrap();
                    println!(
                        "[{}] {}",
                        player.name.unwrap().to_string(),
                        packet.message.to_string()
                    );
                    for (entity) in all_players.iter_mut() {
                        /* let packet = S2cMessage::new(
                            if sending_player.id == player.id {
                                -1
                            } else {
                                sending_player.id.unwrap()
                            },
                            packet.message.clone(),
                        );
                        packet.send(sending_conn.stream.as_mut().unwrap()).unwrap(); */
                    }
                }
                _ => {}
            }
        }
    }
}
