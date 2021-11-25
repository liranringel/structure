#![recursion_limit = "128"]

extern crate proc_macro;

use std::mem;
use std::os::raw::c_void;
use std::string::String;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;

fn new_ident(s: &str) -> Ident {
    Ident::new(s, Span::call_site())
}

#[proc_macro]
pub fn structure(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let token_string = input.to_string();
    let format = trim_quotes(&token_string);
    let struct_name = new_ident(&format_to_struct_name(format));
    let (values, endianness) = format_to_values(&format);
    let (args, fn_decl_args, args_types) = build_args_list(&values);
    let endianness = match endianness {
        Endianness::Native => {
            if cfg!(target_endian = "little") {
                quote!(LittleEndian)
            } else {
                quote!(BigEndian)
            }
        }
        Endianness::LittleEndian => quote!(LittleEndian),
        Endianness::BigEndian => quote!(BigEndian),
    };
    let size = calc_size(&values);
    let pack_fn = build_pack_fn(&args, &fn_decl_args, size);
    let pack_into_fn = build_pack_into_fn(&values, &fn_decl_args, &endianness);
    let unpack_fn = build_unpack_fn(&args_types, size);
    let unpack_from_fn = build_unpack_from_fn(&values, &args, &args_types, &endianness);
    let size_fn = build_size_fn(size);
    let output = quote! {{
        #[derive(Debug)]
        #[allow(non_camel_case_types)]
        struct #struct_name;
        #[allow(unused_imports)]
        use std::io::{Result, Write, Read, Error, ErrorKind, Cursor};
        #[allow(unused_imports)]
        use std::os::raw::c_void;
        #[allow(unused_imports)]
        use structure::byteorder::{WriteBytesExt, ReadBytesExt, BigEndian, LittleEndian};

        #[allow(unused)] static TRUE_BUF: &[u8] = &[1];
        #[allow(unused)] static FALSE_BUF: &[u8] = &[0];

        impl #struct_name {
            #pack_fn
            #pack_into_fn
            #unpack_fn
            #unpack_from_fn
            #size_fn
        }

        #struct_name // Create structure instance
    }};

    output.into()
}

#[derive(PartialEq)]
enum Endianness {
    Native,
    LittleEndian,
    BigEndian,
}

fn build_pack_fn(args: &TokenStream, fn_decl_args: &TokenStream, size: usize) -> TokenStream {
    quote! {
        #[allow(unused)]
        fn pack(&self, #fn_decl_args) -> Result<Vec<u8>> {
            let mut wtr = Vec::with_capacity(#size);
            self.pack_into(&mut wtr, #args)?;
            Ok(wtr)
        }
    }
}

fn build_pack_into_fn(values: &[StructValue], fn_decl_args: &TokenStream, endianness: &TokenStream) -> TokenStream {
    // Pack each argument
    let mut writings = quote!();
    let mut arg_index = 0;
    for value in values {
        let writing = match *value.kind() {
            ValueKind::Number | ValueKind::Boolean | ValueKind::Pointer => {
                let mut tokens = quote!();
                for _ in 0..value.repeat() {
                    arg_index += 1;
                    let current_arg = new_ident(&format!("_{}", arg_index));
                    if *value.kind() == ValueKind::Number {
                        let byteorder_fn = new_ident(&format!("write_{}", value.type_name()));
                        match value.type_name().as_str() {
                            "u8" | "i8" => {
                                tokens.extend(quote! {wtr.#byteorder_fn(#current_arg)?;});
                            }
                            _ => {
                                tokens.extend(quote! {wtr.#byteorder_fn::<#endianness>(#current_arg)?;});
                            }
                        }
                    } else if *value.kind() == ValueKind::Boolean {
                        tokens.extend(quote! {
                            let buf = if #current_arg { TRUE_BUF } else { FALSE_BUF };
                            wtr.write(buf)?;
                        });
                    } else {
                        let size = mem::size_of::<usize>();
                        let integer_type = new_ident(&format!("u{}", size * 8));
                        let byteorder_fn = new_ident(&format!("write_u{}", size * 8));
                        tokens.extend(quote! {
                            let v = #current_arg as #integer_type;
                            wtr.#byteorder_fn::<#endianness>(v)?;
                        });
                    }
                }
                tokens
            }
            ValueKind::Buffer | ValueKind::FixedBuffer => {
                arg_index += 1;
                let current_arg = new_ident(&format!("_{}", arg_index));
                let buffer_length = value.repeat();
                let length_check = if *value.kind() == ValueKind::Buffer {
                    // If the type is `ValueKind::Buffer`, and the given buffer is smaller than the
                    // size determined in the format, the rest will be filled with zeros.
                    quote! { #current_arg.len() <= #buffer_length }
                } else {
                    quote! { #current_arg.len() == #buffer_length }
                };
                let mut tokens = quote! {
                    if !(#length_check) {
                        let msg = format!("Buffer length does not match the format \
                            (buffer size in format: {}, actual size: {}", #current_arg.len(), #buffer_length);
                        return Err(Error::new(ErrorKind::InvalidInput, msg));
                    }
                    wtr.write_all(#current_arg)?;
                };
                if *value.kind() == ValueKind::Buffer {
                    tokens.extend(quote! {
                        if #current_arg.len() != #buffer_length {
                            wtr.write_all(&vec![0; (#buffer_length - #current_arg.len())])?;
                        }
                    });
                }
                tokens
            }
            ValueKind::Padding => {
                let number = value.repeat();
                quote! {
                    wtr.write_all(&[0; #number])?;
                }
            }
        };
        writings.extend(writing);
    }

    quote! {
        #[allow(unused)]
        fn pack_into<T: Write>(&self, wtr: &mut T, #fn_decl_args) -> Result<()> {
            #writings
            Ok(())
        }
    }
}

fn build_unpack_fn(args_types: &TokenStream, size: usize) -> TokenStream {
    quote! {
        #[allow(unused)]
        fn unpack<T: AsRef<[u8]>>(&self, buf: T) -> Result<(#args_types,)> {
            if buf.as_ref().len() != #size {
                let msg = format!("Buffer length does not match the format \
                    (format size: {}, actual size: {}", #size, buf.as_ref().len());
                return Err(Error::new(ErrorKind::InvalidInput, msg))
            }
            let mut rdr = Cursor::new(buf);
            self.unpack_from(&mut rdr)
        }
    }
}

fn build_unpack_from_fn(values: &[StructValue], args: &TokenStream, args_types: &TokenStream, endianness: &TokenStream) -> TokenStream {
    let mut readings = quote!();
    let mut arg_index = 0;
    for value in values {
        let reading = match *value.kind() {
            ValueKind::Number | ValueKind::Boolean | ValueKind::Pointer => {
                let mut tokens = quote!();
                for _ in 0..value.repeat() {
                    arg_index += 1;
                    let current_arg = new_ident(&format!("_{}", arg_index));
                    if *value.kind() == ValueKind::Number {
                        let byteorder_fn = new_ident(&format!("read_{}", value.type_name()));
                        match value.type_name().as_str() {
                            "u8" | "i8" => {
                                tokens.extend(quote! { let #current_arg = rdr.#byteorder_fn()?;});
                            }
                            _ => {
                                tokens.extend(quote! { let #current_arg = rdr.#byteorder_fn::<#endianness>()?;});
                            }
                        }
                    } else if *value.kind() == ValueKind::Boolean {
                        tokens.extend(quote! {
                            let #current_arg = rdr.read_u8()?;
                            let #current_arg = #current_arg != 0; // 0 is false
                        });
                    } else {
                        let pointer_type: TokenStream = value.type_name().parse().unwrap();
                        let size = mem::size_of::<usize>();
                        let byteorder_fn = new_ident(&format!("read_u{}", size * 8));
                        tokens.extend(quote! {
                            let #current_arg = {
                                let v = rdr.#byteorder_fn::<#endianness>()?;
                                v as #pointer_type
                            };
                        });
                    }
                }
                tokens
            }
            ValueKind::Buffer | ValueKind::FixedBuffer => {
                arg_index += 1;
                let current_arg = new_ident(&format!("_{}", arg_index));
                let buffer_length = value.repeat();
                quote! {
                    let mut #current_arg = vec![0; #buffer_length];
                    rdr.read_exact(&mut #current_arg)?;
                }
            }
            ValueKind::Padding => {
                let number = value.repeat();
                quote! {
                    rdr.read_exact(&mut [0; #number])?;
                }
            }
        };
        readings.extend(reading);
    }

    quote! {
        #[allow(unused)]
        fn unpack_from<T: Read>(&self, rdr: &mut T) -> Result<(#args_types,)> {
            #readings
            Ok((#args,))
        }
    }.into()
}

/// Build the args list, the function declaration args list and the type list
fn build_args_list(values: &[StructValue]) -> (TokenStream, TokenStream, TokenStream) {
    let mut args = vec![];
    let mut fn_decl_args = vec![];
    let mut args_types = vec![];
    let mut arg_index = 0;
    for v in values {
        match *v.kind() {
            ValueKind::Padding => continue,
            ValueKind::Buffer | ValueKind::FixedBuffer => {
                arg_index += 1;
                let arg_name = new_ident(&format!("_{}", arg_index));
                args.push(arg_name.clone());
                let arg_type: TokenStream = v.type_name().parse().unwrap();
                fn_decl_args.push(quote! { #arg_name: #arg_type });
                args_types.push(quote! { Vec<u8> });
            }
            _ => {
                for _ in 0..v.repeat() {
                    arg_index += 1;
                    let arg_name = new_ident(&format!("_{}", arg_index));
                    args.push(arg_name.clone());
                    let arg_type: TokenStream = v.type_name().parse().unwrap();
                    fn_decl_args.push(quote! { #arg_name: #arg_type });
                    args_types.push(quote! { #arg_type });
                }
            }
        }
    }
    (quote!(#(#args),*).into(),
     quote!(#(#fn_decl_args),*).into(),
     quote!(#(#args_types),*).into())
}

fn build_size_fn(size: usize) -> TokenStream {
    quote! {
        #[allow(unused)]
        fn size(&self) -> usize {
            #size
        }
    }.into()
}

fn calc_size(values: &[StructValue]) -> usize {
    let mut size = 0;
    for v in values {
        if v.type_name().starts_with("*") {
            mem::size_of::<*const c_void>();
        }
        let type_size = match v.type_name().as_str() {
            "i8" => mem::size_of::<i8>(),
            "&[u8]" | "u8" => mem::size_of::<u8>(),
            "bool" => 1,
            "i16" => mem::size_of::<i16>(),
            "u16" => mem::size_of::<u16>(),
            "i32" => mem::size_of::<i32>(),
            "u32" => mem::size_of::<u32>(),
            "i64" => mem::size_of::<i64>(),
            "u64" => mem::size_of::<u64>(),
            "f32" => mem::size_of::<f32>(),
            "f64" => mem::size_of::<f64>(),
            t if t.starts_with("*") => mem::size_of::<usize>(),
            _ => panic!("Unknown type: '{}'", v.type_name()),
        };
        size += type_size * v.repeat();
    }
    size
}

fn format_to_struct_name(format: &str) -> String {
    format!("Struct_{}", format.replace("?", "Bool")
        .replace("=", "Native")
        .replace("<", "LittleEndian")
        .replace(">", "")
        .replace("!", ""))
}

/// Return the format string without the endianness, and the endianness
fn format_endianness(format: &str) -> (&str, Endianness) {
    let first_char = format.chars().nth(0);
    let endianness = match first_char {
        Some('=') => Endianness::Native,
        Some('<') => Endianness::LittleEndian,
        _ => Endianness::BigEndian,
    };
    let mut chars = format.chars();
    match chars.next() {
        Some('=') | Some('<') | Some('>') | Some('!') => (chars.as_str(), endianness),
        _ => (format, endianness),
    }
}

fn char_to_type(c: char) -> (&'static str, ValueKind) {
    match c {
        'b' => ("i8", ValueKind::Number),
        'B' => ("u8", ValueKind::Number),
        '?' => ("bool", ValueKind::Boolean),
        'h' => ("i16", ValueKind::Number),
        'H' => ("u16", ValueKind::Number),
        'i' => ("i32", ValueKind::Number),
        'I' => ("u32", ValueKind::Number),
        'q' => ("i64", ValueKind::Number),
        'Q' => ("u64", ValueKind::Number),
        'f' => ("f32", ValueKind::Number),
        'd' => ("f64", ValueKind::Number),
        's' => ("&[u8]", ValueKind::Buffer),
        'S' => ("&[u8]", ValueKind::FixedBuffer),
        'P' => ("*const c_void", ValueKind::Pointer),
        'x' => ("u8", ValueKind::Padding),
        _ => panic!("Unknown format: '{}'", c),
    }
}

fn format_to_values(format: &str) -> (Vec<StructValue>, Endianness) {
    let (format, endianness) = format_endianness(format);
    let mut values = vec![];
    let mut chars = format.chars().peekable();
    let mut repeat_str = String::new();
    while let Some(c) = chars.next() {
        if c.is_digit(10) {
            repeat_str.push(c);
        } else {
            let (type_name, kind) = char_to_type(c);
            let mut type_name = type_name.to_owned();
            if kind == ValueKind::Pointer {
                // Parse pointer type
                if endianness != Endianness::Native {
                    panic!("Pointer can be used only if the endianness is native. \
                            To change the endianness to native, start the format with '='");
                }
                if let Some(&'<') = chars.peek() {
                    chars.next();
                    let mut pointer_type_name = String::new();
                    loop {
                        let c = chars.next();
                        if c == None {
                            panic!("Pointer type must end with '>'");
                        } else if c == Some('>') {
                            if pointer_type_name.is_empty() {
                                panic!("Pointer type cannot be empty");
                            }
                            type_name = format!("*const {}", pointer_type_name);
                            break;
                        } else {
                            pointer_type_name.push(c.unwrap());
                        }
                    }
                }
            }
            let mut repeat = 1;
            if !repeat_str.is_empty() {
                repeat = repeat_str.parse().expect("not a number");
                repeat_str.clear();
            }
            values.push(StructValue::new(type_name, repeat, kind));
        }
    }
    if !repeat_str.is_empty() {
        panic!("No format character is followed by the number {}", repeat_str);
    }
    (values, endianness)
}

#[derive(PartialEq)]
enum ValueKind {
    Number,
    Boolean,
    Buffer,
    FixedBuffer,
    Pointer,
    Padding,
}

struct StructValue {
    type_name: String,
    repeat: usize,
    kind: ValueKind
}

impl StructValue {
    fn new(type_name: String, repeat: usize, kind: ValueKind) -> StructValue {
        StructValue { type_name: type_name, repeat: repeat, kind: kind }
    }
    fn type_name(&self) -> &String {
        &self.type_name
    }
    fn repeat(&self) -> usize {
        self.repeat
    }
    fn kind(&self) -> &ValueKind {
        &self.kind
    }
}

fn trim_quotes(input: &str) -> &str {

    if input.chars().nth(0) != Some('"') && input.chars().last() != Some('"') || input.len() < 2 {
        panic!("structure!() macro takes a literal string as an argument");
    }
    &input[1..(input.len()-1)]
}
