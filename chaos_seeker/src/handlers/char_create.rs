use crate::state::{CharacterCreationState, GameProgress, GameState, HandlerOutput};
use std::{cmp::Ordering, collections::HashMap};
use tetanes_core::{input::JoypadBtnState, mem::Read};

/// What slot is the party member currently in?
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot(usize);

impl Slot {
    const ADDRESS: u16 = 0x67;

    fn current(state: &mut GameState) -> Option<Self> {
        match state.read(Self::ADDRESS) {
            0x00 => Some(Slot(0)),
            0x10 => Some(Slot(1)),
            0x20 => Some(Slot(2)),
            0x30 => Some(Slot(3)),
            value => panic!("invalid current slot offset: {value:#04x}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    SelectName,
    SelectClass,
}

impl Mode {
    const MODE_ADDRESS: u16 = 0x1a;
    const MODE_CLASS_SELECT_VALUE: u8 = 0x8;
    const MODE_NAME_SELECT_VALUE: u8 = 0x4;

    fn current(state: &mut GameState) -> Option<Self> {
        match state.read(Self::MODE_ADDRESS) {
            Self::MODE_CLASS_SELECT_VALUE => Some(Mode::SelectClass),
            Self::MODE_NAME_SELECT_VALUE => Some(Mode::SelectName),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
    (Fighter, "LDA "),
    (BlackBelt, "ADC "),
    (WhiteMage, "JMP "),
    (BlackMage, "CMP "),
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
    fn current_class(state: &mut GameState, slot: &Slot) -> Option<Self> {
        Self::selection(state.read(CLASS_SELECTION_ADDRESS + (slot.0 as u16 * SLOT_OFFSET)))
    }
}

pub const CLASS_SELECTION_ADDRESS: u16 = 0x300;
pub const SLOT_OFFSET: u16 = 0x10;

pub type Character = char;
pub type Coordinate = u16;
pub type Value = u8;

#[derive(Debug)]
pub struct NameCoordinate(u16);

impl NameCoordinate {
    const ROW_ADDRESS: u16 = 0x64;
    const COL_ADDRESS: u16 = 0x65;
    const SIZE: usize = 2;

    fn current(state: &mut GameState) -> Self {
        NameCoordinate(u16::from_le_bytes([
            state.read(Self::ROW_ADDRESS),
            state.read(Self::COL_ADDRESS),
        ]))
    }
}

#[derive(Debug)]
struct CurrentName([u8; 4]);

impl CurrentName {
    const ADDRESS: u16 = 0x5c;
    const SIZE: usize = 4;

    fn parse(state: &mut GameState) -> Self {
        CurrentName([
            state.read(Self::ADDRESS + 0),
            state.read(Self::ADDRESS + 1),
            state.read(Self::ADDRESS + 2),
            state.read(Self::ADDRESS + 3),
        ])
    }

    fn next_coordinate(&self, desired_name: &str) -> Option<NameCoordinate> {
        let map = HashMap::from(CHARACTER_MAP); // TODO: LazyLock
        for (idx, desired_character) in desired_name.chars().enumerate() {
            if let Some((coord, desired_value)) = map.get(&desired_character) {
                match self.0[idx] {
                    // name[idx] is set correctly
                    current_value if current_value == *desired_value => continue,
                    // name[idx] is empty
                    0xFF => return Some(NameCoordinate(*coord)),
                    // name[idx] is non-empty and invalid
                    _ => {
                        dbg!(&self);
                        dbg!((coord, desired_value));
                        dbg!(idx);
                        panic!("broken name")
                    }
                }
            } else {
                panic!("unable to find character in map")
            }
        }

        None
    }
}

///    character map         name            pos
/// -------------------  -----------   ---------------
/// A B C D E F G H I J: 0x8A - 0x93 | 0x0000 - 0x0009
/// K L M N O P Q R S T: 0x94 - 0x9D | 0x0100 - 0x0109
/// U V W X Y Z ' , .  : 0x9E - 0xFF | 0x0200 - 0x0209
/// 0 1 2 3 4 5 6 7 8 9: 0x80 - 0x89 | 0x0300 - 0x0309
/// a b c d e f g h i j: 0xA4 - 0xAD | 0x0400 - 0x0409
/// k l m n o p q r s t: 0xAE - 0xB7 | 0x0500 - 0x0509
/// u v w x y z - … ! ?: 0xB8 - 0xC5 | 0x0600 - 0x0609
/// TODO: LazyLock
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

pub const PARTY_COMPLETE_ADDRESS: u16 = 0x11;
pub const PARTY_COMPLETE_VALUE: u8 = 0x0;

fn party_complete(state: &mut GameState) -> bool {
    state.read(PARTY_COMPLETE_ADDRESS) == PARTY_COMPLETE_VALUE
}

/// Navigate towards specified character coordinate.
///
/// Returns the buttons to hold to navigate towards the character.
/// Returns None if the target coordinate is currently selected.
fn navigate(state: &mut GameState, target: NameCoordinate) -> Option<JoypadBtnState> {
    let current = NameCoordinate::current(state);

    let [current_col, current_row] = current.0.to_le_bytes();
    let [target_col, target_row] = target.0.to_le_bytes();

    dbg!((&current_row, &current_col));
    dbg!((&target_row, target_col));

    match target_row.cmp(&current_row) {
        Ordering::Greater => Some(JoypadBtnState::DOWN),
        Ordering::Less => Some(JoypadBtnState::UP),
        Ordering::Equal => match target_col.cmp(&current_col) {
            Ordering::Greater => Some(JoypadBtnState::RIGHT),
            Ordering::Less => Some(JoypadBtnState::LEFT),
            Ordering::Equal => None,
        },
    }
}

pub fn handler(state: &mut GameState) -> HandlerOutput {
    let mode = Mode::current(state);
    let current_slot: Option<Slot> = Slot::current(state);
    let complete: bool = party_complete(state);

    dbg!((&complete, &mode, &current_slot, &state.progress));

    if let GameProgress::CharacterCreation(previous_state) = state.progress {
        if complete {
            return HandlerOutput {
                progress: GameProgress::CharacterCreation(CharacterCreationState {
                    party_complete: complete,
                    mode,
                    ..previous_state
                }),
                buttons: state.tap(JoypadBtnState::A),
            };
        }

        let slot = current_slot.expect("failed to retrieve slot");
        let (desired_class, desired_name) = STARTING_PARTY.get(slot.0).expect("slot out of bounds");
        let current_class = StartingClass::current_class(state, &slot);

        match mode {
            Some(Mode::SelectName) => {
                let current_name = dbg!(CurrentName::parse(state));

                let buttons = if let Some(coordinate) = current_name.next_coordinate(desired_name) {
                    // name is incomplete
                    if let Some(direction) = navigate(state, coordinate) {
                        state.tap(direction)
                    } else {
                        // select current letter
                        state.tap(JoypadBtnState::A)
                    }
                } else {
                    // name is complete: check if we're at last character
                    // (if the last character is 0xFF, we might not be there yet)
                    let last_character: char =
                        desired_name.chars().last().expect("no last character?");
                    let map = HashMap::from(CHARACTER_MAP);
                    let (last_coord, _): (Coordinate, Value) =
                        *map.get(&last_character).expect("failed to lookup name");

                    if let Some(direction) = navigate(state, NameCoordinate(last_coord)) {
                        // navigate to last character
                        state.tap(direction)
                    } else {
                        // tap A to continue
                        state.tap(JoypadBtnState::A)
                    }
                };

                dbg!(&buttons);

                HandlerOutput {
                    progress: GameProgress::CharacterCreation(CharacterCreationState {
                        party_complete: complete,
                        mode,
                        current_class,
                        slot: Some(slot),
                        ..previous_state
                    }),
                    buttons,
                }
            }
            Some(Mode::SelectClass) => HandlerOutput {
                progress: GameProgress::CharacterCreation(CharacterCreationState {
                    party_complete: complete,
                    mode,
                    current_class,
                    slot: Some(slot),
                    ..previous_state
                }),
                buttons: if current_class.is_some_and(|class| class == *desired_class) {
                    JoypadBtnState::A
                } else {
                    state.tap(JoypadBtnState::RIGHT)
                },
            },
            None => HandlerOutput {
                progress: GameProgress::Pause,
                ..
            },
        }
    } else {
        panic!("lost")
    }
}
