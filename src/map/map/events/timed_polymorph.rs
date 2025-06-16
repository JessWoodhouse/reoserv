use eolib::protocol::{Coords, Direction};

use crate::NPC_DB;

use super::super::Map;

impl Map {
    pub fn timed_polymorph(&mut self) {
        use eolib::protocol::net::{
            server::{
                NpcSpecServerPacket, NpcAgreeServerPacket, NpcKilledData, NpcMapInfo,
            },
            PacketAction, PacketFamily,
        };

        let mut npcs_to_reverse: Vec<(i32, i32, i32, Coords, Direction)> = Vec::new();
        
        for (npc_index, npc) in &self.npcs {
            if npc.polymorphed && npc.alive {
                npcs_to_reverse.push((*npc_index, npc.old_id, npc.id, npc.coords, npc.direction));
            }
        }

        for (npc_index, old_id, current_id, coords, direction) in npcs_to_reverse {
            
            self.send_packet_all(
                PacketAction::Spec,
                PacketFamily::Npc,
                NpcSpecServerPacket {
                    npc_killed_data: NpcKilledData {
                        npc_index,
                        ..Default::default()
                    },
                    experience: None,
                },
            );

            if let Some(npc) = self.npcs.get_mut(&npc_index) {
                // Double-check the NPC is still alive and polymorphed before reversing
                if npc.alive && npc.polymorphed {
                    npc.id = old_id; 
                    npc.polymorphed = false; 
                    npc.old_id = 0;  
                    
                    if let Some(original_data) = NPC_DB.npcs.get(old_id as usize - 1) {
                        npc.max_hp = original_data.hp;
                        npc.hp = original_data.hp; 
                    }

                    self.send_packet_all(
                        PacketAction::Agree,
                        PacketFamily::Npc,
                        NpcAgreeServerPacket {
                            npcs: vec![NpcMapInfo {
                                index: npc_index,
                                id: old_id,
                                coords,
                                direction,
                            }],
                        },
                    );
                }
            }
        }
    }
}