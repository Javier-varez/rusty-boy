use std::path::Path;

use goblin::elf::Elf;
use rustc_demangle::demangle;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn parse_leb128(data: &[u8]) -> (u128, &[u8]) {
    const WORD_MASK: u8 = 0x7f;
    const NEXT_BIT: u8 = 0x80;

    let mut value: u128 = 0;
    let mut offset = 0;

    if data.is_empty() {
        return (value, data);
    }

    loop {
        let cur_word = data[offset] & WORD_MASK;
        let cont = (data[offset] & NEXT_BIT) != 0;
        value |= (cur_word as u128) << (7 * offset);
        offset += 1;

        if !cont {
            break;
        }
        if offset == data.len() {
            break;
        }
    }

    (value, &data[offset..])
}

fn resolve_symbol<'a>(elf: &goblin::elf::Elf<'a>, symbol_ptr: u32) -> Option<&'a str> {
    elf.syms.iter().find_map(|s| {
        elf.strtab.get_at(s.st_name).and_then(|name| {
            if !name.starts_with("$") && (symbol_ptr as u64) == s.st_value {
                Some(name)
            } else {
                None
            }
        })
    })
}

fn parse_stack_sizes(elf: &goblin::elf::Elf, mut data: &[u8]) -> Vec<(String, u128)> {
    assert!(!elf.is_64);

    let mut stack_sizes = vec![];

    while !data.is_empty() {
        let symbol_pointer =
            u32::from_le_bytes(data[..std::mem::size_of::<u32>()].try_into().unwrap());
        data = &data[std::mem::size_of::<u32>()..];

        let thumb_pointer = symbol_pointer | 1;

        let symbol = resolve_symbol(elf, symbol_pointer)
            .or_else(|| resolve_symbol(elf, thumb_pointer))
            .unwrap_or("??");

        let symbol = demangle(symbol).to_string();

        let (stack_size, rest) = parse_leb128(data);
        data = rest;

        stack_sizes.push((symbol, stack_size));
    }

    stack_sizes.sort_by(|(_, s1), (_, s2)| s2.cmp(s1));
    stack_sizes
}

pub fn check_stack_sizes(elf_path: &Path) -> Result<Vec<(String, u128)>> {
    let data = std::fs::read(elf_path)?;
    let elf = Elf::parse(&data)?;

    let stack_sizes = elf
        .section_headers
        .iter()
        .find(|s| {
            elf.shdr_strtab
                .get_at(s.sh_name)
                .is_some_and(|n| n == ".stack_sizes")
        })
        .expect(".stack_sizes not found");

    let begin = stack_sizes.sh_offset as usize;
    let end = (stack_sizes.sh_offset + stack_sizes.sh_size) as usize;
    Ok(parse_stack_sizes(&elf, &data[begin..end]))
}

#[cfg(test)]
pub mod test {
    use super::*;
    #[test]
    fn test_leb128_decoding() {
        let data = &[0xE5, 0x8E, 0x26, 123, 34, 53];
        let (value, rest) = parse_leb128(data);

        assert_eq!(value, 624485);
        assert_eq!(rest, &data[3..]);
    }
}
