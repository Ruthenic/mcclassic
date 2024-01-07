use anyhow::Result;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use fixed::{traits::Fixed, types::extra::U5, FixedI16, FixedI8};
use std::io::{Read, Write};
use std::{fmt::Debug, str};
use tracing::debug;

// Data types
pub type Byte = u8;
pub type SByte = i8;
pub type FByte = FixedI8<U5>;
pub type Short = i16;
pub type FShort = FixedI16<U5>;
pub type ByteArray = [Byte; 1024];

#[derive(Copy, Clone)]
pub struct MinecraftString {
    source: [Byte; 64],
}

impl MinecraftString {
    pub fn parse_from_socket(socket: &mut std::net::TcpStream) -> Result<Self> {
        let mut buf = [0x0 as Byte; 64];

        socket.read_exact(&mut buf)?;

        Ok(MinecraftString { source: buf })
    }
}

impl Debug for MinecraftString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("MinecraftString(\"{}\")", self.to_string()))
    }
}

impl ToString for MinecraftString {
    fn to_string(&self) -> String {
        str::from_utf8(&self.source).unwrap().trim().to_string()
    }
}

pub trait IntoMinecraftString {
    fn to_mcstring(self) -> MinecraftString;
}

impl IntoMinecraftString for MinecraftString {
    fn to_mcstring(self) -> MinecraftString {
        self
    }
}

impl IntoMinecraftString for String {
    fn to_mcstring(self) -> MinecraftString {
        let mut buffer: [u8; 64] = [0x20; 64];

        for idx in 0..64 {
            if let Some(character) = self.as_bytes().get(idx) {
                buffer[idx] = *character;
            }
        }

        MinecraftString { source: buffer }
    }
}

// Client-to-server packets
#[derive(Debug)]
pub struct C2sPlayerIdentification {
    pub protocol_version: Byte,
    pub username: MinecraftString,
    pub verification_key: MinecraftString,
}

impl C2sPlayerIdentification {
    pub fn parse_from_socket(socket: &mut std::net::TcpStream) -> Result<Self> {
        let packet_id = loop {
            let packet_id = socket.read_u8();
            if let Ok(packet_id) = packet_id {
                break packet_id;
            }
        };
        if packet_id != 0x00 {
            return Err(anyhow::anyhow!(
                "Expected packet id 0x00, got 0x{:x}",
                packet_id
            ));
        }
        let protocol_version = socket.read_u8()?;
        let username = MinecraftString::parse_from_socket(socket)?;
        let verification_key = MinecraftString::parse_from_socket(socket)?;

        Ok(C2sPlayerIdentification {
            protocol_version,
            username,
            verification_key,
        })
    }
}

#[derive(Debug)]
pub struct C2sMessage {
    pub player_id: SByte,
    pub message: MinecraftString,
}

impl C2sMessage {
    pub fn parse_from_socket(socket: &mut std::net::TcpStream) -> Result<Self> {
        let player_id = socket.read_i8()?;
        let message = MinecraftString::parse_from_socket(socket)?;

        Ok(C2sMessage { player_id, message })
    }
}

// Server-to-client packets
#[derive(Debug)]
pub struct S2cHandshake {
    pub protocol_version: Byte,
    pub server_name: MinecraftString,
    pub server_motd: MinecraftString,
    pub user_type: Byte,
}

impl S2cHandshake {
    pub fn new(
        protocol_version: Byte,
        server_name: MinecraftString,
        server_motd: MinecraftString,
        user_type: Byte,
    ) -> Self {
        S2cHandshake {
            protocol_version,
            server_name,
            server_motd,
            user_type,
        }
    }
    pub fn send(&self, socket: &mut std::net::TcpStream) -> Result<()> {
        socket.write_u8(0x00)?;
        socket.write_u8(self.protocol_version)?;
        socket.write_all(&self.server_name.source)?;
        socket.write_all(&self.server_motd.source)?;
        socket.write_u8(self.user_type)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct S2cLevelInitialize {}

impl S2cLevelInitialize {
    pub fn send(socket: &mut std::net::TcpStream) -> Result<()> {
        socket.write_u8(0x02)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct S2cLevelDataChunk {
    chunk_length: Short,
    chunk_data: ByteArray,
    percent_complete: Byte,
}

impl S2cLevelDataChunk {
    pub fn new(chunk_length: Short, chunk_data: ByteArray, percent_complete: Byte) -> Self {
        S2cLevelDataChunk {
            chunk_length,
            chunk_data,
            percent_complete,
        }
    }

    pub fn send(&self, socket: &mut std::net::TcpStream) -> Result<()> {
        socket.write_u8(0x03)?;
        socket.write_i16::<BigEndian>(self.chunk_length)?;
        socket.write_all(&self.chunk_data)?;
        socket.write_u8(self.percent_complete)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct S2cLevelFinalize {
    x_size: Short,
    y_size: Short,
    z_size: Short,
}

impl S2cLevelFinalize {
    pub fn new(x_size: Short, y_size: Short, z_size: Short) -> Self {
        S2cLevelFinalize {
            x_size,
            y_size,
            z_size,
        }
    }

    pub fn send(&self, socket: &mut std::net::TcpStream) -> Result<()> {
        socket.write_u8(0x04)?;
        socket.write_i16::<BigEndian>(self.x_size)?;
        socket.write_i16::<BigEndian>(self.y_size)?;
        socket.write_i16::<BigEndian>(self.z_size)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct S2cMessage {
    pub player_id: SByte,
    pub message: MinecraftString,
}

impl S2cMessage {
    pub fn new(player_id: SByte, message: MinecraftString) -> Self {
        S2cMessage { player_id, message }
    }

    pub fn send(&self, socket: &mut std::net::TcpStream) -> Result<()> {
        socket.write_u8(0x06)?;
        socket.write_i8(self.player_id)?;
        socket.write_all(&self.message.source)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct S2cSpawnPlayer {
    pub player_id: SByte,
    pub player_name: MinecraftString,
    pub x: FShort,
    pub y: FShort,
    pub z: FShort,
    pub yaw: Byte,
    pub pitch: Byte,
}

impl S2cSpawnPlayer {
    pub fn new(
        player_id: SByte,
        player_name: MinecraftString,
        x: FShort,
        y: FShort,
        z: FShort,
        yaw: Byte,
        pitch: Byte,
    ) -> Self {
        S2cSpawnPlayer {
            player_id,
            player_name,
            x,
            y,
            z,
            yaw,
            pitch,
        }
    }

    pub fn send(&self, socket: &mut std::net::TcpStream) -> Result<()> {
        socket.write_u8(0x07)?;
        socket.write_i8(self.player_id)?;
        socket.write_all(&self.player_name.source)?;
        socket.write_i16::<BigEndian>(self.x.to_bits())?;
        socket.write_i16::<BigEndian>(self.y.to_bits())?;
        socket.write_i16::<BigEndian>(self.z.to_bits())?;
        socket.write_u8(self.yaw)?;
        socket.write_u8(self.pitch)?;
        Ok(())
    }
}
