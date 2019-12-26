#![feature(naked_functions)]
#![feature(asm)]

mod process_reader;
mod model;
mod dlc_data;

use std::io;
use std::fs::{self, File};
use std::mem;

use process_reader::ProcessHandle;
use model::*;
use dlc_data::DlcData;

const DLC_PREFIX: &'static str = "resource/gd_PC/SAVEDATA/DLC/";
static mut dlc: Option<DlcData> = None;
static mut DLC_ARRAY: Option<Vec<DlcData>> = None;

pub trait ProcessHandleExt {
    fn write_jump(&self, address: u32, function: *const fn()) -> Result<(), &'static str>;
    fn write_call(&self, address: u32, function: *const fn()) -> Result<(), &'static str>;
}

impl ProcessHandleExt for ProcessHandle {
    fn write_jump(&self, address: u32, function: *const fn()) -> Result<(), &'static str> {
        let function_address = function as u32;
        self.write_copy(address, 0xe9u8)?;
        self.write_copy(address + 1, function_address - (address + 5))
    }

    fn write_call(&self, address: u32, function: *const fn()) -> Result<(), &'static str> {
        let function_address = function as u32;
        self.write_copy(address, 0xe8u8)?;
        self.write_copy(address + 1, function_address - (address + 5))
    }
}

#[repr(C)]
pub struct ModInfo {
    version: u32,
    init: u32,
    padding: [u32; 8],
}

#[no_mangle]
pub static SA2ModInfo: ModInfo = ModInfo {
    version: 1,
    init: 0,
    padding: [0; 8],
};

#[naked]
unsafe fn kart_dlc_load_some_prs_thing_hook() {
    asm!("
        pushal
        push %eax
        push %ebx
        mov $0, %ecx
        call *%ecx
        add $$0x08, %esp
        popal
        push %ecx
        push %esi
        push %edi
        push 0x01defe20
        mov $1, %ecx
        jmp *%ecx
    "
    :
    : "i" (print_registers as *const fn() as u32), "i" (0x00799aa8)
    );
}

#[naked]
unsafe fn wrap_init_events() {
    asm!("
        pushal
        mov $0, %eax
        call *%eax
        popal
        ret
    "
    :
    : "i" (init_events as *const fn() as u32)
    );
}

fn init_texlist_with_textures(texlist: u32, texture: u32) {
    unsafe {
        asm!("
            mov $0, %eax
            mov $1, %ebx
            mov $2, %ecx
            call *%ecx
        "
        :
        : "r" (texture), "r" (texlist), "i" (0x0042fc30)
        : "eax", "ebx", "ecx"
        );
    }
}

extern "C" fn kart_initialize_data_pre_hook() {
    unsafe {
        if let Some(ref dlc_vec) = DLC_ARRAY {
            let selection = *(0x01d1b848 as *mut u32);
            if let Some(ref dlc_read) = dlc_vec.get(selection as usize) {
                *(0x01d97100 as *mut *const KartDlc) = &dlc_read.prs_data.kart_dlc;
                let set_ptr = dlc_read.prs_data.set_data.as_ptr() as u32;
                *(0x01d97104 as *mut u32) = set_ptr;
                *(0x01d97108 as *mut u32) = dlc_read.prs_data.set_data.len() as u32;
                let track_ptr = dlc_read.prs_data.track_data.as_ptr() as u32;
                *(0x01d9710c as *mut u32) = track_ptr;
                *(0x01d97110 as *mut u32) = dlc_read.prs_data.track_data.len() as u32;
                *(0x01d97054 as *mut u32) = dlc_read.prs_data.model_data.model_ptr;
            }
        }
        let func_ptr = 0x0061a3b0;
        (*(&func_ptr as *const i32 as *const extern "C" fn()))();
    }
}

extern "C" fn print_registers(ebx: u32, eax: u32) {
    println!("eax: 0x{:08x}", eax);
    println!("ebx: 0x{:08x}", ebx);
    let handle = ProcessHandle::open_current_process();
    match handle.read_copy::<u32>(eax) {
        Ok(v) => println!("*eax: 0x{:08x}", v),
        Err(s) => println!("error: {}", s),
    }
    match handle.read_copy::<u32>(ebx) {
        Ok(v) => println!("*ebx: 0x{:08x}", v),
        Err(s) => println!("error: {}", s),
    }

    unsafe {
        if let Some(ref dlc_vec) = DLC_ARRAY {
            let selection = *(0x01d1b848 as *mut u32);
            if let Some(ref dlc_read) = dlc_vec.get(selection as usize) {
                let texlist_addr = dlc_read.prs_data.model_data.texlist_ptr;
                let texture_addr = dlc_read.prs_data.model_data.texture.as_ptr() as u32;
                init_texlist_with_textures(texlist_addr, texture_addr);
                *(0x01d9705c as *mut u32) = texlist_addr;
            }
        }
    }
}

extern "C" fn init_events() {
    let num_events = match read_event_info() {
        Ok(val) => val,
        Err(e) => {
            println!("Error: {}", e);
            0
        },
    };
    unsafe {
        *(0x01a50220 as *mut u32) = 0;
        *(0x01a50224 as *mut u32) = 0;
        *(0x01d1b848 as *mut u32) = 0;
        *(0x01d1b84c as *mut u32) = num_events;
        *(0x01a501d8 as *mut u32) = 0x01d1c660;
    }
}

fn read_event_info() -> io::Result<u32> {
    let mut dlc_vec = Vec::new();
    for entry_res in fs::read_dir(DLC_PREFIX)? {
        let entry = entry_res?;
        let path = entry.path();
        // TODO: gracefully skip fails
        if !path.is_dir() {
            let file = File::open(path)?;
            let dlc_read = DlcData::from_vmu(file)?;
            let num_dlc = dlc_vec.len();
            unsafe {
                *((0x01d1c660 + 0xf3c * num_dlc + 0x4) as *mut u32) = 1;
                *((0x01d1c660 + 0xf3c * num_dlc + 0x18) as *mut u32) = dlc_read.dlc_type;
                *((0x01d1c660 + 0xf3c * num_dlc + 0x3c) as *mut [DlcText; 6]) = dlc_read.dlc_texts;
                *((0x01d1c660 + 0xf3c * num_dlc + 0x1c) as *mut [u32; 8]) = dlc_read.level_ids;
            }
            dlc_vec.push(dlc_read);
        }
    }
    let ret = dlc_vec.len();
    unsafe {
        DLC_ARRAY = Some(dlc_vec);
    }
    Ok(ret as u32)
}

#[no_mangle]
pub extern "C" fn Init(_path: u32, _helper_functions: u32) {
    let handle = ProcessHandle::open_current_process();
    handle.write_jump(0x00799aa0, kart_dlc_load_some_prs_thing_hook as *const fn()).unwrap();

    handle.write_call(0x0068c7c7, wrap_init_events as *const fn()).unwrap();
    handle.write_copy(0x00666f90, 0xc3u8).unwrap();

    handle.write_copy(0x00665547, 0xb).unwrap();

    handle.write_copy(0x0068c2da, 0x50).unwrap();
    handle.write_copy(0x0068c2e1, 0x50).unwrap();
    handle.write_copy(0x0068ab2a, 0x50).unwrap();

    handle.write_copy(0x0100acfc, kart_initialize_data_pre_hook as *const fn()).unwrap();

//    let file = File::open("resource/gd_PC/SAVEDATA/KartFZ.VMS").unwrap();
//    let dlc_read = DlcData::from_vmu(file).unwrap();
//
//    unsafe {
//        dlc = Some(dlc_read);
//
//        if let Some(ref mut dlc_read) = dlc {
//            *(0x01d1c678 as *mut u32) = dlc_read.dlc_type;
//            *(0x01d1c69c as *mut [DlcText; 6]) = dlc_read.dlc_texts;
//            *(0x01d1c67c as *mut [u32; 8]) = dlc_read.level_ids;
//
//            *(0x01d97100 as *mut *const KartDlc) = &dlc_read.prs_data.kart_dlc;
//            let set_ptr = dlc_read.prs_data.set_data.as_ptr() as u32;
//            *(0x01d97104 as *mut u32) = set_ptr;
//            *(0x01d97108 as *mut u32) = dlc_read.prs_data.set_data.len() as u32;
//            let track_ptr = dlc_read.prs_data.track_data.as_ptr() as u32;
//            *(0x01d9710c as *mut u32) = track_ptr;
//            *(0x01d97110 as *mut u32) = dlc_read.prs_data.track_data.len() as u32;
//            *(0x01d97054 as *mut u32) = dlc_read.prs_data.model_data.model_ptr;
//        }
//    }

//    unsafe {
//        *((0x01d1c67c) as *mut u32) = 0x46;
//    }

//    handle.write_copy(0x01d1c678, 0x3).unwrap();
//    handle.write_copy(0x01d1c664, 0x1).unwrap();

//    handle.write_data(0x01d1c91c, b"\tDLC name\x00").unwrap();
//    handle.write_data(0x01d1c99c, b"DLC type\x00").unwrap();
//    handle.write_data(0x01d1ca1c, b"DLC stage\x00").unwrap();
//    handle.write_data(0x01d1ca9c, b"DLC character\x00").unwrap();
//    handle.write_data(0x01d1cb1c, b"DLC description\x00").unwrap();
//
//    handle.write_data(0x01d1d858, b"\tDLC name2\x00").unwrap();
//
//    handle.write_copy(0x01d97100, 0x1d97070).unwrap();
//    handle.write_data(0x01d970b4, b"a_mine.adx\x00").unwrap();
//
//    let mut file = File::open("resource/gd_PC/setCartMini1.bin").unwrap();
//
//    unsafe {
//        let mut set_buf = Vec::new();
//        file.read_to_end(&mut set_buf).unwrap();
//        set_buf.swap(0, 3);
//        set_buf.swap(1, 2);
//        let set_ptr = set_buf.as_ptr() as u32;
//        *(0x01d97104 as *mut u32) = set_ptr;
//        *(0x01d97108 as *mut u32) = set_buf.len() as u32;
//        setfile = Some(set_buf);
//
//        let track_buf: Vec<u8> = iter::repeat(3).take(100).collect();
//        let track_ptr = track_buf.as_ptr() as u32;
//        *(0x01d9710c as *mut u32) = track_ptr;
//        *(0x01d97110 as *mut u32) = track_buf.len() as u32;
//        trackdata = Some(track_buf);
//    }
}
