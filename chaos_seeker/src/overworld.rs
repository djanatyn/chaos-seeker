use std::path::Path;

use image::{imageops, RgbImage};

#[derive(Debug)]
pub struct WorldMap {
    pub map: Map,
    pub properties: TilePropertyTable,
}

impl WorldMap {
    pub fn parse(rom: &[u8]) -> anyhow::Result<Self> {
        let table = PointerTable::parse(rom)?;
        let (map, _length) = TileReader::parse_map(rom, &table)?;
        let properties = TilePropertyTable::load(rom)?;

        Ok(WorldMap { map, properties })
    }

    pub fn get(&self, (x, y): (usize, usize)) -> Option<&Tile> {
        self.map.0.get(y).and_then(|row| row.get(x))
    }

    pub fn lookup(&self, tile: &Tile) -> Option<&TileProperties> {
        self.properties.lookup(tile)
    }
}

/// https://www.romhacking.net/documents/663/
/// (Anomie’s disassembly of Final Fantasy 1 for the NES)
///
/// ;;; ROM offsets 000010 - 00400f
/// ; world map tile data
/// ; 2 bytes per tile:
/// ;  +0: bits 6-7: 01 = chime required
/// ;                10 = caravan required
/// ;                11 = use floater to raise airship
/// ;      bit  5:   Can dock ship here
/// ;      bit  4:   Forest walking effect
/// ;      bit  3:   Can't land airship here
/// ;      bit  2:   Can't move ship here
/// ;      bit  1:   Can't move canoe here
/// ;      bit  0:   Can't walk here
/// ;  +1: If bit 7 is set, bits 0-6 specify the teleport index
/// ;      Otherwise, bit 6: battle here
/// ;                 bits 0-1: 00 = normal battle
/// ;                           01 = river battle
/// ;                           10 = ocean battle
/// ;                           11 = river battle
/// 8000: 06 40 0e 89 0e 89 0e 40 1e 40 0e 40 0e 40 0b 42
#[derive(Debug)]
pub struct TileProperties {
    /// What can move onto this tile?
    traversal: u8,
    /// What kinds of encounters / teleports happen here?
    encounter: u8,
}

/// Types of battle scenes encoded in encounter property.
enum BattleScene {
    Normal,
    River,
    Ocean,
}

/// Index of a teleport encounter property.
struct TeleportIndex(u8);

/// Requirements for moving onto a world map tile.
#[derive(Debug)]
enum TileRequirements {
    None,
    Chime,
    Floater,
    Caravan,
}

impl TileProperties {
    // traversal bitmasks
    const REQUIREMENTS: u8 = 0b1100_0000;
    const DOCK: u8 = 0b0010_0000;
    const FOREST: u8 = 0b0001_0000;
    const AIRSHIP_BLOCKED: u8 = 0b0000_1000;
    const SHIP_BLOCKED: u8 = 0b0000_0100;
    const CANOE_BLOCKED: u8 = 0b0000_0010;
    const WALK_BLOCKED: u8 = 0b0000_0001;

    // encounter bitmasks
    const IS_TELEPORT: u8 = 0b1000_0000;
    const TELEPORT_INDEX: u8 = 0b0111_1111;
    const IS_BATTLE: u8 = 0b0100_0000;
    const BATTLE_SCENE: u8 = 0b0000_0011;

    pub fn walk_allowed(&self) -> bool {
        (self.traversal & Self::WALK_BLOCKED) == 0
    }

    pub fn canoe_allowed(&self) -> bool {
        (self.traversal & Self::CANOE_BLOCKED) == 0
    }

    pub fn ship_allowed(&self) -> bool {
        (self.traversal & Self::SHIP_BLOCKED) == 0
    }

    pub fn airship_allowed(&self) -> bool {
        (self.traversal & Self::AIRSHIP_BLOCKED) == 0
    }

    pub fn requirements(&self) -> TileRequirements {
        match self.traversal & Self::REQUIREMENTS {
            0b0000_0000 => TileRequirements::None,
            0b0100_0000 => TileRequirements::Chime,
            0b1000_0000 => TileRequirements::Caravan,
            0b1100_0000 => TileRequirements::Floater,
            _ => unreachable!(),
        }
    }

    pub fn teleport_index(&self) -> Option<TeleportIndex> {
        (self.encounter & Self::IS_TELEPORT != 0)
            .then_some(TeleportIndex(self.encounter & Self::TELEPORT_INDEX))
    }

    pub fn battle_scene(&self) -> Option<BattleScene> {
        let not_teleport: bool = self.encounter & Self::IS_TELEPORT == 0;
        let is_battle: bool = self.encounter & Self::IS_BATTLE != 0;
        (not_teleport && is_battle).then_some(match self.encounter & Self::BATTLE_SCENE {
            0b00 => BattleScene::Normal,
            0b01 => BattleScene::River,
            0b10 => BattleScene::Ocean,
            0b11 => BattleScene::River,
            _ => {
                unreachable!()
            }
        })
    }

    fn parse(rom: &[u8], offset: usize) -> anyhow::Result<Self> {
        if let [traversal, encounter] = rom[offset..offset + 2] {
            Ok(TileProperties {
                traversal,
                encounter,
            })
        } else {
            Err(anyhow::anyhow!("failed to parse"))
        }
    }

    fn color(&self) -> [u8; 3] {
        match self {
            tile if tile.walk_allowed() => [0, 173, 0],
            tile if tile.canoe_allowed() => [7, 173, 189],
            tile if tile.ship_allowed() => [96, 204, 255],
            tile if tile.airship_allowed() => [255, 111, 32],
            _ => [170, 170, 170],
        }
    }
}

/// 0x0010 -> 0x400F
#[derive(Debug)]
pub struct TilePropertyTable([TileProperties; 128]);

impl TilePropertyTable {
    const WORLD_TILE_PROPERTIES_OFFSET: usize = 0x0010;
    const TILE_PROPERTY_SIZE: usize = 2;
    const NUM_TILES: usize = 128;

    fn load(rom: &[u8]) -> anyhow::Result<Self> {
        let start: usize = Self::WORLD_TILE_PROPERTIES_OFFSET;
        let table: [TileProperties; 128] = (0..Self::NUM_TILES)
            .map(|idx| {
                let offset = start + (idx * Self::TILE_PROPERTY_SIZE);
                TileProperties::parse(rom, offset)
            })
            .collect::<anyhow::Result<Vec<TileProperties>>>()?
            .try_into()
            .map_err(|e: Vec<TileProperties>| {
                anyhow::anyhow!("did not find 128 world map tile properties: {}", e.len())
            })?;

        Ok(TilePropertyTable(table))
    }

    fn lookup(&self, tile: &Tile) -> Option<&TileProperties> {
        self.0.get(tile.0 as usize)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tile(u8);
impl Tile {
    fn color(&self, table: &TilePropertyTable) -> [u8; 3] {
        if let Some(properties) = table.lookup(self) {
            properties.color()
        } else {
            [0, 0, 0]
        }
    }
}

type MapRow = [Tile; 256];

#[derive(Debug)]
pub struct Map([MapRow; 256]);

impl Map {
    pub fn save_image(&self, path: &Path, table: &TilePropertyTable) -> anyhow::Result<()> {
        let mut img = RgbImage::new(256, 256);

        for (y, row) in self.0.iter().enumerate() {
            for (x, tile) in row.iter().enumerate() {
                img.put_pixel(x as u32, y as u32, image::Rgb(tile.color(table)));
            }
        }

        // let scaled = imageops::resize(&img, 256 * 4, 256 * 4, imageops::FilterType::Nearest);
        // scaled.save(path)?;
        img.save(path)?;

        Ok(())
    }

    pub fn save_pathfinding_image(
        &self,
        solution: &[(u8, u8)],
        path: &Path,
        table: &TilePropertyTable,
    ) -> anyhow::Result<()> {
        let mut img = RgbImage::new(256, 256);

        for (y, row) in self.0.iter().enumerate() {
            for (x, tile) in row.iter().enumerate() {
                img.put_pixel(x as u32, y as u32, image::Rgb(tile.color(table)));
            }
        }

        let last = solution.len().saturating_sub(1).max(1);

        for (idx, &(x, y)) in solution.iter().enumerate() {
            let t = (idx as u32 * 255 / last as u32) as u8;
            let color = image::Rgb([t, 255 - t, 255]);
            img.put_pixel(x as u32, y as u32, color);
        }

        let scaled = imageops::resize(&img, 256 * 4, 256 * 4, imageops::FilterType::Nearest);
        scaled.save(path)?;
        // img.save(path)?;

        Ok(())
    }
}

/// https://datacrystal.tcrf.net/wiki/Final_Fantasy/World_map_data
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord)]
struct CpuAddress(u16);

/// iNES ROM-relative overworld row address.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, PartialOrd, Ord)]
struct RomOffset(usize);

impl CpuAddress {
    /// Since this bank will loaded into the NES memory map at readable
    /// address $8000, all memory pointers in the ROM must point to the
    /// row's ROM file address + 0x3FF0. To translate a memory pointer back
    /// into a ROM file address, you subtract 0x3FF0 instead.
    pub const ROM_OFFSET: u16 = 0x3FF0;

    fn to_rom_address(&self) -> RomOffset {
        RomOffset((self.0 - Self::ROM_OFFSET).into())
    }
}

/// 0x4010 in the ROM has 256 16-bit integers representing overworld map
/// offsets.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
struct RowPointer {
    /// Offset of the entry in the ROM.
    entry_offset: usize,
    /// Index to the overworld map data the entry points to.
    map_data_offset: CpuAddress,
}

impl RowPointer {
    fn parse(rom: &[u8], offset: usize) -> anyhow::Result<Self> {
        // read two u8 bytes into LE u16
        let entry_offset = u16::from_le_bytes(rom[offset..(offset + 2)].try_into()?);

        Ok(RowPointer {
            entry_offset: offset,
            map_data_offset: CpuAddress(entry_offset),
        })
    }
}

/// All parsed overworld map entries.
#[derive(Debug)]
struct PointerTable([RowPointer; 256]);

impl PointerTable {
    const ROW_POINTER_SIZE: usize = 2;

    /// Parse the world map entry table from the ROM.
    fn parse(rom: &[u8]) -> anyhow::Result<Self> {
        let starting_offset: usize = 0x4010;
        let table: [RowPointer; 256] = (0..256)
            .map(|i| RowPointer::parse(rom, starting_offset + (Self::ROW_POINTER_SIZE * i)))
            .collect::<anyhow::Result<Vec<RowPointer>>>()?
            .try_into()
            .map_err(|e| anyhow::anyhow!("did not find 256 world map row pointers: {e:?}"))?;

        Ok(PointerTable(table))
    }

    fn get(&self, index: usize) -> Option<RowPointer> {
        self.0.get(index).cloned()
    }
}

#[derive(Debug)]
enum ReaderState {
    /// A single byte ranging 0x00 to 0x7F describes a single tile of a
    /// specific value.
    Null,
    /// A byte ranging 0x80 to 0xFE is the same as its value subtracted by
    /// 0x80 (or exclusive-or'ed by 0x80), but the immediately following
    /// byte indicates repetition of the tile value.
    ///
    /// A repetition byte value of 0x01 means no repetition, 0x0x02 means
    /// repeat once, 0x0x03 means repeat twice, 0x04 means repeat three
    /// times, and so on. 0x00 means to repeat 256 times, so that it can
    /// fill an entire row. A repetition value of 0xFF is prohibited.
    Repeat(Tile),
    /// A byte value of 0xFF after the row's final tile terminates the row data.
    Complete,
}

#[derive(Debug)]
struct TileReader {
    accum: MapRow,
    index: usize,
    state: ReaderState,
}

#[derive(Debug)]
struct ParsedRow {
    starting_offset: usize,
    length: usize,
    tiles: [Tile; 256],
}

impl TileReader {
    fn new() -> Self {
        TileReader {
            accum: [const { Tile(0) }; 256],
            index: 0,
            state: ReaderState::Null,
        }
    }

    fn reset(&mut self) {
        self.index = 0;
        self.state = ReaderState::Null;
    }

    fn step(&mut self, tile: &u8) -> anyhow::Result<&ReaderState> {
        match self.state {
            ReaderState::Null => match tile {
                0x00..=0x7F => {
                    // normal single tile
                    self.accum[self.index] = Tile(*tile);
                    self.index += 1;
                }
                0x80..=0xFE => {
                    // start repetition
                    let tile = Tile(*tile - 0x80);
                    self.accum[self.index] = tile;
                    self.state = ReaderState::Repeat(tile);
                    self.index += 1;
                }
                0xFF => {
                    // finish row
                    if self.index != 256 {
                        Err(anyhow::anyhow!("early end of row"))?;
                    } else {
                        self.state = ReaderState::Complete;
                    }
                }
            },
            ReaderState::Repeat(repeated_tile) => match tile {
                0x00 => {
                    // fill entire row with tile
                    self.accum = [repeated_tile; 256];
                    self.index = 256;
                    self.state = ReaderState::Null;
                }
                0xFF => {
                    // invalid value
                    Err(anyhow::anyhow!("prohibited value follows repeated tile"))?;
                }
                repetitions => {
                    // partial repeated tile
                    let start = self.index - 1;
                    let end = start + (*repetitions as usize);
                    self.accum[start..end].fill(repeated_tile);
                    self.index = end;
                    self.state = ReaderState::Null;
                }
            },
            ReaderState::Complete => Err(anyhow::anyhow!("already completed"))?,
        };

        Ok(&self.state)
    }

    fn parse_row(&mut self, rom: &[u8], pointer: &RomOffset) -> anyhow::Result<ParsedRow> {
        self.reset();
        let input = &rom[pointer.0..];
        for (offset, byte) in input.iter().enumerate() {
            if let ReaderState::Complete = self.step(byte)? {
                // return Ok((self.accum, offset + 1));
                return Ok(ParsedRow {
                    starting_offset: pointer.0,
                    length: offset + 1,
                    tiles: self.accum,
                });
            };
        }

        Err(anyhow::anyhow!("reached end of input"))?
    }

    fn parse_map(rom: &[u8], table: &PointerTable) -> anyhow::Result<(Map, usize)> {
        let mut map = [[const { Tile(1) }; 256]; 256];
        let mut length = 0;
        let mut fsm = TileReader::new();

        for (idx, output_row) in map.iter_mut().enumerate() {
            let row_offset: RomOffset = table.0[idx].map_data_offset.to_rom_address();
            let parsed_row = fsm.parse_row(rom, &row_offset)?;
            *output_row = parsed_row.tiles;
            length += parsed_row.length;
        }

        Ok((Map(map), length))
    }
}

#[test]
fn test_parse_row() -> anyhow::Result<()> {
    let mut fsm = TileReader::new();
    let ParsedRow { length, tiles, .. } = fsm.parse_row(&[0x97, 0x00, 0xFF], &RomOffset(0x00))?;

    let mut ocean = tiles.to_vec();

    assert_eq!(length, 3);
    assert_eq!(ocean.len(), 256);
    ocean.dedup();
    assert_eq!(&ocean, &[Tile(0x17)]);

    Ok(())
}

/// - offset -   0 1  2 3  4 5  6 7  8 9  A B  C D  E F  0123456789ABCDEF
/// 0x00004010  0082 0382 0682 0982 0c82 0f82 1282 1582  ................
#[test]
fn test_parse_world_map_entry() -> anyhow::Result<()> {
    let bytes = [0x00, 0x82];
    let parsed = RowPointer::parse(&bytes, 0)?;
    let expected = RowPointer {
        entry_offset: 0,
        map_data_offset: CpuAddress(0x8200),
    };

    assert_eq!(parsed, expected);

    Ok(())
}

#[test]
fn test_parse_overworld() -> anyhow::Result<()> {
    let rom = std::fs::read("ff1.nes")?;
    let table = PointerTable::parse(&rom)?;
    let (map, length) = TileReader::parse_map(&rom, &table)?;

    let properties = TilePropertyTable::load(&rom)?;
    map.save_image(&PathBuf::from("map.png"), &properties)?;

    insta::assert_compact_debug_snapshot!(table);
    insta::assert_snapshot!(length);
    insta::assert_compact_debug_snapshot!(properties);
    insta::assert_compact_debug_snapshot!(map);

    Ok(())
}
