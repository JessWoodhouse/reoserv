use eolib::protocol::{
    net::{
        server::{NpcAgreeServerPacket, NpcKilledData, NpcMapInfo, NpcSpecServerPacket},
        PacketAction, PacketFamily,
    },
    Coords, Direction,
};

use crate::NPC_DB;

use super::Map;

impl Map {
    /// Check for expired polymorphs and restore NPCs to their original form
    pub fn timed_polymorph(&mut self) {
        let now = std::time::Instant::now();
        let mut npcs_to_restore = Vec::new();

        // Find all NPCs whose polymorph has expired
        for (&npc_index, npc) in &self.npcs {
            if npc.polymorphed {
                if let Some(expire_time) = npc.polymorph_expire_time {
                    if now >= expire_time {
                        if let Some(original_id) = npc.original_id {
                            npcs_to_restore.push((npc_index, original_id, npc.coords, npc.direction));
                        }
                    }
                }
            }
        }

        // Restore NPCs that have expired polymorph
        for (npc_index, original_id, coords, direction) in npcs_to_restore {
            self.restore_npc_from_polymorph(npc_index, original_id, coords, direction);
        }
    }

    /// Polymorph an NPC into a different form for a specified duration
    pub fn polymorph_npc(&mut self, npc_index: i32, new_npc_id: i32, duration_seconds: u32){
        // Check if target NPC exists and isn't already polymorphed
        let (coords, direction, original_id) = match self.npcs.get(&npc_index) {
            Some(npc) => {
                if npc.polymorphed {
                    return; // Already polymorphed
                }
                (npc.coords, npc.direction, npc.id)
            }
            None => return,
        };

        // Clear the NPC slot on clients first
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

        // Update server logic - store original data and set expiration
        if let Some(npc) = self.npcs.get_mut(&npc_index) {
            npc.original_id = Some(original_id); // Store original ID
            npc.id = new_npc_id;
            npc.polymorphed = true;
            
            // Set polymorph to expire after specified duration
            npc.polymorph_expire_time = Some(std::time::Instant::now() + std::time::Duration::from_secs(duration_seconds as u64));
            
            // Reset HP to max for the new form
            if let Some(new_data) = NPC_DB.npcs.get(new_npc_id as usize - 1) {
                npc.max_hp = new_data.hp;
                npc.hp = new_data.hp;
            }
        }

        // Spawn "polymorphed" NPC on clients
        self.send_packet_all(
            PacketAction::Agree,
            PacketFamily::Npc,
            NpcAgreeServerPacket {
                npcs: vec![NpcMapInfo {
                    index: npc_index,
                    id: new_npc_id,
                    coords,
                    direction,
                }],
            },
        );
    }

    /// Restore an NPC from its polymorphed form back to original
    fn restore_npc_from_polymorph(&mut self, npc_index: i32, original_id: i32, coords: Coords, direction: Direction) {
        // Clear the polymorphed NPC from clients first
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

        // Update server logic - restore original form
        if let Some(npc) = self.npcs.get_mut(&npc_index) {
            npc.id = original_id;
            npc.polymorphed = false;
            npc.original_id = None;
            npc.polymorph_expire_time = None;
            
            // Reset HP to max for the original form
            if let Some(original_data) = NPC_DB.npcs.get(original_id as usize - 1) {
                npc.max_hp = original_data.hp;
                npc.hp = original_data.hp;
            }
        }

        // Spawn the original NPC back on clients
        self.send_packet_all(
            PacketAction::Agree,
            PacketFamily::Npc,
            NpcAgreeServerPacket {
                npcs: vec![NpcMapInfo {
                    index: npc_index,
                    id: original_id,
                    coords,
                    direction,
                }],
            },
        );
    }

    /// Manually restore a specific NPC (can be called from other parts of code if needed)
    pub fn force_restore_npc(&mut self, npc_index: i32) {
        if let Some(npc) = self.npcs.get(&npc_index) {
            if npc.polymorphed {
                if let Some(original_id) = npc.original_id {
                    let coords = npc.coords;
                    let direction = npc.direction;
                    self.restore_npc_from_polymorph(npc_index, original_id, coords, direction);
                }
            }
        }
    }
}