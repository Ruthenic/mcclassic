mod blocks;
mod network_system;
mod packets;
mod player;
mod world;

use bevy_ecs::prelude::*;

#[derive(Resource)]
pub struct PlayerIdCounter {
    pub next_id: i8,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tcp_listener = std::net::TcpListener::bind("127.0.0.1:25565")?;

    tcp_listener.set_nonblocking(true)?;

    let handle = network_system::NetworkHandle {
        listener: tcp_listener,
    };

    let mut world = World::default();
    world.insert_resource(handle);

    let mc_world = world::World::new(16, 16, 16);
    world.insert_resource(mc_world);

    world.insert_resource(PlayerIdCounter { next_id: 0 });

    let mut schedule = Schedule::default();
    schedule.add_systems(
        (
            network_system::accept_connections,
            player::initialize_player,
            player::get_player_packet,
        )
            .chain(),
    );

    loop {
        schedule.run(&mut world);
    }
}
