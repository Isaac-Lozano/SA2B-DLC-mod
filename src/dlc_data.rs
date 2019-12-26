use std::io::{self, Read, Seek, SeekFrom, Cursor};
use std::iter;

use byteorder::{ReadBytesExt, WriteBytesExt, LE};
use prs_util::decoder::Decoder;

use crate::model::{DlcText, KartStats, KartDlc};

const SAVE_BASE: u32 = 0x8cb00000;

trait DlcRead: Sized {
    fn read_from<R>(read: R) -> io::Result<Self> where R: Read + Seek;
}

struct Pointer<T>(T);

impl<T> DlcRead for Pointer<T>
where
    T: DlcRead,
{
    fn read_from<R>(mut read: R) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let addr = read.read_u32::<LE>()?;
        let save_addr = read.seek(SeekFrom::Current(0))?;
        read.seek(SeekFrom::Start(addr as u64))?;
        let inner = T::read_from(&mut read)?;
        read.seek(SeekFrom::Start(save_addr))?;
        Ok(Pointer(inner))
    }
}

struct VmuPointer<T>(T);

impl<T> DlcRead for VmuPointer<T>
where
    T: DlcRead,
{
    fn read_from<R>(mut read: R) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let addr = read.read_u32::<LE>()? - SAVE_BASE;
        let save_addr = read.seek(SeekFrom::Current(0))?;
        read.seek(SeekFrom::Start(addr as u64))?;
        let inner = T::read_from(&mut read)?;
        read.seek(SeekFrom::Start(save_addr))?;
        Ok(VmuPointer(inner))
    }
}

struct OffsetLen(Vec<u8>);

impl DlcRead for OffsetLen {
    fn read_from<R>(mut read: R) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let addr = read.read_u32::<LE>()?;
        println!("Addr: 0x{:08x}", addr);
        let len = read.read_u32::<LE>()?;
        println!("Len: 0x{:08x}", len);
        let save_addr = read.seek(SeekFrom::Current(0))?;
        read.seek(SeekFrom::Start(addr as u64))?;
        let mut inner: Vec<u8> = iter::repeat(0).take(len as usize).collect();
        read.read_exact(&mut inner)?;
        read.seek(SeekFrom::Start(save_addr))?;
        Ok(OffsetLen(inner))
    }
}

impl DlcRead for [u8; 128] {
    fn read_from<R>(read: R) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let mut data = [0; 128];
        for (idx, byte_res) in read.bytes().take(128).enumerate() {
            let byte = byte_res?;
            if byte != 0 {
                data[idx] = byte;
            } else {
                break;
            }
        }
        Ok(data)
    }
}

impl DlcRead for DlcText {
    fn read_from<R>(mut read: R) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let title = VmuPointer::<[u8;128]>::read_from(&mut read)?;
        let dlc_type = VmuPointer::<[u8;128]>::read_from(&mut read)?;
        let stage = VmuPointer::<[u8;128]>::read_from(&mut read)?;
        let character = VmuPointer::<[u8;128]>::read_from(&mut read)?;
        let description = VmuPointer::<[u8;128]>::read_from(&mut read)?;

        Ok(DlcText {
            title: title.0,
            dlc_type: dlc_type.0,
            stage: stage.0,
            character: character.0,
            description: description.0,
        })
    }
}

impl DlcRead for KartStats {
    fn read_from<R>(mut read: R) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let accel = read.read_f32::<LE>()?;
        let brake_force = read.read_f32::<LE>()?;
        let no_accel_force = read.read_f32::<LE>()?;
        let max_drive_speed = read.read_f32::<LE>()?;
        let gravity = read.read_f32::<LE>()?;
        let unknown1 = read.read_f32::<LE>()?;
        let drift_factor = read.read_f32::<LE>()?;
        let drift_threshold = read.read_f32::<LE>()?;
        let unknown2 = read.read_f32::<LE>()?;
        let hard_speed_cap = read.read_f32::<LE>()?;

        Ok(KartStats {
            accel: accel,
            brake_force: brake_force,
            no_accel_force: no_accel_force,
            max_drive_speed: max_drive_speed,
            gravity: gravity,
            unknown1: unknown1,
            drift_factor: drift_factor,
            drift_threshold: drift_threshold,
            unknown2: unknown2,
            hard_speed_cap: hard_speed_cap,
        })
    }
}

impl DlcRead for KartDlc {
    fn read_from<R>(mut read: R) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let stats = KartStats::read_from(&mut read)?;
        let autorun_slot_handicap_1 = read.read_f32::<LE>()?;
        let autorun_rank_handicap_1 = read.read_f32::<LE>()?;
        let autorun_not_first_handicap_1 = read.read_f32::<LE>()?;
        let autorun_slot_handicap_2 = read.read_f32::<LE>()?;
        let autorun_rank_handicap_2 = read.read_f32::<LE>()?;
        let autorun_not_first_handicap_2 = read.read_f32::<LE>()?;
        let ai_use_dlc_kart = read.read_u32::<LE>()?;
        let mut song_name = [0; 64];
        read.read_exact(&mut song_name)?;

        Ok(KartDlc {
            stats: stats,
            autorun_slot_handicap_1: autorun_slot_handicap_1,
            autorun_rank_handicap_1: autorun_rank_handicap_1,
            autorun_not_first_handicap_1: autorun_not_first_handicap_1,
            autorun_slot_handicap_2: autorun_slot_handicap_2,
            autorun_rank_handicap_2: autorun_rank_handicap_2,
            autorun_not_first_handicap_2: autorun_not_first_handicap_2,
            ai_use_dlc_kart: ai_use_dlc_kart,
            song_name: song_name,
        })
    }
}

fn rebase_njs_model(data: &mut [u8], model_offset: usize) -> io::Result<()> {
    let data_base = data.as_ptr() as u32;
    let mut cursed = Cursor::new(data);

    cursed.seek(SeekFrom::Start(model_offset as u64 + 0x00))?;
    let vert_offset = cursed.read_u32::<LE>()?;
    if vert_offset != 0 {
        cursed.seek(SeekFrom::Start(model_offset as u64 + 0x00))?;
        cursed.write_u32::<LE>(vert_offset + data_base)?;
    }

    cursed.seek(SeekFrom::Start(model_offset as u64 + 0x04))?;
    let norm_offset = cursed.read_u32::<LE>()?;
    if norm_offset != 0 {
        cursed.seek(SeekFrom::Start(model_offset as u64 + 0x04))?;
        cursed.write_u32::<LE>(norm_offset + data_base)?;
    }

    Ok(())
}

fn rebase_njs_obj(data: &mut [u8], obj_offset: usize) -> io::Result<()> {
    let data_base = data.as_ptr() as u32;
    let mut cursed = Cursor::new(data);

    cursed.seek(SeekFrom::Start(obj_offset as u64 + 0x04))?;
    let model_offset = cursed.read_u32::<LE>()?;
    if model_offset != 0 {
        rebase_njs_model(cursed.get_mut(), model_offset as usize)?;
        cursed.seek(SeekFrom::Start(obj_offset as u64 + 0x04))?;
        cursed.write_u32::<LE>(model_offset + data_base)?;
    }

    cursed.seek(SeekFrom::Start(obj_offset as u64 + 0x2c))?;
    let child_offset = cursed.read_u32::<LE>()?;
    if child_offset != 0 {
        rebase_njs_obj(cursed.get_mut(), child_offset as usize)?;
        cursed.seek(SeekFrom::Start(obj_offset as u64 + 0x2c))?;
        cursed.write_u32::<LE>(child_offset + data_base)?;
    }

    cursed.seek(SeekFrom::Start(obj_offset as u64 + 0x30))?;
    let sibling_offset = cursed.read_u32::<LE>()?;
    if sibling_offset != 0 {
        rebase_njs_obj(cursed.get_mut(), sibling_offset as usize)?;
        cursed.seek(SeekFrom::Start(obj_offset as u64 + 0x30))?;
        cursed.write_u32::<LE>(sibling_offset + data_base)?;
    }

    Ok(())
}

fn rebase_njs_texname(data: &mut [u8], name_offset: usize) -> io::Result<()> {
    let data_base = data.as_ptr() as u32;
    let mut cursed = Cursor::new(data);
    println!("texname {:08x}", name_offset);

    cursed.seek(SeekFrom::Start(name_offset as u64))?;
    let filename_offset = cursed.read_u32::<LE>()?;
    if filename_offset != 0 {
        cursed.seek(SeekFrom::Start(name_offset as u64))?;
        cursed.write_u32::<LE>(filename_offset + data_base)?;
    }

    Ok(())
}

fn rebase_njs_texlist(data: &mut [u8], tex_offset: usize) -> io::Result<()> {
    println!("texlist {:08x}", tex_offset);
    let data_base = data.as_ptr() as u32;
    let mut cursed = Cursor::new(data);

    cursed.seek(SeekFrom::Start(tex_offset as u64))?;
    let name_offset = cursed.read_u32::<LE>()?;
    let num_names = cursed.read_u32::<LE>()?;
    if name_offset != 0 && num_names != 0 {
        for idx in 0..num_names {
            rebase_njs_texname(cursed.get_mut(), (name_offset + 0xc * idx) as usize)?;
        }
        cursed.seek(SeekFrom::Start(tex_offset as u64))?;
        cursed.write_u32::<LE>(name_offset + data_base)?;
    }

    Ok(())
}

pub struct DlcModelData {
    model: Vec<u8>,
    texlist: Vec<u8>,
    pub texture: Vec<u8>,
    pub model_ptr: u32,
    pub texlist_ptr: u32,
}

impl DlcRead for DlcModelData {
    fn read_from<R>(mut read: R) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let mut model = OffsetLen::read_from(&mut read)?.0;
        let mut texlist = OffsetLen::read_from(&mut read)?.0;
        let mut texture = OffsetLen::read_from(&mut read)?.0;

        let mut model_slice: &[u8] = &model;
        let obj_offset = model_slice.read_u32::<LE>()?;
        rebase_njs_obj(&mut model, obj_offset as usize)?;
        let obj_raw_ptr = model.as_ptr() as u32 + obj_offset;

        let mut texlist_slice: &[u8] = &texlist;
        let texlist_offset = texlist_slice.read_u32::<LE>()?;
        rebase_njs_texlist(&mut texlist, texlist_offset as usize)?;
        let texlist_raw_ptr = texlist.as_ptr() as u32 + texlist_offset;

        // swap endianness of thing
        texture.swap(8, 9);
        // swap endianness of num textures
        texture.swap(10, 11);

        let mut offset = 0x2e;
        for _ in 0..4 {
            texture.swap(offset, offset + 3);
            texture.swap(offset + 1, offset + 2);
            offset += 0x26;
        }

        Ok(DlcModelData {
            model: model,
            texlist: texlist,
            texture: texture,
            model_ptr: obj_raw_ptr,
            texlist_ptr: texlist_raw_ptr,
        })
    }
}

pub struct DlcPrsData {
    pub kart_dlc: KartDlc,
    pub set_data: Vec<u8>,
    pub track_data: Vec<u8>,
    pub model_data: DlcModelData,
}

impl DlcRead for DlcPrsData {
    fn read_from<R>(mut read: R) -> io::Result<Self>
    where
        R: Read + Seek,
    {
        let kart_dlc = Pointer::<KartDlc>::read_from(&mut read)?.0;
        let mut set_data = OffsetLen::read_from(&mut read)?.0;
        let mut offset = 32;
        while offset < set_data.len() {
            set_data.swap(offset + 0, offset + 1);
            set_data.swap(offset + 2, offset + 3);
            set_data.swap(offset + 4, offset + 5);
            set_data.swap(offset + 6, offset + 7);

            set_data.swap(offset + 8, offset + 11);
            set_data.swap(offset + 9, offset + 10);

            set_data.swap(offset + 12, offset + 15);
            set_data.swap(offset + 13, offset + 14);

            set_data.swap(offset + 16, offset + 19);
            set_data.swap(offset + 17, offset + 18);

            set_data.swap(offset + 20, offset + 23);
            set_data.swap(offset + 21, offset + 22);

            set_data.swap(offset + 24, offset + 27);
            set_data.swap(offset + 25, offset + 26);

            set_data.swap(offset + 28, offset + 31);
            set_data.swap(offset + 29, offset + 30);

            offset += 32;
        }
        let track_data = OffsetLen::read_from(&mut read)?.0;
        let model_data = OffsetLen::read_from(&mut read)?.0;
        let model = DlcModelData::read_from(Cursor::new(model_data))?;

        Ok(DlcPrsData {
            kart_dlc: kart_dlc,
            set_data: set_data,
            track_data: track_data,
            model_data: model,
        })
    }
}

pub struct DlcData {
    pub dlc_type: u32,
    pub dlc_texts: [DlcText; 6],
    pub level_ids: [u32; 8],
    pub prs_data: DlcPrsData,
}

impl DlcData {
    pub fn from_vmu<R>(mut read: R) -> io::Result<DlcData>
    where
        R: Read + Seek,
    {
        read.seek(SeekFrom::Start(0x48))?;
        let len = read.read_u32::<LE>()?;
        read.seek(SeekFrom::Start(0x280))?;
        let mut data: Vec<u8> = iter::repeat(0).take(len as usize).collect();
        read.read_exact(&mut data)?;
        let mut vmu_data = Cursor::new(data);

        let dlc_type = vmu_data.read_u32::<LE>()?;

        let mut dlc_texts = [DlcText::default(); 6];
        for idx in 0..5 {
            let text = VmuPointer::<DlcText>::read_from(&mut vmu_data)?;
            dlc_texts[idx] = text.0;
        }

        let mut level_ids = [0; 8];
        for idx in 0..8 {
            level_ids[idx] = vmu_data.read_u32::<LE>()?;
        }

        let prs_pointer = vmu_data.read_u32::<LE>()? - SAVE_BASE;
        vmu_data.seek(SeekFrom::Start(prs_pointer as u64))?;

        let mut decoder = Decoder::new(vmu_data);
        let decoded = decoder.decode_to_vec()?;

        println!("Dlc type: {}", dlc_type);

        for (idx, text) in dlc_texts.iter().enumerate() {
            println!("TEXT {}", idx);
            dump_hex(&text.title);
            dump_hex(&text.dlc_type);
            dump_hex(&text.stage);
            dump_hex(&text.character);
            dump_hex(&text.description);
        }

        dump_hex(&decoded);

        let prs_data = DlcPrsData::read_from(Cursor::new(decoded))?;

        Ok(DlcData {
            dlc_type: dlc_type,
            dlc_texts: dlc_texts,
            level_ids: level_ids,
            prs_data: prs_data,
        })
    }
}

fn dump_hex(data: &[u8]) {
    let mut offset = 0;
    while data.len() - offset >= 0x10 {
        print!("{:08x} | ", offset);
        print!("{:02x} {:02x} {:02x} {:02x}  ",
            data[offset + 0],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3]
        );
        print!("{:02x} {:02x} {:02x} {:02x}  ",
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7]
        );
        print!("{:02x} {:02x} {:02x} {:02x}  ",
            data[offset + 8],
            data[offset + 9],
            data[offset + 10],
            data[offset + 11]
        );
        print!("{:02x} {:02x} {:02x} {:02x} | ",
            data[offset + 12],
            data[offset + 13],
            data[offset + 14],
            data[offset + 15]
        );
        for idx in 0..0x10 {
            let val = data[offset + idx];
            if val >= 0x20 && val <= 0x7e {
                print!("{}", val as char);
            } else {
                print!(".");
            }
        }
        println!();
        offset += 0x10;
    }
    println!("Plus {} more bytes.", data.len() - offset);
}
