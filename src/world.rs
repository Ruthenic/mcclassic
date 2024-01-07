use anyhow::{bail, Context, Result};
use bevy_ecs::prelude::*;
use libflate::gzip::Encoder as GzipEncoder;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::Write;
use tracing::info;

use crate::blocks::Block;

type Coord = (u16, u16, u16);

#[derive(Serialize, Deserialize, Resource)]
pub struct World {
    blocks: HashMap<Coord, Block>,
}

impl World {
    pub fn new(x_size: u16, z_size: u16, y_size: u16) -> Self {
        let mut world = Self {
            blocks: HashMap::new(),
        };

        for x in 0..x_size {
            for z in 0..z_size {
                for y in 0..y_size {
                    world.blocks.insert(
                        (x as u16, y as u16, z as u16),
                        if y < y_size / 2 {
                            Block::Grass
                        } else {
                            Block::Air
                        },
                    );
                }
            }
        }

        world
    }

    pub fn set_block(&mut self, coords: Coord, block_type: Block) -> Result<()> {
        if !self.blocks.contains_key(&coords) {
            bail!("you can't place a block OOB")
        }

        self.blocks
            .insert(coords, block_type)
            .context("oopsie daisy")?;

        Ok(())
    }

    pub fn into_byte_array(&self) -> Vec<([u8; 1024], usize)> {
        // thing
        let mut blocks_vec = self.blocks.iter().collect::<Vec<(&Coord, &Block)>>();
        blocks_vec.sort_by_key(|(coord, _)| coord.0);
        blocks_vec.sort_by_key(|(coord, _)| coord.2);
        blocks_vec.sort_by_key(|(coord, _)| coord.1 as i16);

        let mut bytes_vec: VecDeque<u8> = VecDeque::new();

        for (_, block) in blocks_vec {
            bytes_vec.push_back(*block as u8)
        }

        let length = bytes_vec.len() as u32;
        let mut len_iter = length.to_be_bytes();
        len_iter.reverse();
        for len_byte in len_iter {
            bytes_vec.push_front(len_byte);
        }

        // cry (gzip compression)
        let mut encoder = GzipEncoder::new(Vec::new()).unwrap();
        for byte in bytes_vec {
            encoder.write_all(&[byte]).unwrap();
        }
        let res = encoder.finish().into_result().unwrap();

        let chunks = res.chunks(1024);
        let mut final_vec: Vec<([u8; 1024], usize)> = vec![];

        for chunk in chunks {
            if chunk.len() == 1024 {
                final_vec.push(((*chunk).try_into().unwrap(), 1024))
            } else {
                let mut final_chunk = [0 as u8; 1024];
                final_chunk[0..chunk.len()].copy_from_slice(chunk);
                final_vec.push((final_chunk, chunk.len()))
            }
        }

        final_vec
    }

    pub fn try_load_from_file(&mut self, file_name: impl ToString) -> Result<()> {
        let file = File::open(file_name.to_string());
        if let Ok(file) = file {
            let saved_file: Self = ciborium::from_reader(file)?;
            self.blocks = saved_file.blocks
        }

        Ok(())
    }

    fn save_to_file(&self, file_name: impl ToString) -> Result<()> {
        info!("Saving world...");

        let file = File::create(file_name.to_string())?;
        ciborium::into_writer(&self, file)?;

        info!("Saved world!");

        Ok(())
    }
}
