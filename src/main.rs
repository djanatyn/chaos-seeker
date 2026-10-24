use std::{thread::sleep, time::Duration};

use tetanes_core::{input::JoypadBtnState, mem::Read, prelude::*};
use tracing::{self, instrument};

/// Progress through Final Fantasy 1.
#[derive(Debug)]
pub enum GameProgress {
    /// Stuck in the intro.
    BootIntro,
    /// Main menu after intro.
    BootMenu,
    /// Character creation for new game.
    CharacterCreation,
    /// Unimplemented.
    Unknown,
}

/// State of the game.
#[derive(Debug)]
pub struct GameState {
    pub frame: u64,
    pub progress: GameProgress,
    control_deck: ControlDeck,
}

impl GameState {
    pub fn new(control_deck: ControlDeck) -> Self {
        Self {
            frame: 0,
            progress: GameProgress::BootIntro,
            control_deck,
        }
    }

    pub fn is_running(&self) -> bool {
        self.control_deck.is_running()
    }

    pub fn tick(&mut self) -> anyhow::Result<u64> {
        let time = self.control_deck.clock_frame()?;
        self.frame += 1;

        self.progress = self.handler();
        Ok(time)
    }

    pub fn set_button(&mut self, btn: JoypadBtnState, state: bool) {
        self.control_deck
            .joypad_mut(Player::One)
            .set_button(btn, state)
    }

    /// Take a screenshot of the game.
    pub fn screenshot(&mut self, filename: &str) {
        // Process frame buffer (e.g. by rendering it to the screen)
        let image = image::ImageBuffer::<image::Rgba<u8>, &[u8]>::from_raw(
            Ppu::WIDTH,
            Ppu::HEIGHT,
            self.control_deck.frame_buffer(),
        );

        if let Some(image) = image {
            let _ = image.save(&filename);
            // tracing::warn!(filename, "saved screenshot");
        }
    }

    /// Toggle the state of a button.
    pub fn toggle(&mut self, button: JoypadBtnState) {
        let p1 = self.control_deck.joypad_mut(Player::One);
        p1.set_button(button, !p1.button(button));
    }

    /// Release a button.
    pub fn _release(&mut self, button: JoypadBtnState) {
        self.control_deck
            .joypad_mut(Player::One)
            .set_button(button, false);
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        self.control_deck.bus_mut().read(addr)
    }

    /// Take a screenshot using a name based on the current frame.
    fn _snap(&mut self) {
        let filename = format!("screenshots/{:04}.png", self.frame);
        self.screenshot(&filename);
    }

    /// Advance game state.
    fn handler(&mut self) -> GameProgress {
        let task = match self.progress {
            GameProgress::BootIntro => intro::handler,
            GameProgress::BootMenu => menu::handler,
            GameProgress::CharacterCreation => char_create::handler,
            GameProgress::Unknown => todo!(),
        };

        task(self)
    }
}

mod intro {
    //! Intro cutscene handler.
    //!
    //! We wait a few frames before hitting the start button.

    use super::*;

    /// How long to wait before pressing buttons.
    pub const WAIT_FRAMES: u64 = 20;
    /// Address to check to see if we're at the main menu.
    pub const INTRO_PROGRESS_ADDRESS: u16 = 0x26;
    /// Value we expect to see if we're at the main menu.
    pub const INTRO_COMPLETE_VALUE: u8 = 0x10;

    #[instrument]
    fn at_main_menu(state: &mut GameState) -> bool {
        state.read(INTRO_PROGRESS_ADDRESS) == INTRO_COMPLETE_VALUE
    }

    /// Wait until the cutscene has played a bit, then hit start until we reach the main menu.
    #[instrument]
    pub fn handler(state: &mut GameState) -> GameProgress {
        // wait for intro to play before hitting buttons
        if state.frame <= WAIT_FRAMES {
            return GameProgress::BootIntro;
        }

        if at_main_menu(state) {
            tracing::warn!(state.frame, "reached menu");
            state.screenshot("menu.png");
            state.set_button(JoypadBtnState::START, false);

            return GameProgress::BootMenu;
        }

        // toggle start button until we reach the main menu
        state.toggle(JoypadBtnState::START);
        GameProgress::BootIntro
    }
}

mod menu {
    use super::*;

    /// Address showing cursor position in menu.
    ///
    /// Cursor can point at CONTINUE, or NEW GAME.
    pub const CURSOR_ADDRESS: u16 = 0x62;
    /// Value of CURSOR_ADDRESS when pointing at NEW GAME.
    pub const NEW_GAME_VALUE: u8 = 0x1;

    fn cursor_new_game(state: &mut GameState) -> bool {
        state.read(CURSOR_ADDRESS) == NEW_GAME_VALUE
    }

    fn at_character_creation(state: &mut GameState) -> bool {
        state.read(char_create::SLOT_SELECTED_ADDRESS) == 0x0
    }

    /// Move cursor to new game and hit start.
    #[instrument]
    pub fn handler(state: &mut GameState) -> GameProgress {
        if at_character_creation(state) {
            tracing::warn!(state.frame, "reached char creation");
            return GameProgress::CharacterCreation;
        }

        if cursor_new_game(state) {
            // hit start for new game
            state.toggle(JoypadBtnState::START);
        } else {
            // move cursor down to new game
            state.toggle(JoypadBtnState::DOWN);
        }

        GameProgress::BootMenu
    }
}

mod char_create {
    use std::collections::HashMap;

    use super::*;

    pub const SLOT_SELECTED_ADDRESS: u16 = 0x67;
    pub const CLASS_SELECTED_ADDRESS: u16 = 0x300;

    #[derive(Debug)]
    pub enum Mode {
        SelectName,
        SelectClass,
    }

    impl Mode {
        fn current(state: &mut GameState) -> Option<Self> {
            match state.read(MODE_ADDRESS) {
                MODE_CLASS_SELECT_VALUE => Some(Mode::SelectClass),
                MODE_NAME_SELECT_VALUE => Some(Mode::SelectName),
                _ => None,
            }
        }
    }

    pub const MODE_ADDRESS: u16 = 0x1a;
    pub const MODE_CLASS_SELECT_VALUE: u8 = 0x8;
    pub const MODE_NAME_SELECT_VALUE: u8 = 0x4;

    pub const PARTY_COMPLETE_ADDRESS: u16 = 0x11;
    pub const PARTY_COMPLETE_VALUE: u8 = 0x0;

    #[derive(Debug, PartialEq)]
    pub enum StartingClass {
        Fighter,
        Thief,
        BlackBelt,
        RedMage,
        WhiteMage,
        BlackMage,
    }

    use StartingClass::*;

    pub const STARTING_PARTY: [(StartingClass, &str); 4] = [
        (Fighter, "LDA"),
        (BlackBelt, "ADC"),
        (WhiteMage, "JMP"),
        (BlackMage, "CMP"),
    ];

    impl StartingClass {
        fn selection(value: u8) -> Option<StartingClass> {
            match value {
                0x0 => Some(Fighter),
                0x1 => Some(Thief),
                0x2 => Some(BlackBelt),
                0x3 => Some(RedMage),
                0x4 => Some(WhiteMage),
                0x5 => Some(BlackMage),
                _ => None,
            }
        }

        /// Current party member slot selected class.
        fn current_class(state: &mut GameState) -> Option<Self> {
            Self::selection(state.read(CLASS_SELECTED_ADDRESS))
        }
    }

    pub type Character = char;
    pub type Coordinate = u16;
    pub type Value = u8;

    ///    character map         name            pos
    /// -------------------  -----------   ---------------
    /// A B C D E F G H I J: 0x8A - 0x93 | 0x0000 - 0x0009
    /// K L M N O P Q R S T: 0x94 - 0x9D | 0x0100 - 0x0109
    /// U V W X Y Z ' , .  : 0x9E - 0xFF | 0x0200 - 0x0209
    /// 0 1 2 3 4 5 6 7 8 9: 0x80 - 0x89 | 0x0300 - 0x0309
    /// a b c d e f g h i j: 0xA4 - 0xAD | 0x0400 - 0x0409
    /// k l m n o p q r s t: 0xAE - 0xB7 | 0x0500 - 0x0509
    /// u v w x y z - … ! ?: 0xB8 - 0xC5 | 0x0600 - 0x0609
    pub const CHARACTER_MAP: [(Character, (Coordinate, Value)); 40] = [
        // 0x0000
        ('A', (0x0000, 0x8A)),
        ('B', (0x0001, 0x8B)),
        ('C', (0x0002, 0x8C)),
        ('D', (0x0003, 0x8D)),
        ('E', (0x0004, 0x8E)),
        ('F', (0x0005, 0x8F)),
        ('G', (0x0006, 0x90)),
        ('H', (0x0007, 0x91)),
        ('I', (0x0008, 0x92)),
        ('J', (0x0009, 0x93)),
        // 0x0100
        ('K', (0x0100, 0x94)),
        ('L', (0x0101, 0x95)),
        ('M', (0x0102, 0x96)),
        ('N', (0x0103, 0x97)),
        ('O', (0x0104, 0x98)),
        ('P', (0x0105, 0x99)),
        ('Q', (0x0106, 0x9A)),
        ('R', (0x0107, 0x9B)),
        ('S', (0x0108, 0x9C)),
        ('T', (0x0109, 0x9D)),
        // 0x0200
        ('U', (0x0200, 0x9E)),
        ('V', (0x0201, 0x9F)),
        ('W', (0x0202, 0xA0)),
        ('X', (0x0203, 0xA1)),
        ('Y', (0x0204, 0xA2)),
        ('Z', (0x0205, 0xA3)),
        ('"', (0x0206, 0xA4)),
        (',', (0x0207, 0xA5)),
        ('.', (0x0208, 0xA6)),
        (' ', (0x0209, 0xFF)),
        // 0x0300
        ('0', (0x0300, 0x80)),
        ('1', (0x0301, 0x81)),
        ('2', (0x0302, 0x82)),
        ('3', (0x0303, 0x83)),
        ('4', (0x0304, 0x84)),
        ('5', (0x0305, 0x85)),
        ('6', (0x0306, 0x86)),
        ('7', (0x0307, 0x87)),
        ('8', (0x0308, 0x88)),
        ('9', (0x0309, 0x89)),
        // 0x0400
        // ...
    ];

    /// Current party member slot (0x0 -> 0x4)
    fn current_slot(state: &mut GameState) -> Option<u8> {
        let value = state.read(SLOT_SELECTED_ADDRESS);
        (0..4).contains(&value).then_some(value)
    }

    fn party_complete(state: &mut GameState) -> bool {
        state.read(PARTY_COMPLETE_ADDRESS) == PARTY_COMPLETE_VALUE
    }

    /// Navigate towards specified character.
    ///
    /// Returns:
    /// - true if at desired character,
    /// - false if not there yet.
    fn navigate(_state: &mut GameState, target: char) -> bool {
        let map = HashMap::from(CHARACTER_MAP);
        if let Some((_coord, _value)) = map.get(&target) {
            todo!()
        } else {
            false
        }
    }

    pub fn handler(state: &mut GameState) -> GameProgress {
        if party_complete(state) {
            state.toggle(JoypadBtnState::START);
            return GameProgress::Unknown;
        }

        match Mode::current(state) {
            Some(Mode::SelectName) => {
                let current_class =
                    StartingClass::current_class(state).expect("failed to find current class");
                dbg!(current_class);
            }
            Some(Mode::SelectClass) => {
                let slot = current_slot(state).expect("failed to identify party slot");
                let (desired_class, desired_name) = STARTING_PARTY
                    .get(slot as usize)
                    .expect("slot out of bounds");
                let current_class =
                    StartingClass::current_class(state).expect("failed to find current class");

                if current_class == *desired_class {
                    state.toggle(JoypadBtnState::A);
                } else {
                    state.toggle(JoypadBtnState::B);
                }

                dbg!(slot, desired_class, desired_name, current_class);
            }
            None => panic!("invalid character creation mode"),
        }

        GameProgress::CharacterCreation
    }
}

mod overworld {
    use std::path::{Path, PathBuf};

    use image::{imageops, RgbImage};
    use strum::FromRepr;

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
    struct TileProperties {
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

        fn walk_allowed(&self) -> bool {
            (self.traversal & Self::WALK_BLOCKED) == 0
        }

        fn canoe_allowed(&self) -> bool {
            (self.traversal & Self::CANOE_BLOCKED) == 0
        }

        fn ship_allowed(&self) -> bool {
            (self.traversal & Self::SHIP_BLOCKED) == 0
        }

        fn airship_allowed(&self) -> bool {
            (self.traversal & Self::AIRSHIP_BLOCKED) == 0
        }

        fn requirements(&self) -> TileRequirements {
            match self.traversal & Self::REQUIREMENTS {
                0b0000_0000 => TileRequirements::None,
                0b0100_0000 => TileRequirements::Chime,
                0b1000_0000 => TileRequirements::Caravan,
                0b1100_0000 => TileRequirements::Floater,
                _ => unreachable!(),
            }
        }

        fn teleport_index(&self) -> Option<TeleportIndex> {
            (self.encounter & Self::IS_TELEPORT != 0)
                .then_some(TeleportIndex(self.encounter & Self::TELEPORT_INDEX))
        }

        fn battle_scene(&self) -> Option<BattleScene> {
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
    struct TilePropertyTable([TileProperties; 128]);

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
            self.0.get(*tile as usize)
        }
    }

    #[derive(Debug, Clone, Copy, FromRepr, PartialEq)]
    #[repr(u8)]
    enum Tile {
        Grass1 = 0x00,
        Castle1LowerLeft = 0x01,
        Castle1LowerRight = 0x02,
        ForestTopLeft = 0x03,
        ForestTop = 0x04,
        ForestTopRight = 0x05,
        GrassBottomRight = 0x06,
        GrassBottom = 0x07,
        GrassBottomLeft = 0x08,
        Castle1TopLeft = 0x09,
        Castle1TopRight = 0x0A,
        Castle2TopLeft = 0x0B,
        Castle2TopRight = 0x0C,
        TowerofIllusionTop = 0x0D,
        Grotto1 = 0x0E,
        DockVerticalLeft = 0x0F,
        MountainTopLeft = 0x10,
        MountainTop = 0x11,
        MountainTopRight = 0x12,
        ForestLeft = 0x13,
        ForestBottom1 = 0x14,
        ForestRight = 0x15,
        GrassRight = 0x16,
        Ocean = 0x17,
        GrassLeft = 0x18,
        Castle1MiddleLeft = 0x19,
        Castle1MiddleRight = 0x1A,
        Castle2BottomLeft = 0x1B,
        Castle2BottomRight = 0x1C,
        TowerofIllusionBottom = 0x1D,
        TowerofIllusionShadow = 0x1E,
        DockVerticalRight = 0x1F,
        MountainLeft = 0x20,
        Mountain = 0x21,
        MountainRight = 0x22,
        ForestBottomLeft = 0x23,
        ForestBottom2 = 0x24,
        ForestBottomRight = 0x25,
        GrassTopRight = 0x26,
        GrassTop = 0x27,
        GrassTopLeft = 0x28,
        Castle2BottomLeft1 = 0x29,
        Castle2BottomRight1 = 0x2A,
        Grotto2 = 0x2B,
        VillageFenceTopLeft = 0x2C,
        VillageFenceTop = 0x2D,
        VillageFenceTopRight = 0x2E,
        Grotto3 = 0x2F,
        MountainBottomLeft = 0x30,
        MountainBottom = 0x31,
        Grotto4 = 0x32,
        MountainBottomRight = 0x33,
        Grotto5 = 0x34,
        Grotto6 = 0x35,
        Desert1 = 0x36,
        Desert2 = 0x37,
        Castle2BottomLeft2 = 0x38,
        Castle2BottomRight2 = 0x39,
        Grotto7 = 0x3A,
        VillageFenceTopLeftExterior = 0x3B,
        VillageFenceTopLeftInterior = 0x3C,
        VillageAreaGray = 0x3D,
        VillageFenceTopRightInterior = 0x3E,
        VillageFenceTopRightExterior = 0x3F,
        RiverTopLeft = 0x40,
        RiverTopRight = 0x41,
        DesertTopLeft = 0x42,
        DesertTopRight = 0x43,
        River = 0x44,
        Desert = 0x45,
        Waterfall = 0x46,
        CastleRuinsTopLeft = 0x47,
        CastleRuinsTopRight = 0x48,
        Village1 = 0x49,
        Village2 = 0x4A,
        VillageFenceMiddleLeft = 0x4B,
        Village3 = 0x4C,
        Village4 = 0x4D,
        Village5 = 0x4E,
        VillageFenceMiddleRight = 0x4F,
        RiverBottomLeft = 0x50,
        RiverBottomRight = 0x51,
        DesertBottomLeft = 0x52,
        DesertBottomRight = 0x53,
        TallGrass = 0x54,
        Swamp = 0x55,
        CastleRuinsBottomLeft = 0x56,
        CastleRuinsBottomMiddleLeft = 0x57,
        CastleRuinsBottomMiddleRight = 0x58,
        CastleRuinsBottomRight = 0x59,
        Village6 = 0x5A,
        VillageFenceMiddleLeft2 = 0x5B,
        VillageFenceBottomLeft2 = 0x5C,
        Village7 = 0x5D,
        VillageFenceBottomRight2 = 0x5E,
        VillageFenceMiddleRight2 = 0x5F,
        TallGrassTopLeft = 0x60,
        TallGrassTopRight = 0x61,
        SwampTopLeft = 0x62,
        SwampTopRight = 0x63,
        VolcanoTopLeft = 0x64,
        VolcanoTopRight = 0x65,
        Hole1 = 0x66,
        Hole2 = 0x67,
        Hole3 = 0x68,
        Hole4 = 0x69,
        Hole5 = 0x6A,
        VillageFenceMiddleLeft3 = 0x6B,
        Hole6 = 0x6C,
        Village8 = 0x6D,
        Hole7 = 0x6E,
        VillageFenceMiddleRight3 = 0x6F,
        TallGrassBottomLeft = 0x70,
        TallGrassBottomRight = 0x71,
        SwampBottomLeft = 0x72,
        SwampBottomRight = 0x73,
        VolcanoBottomLeft = 0x74,
        VolcanoBottomRight = 0x75,
        Grass2 = 0x76,
        DockBottomRight = 0x77,
        DockBottom = 0x78,
        DockBottomLeft = 0x79,
        DockTopLeft = 0x7A,
        VillageFenceBottomLeft = 0x7B,
        VillageWall1 = 0x7C,
        VillageWall2 = 0x7D,
        VillageWall3 = 0x7E,
        VillageFenceBottomRight = 0x7F,
    }

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
    struct Map([MapRow; 256]);

    impl Map {
        fn save_image(&self, path: &Path, table: &TilePropertyTable) -> anyhow::Result<()> {
            let mut img = RgbImage::new(256, 256);

            for (y, row) in self.0.iter().enumerate() {
                for (x, tile) in row.iter().enumerate() {
                    img.put_pixel(x as u32, y as u32, image::Rgb(tile.color(table)));
                }
            }

            let scaled = imageops::resize(&img, 256 * 4, 256 * 4, imageops::FilterType::Nearest);

            scaled.save(path)?;
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
                accum: [const { Tile::Grass1 }; 256],
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
                        self.accum[self.index] =
                            Tile::from_repr(*tile).expect("failed to parse tile");
                        self.index += 1;
                    }
                    0x80..=0xFE => {
                        // start repetition
                        let tile = Tile::from_repr(*tile - 0x80).expect("failed to parse tile");
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
            let mut map = [[const { Tile::Grass1 }; 256]; 256];
            let mut offset = 0;
            let mut fsm = TileReader::new();

            for (idx, output_row) in map.iter_mut().enumerate() {
                let row_offset: RomOffset = table.0[idx].map_data_offset.to_rom_address();
                let parsed_row = fsm.parse_row(rom, &row_offset)?;
                *output_row = parsed_row.tiles;
                offset += parsed_row.length;
            }

            Ok((Map(map), offset))
        }
    }

    #[test]
    fn test_parse_row() -> anyhow::Result<()> {
        let mut fsm = TileReader::new();
        let ParsedRow { length, tiles, .. } =
            fsm.parse_row(&[0x97, 0x00, 0xFF], &RomOffset(0x00))?;

        let mut ocean = tiles.to_vec();

        assert_eq!(length, 3);
        assert_eq!(ocean.len(), 256);
        ocean.dedup();
        assert_eq!(&ocean, &[Tile::Ocean]);

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
        insta::assert_compact_debug_snapshot!(map);

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_thread_names(true)
        .with_max_level(tracing::Level::WARN)
        .init();

    let _ = std::fs::create_dir("screenshots");

    let mut control_deck = ControlDeck::new();
    control_deck.load_rom_path("ff1.nes")?;

    let mut game = GameState::new(control_deck);

    while game.is_running() {
        let _ = game.tick();
        sleep(Duration::from_millis(50));
    }

    Ok(())
}
