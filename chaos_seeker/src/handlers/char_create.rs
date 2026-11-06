use crate::state::{CharacterCreationState, GameProgress, GameState};
use std::collections::HashMap;
use tetanes_core::{input::JoypadBtnState, mem::Read};

pub const SLOT_SELECTED_ADDRESS: u16 = 0x67;
pub const CLASS_SELECTED_ADDRESS: u16 = 0x300;

#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug)]
pub struct NameCoordinate(u16);

impl NameCoordinate {
    const ADDRESS: u16 = 0x40;
    const SIZE: usize = 2;

    fn parse(state: &mut GameState) -> Self {
        let bytes: [u8; 2] = [state.read(Self::ADDRESS), state.read(Self::ADDRESS + 1)];
        NameCoordinate(u16::from_le_bytes(bytes))
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
    let complete: bool = party_complete(state);
    let mode = Mode::current(state);

    if let GameProgress::CharacterCreation(previous_state) = state.progress {
        if complete {
            state.toggle(JoypadBtnState::START);
            dbg!(NameCoordinate::parse(state));

            return GameProgress::CharacterCreation(CharacterCreationState {
                party_complete: complete,
                mode,
                ..previous_state
            });
        }

        match mode {
            Some(Mode::SelectName) => {
                // TODO: review accumulated name bytes (how?)
                // TODO: decide if name completed
                // TODO: retrieve current position + current character
                // TODO: decide if movement needed
                // TODO: decide direction if movement is needed
                // TODO: hit direction if movement is needed
                // TODO: hit A if character is correct
                GameProgress::CharacterCreation(CharacterCreationState {
                    party_complete: complete,
                    mode,
                    ..previous_state
                })
            }
            Some(Mode::SelectClass) => {
                let slot = current_slot(state);
                let (desired_class, _desired_name) = STARTING_PARTY
                    .get(slot.expect("failed to retrieve slot") as usize)
                    .expect("slot out of bounds");
                let current_class = StartingClass::current_class(state);

                if current_class.is_some_and(|class| class == *desired_class) {
                    state.toggle(JoypadBtnState::A);
                } else {
                    state.toggle(JoypadBtnState::B);
                }

                GameProgress::CharacterCreation(CharacterCreationState {
                    party_complete: complete,
                    mode,
                    current_class,
                    slot,
                    ..previous_state
                })
            }
            None => panic!("invalid character creation mode"),
        }
    } else {
        panic!("lost")
    }
}
