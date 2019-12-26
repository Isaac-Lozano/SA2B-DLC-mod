#[derive(Clone, Copy, Debug, Default)]
pub struct KartStats {
    pub accel: f32,
    pub brake_force: f32,
    pub no_accel_force: f32,
    pub max_drive_speed: f32,
    pub gravity: f32,
    pub unknown1: f32,
    pub drift_factor: f32,
    pub drift_threshold: f32,
    pub unknown2: f32,
    pub hard_speed_cap: f32,
}

#[derive(Clone, Copy)]
pub struct KartDlc {
    pub stats: KartStats,
    pub autorun_slot_handicap_1: f32,
    pub autorun_rank_handicap_1: f32,
    pub autorun_not_first_handicap_1: f32,
    pub autorun_slot_handicap_2: f32,
    pub autorun_rank_handicap_2: f32,
    pub autorun_not_first_handicap_2: f32,
    pub ai_use_dlc_kart: u32,
    // char is actually a u32 in rust
    // this is because it actually represents unicode well
    pub song_name: [u8; 64],
}

#[derive(Clone, Copy)]
pub struct DlcText {
    pub title: [u8; 128],
    pub dlc_type: [u8; 128],
    pub stage: [u8; 128],
    pub character: [u8; 128],
    pub description: [u8; 128],
}

impl Default for DlcText {
    fn default() -> DlcText {
        DlcText {
            title: [0; 128],
            dlc_type: [0; 128],
            stage: [0; 128],
            character: [0; 128],
            description: [0; 128],
        }
    }
}

#[derive(Clone, Copy)]
pub struct DlcDescriptor {
    pub unknown1: u32,
    pub event_id: u32,
    pub unknown2: u32,
    pub unknown3: u32,
    pub unknown4: u32,
    pub unknown5: u32,
    pub dlc_type: u32,
    pub levels: [u32; 8],
    pub text: [DlcText; 6],
}
