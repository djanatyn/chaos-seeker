#[derive(Debug)]
pub struct MapTile(u8);
pub type MapRow = [MapTile; 64];

#[derive(Debug)]
pub struct StandardMap([MapRow; 64]);

impl StandardMap {
    fn parse(rom: &[u8], offset: &MapOffset) -> anyhow::Result<Self> {
        todo!()
    }
}

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
    pub const ROM_HEADER_SIZE: usize = 0x10;
    pub const BANK_SIZE: usize = 0x4000;

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

#[derive(Debug)]
struct MapPointerTable([MapOffset; 64]);

impl MapPointerTable {
    fn parse(rom: &[u8]) -> anyhow::Result<Self> {
        todo!()
    }
}

#[derive(Debug)]
pub struct MapAtlas([StandardMap; 64]);

impl MapAtlas {
    fn parse(rom: &[u8], table: &MapPointerTable) -> anyhow::Result<Self> {
        todo!()
    }
}
