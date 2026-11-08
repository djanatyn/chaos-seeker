use image::{ImageBuffer, Rgba, RgbaImage, imageops};

use crate::overworld::TilePropertyTable;

pub const STANDARD_MAP_COUNT: usize = 60;
pub const MAP_DIMENSIONS: usize = 64;

#[derive(Debug, Clone)]
pub struct MapTile(u8);

impl MapTile {
    fn color(&self, table: &TilePropertyTable) -> [u8; 3] {
        if let Some(properties) = table.lookup(self.0) {
            properties.color()
        } else {
            [0, 0, 0]
        }
    }
}

pub type Map = [MapTile; MAP_DIMENSIONS.pow(2)];

/// Encoded map offsets:
/// > Of the 16-bit value, bits 0-13 are the pointer
/// > within the bank and bits 14-15 select the bank.
///
/// We use 0x3FFF as a bitmask for the pointer, and shift right 14 bits for the
/// bank.
///
/// Pointers are offsets from CPU address 0x8000. For the ROM, they're offsets
/// from the base bank address.
///
/// Bank values from bits 14-15 are added to the base bank (4):
/// - 0x3FFF (0b00...) selects bank 4
/// - 0x4000 (0b01...) selects bank 5 (4 + 1)
/// - 0x8000 (0b11...) selects bank 6 (4 + 2)
#[derive(Debug)]
struct MapOffset(u16);

impl MapOffset {
    pub const SIZE: usize = 2;
    pub const ROM_HEADER_SIZE: usize = 0x10;
    pub const BANK_SIZE: usize = 0x4000;

    pub const STARTING_BANK: usize = 0x4;
    pub const STARTING_OFFSET: usize =
        Self::ROM_HEADER_SIZE + (Self::BANK_SIZE * Self::STARTING_BANK);

    fn parse(rom: &[u8], offset: usize) -> Self {
        MapOffset(u16::from_le_bytes([rom[offset], rom[offset + 1]]))
    }

    /// bits 14-15 select the bank
    fn bank(&self) -> usize {
        4 + (self.0 >> 14) as usize
    }

    /// bits 0-13 are the pointer within the bank
    fn pointer(&self) -> usize {
        (self.0 & 0x3FFF).into()
    }

    /// Calculate ROM offset using bank and pointer encoding.
    fn to_rom_offset(&self) -> usize {
        Self::ROM_HEADER_SIZE + (self.bank() * Self::BANK_SIZE) + self.pointer()
    }
}

#[test]
fn test_map_offsets() {
    {
        // first offset of bank 4
        let offset = MapOffset(0x0080);
        assert_eq!(offset.bank(), 4);
        assert_eq!(offset.pointer(), 0x80);
        assert_eq!(offset.to_rom_offset(), 0x10090);
    }
    {
        // last offset of bank 4
        let offset = MapOffset(0x3FFF);
        assert_eq!(offset.bank(), 4);
        assert_eq!(offset.pointer(), 0x3FFF);
        assert_eq!(offset.to_rom_offset(), 0x1400F);
    }
    {
        // first offset of bank 5
        let offset = MapOffset(0x4000);
        assert_eq!(offset.bank(), 5);
        assert_eq!(offset.pointer(), 0x00);
        assert_eq!(offset.to_rom_offset(), 0x14010);
    }
    {
        // last offset of bank 5
        let offset = MapOffset(0x7FFF);
        assert_eq!(offset.bank(), 5);
        assert_eq!(offset.pointer(), 0x3FFF);
        assert_eq!(offset.to_rom_offset(), 0x1800F);
    }
}

/// Contains MapOffset(0), which is invalid!
#[derive(Debug)]
struct MapPointerTable([MapOffset; STANDARD_MAP_COUNT]);

impl MapPointerTable {
    fn parse(rom: &[u8]) -> anyhow::Result<Self> {
        let table: [MapOffset; STANDARD_MAP_COUNT] = std::array::from_fn(|i| {
            let offset = MapOffset::STARTING_OFFSET + MapOffset::SIZE * i;
            MapOffset::parse(rom, offset)
        });

        Ok(MapPointerTable(table))
    }
}

#[test]
fn test_map_pointer_table() -> anyhow::Result<()> {
    let rom = std::fs::read("../ff1.nes")?;
    let table = MapPointerTable::parse(&rom)?;

    insta::assert_compact_debug_snapshot!(table);

    Ok(())
}

#[derive(Debug)]
pub struct StandardMap(Map);

impl StandardMap {
    fn parse(rom: &[u8], offset: &MapOffset) -> anyhow::Result<Self> {
        Ok(StandardMap(Parser::new().parse_map(rom, offset)?))
    }

    pub fn scaled_image(
        &self,
        scale: u32,
        table: &TilePropertyTable,
    ) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let size = MAP_DIMENSIONS as u32;
        let mut img = RgbaImage::new(size, size);

        for (idx, tile) in self.0.iter().enumerate() {
            let (x, y) = ((idx as u32).rem_euclid(size), (idx as u32).div_euclid(size));
            let [r, g, b] = tile.color(table);
            img.put_pixel(x as u32, y as u32, Rgba([r, g, b, 255]));
        }

        imageops::resize(
            &img,
            size * scale,
            size * scale,
            imageops::FilterType::Nearest,
        )
    }

    pub fn image(&self, table: &TilePropertyTable) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        let size = MAP_DIMENSIONS as u32;
        let mut img = RgbaImage::new(size, size);

        for (idx, tile) in self.0.iter().enumerate() {
            let (x, y) = ((idx as u32).rem_euclid(size), (idx as u32).div_euclid(size));
            let [r, g, b] = tile.color(table);
            img.put_pixel(x as u32, y as u32, Rgba([r, g, b, 255]));
        }

        img
    }
}

#[derive(Debug)]
pub enum ParserState {
    Null,
    Repeat(MapTile),
    Complete,
}

#[derive(Debug)]
pub struct Parser {
    pub accum: Map,
    pub index: usize,
    pub state: ParserState,
}

impl Parser {
    fn new() -> Self {
        Parser {
            accum: [const { MapTile(0) }; MAP_DIMENSIONS.pow(2)],
            index: 0,
            state: ParserState::Null,
        }
    }

    fn reset(&mut self) {
        self.index = 0;
        self.state = ParserState::Null;
    }

    fn step(&mut self, tile: &u8) -> anyhow::Result<&ParserState> {
        match self.state {
            ParserState::Null => match tile {
                0x00..=0x7F => {
                    self.accum[self.index] = MapTile(*tile);
                    self.index += 1;
                }
                0x80..=0xFE => {
                    self.accum[self.index] = MapTile(*tile & 0x7F);
                    self.state = ParserState::Repeat(MapTile(*tile & 0x7F));
                    self.index += 1;
                }
                0xFF => {
                    if self.index != MAP_DIMENSIONS.pow(2) {
                        Err(anyhow::anyhow!("early end of map"))?;
                    } else {
                        self.state = ParserState::Complete;
                    }
                }
            },
            ParserState::Repeat(ref repeated_tile) => {
                let repetitions: usize = match tile {
                    0x00 => 256,
                    repetitions => *repetitions as usize,
                };

                let start = self.index - 1;
                let end = start + repetitions;
                dbg!((&self.index, &repetitions, start, end));
                if end > MAP_DIMENSIONS.pow(2) {
                    Err(anyhow::anyhow!(
                        "repetition count out of range: {repetitions:#04x} ({repetitions})"
                    ))?
                }

                self.accum[start..end].fill(repeated_tile.clone());
                self.index = end;
                self.state = ParserState::Null;
            }
            ParserState::Complete => Err(anyhow::anyhow!("already completed"))?,
        };

        Ok(&self.state)
    }

    fn parse_map(mut self, rom: &[u8], offset: &MapOffset) -> anyhow::Result<Map> {
        let input = &rom[offset.to_rom_offset()..];

        for byte in input.iter() {
            if let ParserState::Complete = self.step(byte)? {
                return Ok(self.accum);
            }
        }

        Err(anyhow::anyhow!("reached end of input"))
    }
}

#[test]
fn test_parse_maps() -> anyhow::Result<()> {
    let rom = std::fs::read("../ff1.nes")?;
    let table = MapPointerTable::parse(&rom)?;

    let maps: Vec<StandardMap> = table
        .0
        .iter()
        .map(|offset| StandardMap::parse(&rom, offset))
        .collect::<anyhow::Result<Vec<StandardMap>>>()?;

    insta::assert_compact_debug_snapshot!(maps);

    let properties = TilePropertyTable::load(&rom)?;

    for (idx, map) in maps.iter().enumerate() {
        map.scaled_image(4, &properties)
            .save(format!("../images/map_{idx:02}.png"))?
    }

    Ok(())
}

#[derive(Debug)]
pub struct MapAtlas([StandardMap; STANDARD_MAP_COUNT]);

impl MapAtlas {
    fn parse(rom: &[u8], table: &MapPointerTable) -> anyhow::Result<Self> {
        todo!()
    }
}
