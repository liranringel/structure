#[macro_use]
extern crate structure;

use std::os::raw::c_void;
use std::mem::transmute;
use std::io::ErrorKind;
use std::io::Cursor;


#[test]
fn pack() {
    assert_eq!(structure!("I").pack(3).unwrap(), vec![0, 0, 0, 3]);
}

#[test]
fn pack_into() {
    let mut v = Vec::new();
    structure!("I").pack_into(&mut v, 3).unwrap();
    assert_eq!(v, vec![0, 0, 0, 3]);
}

#[test]
fn unpack() {
    assert_eq!(structure!("I").unpack(&[0, 0, 0, 3]).unwrap(), (3, ));
}

#[test]
fn unpack_from() {
    assert_eq!(structure!("I").unpack_from(&mut Cursor::new(&[0, 0, 0, 3])).unwrap(), (3, ));
}

#[test]
fn pack_2_values() {
    assert_eq!(structure!("If").pack(6, 5.2).unwrap(), vec![0, 0, 0, 6, 64, 166, 102, 102]);
}

#[test]
fn unpack_2_values() {
    assert_eq!(structure!("If").unpack(&[0, 0, 0, 6, 64, 166, 102, 102]).unwrap(), (6, 5.2));
}

#[test]
fn pack_bool() {
    assert_eq!(structure!("?").pack(true).unwrap(), vec![1]);
    assert_eq!(structure!("?").pack(false).unwrap(), vec![0]);
}

#[test]
fn unpack_bool() {
    assert_eq!(structure!("?").unpack(&[1]).unwrap(), (true, ));
    assert_eq!(structure!("?").unpack(&[0]).unwrap(), (false, ));
}

#[test]
fn pack_big_endian() {
    assert_eq!(structure!("I").pack(1).unwrap(), vec![0, 0, 0, 1]);
    assert_eq!(structure!(">I").pack(1).unwrap(), vec![0, 0, 0, 1]);
    assert_eq!(structure!("!I").pack(1).unwrap(), vec![0, 0, 0, 1]);
}

#[test]
fn unpack_big_endian() {
    assert_eq!(structure!("I").unpack(&[0, 0, 0, 1]).unwrap(), (1, ));
    assert_eq!(structure!(">I").unpack(&[0, 0, 0, 1]).unwrap(), (1, ));
    assert_eq!(structure!("!I").unpack(&[0, 0, 0, 1]).unwrap(), (1, ));
}

#[test]
fn pack_little_endian() {
    assert_eq!(structure!("<I").pack(1).unwrap(), vec![1, 0, 0, 0]);
}

#[test]
fn unpack_little_endian() {
    assert_eq!(structure!("<I").unpack(&[1, 0, 0, 0]).unwrap(), (1, ));
}

#[test]
fn pack_native_endian() {
    let s = structure!("=I");
    if cfg!(target_endian = "little") {
        assert_eq!(s.pack(1).unwrap(), vec![1, 0, 0, 0]);
    } else {
        assert_eq!(s.pack(1).unwrap(), vec![0, 0, 0, 1]);
    }
}

#[test]
fn unpack_native_endian() {
    let s = structure!("=I");
    if cfg!(target_endian = "little") {
        assert_eq!(s.unpack(&[1, 0, 0, 0]).unwrap(), (1, ));
    } else {
        assert_eq!(s.unpack(&[0, 0, 0, 1]).unwrap(), (1, ));
    }
}

#[test]
fn pack_repeat_count() {
    assert_eq!(structure!("2B2?").pack(2, 3, true, false).unwrap(), vec![2, 3, 1, 0]);
}

#[test]
fn unpack_repeat_count() {
    assert_eq!(structure!("2B2?").unpack(&[2, 3, 1, 0]).unwrap(), (2, 3, true, false));
}

#[test]
fn pack_buffer() {
    assert_eq!(structure!("3s").pack(&[1, 2, 3]).unwrap(), vec![1, 2, 3]);
    assert_eq!(structure!("s").pack(&[4]).unwrap(), vec![4]);
    assert_eq!(structure!("0s").pack(&[]).unwrap(), vec![]);
    assert_eq!(structure!("2s").pack(&[5, 6, 7]).unwrap_err().kind(), ErrorKind::InvalidInput);
    assert_eq!(structure!("3s").pack(&[8, 9]).unwrap(), vec![8, 9, 0]);
}

#[test]
fn unpack_buffer() {
    assert_eq!(structure!("3s").unpack(&[1, 2, 3]).unwrap(), (vec![1, 2, 3], ));
    assert_eq!(structure!("s").unpack(&[4]).unwrap(), (vec![4], ));
    assert_eq!(structure!("0s").unpack(&[]).unwrap(), (vec![], ));
    assert_eq!(structure!("2s").unpack(&[5, 6, 7]).unwrap_err().kind(), ErrorKind::InvalidInput);
    assert_eq!(structure!("3s").unpack(&[8, 9]).unwrap_err().kind(), ErrorKind::InvalidInput);
}

#[test]
fn pack_fixed_buffer() {
    assert_eq!(structure!("3S").pack(&[1, 2, 3]).unwrap(), vec![1, 2, 3]);
    assert_eq!(structure!("S").pack(&[4]).unwrap(), vec![4]);
    assert_eq!(structure!("0S").pack(&[]).unwrap(), vec![]);
    assert_eq!(structure!("2S").pack(&[5, 6, 7]).unwrap_err().kind(), ErrorKind::InvalidInput);
    assert_eq!(structure!("3S").pack(&[8, 9]).unwrap_err().kind(), ErrorKind::InvalidInput);
}

#[test]
fn unpack_fixed_buffer() {
    assert_eq!(structure!("3S").unpack(&[1, 2, 3]).unwrap(), (vec![1, 2, 3], ));
    assert_eq!(structure!("S").unpack(&[4]).unwrap(), (vec![4], ));
    assert_eq!(structure!("0S").unpack(&[]).unwrap(), (vec![], ));
    assert_eq!(structure!("2S").unpack(&[5, 6, 7]).unwrap_err().kind(), ErrorKind::InvalidInput);
    assert_eq!(structure!("3S").unpack(&[8, 9]).unwrap_err().kind(), ErrorKind::InvalidInput);
}

#[test]
fn pack_and_unpack_pointer() {
    let num: u32 = 6;
    let num2: u32 = 7;
    assert_eq!(structure!("=P").pack(unsafe { transmute::<&u32, *const c_void>(&num) }).unwrap(), unsafe { transmute::<&u32, [u8; 8]>(&num) });
    let packed_pointers = structure!("=PP").pack(unsafe { transmute::<&u32, *const c_void>(&num) }, unsafe { transmute::<&u32, *const c_void>(&num2) }).unwrap();
    let (p, p2) = structure!("=PP").unpack(packed_pointers).unwrap();
    assert_eq!(p, unsafe { transmute::<&u32, *const c_void>(&num) });
    assert_eq!(p2, unsafe { transmute::<&u32, *const c_void>(&num2) });
}

#[test]
fn pack_and_unpack_typed_pointer() {
    let num: u32 = 6;
    let num2: u8 = 7;
    let num3: u8 = 8;
    let s = structure!("=P<u32>2P<u8>");
    let packed_pointers = s.pack(&num as *const u32, &num2 as *const u8, &num3 as *const u8).unwrap();
    let (p, p2, p3) = s.unpack(packed_pointers).unwrap();
    assert_eq!(p, &num as *const u32);
    assert_eq!(p2, &num2 as *const u8);
    assert_eq!(p3, &num3 as *const u8);
}
